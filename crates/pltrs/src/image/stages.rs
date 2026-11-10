//! The two halves of `_make_image`: resampling the data and then mapping
//! it to colours (*data* stage), or mapping to colours and resampling the
//! premultiplied colours (*RGBA* stage).
//!
//! The arithmetic follows matplotlib's numpy dtypes: masks resample in
//! single precision, 8-bit colours become `float32`, `float32` colours stay
//! so, and mixing with `float64` arrays widens, so that the 8-bit output
//! matches bit for bit.

use std::ops::{Div, Mul};

use pltrs_raster::resample::{Pixel, Source};

use super::normalize::{linear_rows, normalize_in};
use super::resample::{PixelMap, Resampling};
use super::{ImageAlpha, ImageData, ImageError, Precision, ScalarField};
use crate::colors::ScalarMappable;
use crate::numeric::Grid;

/// The data stage for scalar images: resample the values, the mask and a
/// per-pixel alpha, then normalise and colour-map the result.
pub(crate) fn data_stage(
    field: &ScalarField,
    alpha: Option<&ImageAlpha>,
    mappable: &mut ScalarMappable,
    resampling: &Resampling,
    out_shape: (usize, usize),
    map: &PixelMap,
) -> Result<Grid<[u8; 4]>, ImageError> {
    let values = field.values();
    if let Some(bits) = field.narrowed_from() {
        crate::warnings::warn(&format!(
            "Casting input data from float{bits} to float64 for imshow."
        ));
    }
    let precision = if field.resamples_in_f32() {
        Precision::Single
    } else {
        Precision::Double
    };
    let resampled: Vec<f64> = if precision == Precision::Single {
        let single = values.map(|&v| v as f32);
        resampling
            .run(&single, out_shape, map, None)?
            .into_vec()
            .into_iter()
            .map(f64::from)
            .collect()
    } else {
        resampling.run(values, out_shape, map, None)?.into_vec()
    };
    let mask = field
        .mask()
        .map(|&masked| if masked { f32::NAN } else { 1.0 });
    let mut coverage = resampling
        .run(&mask, out_shape, map, Some(true))?
        .into_vec();
    let out_mask: Vec<bool> = coverage.iter().map(|v| v.is_nan()).collect();
    for (value, &masked) in coverage.iter_mut().zip(&out_mask) {
        if masked {
            *value = 1.0;
        }
    }
    if let Some(ImageAlpha::Each(grid)) = alpha {
        check_shape("alpha", values.shape(), grid.shape())?;
        let resampled_alpha = resampling.run(grid, out_shape, map, Some(true))?;
        for (value, &a) in coverage.iter_mut().zip(resampled_alpha.as_slice()) {
            *value = (f64::from(*value) * a) as f32;
        }
    }
    let masked: Vec<Option<f64>> = resampled
        .into_iter()
        .zip(&out_mask)
        .map(|(v, &masked)| (!masked).then_some(v))
        .collect();
    // matplotlib normalises the resampled data as a masked array of the
    // output's shape.
    let normalized = normalize_in(mappable, &masked, field.kind(), precision, out_shape)?;
    let cmap = mappable.cmap();
    // A Python float multiplies a float32 array in single precision.
    let scalar_alpha = alpha.map_or(1.0, ImageAlpha::scalar) as f32;
    let pixels = normalized
        .into_iter()
        .zip(coverage)
        .map(|(value, cover)| {
            let [r, g, b, a] = cmap.lookup_bytes(value, None);
            [r, g, b, to_byte(f32::from(a) * cover * scalar_alpha)]
        })
        .collect();
    Ok(Grid::from_vec(out_shape.0, out_shape.1, pixels).expect("one pixel per output cell"))
}

