//! Image resampling: a port of matplotlib's `_image.resample`.
//!
//! [`resample`] maps a source [`Grid`] through a [`PixelTransform`] onto a
//! destination grid, blending into whatever the destination already holds,
//! with Agg's interpolation kernels ([`Interpolation`]). It reproduces
//! `_image_resample.h` step by step:
//!
//! 1. a pure translation (unit scale, no shear) always samples
//!    nearest-neighbour;
//! 2. the destination area covered by the source is rasterized with Agg's
//!    anti-aliased scanline rasterizer (edges within half a pixel of a
//!    pixel boundary are pushed out to it for axis-aligned transforms);
//! 3. every covered run of destination pixels is coloured by one of Agg's
//!    span generators — nearest-neighbour, the kernel at its natural size,
//!    or (for affine transforms with `resample`) the kernel stretched over
//!    the source footprint — scaled by `alpha` and blended with the run's
//!    coverage.
//!
//! Pixels are single-channel data (`f32`, `f64`, `u8`) or RGBA (`[u8; 4]`,
//! `[f32; 4]`, `[f64; 4]`); see [`Pixel`].
//!
//! ```
//! use plt_core::Affine2D;
//! use plt_raster::resample::{Grid, Interpolation, ResampleParams, resample};
//! let src = Grid::from_vec(2, 1, vec![0.0_f64, 1.0])?;
//! let mut dst = Grid::filled(4, 1, 0.0_f64)?;
//! let params = ResampleParams::new(Affine2D::scaling(2.0, 1.0))
//!     .with_interpolation(Interpolation::Bilinear);
//! resample(&src, &mut dst, &params)?;
//! assert_eq!(dst.as_slice()[0], 0.0);
//! assert_eq!(dst.as_slice()[3], 1.0);
//! # Ok::<(), plt_raster::resample::ResampleError>(())
//! ```

mod accessor;
mod error;
mod filters;
mod grid;
mod interpolator;
mod lut;
mod numeric;
mod params;
mod pixel;
mod span;

pub use error::ResampleError;
pub use grid::{Grid, GridRef};
pub use params::{Interpolation, PixelTransform, ResampleParams};
pub use pixel::Pixel;

use plt_core::Point;

use self::filters::FilterShape;
use self::interpolator::{
    LinearInterpolator, MeshAlignment, MeshInterpolator, NearestInterpolator,
};
use self::lut::WeightLut;
use self::numeric::{AggAffine, iround};
use self::span::Footprint;
use crate::geometry::PathSink;
use crate::raster::{Rasterizer, Scanline};

/// Exclusive upper bound on each side of a source or destination grid:
/// Agg's sub-pixel coordinates must fit in 24-bit signed integers.
pub const MAX_DIMENSION: usize = 1 << 23;

/// Largest accepted kernel radius ([`ResampleParams::radius`]).
pub const MAX_FILTER_RADIUS: f64 = 256.0;

/// Resamples `src` into `dst` through `params.transform`, blending the
/// result into the existing destination pixels.
///
/// # Errors
///
/// When the source is empty, a grid side reaches [`MAX_DIMENSION`], the
/// affine transform is not finite and invertible, a lookup mesh does not
/// match the destination, or `alpha` / `radius` are out of range.
pub fn resample<'a, P: Pixel>(
    src: impl Into<GridRef<'a, P>>,
    dst: &mut Grid<P>,
    params: &ResampleParams,
) -> Result<(), ResampleError> {
    let src = src.into();
    validate(src, dst, params)?;
    if dst.is_empty() {
        return Ok(());
    }
    let affine = match &params.transform {
        PixelTransform::Affine(affine) => Some(AggAffine::from_affine(affine)),
        PixelTransform::Mesh(_) => None,
    };
    let mut interpolation = params.interpolation;
    if affine.is_some_and(|a| a.is_unit_scale_translation()) {
        interpolation = Interpolation::Nearest;
    }
    let inverted = match affine {
        Some(a) => {
            let inverted = a.inverted();
            if !a.is_finite() || !inverted.is_finite() {
                return Err(ResampleError::SingularTransform);
            }
            inverted
        }
        None => AggAffine::IDENTITY,
    };
    let mesh = match &params.transform {
        PixelTransform::Mesh(mesh) => Some(mesh),
        PixelTransform::Affine(_) => None,
    };

    let mut rasterizer = coverage(src, dst, affine, interpolation);
    let mut generator = match FilterShape::for_interpolation(interpolation, params.radius) {
        None => match mesh {
            None => Generator::NearestAffine(NearestInterpolator::new(inverted)),
            Some(mesh) => Generator::NearestMesh(MeshInterpolator::new(
                inverted,
                Some(mesh),
                MeshAlignment::Edge,
            )),
        },
        Some(shape) => {
            let lut = WeightLut::new(&shape, params.norm);
            if affine.is_some() && params.resample {
                let interpolator = LinearInterpolator::new(inverted);
                let footprint = Footprint::new(&interpolator);
                Generator::Resample {
                    lut,
                    footprint,
                    interpolator,
                }
            } else {
                Generator::Filter {
                    lut,
                    interpolator: MeshInterpolator::new(inverted, mesh, MeshAlignment::Centre),
                }
            }
        }
    };

    let mut scanline = Scanline::new();
    let mut colors: Vec<P::Color> = Vec::new();
    let mut covers: Vec<u8> = Vec::new();
    rasterizer
        .sweep(&mut scanline, |sl| {
            for (x, run_covers) in unpacked_runs(sl, &mut covers) {
                colors.clear();
                colors.resize(run_covers.len(), P::Color::default());
                generator.generate(src, x, sl.y(), &mut colors);
                #[expect(clippy::float_cmp, reason = "Agg skips the scaling for exactly 1")]
                let scaled = params.alpha != 1.0;
                if scaled {
                    for color in &mut colors {
                        P::scale_alpha(color, params.alpha);
                    }
                }
                blend_run(dst, x, sl.y(), &colors, run_covers);
            }
        })
        .map_err(ResampleError::from)
}

