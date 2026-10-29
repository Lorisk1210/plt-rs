//! Extremes of the lifted points, the `Qbb` scaling of the lifted
//! coordinate and qhull's roundoff-derived constants (`qh_maxmin`,
//! `qh_scalelast`, `qh_detroundoff`).

use super::geom::{Vec3, div_zero};
use super::hull::{HullError, HullResult};

/// qhull's `REALmin` floor for denominators (`qh.MINdenom_1`).
const MIN_DENOM_1: f64 = f64::MIN_POSITIVE;

/// The hull dimension as a float.
const DIM: f64 = 3.0;

/// Constants derived from the coordinates' magnitude and machine epsilon,
/// with qhull's names in the field docs.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Precision {
    /// Maximum roundoff of a distance computation (`qh.DISTround`).
    pub(super) dist_round: f64,
    /// Maximum roundoff of an angle computation (`qh.ANGLEround`).
    pub(super) angle_round: f64,
    /// Centrum radius for pre-merging (`qh.premerge_centrum`).
    pub(super) centrum_radius: f64,
    /// Minimum distance for a point to see a facet (`qh.MINvisible`).
    pub(super) min_visible: f64,
    /// Maximum distance of a coplanar point below a facet (`qh.MAXcoplanar`).
    pub(super) max_coplanar: f64,
    /// Minimum distance of an outside point (`qh.MINoutside`).
    pub(super) min_outside: f64,
    /// Maximum vertex offset from merging simplicial facets (`qh.ONEmerge`).
    pub(super) one_merge: f64,
    /// Width above which a merged facet keeps its centrum (`qh.WIDEfacet`).
    pub(super) wide_facet: f64,
    /// Minimum norm that can be divided into (`qh.MINdenom`).
    pub(super) min_denom: f64,
    /// Minimum denominator for dividing into 1.0 (`qh.MINdenom_1`).
    pub(super) min_denom_1: f64,
    /// Denominator floor for normalised quotients (`qh.MINdenom_1_2`).
    pub(super) min_denom_1_2: f64,
    /// Denominator floor for back substitution (`qh.MINdenom_2`).
    pub(super) min_denom_2: f64,
    /// Pivots below these are nearly zero (`qh.NEARzero`).
    pub(super) near_zero: [f64; 3],
    /// Largest input coordinate magnitude (`qh.MAXabs_coord`).
    pub(super) max_abs_coord: f64,
    /// Largest width of the input in x or y (`qh.MAXwidth`).
    pub(super) max_width: f64,
}

/// For each coordinate the indices of the points with the minimum and the
/// maximum value, in that order (qhull's `maxpoints` set).
pub(super) type ExtremePoints = [usize; 6];

impl Precision {
    /// Finds the extreme points, scales the lifted coordinate of `points`
    /// to `[0, max |x|, |y|]` (option `Qbb`) and derives the constants.
    pub(super) fn scale_and_measure(points: &mut [Vec3]) -> HullResult<(Self, ExtremePoints)> {
        let mut extremes = [0; 6];
        let mut max_abs_coord = 0.0_f64;
        let mut max_width = -f64::MAX;
        let mut max_sum_coord = 0.0;
        let mut near_zero = [0.0; 3];
        let (mut low, mut high) = (0.0, 0.0);
        for k in 0..3 {
            let (mut min, mut max) = (0, 0);
            for (i, p) in points.iter().enumerate() {
                if points[max][k] < p[k] {
                    max = i;
                } else if points[min][k] > p[k] {
                    min = i;
                }
            }
            let (lo, hi) = (points[min][k], points[max][k]);
            let max_coord = if k == 2 {
                (low, high) = (lo, hi);
                max_abs_coord
            } else {
                max_width = max_width.max_qhull(hi - lo);
                hi.max_qhull(-lo)
            };
            max_abs_coord = max_abs_coord.max_qhull(max_coord);
            max_sum_coord += max_coord;
            extremes[2 * k] = min;
            extremes[2 * k + 1] = max;
            near_zero[k] = 80.0 * max_sum_coord * f64::EPSILON;
        }
        let scale = div_zero(max_abs_coord, high - low, MIN_DENOM_1).ok_or(HullError::Singular)?;
        // qhull's `newlow - low * scale` with `newlow == 0`.
        let shift = 0.0 - low * scale;
        for p in points.iter_mut() {
            p[2] = p[2] * scale + shift;
        }
        Ok((
            Self::from_extent(max_abs_coord, max_sum_coord, max_width, near_zero),
            extremes,
        ))
    }