/// The RGBA stage: colours (mapped scalars or explicit RGB(A)) with the
/// image alpha folded in are premultiplied, resampled and demultiplied.
pub(crate) fn rgba_stage(
    data: &ImageData,
    alpha: Option<&ImageAlpha>,
    mappable: &mut ScalarMappable,
    resampling: &Resampling,
    out_shape: (usize, usize),
    map: &PixelMap,
) -> Result<Grid<[u8; 4]>, ImageError> {
    let shape = data.shape();
    let uniform = |alpha: Option<&ImageAlpha>| match alpha {
        Some(ImageAlpha::Uniform(a)) => Some(*a),
        _ => None,
    };
    let grid_alpha = match alpha {
        Some(ImageAlpha::Each(grid)) => {
            check_shape("alpha", shape, grid.shape())?;
            Some(grid)
        }
        _ => None,
    };
    let colors = match data {
        ImageData::Scalar(field) => Colors::Mapped(mapped_colors(field, mappable)?),
        ImageData::Rgb(grid) => return finish_rgb(grid, alpha, resampling, out_shape, map),
        ImageData::RgbF32(grid) => return finish_rgb(grid, alpha, resampling, out_shape, map),
        ImageData::RgbBytes(grid) => {
            let single = grid.map(|p| p.map(unit_f32));
            return finish_rgb(&single, alpha, resampling, out_shape, map);
        }
        ImageData::Rgba(grid) => Colors::F64(grid.clone()),
        ImageData::RgbaF32(grid) => Colors::F32(grid.clone()),
        ImageData::RgbaBytes(grid) => Colors::F32(grid.map(|p| p.map(unit_f32))),
    };
    match (colors, grid_alpha) {
        (colors, Some(grid)) => {
            let rgba = colors.widened();
            let scaled = Grid::from_fn(shape.0, shape.1, |r, c| {
                let [red, green, blue, a] = rgba[(r, c)];
                [red, green, blue, a * grid[(r, c)]]
            });
            finish(&scaled, None, resampling, out_shape, map)
        }
        (Colors::F32(rgba), None) => finish(&rgba, uniform(alpha), resampling, out_shape, map),
        (Colors::F64(rgba), None) => finish(&rgba, uniform(alpha), resampling, out_shape, map),
        (Colors::Mapped(mapped), None) => {
            finish_mapped(&mapped, uniform(alpha), resampling, out_shape, map)
        }
    }
}

/// Values normalised and looked up at a time by [`mapped_colors`]: few
/// enough for the intermediate arrays to stay in the cache.
const COLOR_BLOCK: usize = 4096;

/// The colours of scalar data (matplotlib's `to_rgba` of the whole
/// array), as rows of the colormap's table.
#[expect(
    clippy::float_cmp,
    reason = "colours are premultiplied unless every alpha is exactly 1"
)]
fn mapped_colors(field: &ScalarField, mappable: &mut ScalarMappable) -> Result<Mapped, ImageError> {
    let (values, mask) = (field.values().as_slice(), field.mask().as_slice());
    let rows = match linear_rows(mappable, values, mask, field.precision()) {
        Some(rows) => rows,
        None => blockwise_rows(field, mappable)?,
    };
    let palette: Vec<[f64; 4]> = mappable
        .cmap()
        .table()
        .iter()
        .map(|c| c.to_array())
        .collect();
    let opaque = rows.iter().all(|&row| palette[row as usize][3] == 1.0);
    let (height, width) = field.values().shape();
    Ok(Mapped {
        palette,
        rows,
        width,
        height,
        opaque,
    })
}

/// The colormap rows of scalar data under any norm, normalised and looked
/// up a block at a time so that the intermediate arrays stay small. An
/// unscaled norm autoscales to the first values it normalises, and a norm
/// of the program's sees the array matplotlib hands it, so they get all of
/// them at once.
fn blockwise_rows(
    field: &ScalarField,
    mappable: &mut ScalarMappable,
) -> Result<Vec<u32>, ImageError> {
    let (values, mask) = (field.values().as_slice(), field.mask().as_slice());
    let norm = mappable.norm();
    let whole = !norm.scaled() || norm.calls_program();
    let block = if whole {
        values.len().max(1)
    } else {
        COLOR_BLOCK
    };
    let shape = field.values().shape();
    let mut rows = Vec::with_capacity(values.len());
    let mut masked = Vec::with_capacity(block.min(values.len()));
    for (values, mask) in values.chunks(block).zip(mask.chunks(block)) {
        masked.clear();
        masked.extend(
            values
                .iter()
                .zip(mask)
                .map(|(&v, &masked)| (!masked).then_some(v)),
        );
        // Only a norm of the program's, which gets the whole array, sees
        // the shape.
        let shape = if whole { shape } else { (1, masked.len()) };
        let normalized = normalize_in(mappable, &masked, field.kind(), field.precision(), shape)?;
        let cmap = mappable.cmap();
        rows.extend(normalized.into_iter().map(|n| cmap.lut_index(n) as u32));
    }
    Ok(rows)
}

