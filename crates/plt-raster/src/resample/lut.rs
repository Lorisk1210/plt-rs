//! Agg's fixed-point filter weight table (`image_filter_lut`).
//!
//! matplotlib builds Agg with `MPL_FIX_AGG_IMAGE_FILTER_LUT_BUGS`, which
//! changes where the kernel is sampled and drops Agg's final
//! re-symmetrisation; both variants are reproduced exactly here.

use super::filters::FilterShape;
use super::numeric::{FILTER_SCALE, SUBPIXEL_SCALE, SUBPIXEL_SHIFT, iround};

/// Upper bound on normalisation passes per sub-pixel phase. Agg loops until
/// the weights sum to the scale, which always happens within a few passes
/// for real kernels; the bound only guards against pathological input.
const MAX_NORMALIZE_PASSES: u32 = 64;

/// Tabulated filter weights, `diameter × 256` entries scaled by `1 << 14`.
#[derive(Clone, Debug)]
pub(super) struct WeightLut {
    diameter: i32,
    start: i32,
    weights: Vec<i16>,
}

impl WeightLut {
    /// `image_filter_lut::calculate(filter, normalization)`.
    pub(super) fn new(shape: &FilterShape, normalize: bool) -> Self {
        let radius = shape.radius();
        // `uceil(radius) * 2`: the radius is validated to be small and positive.
        let diameter = radius.ceil() as i32 * 2;
        let start = -(diameter / 2 - 1);
        let size = (diameter << SUBPIXEL_SHIFT) as usize;
        let mut weights = vec![0_i16; size];

        let pivot = ((diameter << (SUBPIXEL_SHIFT - 1)) - 1) as usize;
        for i in 0..=pivot {
            let x = i as f64 / f64::from(SUBPIXEL_SCALE);
            let w = to_weight(shape.weight(x));
            weights[pivot + i] = w;
            weights[pivot - i] = w;
        }
        weights[size - 1] = to_weight(shape.weight(f64::from(diameter / 2)));

        let mut lut = Self {
            diameter,
            start,
            weights,
        };
        if normalize {
            lut.normalize();
        }
        lut
    }

    /// Number of source pixels the unscaled kernel spans.
    pub(super) fn diameter(&self) -> i32 {
        self.diameter
    }

    /// Offset of the first tap relative to the sample's pixel.
    pub(super) fn start(&self) -> i32 {
        self.start
    }

    /// The weight at table index `index` (sub-pixel resolution).
    pub(super) fn weight(&self, index: i32) -> i32 {
        i32::from(self.weights[index as usize])
    }

    /// `image_filter_lut::normalize`: makes the taps of every sub-pixel
    /// phase sum exactly to [`FILTER_SCALE`], distributing rounding errors
    /// alternately around the centre tap.
    fn normalize(&mut self) {
        let diameter = self.diameter as usize;
        let phases = SUBPIXEL_SCALE as usize;
        let mut flip = true;
        for phase in 0..phases {
            let index = |tap: usize| tap * phases + phase;
            for _ in 0..MAX_NORMALIZE_PASSES {
                let sum: i32 = (0..diameter)
                    .map(|tap| i32::from(self.weights[index(tap)]))
                    .sum();
                if sum == FILTER_SCALE {
                    break;
                }
                let k = f64::from(FILTER_SCALE) / f64::from(sum);
                let mut sum = 0;
                for tap in 0..diameter {
                    let w = &mut self.weights[index(tap)];
                    *w = iround(f64::from(*w) * k) as i16;
                    sum += i32::from(*w);
                }
                sum -= FILTER_SCALE;
                let inc: i16 = if sum > 0 { -1 } else { 1 };
                for tap in 0..diameter {
                    if sum == 0 {
                        break;
                    }
                    flip = !flip;
                    let centre = diameter / 2;
                    let tap = if flip { centre + tap / 2 } else { centre - tap / 2 };
                    let w = &mut self.weights[index(tap)];
                    if i32::from(*w) < FILTER_SCALE {
                        *w = w.wrapping_add(inc);
                        sum += i32::from(inc);
                    }
                }
            }
        }
    }
}

/// `(int16)iround(y * image_filter_scale)`, wrapping like the C++ cast.
fn to_weight(y: f64) -> i16 {
    iround(y * f64::from(FILTER_SCALE)) as i16
}

#[cfg(test)]
mod tests {
    use super::super::params::Interpolation;
    use super::*;

    fn lut(interpolation: Interpolation, normalize: bool) -> WeightLut {
        let shape = FilterShape::for_interpolation(interpolation, 4.0).expect("filter");
        WeightLut::new(&shape, normalize)
    }

    fn phase_sum(lut: &WeightLut, phase: i32) -> i32 {
        (0..lut.diameter())
            .map(|tap| lut.weight(tap * SUBPIXEL_SCALE + phase))
            .sum()
    }

    #[test]
    fn geometry_follows_radius() {
        let l = lut(Interpolation::Bilinear, true);
        assert_eq!((l.diameter(), l.start()), (2, 0));
        let l = lut(Interpolation::Quadric, true);
        assert_eq!((l.diameter(), l.start()), (4, -1));
        let l = lut(Interpolation::Bessel, true);
        assert_eq!((l.diameter(), l.start()), (8, -3));
    }

    #[test]
    fn bilinear_table_is_the_tent() {
        let l = lut(Interpolation::Bilinear, false);
        // Index 255 is distance 0 with matplotlib's LUT fix.
        assert_eq!(l.weight(255), FILTER_SCALE);
        assert_eq!(l.weight(255 + 128), FILTER_SCALE / 2);
        assert_eq!(l.weight(511), 0);
    }

    #[test]
    fn normalized_phases_sum_to_scale() {
        for &interpolation in Interpolation::ALL.iter().skip(1) {
            let l = lut(interpolation, true);
            for phase in 0..SUBPIXEL_SCALE {
                assert_eq!(phase_sum(&l, phase), FILTER_SCALE, "{interpolation:?} {phase}");
            }
        }
    }
}
