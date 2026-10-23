//! Agg's image span generators: compute the colours of a horizontal run of
//! destination pixels.
//!
//! * [`nearest`] — `span_image_filter_{gray,rgba}_nn`.
//! * [`filter`] — `span_image_filter_{gray,rgba}`: the kernel at its
//!   natural size.
//! * [`resample_affine`] — `span_image_resample_{gray,rgba}_affine`: the
//!   kernel stretched over the source footprint of a destination pixel.
//!
//! All generators sample at destination pixel centres.

use super::accessor::{ReflectAccessor, clipped};
use super::grid::GridRef;
use super::interpolator::{LinearInterpolator, SpanInterpolator};
use super::lut::WeightLut;
use super::numeric::{FILTER_SCALE, FILTER_SHIFT, SUBPIXEL_MASK, SUBPIXEL_SHIFT, uround};
use super::pixel::ops::PixelOps;

/// The default `filter_offset` of `span_image_filter` (half a pixel).
const CENTRE: f64 = 0.5;
/// The same offset in sub-pixel units.
const CENTRE_SUBPIXEL: i32 = 1 << (SUBPIXEL_SHIFT - 1);
/// `span_image_resample_affine`'s default `scale_limit`.
const SCALE_LIMIT: f64 = 200.0;

/// Starts `interpolator` at the centre of destination pixel `(x, y)`.
fn begin(interpolator: &mut impl SpanInterpolator, x: i32, y: i32, len: usize) {
    interpolator.begin(f64::from(x) + CENTRE, f64::from(y) + CENTRE, len as u32);
}

/// The fixed-point product of two tabulated weights.
fn product(lut: &WeightLut, index_y: i32, index_x: i32) -> i32 {
    (lut.weight(index_y) * lut.weight(index_x) + FILTER_SCALE / 2) >> FILTER_SHIFT
}

/// Nearest-neighbour samples with a zero background outside the source.
pub(super) fn nearest<P: PixelOps>(
    src: GridRef<'_, P>,
    interpolator: &mut impl SpanInterpolator,
    x: i32,
    y: i32,
    out: &mut [P::Color],
) {
    begin(interpolator, x, y, out.len());
    for color in out {
        let (sx, sy) = interpolator.coordinates();
        *color = clipped(src, sx >> SUBPIXEL_SHIFT, sy >> SUBPIXEL_SHIFT).nearest();
        interpolator.advance();
    }
}

/// Kernel-filtered samples, `diameter × diameter` taps per pixel.
pub(super) fn filter<P: PixelOps>(
    src: GridRef<'_, P>,
    lut: &WeightLut,
    interpolator: &mut impl SpanInterpolator,
    x: i32,
    y: i32,
    out: &mut [P::Color],
) {
    begin(interpolator, x, y, out.len());
    let mut pixels = ReflectAccessor::new(src);
    let diameter = lut.diameter();
    let start = lut.start();
    for color in out {
        let (sx, sy) = interpolator.coordinates();
        let sx = sx.wrapping_sub(CENTRE_SUBPIXEL);
        let sy = sy.wrapping_sub(CENTRE_SUBPIXEL);
        let x_fract = sx & SUBPIXEL_MASK;
        let mut index_y = SUBPIXEL_MASK - (sy & SUBPIXEL_MASK);
        let mut acc = P::ACCUM_ZERO;
        let mut px = pixels.span(
            (sx >> SUBPIXEL_SHIFT).wrapping_add(start),
            (sy >> SUBPIXEL_SHIFT).wrapping_add(start),
        );
        for row in 0..diameter {
            if row > 0 {
                index_y += 1 << SUBPIXEL_SHIFT;
                px = pixels.next_y();
            }
            let mut index_x = SUBPIXEL_MASK - x_fract;
            for column in 0..diameter {
                if column > 0 {
                    index_x += 1 << SUBPIXEL_SHIFT;
                    px = pixels.next_x();
                }
                px.accumulate(&mut acc, product(lut, index_y, index_x));
            }
        }
        *color = P::filtered(acc);
        interpolator.advance();
    }
}

/// The kernel footprint of [`resample_affine`]: how many source pixels one
/// destination pixel spans along each axis, in sub-pixel units, and the
/// reciprocal step through the weight table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Footprint {
    rx: i32,
    ry: i32,
    rx_inv: i32,
    ry_inv: i32,
}

impl Footprint {
    /// `span_image_resample_affine::prepare` for a destination-to-source
    /// transform: never smaller than one pixel, scale product capped at 200.
    pub(super) fn new(interpolator: &LinearInterpolator) -> Self {
        let (mut sx, mut sy) = interpolator.transform().scaling_abs();
        if sx * sy > SCALE_LIMIT {
            // The second line deliberately sees the updated `sx`, as in Agg.
            sx = sx * SCALE_LIMIT / (sx * sy);
            sy = sy * SCALE_LIMIT / (sx * sy);
        }
        let sx = sx.clamp(1.0, SCALE_LIMIT);
        let sy = sy.clamp(1.0, SCALE_LIMIT);
        let scale = f64::from(1 << SUBPIXEL_SHIFT);
        Self {
            rx: uround(sx * scale),
            ry: uround(sy * scale),
            rx_inv: uround(1.0 / sx * scale),
            ry_inv: uround(1.0 / sy * scale),
        }
    }
}