/// Colour-mapped data: each element's row of the colormap's table, which
/// holds every colour once, so that the colours of a large image take a
/// few bytes each until the resampler reads them.
#[derive(Debug)]
struct Mapped {
    /// The colormap's table.
    palette: Vec<[f64; 4]>,
    /// Each element's row of `palette`, row-major.
    rows: Vec<u32>,
    width: usize,
    height: usize,
    /// Whether every element's alpha is 1.
    opaque: bool,
}

/// The colours of [`Mapped`] data from a palette, looked up as the
/// resampler reads them.
struct PaletteSource<'a> {
    palette: &'a [[f64; 4]],
    mapped: &'a Mapped,
}

impl Source<[f64; 4]> for PaletteSource<'_> {
    fn width(&self) -> usize {
        self.mapped.width
    }

    fn height(&self) -> usize {
        self.mapped.height
    }

    #[inline]
    fn pixel(&self, index: usize) -> [f64; 4] {
        self.palette[self.mapped.rows[index] as usize]
    }
}

/// RGBA colours in the precision numpy would hold them.
enum Colors {
    F32(Grid<[f32; 4]>),
    F64(Grid<[f64; 4]>),
    Mapped(Mapped),
}

impl Colors {
    /// The colours as `float64` (numpy's result of mixing with `float64`).
    fn widened(self) -> Grid<[f64; 4]> {
        match self {
            Self::F32(grid) => grid.map(|p| p.map(f64::from)),
            Self::F64(grid) => grid,
            Self::Mapped(mapped) => {
                let colors = mapped
                    .rows
                    .iter()
                    .map(|&row| mapped.palette[row as usize])
                    .collect();
                Grid::from_vec(mapped.height, mapped.width, colors)
                    .expect("one colour per data element")
            }
        }
    }
}

/// RGB colours get their alpha channel: ones (as `float64`), the
/// per-pixel alpha, or the uniform alpha in the colours' precision.
fn finish_rgb<T: Real>(
    rgb: &Grid<[T; 3]>,
    alpha: Option<&ImageAlpha>,
    resampling: &Resampling,
    out_shape: (usize, usize),
    map: &PixelMap,
) -> Result<Grid<[u8; 4]>, ImageError>
where
    [T; 4]: Pixel,
{
    let widen = |[r, g, b]: [T; 3], a: f64| [r.to_f64(), g.to_f64(), b.to_f64(), a];
    match alpha {
        None => {
            let rgba = rgb.map(|&p| widen(p, 1.0));
            finish::<f64>(&rgba, None, resampling, out_shape, map)
        }
        Some(ImageAlpha::Each(grid)) => {
            check_shape("alpha", rgb.shape(), grid.shape())?;
            let rgba = Grid::from_fn(rgb.rows(), rgb.cols(), |r, c| {
                widen(rgb[(r, c)], grid[(r, c)])
            });
            finish::<f64>(&rgba, None, resampling, out_shape, map)
        }
        Some(ImageAlpha::Uniform(a)) => {
            // np.full(..., alpha, np.float32), stacked in the colours' dtype.
            let a = T::from_f64(f64::from(*a as f32));
            let rgba = rgb.map(|&[r, g, b]| [r, g, b, a]);
            finish(&rgba, None, resampling, out_shape, map)
        }
    }
}

/// Premultiplies (when any alpha differs from 1), resamples, demultiplies,
/// applies a uniform alpha to RGBA input and converts to bytes.
fn finish<T: Real>(
    rgba: &Grid<[T; 4]>,
    post_alpha: Option<f64>,
    resampling: &Resampling,
    out_shape: (usize, usize),
    map: &PixelMap,
) -> Result<Grid<[u8; 4]>, ImageError>
where
    [T; 4]: Pixel,
{
    let premultiplied;
    let source = if rgba.as_slice().iter().any(|p| p[3] != T::ONE) {
        premultiplied = rgba.map(|&p| premultiply(p));
        &premultiplied
    } else {
        rgba
    };
    let resampled = resampling.run(source, out_shape, map, None)?;
    Ok(to_bytes(&resampled, post_alpha))
}

