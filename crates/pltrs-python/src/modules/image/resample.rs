//! `_image.resample`: Agg's resampler on arrays, as matplotlib's native
//! `_image` exposes it (`pltrs::image::resample_pixels`).
//!
//! Python checks the arrays as matplotlib's wrapper does, computes the
//! lookup mesh of a non-affine transform through the transform object,
//! and writes the result back into the output array; the arrays cross as
//! `(M, N)` scalars or `(M, N, 4)` RGBA pixels, of bytes or floats.

use pltrs::image::{Pixel, PixelMapping, ResampleOptions, resample_pixels};
use pltrs::numeric::Grid;

use super::super::transforms::affine;
use super::parse;
use crate::binding::Element;
use crate::binding::prelude::*;

/// The map from source to output pixels: `("affine", 3×3 matrix)`, or
/// `("mesh", (M, N, 2) array)` holding the source position of every
/// output pixel centre.
struct Mapping(PixelMapping);

impl FromPy<'_> for Mapping {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (kind, values): (String, NdArray) = object.extract()?;
        Ok(Self(match (kind.as_str(), &values.shape[..]) {
            ("affine", _) => PixelMapping::Affine(affine(&values)?),
            ("mesh", &[rows, cols, 2]) => {
                PixelMapping::Mesh(grid((rows, cols), values.data.as_chunks::<2>().0.to_vec())?)
            }
            _ => {
                return Err(PyErr::value_error(format!(
                    "no resampling map {kind:?} of shape {:?}",
                    values.shape
                )));
            }
        }))
    }
}

/// The keyword arguments of `_image.resample`: `(interpolation name,
/// resample, alpha, norm, radius)`.
struct Settings(ResampleOptions);

impl FromPy<'_> for Settings {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (interpolation, resample, alpha, norm, radius): (String, bool, f64, bool, f64) =
            object.extract()?;
        Ok(Self(ResampleOptions {
            interpolation: parse(&interpolation)?,
            resample,
            alpha,
            norm,
            radius,
        }))
    }
}

/// `cells` as a grid of `(rows, cols)`.
fn grid<P>((rows, cols): (usize, usize), cells: Vec<P>) -> PyResult<Grid<P>> {
    Grid::from_vec(rows, cols, cells).map_err(|err| PyErr::value_error(err.to_string()))
}

/// The `(rows, cols)` of an image array.
fn plane<T: Element>(array: &NdArray<T>) -> PyResult<(usize, usize)> {
    match array.shape[..] {
        [rows, cols] | [rows, cols, 4] => Ok((rows, cols)),
        _ => Err(PyErr::value_error(format!(
            "an image array is (M, N) or (M, N, 4), not {:?}",
            array.shape
        ))),
    }
}

/// The RGBA pixels of the channels `values`, four by four.
fn quads<T: Copy>(values: &[T]) -> Vec<[T; 4]> {
    values.as_chunks::<4>().0.to_vec()
}

/// Single-precision values, which numpy widened, narrowed back exactly.
#[expect(
    clippy::cast_possible_truncation,
    reason = "single-precision data, widened by numpy, narrows back exactly"
)]
fn narrow(values: &[f64]) -> Vec<f32> {
    values.iter().map(|&v| v as f32).collect()
}

/// The pixels of a grid of `shape` resampled into a copy of `out`'s.
fn resample_cells<P: Pixel>(
    (shape, cells): ((usize, usize), Vec<P>),
    (out_shape, out): ((usize, usize), Vec<P>),
    mapping: &Mapping,
    settings: &Settings,
) -> PyResult<Vec<P>> {
    let source = grid(shape, cells)?;
    let mut target = grid(out_shape, out)?;
    resample_pixels(&source, &mut target, &mapping.0, settings.0)?;
    Ok(target.into_vec())
}