/// Area-resampled samples: the kernel is stretched by the footprint and
/// the result divided by the sum of the weights actually used.
pub(super) fn resample_affine<P: PixelOps>(
    src: GridRef<'_, P>,
    lut: &WeightLut,
    footprint: Footprint,
    interpolator: &mut LinearInterpolator,
    x: i32,
    y: i32,
    out: &mut [P::Color],
) {
    begin(interpolator, x, y, out.len());
    let mut pixels = ReflectAccessor::new(src);
    let diameter = lut.diameter();
    let filter_scale = diameter << SUBPIXEL_SHIFT;
    let radius_x = (diameter * footprint.rx) >> 1;
    let radius_y = (diameter * footprint.ry) >> 1;
    for color in out {
        let (sx, sy) = interpolator.coordinates();
        let sx = sx.wrapping_add(CENTRE_SUBPIXEL - radius_x);
        let sy = sy.wrapping_add(CENTRE_SUBPIXEL - radius_y);
        let mut index_y =
            ((SUBPIXEL_MASK - (sy & SUBPIXEL_MASK)) * footprint.ry_inv) >> SUBPIXEL_SHIFT;
        let first_x = ((SUBPIXEL_MASK - (sx & SUBPIXEL_MASK)) * footprint.rx_inv) >> SUBPIXEL_SHIFT;
        let mut acc = P::ACCUM_ZERO;
        let mut total_weight = 0_i32;
        let mut px = pixels.span(sx >> SUBPIXEL_SHIFT, sy >> SUBPIXEL_SHIFT);
        loop {
            let weight_y = lut.weight(index_y);
            let mut index_x = first_x;
            loop {
                let weight = (weight_y * lut.weight(index_x) + FILTER_SCALE / 2) >> FILTER_SHIFT;
                px.accumulate(&mut acc, weight);
                total_weight = total_weight.wrapping_add(weight);
                index_x += footprint.rx_inv;
                if index_x >= filter_scale {
                    break;
                }
                px = pixels.next_x();
            }
            index_y += footprint.ry_inv;
            if index_y >= filter_scale {
                break;
            }
            px = pixels.next_y();
        }
        *color = P::resampled(acc, total_weight);
        interpolator.advance();
    }
}

#[cfg(test)]
mod tests {
    use super::super::filters::FilterShape;
    use super::super::grid::Grid;
    use super::super::interpolator::NearestInterpolator;
    use super::super::numeric::AggAffine;
    use super::super::params::Interpolation;
    use super::*;

    fn lut(interpolation: Interpolation) -> WeightLut {
        WeightLut::new(
            &FilterShape::for_interpolation(interpolation, 4.0).expect("filter"),
            true,
        )
    }

    fn values(colors: &[<f64 as PixelOps>::Color]) -> Vec<f64> {
        colors
            .iter()
            .map(|&c| {
                let mut px = f64::NAN;
                px.blend(c, 255);
                px
            })
            .collect()
    }

    #[test]
    fn identity_filter_reproduces_the_source() {
        let src = Grid::from_vec(3, 1, vec![1.0, 2.0, 4.0]).expect("grid");
        let mut out = [<f64 as PixelOps>::Color::default(); 3];
        let mut li = LinearInterpolator::new(AggAffine::IDENTITY);
        filter(
            src.view(),
            &lut(Interpolation::Bilinear),
            &mut li,
            0,
            0,
            &mut out,
        );
        assert_eq!(values(&out), vec![1.0, 2.0, 4.0]);
    }

    #[test]
    fn nearest_outside_is_background() {
        let src = Grid::from_vec(2, 1, vec![5.0, 6.0]).expect("grid");
        let mut out = [<f64 as PixelOps>::Color::default(); 4];
        let mut nn = NearestInterpolator::new(AggAffine::IDENTITY);
        nearest(src.view(), &mut nn, -1, 0, &mut out);
        assert_eq!(values(&out), vec![0.0, 5.0, 6.0, 0.0]);
    }

    #[test]
    fn footprint_tracks_downsampling() {
        let shrink = AggAffine {
            sx: 2.5,
            shy: 0.0,
            shx: 0.0,
            sy: 0.5,
            tx: 0.0,
            ty: 0.0,
        };
        let fp = Footprint::new(&LinearInterpolator::new(shrink));
        assert_eq!(
            fp,
            Footprint {
                rx: 640,
                ry: 256,
                rx_inv: 102,
                ry_inv: 256
            }
        );
        let huge = AggAffine {
            sx: 40.0,
            sy: 40.0,
            ..shrink
        };
        let fp = Footprint::new(&LinearInterpolator::new(huge));
        // sx becomes 40·200/1600 = 5, then sy = 40·200/(5·40) = 40.
        assert_eq!((fp.rx, fp.ry), (1280, 10240));
    }

    #[test]
    fn resampling_a_constant_is_exact() {
        let src = Grid::from_vec(8, 8, vec![[200_u8, 100, 50, 255]; 64]).expect("grid");
        let mut out = [[0_u8; 4]; 3];
        let shrink = AggAffine {
            sx: 3.0,
            shy: 0.0,
            shx: 0.0,
            sy: 3.0,
            tx: 0.0,
            ty: 0.0,
        };
        let mut li = LinearInterpolator::new(shrink);
        let fp = Footprint::new(&li);
        resample_affine(
            src.view(),
            &lut(Interpolation::Hanning),
            fp,
            &mut li,
            0,
            1,
            &mut out,
        );
        assert!(out.iter().all(|&c| c == [200, 100, 50, 255]));
    }
}