/// [`finish`] for colour-mapped data, whose palette is premultiplied
/// rather than every element.
fn finish_mapped(
    mapped: &Mapped,
    post_alpha: Option<f64>,
    resampling: &Resampling,
    out_shape: (usize, usize),
    map: &PixelMap,
) -> Result<Grid<[u8; 4]>, ImageError> {
    let premultiplied: Vec<[f64; 4]>;
    let palette = if mapped.opaque {
        &mapped.palette
    } else {
        premultiplied = mapped.palette.iter().map(|&p| premultiply(p)).collect();
        &premultiplied
    };
    let source = PaletteSource { palette, mapped };
    let resampled = resampling.run_source(&source, out_shape, map, None)?;
    Ok(to_bytes(&resampled, post_alpha))
}

/// A colour with its alpha multiplied in.
fn premultiply<T: Real>([r, g, b, a]: [T; 4]) -> [T; 4] {
    [r * a, g * a, b * a, a]
}

/// Resampled premultiplied colours demultiplied, with a uniform alpha
/// applied, as bytes.
fn to_bytes<T: Real>(resampled: &Grid<[T; 4]>, post_alpha: Option<f64>) -> Grid<[u8; 4]> {
    let post = post_alpha.map(T::from_f64);
    resampled.map(|&[r, g, b, a]| {
        let divisor = if a == T::ZERO { T::ONE } else { a };
        let a = post.map_or(a, |post| a * post);
        let pixel = [r / divisor, g / divisor, b / divisor, a];
        if pixel.iter().any(|v| v.is_nan()) {
            [0; 4]
        } else {
            pixel.map(|v| to_byte(v * T::from_f64(255.0)))
        }
    })
}

/// numpy's `np.divide(A, 0xff, dtype=np.float32)` for one channel.
fn unit_f32(v: u8) -> f32 {
    f32::from(v) / 255.0
}

/// numpy's float → `uint8` cast as compiled for x86-64: truncation to a
/// 32-bit integer, keeping the low byte.
fn to_byte<T: Real>(v: T) -> u8 {
    v.to_f64() as i32 as u8
}

fn check_shape(
    what: &'static str,
    expected: (usize, usize),
    actual: (usize, usize),
) -> Result<(), ImageError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ImageError::ShapeMismatch {
            what,
            expected,
            actual,
        })
    }
}

/// The float types numpy uses for image colours (`float32`, `float64`).
trait Real: Copy + PartialEq + Mul<Output = Self> + Div<Output = Self> {
    const ZERO: Self;
    const ONE: Self;
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
    fn is_nan(self) -> bool;
}

impl Real for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn from_f64(v: f64) -> Self {
        v as f32
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    fn is_nan(self) -> bool {
        self.is_nan()
    }
}