/// Checks the arguments the way matplotlib's wrapper and Agg would fail.
fn validate<P>(
    src: GridRef<'_, P>,
    dst: &Grid<P>,
    params: &ResampleParams,
) -> Result<(), ResampleError> {
    if src.is_empty() {
        return Err(ResampleError::EmptySource);
    }
    for grid in [(src.width(), src.height()), (dst.width(), dst.height())] {
        if grid.0 >= MAX_DIMENSION || grid.1 >= MAX_DIMENSION {
            return Err(ResampleError::TooLarge {
                width: grid.0,
                height: grid.1,
            });
        }
    }
    if let PixelTransform::Mesh(mesh) = &params.transform
        && (mesh.width(), mesh.height()) != (dst.width(), dst.height())
    {
        return Err(ResampleError::MeshSize {
            mesh: (mesh.width(), mesh.height()),
            destination: (dst.width(), dst.height()),
        });
    }
    if !(0.0..=1.0).contains(&params.alpha) {
        return Err(ResampleError::InvalidAlpha(params.alpha));
    }
    if !(params.radius > 0.0 && params.radius <= MAX_FILTER_RADIUS) {
        return Err(ResampleError::InvalidRadius(params.radius));
    }
    Ok(())
}

/// The destination area to colour, as a rasterizer loaded with Agg's
/// coverage polygon: the transformed source outline for filtered affine
/// resampling (axis-aligned edges bumped out to whole pixels when within
/// half a pixel), otherwise the whole destination.
fn coverage<P>(
    src: GridRef<'_, P>,
    dst: &Grid<P>,
    affine: Option<AggAffine>,
    interpolation: Interpolation,
) -> Rasterizer {
    let (in_w, in_h) = (src.width() as f64, src.height() as f64);
    let (out_w, out_h) = (dst.width() as f64, dst.height() as f64);
    let mut rasterizer = Rasterizer::new();
    rasterizer.set_clip_box(0.0, 0.0, out_w, out_h);
    let corners = match affine {
        Some(a) if interpolation != Interpolation::Nearest => {
            if a.has_shear() {
                [(0.0, 0.0), (in_w, 0.0), (in_w, in_h), (0.0, in_h)].map(|(x, y)| a.transform(x, y))
            } else {
                bumped_rectangle(&a, in_w, in_h)
            }
        }
        _ => [(0.0, 0.0), (out_w, 0.0), (out_w, out_h), (0.0, out_h)],
    };
    rasterizer.move_to(Point::new(corners[0].0, corners[0].1));
    for &(x, y) in &corners[1..] {
        rasterizer.line_to(Point::new(x, y));
    }
    rasterizer.close();
    rasterizer
}