/// `data` resampled into a copy of `out`, both scalars (2-D) or RGBA
/// pixels (3-D) of bytes.
fn resample_bytes(
    data: NdArray<u8>,
    out: NdArray<u8>,
    mapping: Mapping,
    settings: Settings,
) -> PyResult<NdArray<u8>> {
    let (shape, out_shape) = (plane(&data)?, plane(&out)?);
    let cells = if data.shape.len() == 2 {
        resample_cells(
            (shape, data.data),
            (out_shape, out.data),
            &mapping,
            &settings,
        )?
    } else {
        let source = (shape, quads(&data.data));
        resample_cells(source, (out_shape, quads(&out.data)), &mapping, &settings)?.concat()
    };
    NdArray::with_shape(cells, out.shape)
}

/// `data` resampled into a copy of `out`, both scalars (2-D) or RGBA
/// pixels (3-D) of floats, in single precision when `single`.
fn resample_floats(
    data: NdArray,
    out: NdArray,
    single: bool,
    mapping: Mapping,
    settings: Settings,
) -> PyResult<NdArray> {
    let (shape, out_shape) = (plane(&data)?, plane(&out)?);
    let rgba = data.shape.len() == 3;
    let widen = |cells: Vec<f32>| cells.into_iter().map(f64::from).collect::<Vec<_>>();
    let cells = match (single, rgba) {
        (false, false) => resample_cells(
            (shape, data.data),
            (out_shape, out.data),
            &mapping,
            &settings,
        )?,
        (false, true) => {
            let source = (shape, quads(&data.data));
            resample_cells(source, (out_shape, quads(&out.data)), &mapping, &settings)?.concat()
        }
        (true, false) => widen(resample_cells(
            (shape, narrow(&data.data)),
            (out_shape, narrow(&out.data)),
            &mapping,
            &settings,
        )?),
        (true, true) => widen(
            resample_cells(
                (shape, quads(&narrow(&data.data))),
                (out_shape, quads(&narrow(&out.data))),
                &mapping,
                &settings,
            )?
            .concat(),
        ),
    };
    NdArray::with_shape(cells, out.shape)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("resample_bytes", resample_bytes)?;
    module.add("resample_floats", resample_floats)
}

#[cfg(test)]
mod tests {
    use pltrs::image::Interpolation;
    use pltrs::transforms::Affine2D;

    use super::*;

    fn stretch(sx: f64) -> Mapping {
        Mapping(PixelMapping::Affine(Affine2D::scaling(sx, 1.0)))
    }

    fn nearest() -> Settings {
        Settings(ResampleOptions {
            interpolation: Interpolation::Nearest,
            ..ResampleOptions::default()
        })
    }

    #[test]
    fn floats_resample_in_their_precision_and_shape() {
        let data = NdArray::with_shape(vec![0.1, 0.3, 0.2], vec![1, 3]).unwrap();
        let out = NdArray::with_shape(vec![0.0; 10], vec![1, 10]).unwrap();
        let wide = stretch(10.0 / 3.0);
        let double = resample_floats(data.clone(), out.clone(), false, wide, nearest()).unwrap();
        assert_eq!(double.shape, [1, 10]);
        assert_eq!(double.data[..4], [0.1, 0.1, 0.1, 0.3]);
        let single = resample_floats(data, out, true, stretch(10.0 / 3.0), nearest()).unwrap();
        assert_eq!(
            single.data[0],
            f64::from(0.1_f32),
            "computed in single precision"
        );
    }

    #[test]
    fn rgba_pixels_resample_whole() {
        let data = NdArray::with_shape(vec![1, 2, 3, 255, 4, 5, 6, 255], vec![1, 2, 4]).unwrap();
        let out = NdArray::with_shape(vec![0; 16], vec![1, 4, 4]).unwrap();
        let wide = resample_bytes(data, out, stretch(2.0), nearest()).unwrap();
        assert_eq!(wide.shape, [1, 4, 4]);
        assert_eq!(wide.data[..8], [1, 2, 3, 255, 1, 2, 3, 255]);
        assert_eq!(wide.data[12..], [4, 5, 6, 255]);
    }

    #[test]
    fn arrays_of_other_shapes_are_refused() {
        let cube = NdArray::with_shape(vec![0.0; 8], vec![2, 2, 2]).unwrap();
        assert!(resample_floats(cube.clone(), cube, false, stretch(1.0), nearest()).is_err());
    }
}