    /// qhull's `qh_detroundoff` for pre-merging with a zero centrum radius.
    fn from_extent(
        max_abs_coord: f64,
        max_sum_coord: f64,
        max_width: f64,
        near_zero: [f64; 3],
    ) -> Self {
        let mut max_dist_sum = DIM.sqrt() * max_abs_coord;
        if max_dist_sum > max_sum_coord {
            max_dist_sum = max_sum_coord;
        }
        let dist_round = f64::EPSILON * (DIM * max_dist_sum * 1.01 + max_abs_coord);
        let centrum_radius = 2.0 * dist_round;
        // qhull adds `sqrt(dim) * width * sin(max merge angle)` to the
        // roundoff; without angle merging the angle term is zero.
        let one_merge = dist_round.max_qhull(DIM * centrum_radius + dist_round);
        let min_visible = centrum_radius;
        let max_coplanar = min_visible;
        let min_outside = 2.0 * min_visible;
        let wide_facet = min_outside
            .max_qhull(6.0 * max_coplanar)
            .max_qhull(6.0 * min_visible);
        let min_denom_1_2 = (MIN_DENOM_1 * DIM).sqrt();
        Self {
            dist_round,
            angle_round: 1.01 * DIM * f64::EPSILON,
            centrum_radius,
            min_visible,
            max_coplanar,
            min_outside,
            one_merge,
            wide_facet,
            min_denom: MIN_DENOM_1 * max_abs_coord,
            min_denom_1: MIN_DENOM_1,
            min_denom_1_2,
            min_denom_2: min_denom_1_2 * max_abs_coord,
            near_zero,
            max_abs_coord,
            max_width,
        }
    }

    /// Widest duplicate-ridge merge that is still preferred, qhull's
    /// `qh_WIDEdupridge * (ONEmerge + DISTround)`.
    pub(super) fn narrow_dupridge(&self) -> f64 {
        50.0 * (self.one_merge + self.dist_round)
    }

    /// Constants for unit-sized test data with the given distance roundoff.
    #[cfg(test)]
    pub(super) fn for_tests(dist_round: f64) -> Self {
        let mut prec = Self::from_extent(1.0, 3.0, 1.0, [1e-14; 3]);
        prec.dist_round = dist_round;
        prec
    }
}

/// qhull's `fmax_`/`maximize_` and `fmin_`/`minimize_`: the second value
/// only if strictly better, so a NaN never replaces the first.
pub(super) trait QhullMinMax {
    /// `if self < other { other } else { self }`.
    fn max_qhull(self, other: Self) -> Self;

    /// `if self > other { other } else { self }`.
    fn min_qhull(self, other: Self) -> Self;
}

impl QhullMinMax for f64 {
    fn max_qhull(self, other: f64) -> f64 {
        if self < other { other } else { self }
    }

    fn min_qhull(self, other: f64) -> f64 {
        if self > other { other } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_maps_the_lifted_coordinate_to_the_input_extent() {
        let mut points = vec![
            [-2.0, 0.0, 4.0],
            [1.0, 0.5, 1.25],
            [0.0, -1.0, 1.0],
            [0.0, 0.0, 4.4],
        ];
        let (prec, extremes) = Precision::scale_and_measure(&mut points).unwrap();
        assert_eq!(extremes, [0, 1, 2, 1, 2, 3]);
        assert_eq!(prec.max_abs_coord, 2.0);
        assert_eq!(prec.max_width, 3.0);
        assert_eq!(points[2][2], 0.0);
        assert!((points[3][2] - 2.0).abs() < 1e-15);
        let dist_round = prec.dist_round;
        assert!(dist_round > 0.0 && dist_round < 1e-14);
        assert_eq!(prec.one_merge, 3.0 * (2.0 * dist_round) + dist_round);
    }

    #[test]
    fn equal_lifted_coordinates_cannot_be_scaled() {
        let mut points = vec![[1.0, 0.0, 1.0], [0.0, 1.0, 1.0], [-1.0, 0.0, 1.0]];
        assert_eq!(
            Precision::scale_and_measure(&mut points),
            Err(HullError::Singular)
        );
    }
}