/// The transformed source rectangle of an axis-aligned transform, with
/// each edge that lies within half a pixel outside a pixel boundary moved
/// out to that boundary, so that edge pixels at least half covered by the
/// source count as fully covered.
fn bumped_rectangle(affine: &AggAffine, in_w: f64, in_h: f64) -> [(f64, f64); 4] {
    let (mut left, mut bottom) = affine.transform(0.0, 0.0);
    let (mut right, mut top) = affine.transform(in_w, in_h);
    if left > right {
        std::mem::swap(&mut left, &mut right);
    }
    if bottom > top {
        std::mem::swap(&mut bottom, &mut top);
    }
    // The tiny fudge compensates for numerical precision loss, as in Agg.
    let rleft = f64::from(iround(left - 1e-8));
    let rright = f64::from(iround(right + 1e-8));
    let rbottom = f64::from(iround(bottom - 1e-8));
    let rtop = f64::from(iround(top + 1e-8));
    if rleft < left {
        left = rleft;
    }
    if rright > right {
        right = rright;
    }
    if rbottom < bottom {
        bottom = rbottom;
    }
    if rtop > top {
        top = rtop;
    }
    [(left, bottom), (right, bottom), (right, top), (left, top)]
}

/// The runs of an unpacked scanline (Agg's `scanline32_u8`): adjacent
/// spans merge into one run with one cover per pixel. Span generators
/// interpolate across whole runs, so the grouping affects the result.
fn unpacked_runs<'c>(sl: &Scanline, covers: &'c mut Vec<u8>) -> Vec<(i32, &'c [u8])> {
    covers.clear();
    let mut bounds: Vec<(i32, usize, usize)> = Vec::new();
    for span in sl.spans() {
        let start = covers.len();
        covers.extend((0..span.len()).map(|i| span.cover_at(i)));
        match bounds.last_mut() {
            Some((x, first, end))
                if i64::from(*x) + (*end - *first) as i64 == i64::from(span.x) =>
            {
                *end = covers.len();
            }
            _ => bounds.push((span.x, start, covers.len())),
        }
    }
    let covers: &'c [u8] = covers;
    bounds
        .into_iter()
        .map(|(x, start, end)| (x, &covers[start..end]))
        .collect()
}

/// Blends a generated run into row `y` of `dst` with per-pixel coverage,
/// clipped to the destination (`renderer_base::blend_color_hspan`).
fn blend_run<P: Pixel>(dst: &mut Grid<P>, x: i32, y: i32, colors: &[P::Color], covers: &[u8]) {
    let Ok(row) = usize::try_from(y) else {
        return;
    };
    for (offset, (&color, &cover)) in colors.iter().zip(covers).enumerate() {
        let column = i64::from(x) + offset as i64;
        let Ok(column) = usize::try_from(column) else {
            continue;
        };
        if let Some(pixel) = dst.get_mut(column, row) {
            pixel.blend(color, cover);
        }
    }
}

/// The span generator matplotlib selects for the transform and kernel.
enum Generator<'m> {
    /// `span_image_filter_*_nn` with `accurate_interpolator_affine_nn`.
    NearestAffine(NearestInterpolator),
    /// `span_image_filter_*_nn` through a lookup mesh.
    NearestMesh(MeshInterpolator<'m>),
    /// `span_image_resample_*_affine`.
    Resample {
        lut: WeightLut,
        footprint: Footprint,
        interpolator: LinearInterpolator,
    },
    /// `span_image_filter_*` (affine, or through a lookup mesh).
    Filter {
        lut: WeightLut,
        interpolator: MeshInterpolator<'m>,
    },
}