impl Real for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;

    fn from_f64(v: f64) -> Self {
        v
    }

    fn to_f64(self) -> f64 {
        self
    }

    fn is_nan(self) -> bool {
        self.is_nan()
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Affine2D;

    use super::*;
    use crate::colors::{Normalize, colormaps};
    use crate::image::{Interpolation, Origin};

    fn resampling(interpolation: Interpolation) -> Resampling {
        Resampling {
            interpolation,
            resample: true,
            filternorm: true,
            filterrad: 4.0,
            origin: Origin::Lower,
        }
    }

    fn gray() -> ScalarMappable {
        ScalarMappable::with_norm(
            colormaps::get("gray").unwrap(),
            Box::new(Normalize::new(0.0, 1.0)),
        )
    }

    fn identity() -> PixelMap {
        PixelMap::Affine(Affine2D::identity())
    }

    #[test]
    fn data_stage_masks_and_scales_alpha() {
        let values = Grid::from_vec(1, 3, vec![0.0, f64::NAN, 1.0]).unwrap();
        let field = ScalarField::new(values);
        let out = data_stage(
            &field,
            Some(&ImageAlpha::Uniform(0.5)),
            &mut gray(),
            &resampling(Interpolation::Nearest),
            (1, 3),
            &identity(),
        )
        .unwrap();
        assert_eq!(
            out.as_slice(),
            &[[0, 0, 0, 127], [0, 0, 0, 0], [255, 255, 255, 127]]
        );
    }

    #[test]
    fn data_stage_outside_the_image_is_transparent() {
        let field = ScalarField::new(Grid::filled(1, 1, 1.0));
        let out = data_stage(
            &field,
            None,
            &mut gray(),
            &resampling(Interpolation::Bilinear),
            (1, 3),
            &PixelMap::Affine(Affine2D::scaling(2.0, 1.0)),
        )
        .unwrap();
        assert_eq!(out.as_slice()[2][3], 0);
        assert_eq!(out.as_slice()[0], [255, 255, 255, 255]);
    }

    #[test]
    fn rgba_stage_premultiplies_and_demultiplies() {
        let rgba = Grid::from_vec(1, 2, vec![[1.0, 0.0, 0.0, 0.5], [0.0, 0.0, 1.0, 1.0]]).unwrap();
        let out = rgba_stage(
            &ImageData::Rgba(rgba),
            None,
            &mut gray(),
            &resampling(Interpolation::Nearest),
            (1, 2),
            &identity(),
        )
        .unwrap();
        assert_eq!(out.as_slice(), &[[255, 0, 0, 127], [0, 0, 255, 255]]);
    }

    #[test]
    fn rgba_stage_alpha_variants() {
        let rgb = Grid::from_vec(1, 1, vec![[255_u8, 128, 0]]).unwrap();
        let data = ImageData::RgbBytes(rgb);
        let run = |alpha: Option<ImageAlpha>| {
            rgba_stage(
                &data,
                alpha.as_ref(),
                &mut gray(),
                &resampling(Interpolation::Nearest),
                (1, 1),
                &identity(),
            )
            .unwrap()
            .as_slice()[0]
        };
        assert_eq!(run(None), [255, 128, 0, 255]);
        assert_eq!(run(Some(ImageAlpha::Uniform(0.6))), [255, 128, 0, 153]);
        assert_eq!(
            run(Some(ImageAlpha::Each(Grid::filled(1, 1, 0.5)))),
            [255, 128, 0, 127]
        );
        let wrong = rgba_stage(
            &data,
            Some(&ImageAlpha::Each(Grid::filled(2, 2, 0.5))),
            &mut gray(),
            &resampling(Interpolation::Nearest),
            (1, 1),
            &identity(),
        );
        assert!(matches!(wrong, Err(ImageError::ShapeMismatch { .. })));
    }

    #[test]
    fn single_precision_colours_stay_single() {
        // 3/255 premultiplied by an alpha of 11/255 and demultiplied comes
        // back just below 3/255 in float32 (byte 2), exactly in float64.
        let (level, alpha) = (3.0_f32 / 255.0, 11.0_f32 / 255.0);
        let run = |data: ImageData| {
            rgba_stage(
                &data,
                None,
                &mut gray(),
                &resampling(Interpolation::Nearest),
                (1, 1),
                &identity(),
            )
            .unwrap()
            .as_slice()[0]
        };
        let single = Grid::from_vec(1, 1, vec![[level, 0.0, 0.0, alpha]]).unwrap();
        let double = single.map(|p| p.map(f64::from));
        assert_eq!(run(ImageData::RgbaF32(single))[0], 2);
        assert_eq!(run(ImageData::Rgba(double))[0], 3);
        let rgb = Grid::from_vec(1, 1, vec![[level, 1.0, 0.0]]).unwrap();
        assert_eq!(
            run(ImageData::RgbF32(rgb)),
            [3, 255, 0, 255],
            "widened with ones"
        );
    }

    #[test]
    fn nan_colours_become_transparent() {
        let rgba = Grid::from_vec(1, 1, vec![[f64::NAN, 0.0, 0.0, 1.0]]).unwrap();
        let out = rgba_stage(
            &ImageData::Rgba(rgba),
            Some(&ImageAlpha::Uniform(0.5)),
            &mut gray(),
            &resampling(Interpolation::Nearest),
            (1, 1),
            &identity(),
        )
        .unwrap();
        assert_eq!(out.as_slice()[0], [0; 4]);
    }

    #[test]
    fn byte_conversion_wraps_like_numpy_on_x86() {
        assert_eq!(to_byte(254.9_f32), 254);
        assert_eq!(to_byte(267.0_f64), 11);
        assert_eq!(to_byte(f64::NAN), 0);
    }
}