impl Generator<'_> {
    /// Colours the run of `out.len()` pixels starting at `(x, y)`.
    fn generate<P: Pixel>(&mut self, src: GridRef<'_, P>, x: i32, y: i32, out: &mut [P::Color]) {
        match self {
            Self::NearestAffine(interpolator) => span::nearest(src, interpolator, x, y, out),
            Self::NearestMesh(interpolator) => span::nearest(src, interpolator, x, y, out),
            Self::Resample {
                lut,
                footprint,
                interpolator,
            } => span::resample_affine(src, lut, *footprint, interpolator, x, y, out),
            Self::Filter { lut, interpolator } => span::filter(src, lut, interpolator, x, y, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use plt_core::Affine2D;

    use super::*;

    fn gray(width: usize, height: usize, values: &[f64]) -> Grid<f64> {
        Grid::from_vec(width, height, values.to_vec()).expect("grid")
    }

    #[test]
    fn translation_copies_pixels() {
        let src = gray(2, 2, &[1.0, 2.0, 3.0, 4.0]);
        let mut dst = Grid::filled(3, 3, 0.0).expect("grid");
        let params = ResampleParams::new(Affine2D::translation(1.0, 1.0))
            .with_interpolation(Interpolation::Hanning);
        resample(&src, &mut dst, &params).expect("resample");
        assert_eq!(
            dst.as_slice(),
            &[0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 0.0, 3.0, 4.0]
        );
    }

    #[test]
    fn nearest_upsampling_repeats_pixels() {
        let src = gray(2, 1, &[5.0, 7.0]);
        let mut dst = Grid::filled(4, 1, 0.0).expect("grid");
        resample(
            &src,
            &mut dst,
            &ResampleParams::new(Affine2D::scaling(2.0, 1.0)),
        )
        .expect("resample");
        assert_eq!(dst.as_slice(), &[5.0, 5.0, 7.0, 7.0]);
    }

    #[test]
    fn partial_coverage_blends_with_the_destination() {
        let src = Grid::from_vec(1, 1, vec![[255_u8, 0, 0, 255]]).expect("grid");
        let mut dst = Grid::filled(2, 1, [0_u8; 4]).expect("grid");
        let params = ResampleParams::new(Affine2D::from_values(1.25, 0.0, 0.0, 1.0, 0.0, 0.0))
            .with_interpolation(Interpolation::Bilinear);
        resample(&src, &mut dst, &params).expect("resample");
        // The right edge at x = 1.25 rounds inwards, so it is not bumped
        // out and the second pixel is only a quarter covered.
        assert_eq!(dst.as_slice()[0], [255, 0, 0, 255]);
        assert_eq!(dst.as_slice()[1][3], 64);
    }

    #[test]
    fn alpha_scales_generated_pixels() {
        let src = Grid::from_vec(1, 1, vec![[0.5_f64, 0.25, 1.0, 1.0]]).expect("grid");
        let mut dst = Grid::filled(1, 1, [0.0_f64; 4]).expect("grid");
        let params = ResampleParams::new(Affine2D::identity()).with_alpha(0.5);
        resample(&src, &mut dst, &params).expect("resample");
        // Float RGBA blends as premultiplied colour: only alpha is scaled.
        assert_eq!(dst.as_slice()[0], [0.5, 0.25, 1.0, 0.5]);
    }

    #[test]
    fn mesh_transforms_look_up_source_positions() {
        let src = gray(2, 1, &[1.0, 2.0]);
        let mut dst = Grid::filled(2, 1, 0.0).expect("grid");
        let mesh = Grid::from_vec(2, 1, vec![[1.5, 0.5], [0.5, 0.5]]).expect("mesh");
        resample(
            &src,
            &mut dst,
            &ResampleParams::new(PixelTransform::Mesh(mesh)),
        )
        .expect("resample");
        assert_eq!(dst.as_slice(), &[2.0, 1.0]);
    }

    #[test]
    fn invalid_arguments_are_rejected() {
        let src = gray(1, 1, &[1.0]);
        let mut dst = Grid::filled(1, 1, 0.0).expect("grid");
        let empty: Grid<f64> = Grid::filled(0, 0, 0.0).expect("grid");
        let identity = ResampleParams::new(Affine2D::identity());
        assert_eq!(
            resample(&empty, &mut dst, &identity),
            Err(ResampleError::EmptySource)
        );
        assert_eq!(
            resample(
                &src,
                &mut dst,
                &ResampleParams::new(Affine2D::scaling(0.0, 1.0))
            ),
            Err(ResampleError::SingularTransform)
        );
        assert_eq!(
            resample(&src, &mut dst, &identity.clone().with_alpha(f64::NAN))
                .map_err(|e| e.to_string().contains("alpha")),
            Err(true)
        );
        assert_eq!(
            resample(&src, &mut dst, &identity.clone().with_radius(0.0)),
            Err(ResampleError::InvalidRadius(0.0))
        );
        let mesh = Grid::filled(2, 2, [0.0; 2]).expect("mesh");
        assert_eq!(
            resample(
                &src,
                &mut dst,
                &ResampleParams::new(PixelTransform::Mesh(mesh))
            ),
            Err(ResampleError::MeshSize {
                mesh: (2, 2),
                destination: (1, 1)
            })
        );
        let mut nothing: Grid<f64> = Grid::filled(0, 3, 0.0).expect("grid");
        assert_eq!(resample(&src, &mut nothing, &identity), Ok(()));
    }

    /// Outputs of matplotlib 3.11's `_image.resample` for the same inputs.
    #[test]
    fn matches_matplotlib_reference_values() {
        let src = Grid::from_vec(4, 4, (0..16).map(|v| f64::from(v) / 15.0).collect()).unwrap();
        let transform = Affine2D::identity().scale(0.75, 0.7).translate(0.1, 0.2);
        let expected: [(Interpolation, bool, [f64; 9]); 4] = [
            (
                Interpolation::Hanning,
                true,
                [
                    0.037_591_962_837_011_85,
                    0.121_611_258_585_092_49,
                    0.205_708_100_229_352_87,
                    0.382_018_420_793_864_17,
                    0.466_034_882_757_477_8,
                    0.550_112_217_480_638_5,
                    0.730_383_187_483_426_3,
                    0.814_384_013_573_381_1,
                    0.898_481_700_495_445_1,
                ],
            ),
            (
                Interpolation::Hanning,
                false,
                [
                    0.000_203_450_520_833_333_34,
                    0.086_490_885_416_666_66,
                    0.186_193_847_656_250_02,
                    0.341_723_632_812_499_96,
                    0.428_011_067_708_333_3,
                    0.527_714_029_947_916_7,
                    0.770_971_679_687_500_1,
                    0.857_259_114_583_333_3,
                    0.956_962_076_822_916_7,
                ],
            ),
            (
                Interpolation::Bicubic,
                true,
                [
                    0.097_882_522_339_389_32,
                    0.169_587_541_011_547_13,
                    0.248_395_298_122_440_32,
                    0.383_187_831_110_826_8,
                    0.454_896_282_472_059_6,
                    0.533_706_107_933_243_2,
                    0.707_607_518_317_935_7,
                    0.779_278_586_234_160_2,
                    0.858_101_762_923_122_8,
                ],
            ),
            (
                Interpolation::Bicubic,
                false,
                [
                    0.048_026_529_947_916_66,
                    0.126_863_606_770_833_33,
                    0.211_877_441_406_250_03,
                    0.373_795_572_916_666_7,
                    0.452_563_476_562_499_96,
                    0.537_642_415_364_583_3,
                    0.733_512_369_791_666_7,
                    0.812_292_480_468_749_9,
                    0.897_387_695_312_500_1,
                ],
            ),
        ];
        for (interpolation, full, values) in expected {
            let mut dst = Grid::filled(3, 3, 0.0).unwrap();
            let params = ResampleParams::new(transform)
                .with_interpolation(interpolation)
                .with_resample(full)
                .with_norm(true)
                .with_radius(4.0);
            resample(&src, &mut dst, &params).unwrap();
            assert_eq!(dst.as_slice(), &values, "{interpolation} resample={full}");
        }
    }

    #[test]
    fn matches_matplotlib_for_rgba8() {
        let src = Grid::from_vec(
            2,
            2,
            vec![
                [255, 0, 0, 255],
                [0, 0, 0, 128],
                [128, 0, 0, 255],
                [64, 0, 0, 0],
            ],
        )
        .unwrap();
        let mut dst = Grid::filled(5, 5, [0_u8; 4]).unwrap();
        let params = ResampleParams::new(Affine2D::identity().scale(2.3, 2.1).translate(0.3, 0.4))
            .with_interpolation(Interpolation::Bilinear)
            .with_resample(true)
            .with_norm(true);
        resample(&src, &mut dst, &params).unwrap();
        let reds: Vec<u8> = dst.as_slice().iter().map(|p| p[0]).collect();
        let alphas: Vec<u8> = dst.as_slice().iter().map(|p| p[3]).collect();
        assert_eq!(
            reds,
            [
                255, 250, 139, 28, 0, 252, 247, 138, 29, 1, 191, 188, 119, 50, 32, 130, 129, 99,
                31, 3, 128, 126, 99, 28, 0
            ]
        );
        assert_eq!(
            alphas,
            [
                255, 252, 197, 142, 128, 255, 252, 196, 139, 124, 255, 251, 168, 85, 64, 255, 250,
                140, 31, 3, 255, 250, 139, 28, 0
            ]
        );
    }

    #[test]
    fn adjacent_spans_merge_into_runs() {
        let mut rasterizer = Rasterizer::new();
        rasterizer.move_to(Point::new(0.5, 0.0));
        rasterizer.line_to(Point::new(3.0, 0.0));
        rasterizer.line_to(Point::new(3.0, 1.0));
        rasterizer.line_to(Point::new(0.5, 1.0));
        let mut sl = Scanline::new();
        let mut runs = Vec::new();
        rasterizer
            .sweep(&mut sl, |sl| {
                let mut covers = Vec::new();
                runs.extend(
                    unpacked_runs(sl, &mut covers)
                        .into_iter()
                        .map(|(x, c)| (x, c.to_vec())),
                );
            })
            .expect("sweep");
        assert_eq!(runs, vec![(0, vec![128, 255, 255])]);
    }
}
