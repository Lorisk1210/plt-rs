//! Data limits with the smallest positive coordinates, as matplotlib's
//! autoscaling accumulates them (`Bbox.update_from_path`, C++
//! `update_path_extents`), and bounding-box overlap counting.

use super::convert::VertexSource;
use super::{Path, PathCode};
use crate::{Affine2D, Bbox, Point};

/// A bounding box plus the smallest strictly positive x and y seen
/// (`minpos`, used to autoscale logarithmic axes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataLimits {
    /// The accumulated limits ([`Bbox::NULL`] when nothing was added).
    pub bbox: Bbox,
    /// The smallest positive coordinates (`inf` when there were none).
    pub minpos: Point,
}

crate::archive_struct!(DataLimits { bbox, minpos });

impl Default for DataLimits {
    fn default() -> Self {
        Self::NULL
    }
}

impl DataLimits {
    /// No data: the null box and infinite `minpos`.
    pub const NULL: Self = Self {
        bbox: Bbox::NULL,
        minpos: Point::new(f64::INFINITY, f64::INFINITY),
    };

    /// Grows the limits to include `p` (C++ `update_limits`).
    fn include(&mut self, p: Point) {
        let (b, m) = (&mut self.bbox, &mut self.minpos);
        if p.x < b.p0.x {
            b.p0.x = p.x;
        }
        if p.y < b.p0.y {
            b.p0.y = p.y;
        }
        if p.x > b.p1.x {
            b.p1.x = p.x;
        }
        if p.y > b.p1.y {
            b.p1.y = p.y;
        }
        if p.x > 0.0 && p.x < m.x {
            m.x = p.x;
        }
        if p.y > 0.0 && p.y < m.y {
            m.y = p.y;
        }
    }

    /// matplotlib's `Bbox.update_from_path(path, ignore=False, updatex,
    /// updatey)`: includes every vertex with two finite coordinates —
    /// curve control points and the stored points of closures too.
    pub fn update_from_path(&mut self, path: &Path, update_x: bool, update_y: bool) {
        self.update_from_points(path.vertices(), update_x, update_y);
    }

    /// [`DataLimits::update_from_path`] for the polyline through `points`
    /// (matplotlib's `Bbox.update_from_data_xy`).
    pub fn update_from_points(&mut self, points: &[Point], update_x: bool, update_y: bool) {
        self.update_from_iter(&points.iter().copied(), update_x, update_y);
    }

    /// [`DataLimits::update_from_points`] for the points `points` yields
    /// (read once per direction updated), which need not be gathered
    /// first.
    pub fn update_from_iter<I>(&mut self, points: &I, update_x: bool, update_y: bool)
    where
        I: Iterator<Item = Point> + Clone,
    {
        let finite = || points.clone().filter(|p| p.is_finite());
        let (x, y) = match (update_x, update_y) {
            // Both directions in one pass, each folded as alone.
            (true, true) => {
                let (x, y) = finite().fold((AXIS_START, AXIS_START), |(x, y), p| {
                    (axis_step(x, p.x), axis_step(y, p.y))
                });
                (Some(x), Some(y))
            }
            (true, false) => (Some(axis_limits(finite().map(|p| p.x))), None),
            (false, true) => (None, Some(axis_limits(finite().map(|p| p.y)))),
            (false, false) => (None, None),
        };
        if let Some((lo, hi, minpos)) = x {
            self.bbox.p0.x = self.bbox.p0.x.min(lo);
            self.bbox.p1.x = self.bbox.p1.x.max(hi);
            self.minpos.x = self.minpos.x.min(minpos);
        }
        if let Some((lo, hi, minpos)) = y {
            self.bbox.p0.y = self.bbox.p0.y.min(lo);
            self.bbox.p1.y = self.bbox.p1.y.max(hi);
            self.minpos.y = self.minpos.y.min(minpos);
        }
    }

    /// The C++ `update_path_extents`: includes the transformed, NaN-free
    /// vertices of `path` (curve control points included, closures
    /// skipped).
    pub fn update_from_transformed_path(&mut self, path: &Path, transform: &Affine2D) {
        let stream = path
            .iter_vertices()
            .transform(*transform)
            .remove_nans(true, path.codes().is_some());
        for vertex in stream.filter(|v| v.code != PathCode::ClosePoly) {
            self.include(vertex.point);
        }
    }
}

/// The minimum, maximum and smallest positive value (each `±inf` when
/// there is none).
fn axis_limits(values: impl Iterator<Item = f64>) -> (f64, f64, f64) {
    values.fold(AXIS_START, axis_step)
}

/// The limits of no values along an axis: `(lo, hi, minpos)`.
const AXIS_START: (f64, f64, f64) = (f64::INFINITY, f64::NEG_INFINITY, f64::INFINITY);

/// The limits along an axis grown by the value `v`.
fn axis_step((lo, hi, minpos): (f64, f64, f64), v: f64) -> (f64, f64, f64) {
    (
        lo.min(v),
        hi.max(v),
        if v > 0.0 { minpos.min(v) } else { minpos },
    )
}

impl Path {
    /// The data limits of the path alone (`Bbox.null()` updated from it).
    pub fn data_limits(&self) -> DataLimits {
        let mut limits = DataLimits::NULL;
        limits.update_from_path(self, true, true);
        limits
    }
}

/// matplotlib's `BboxBase.count_overlaps` (`count_bboxes_overlapping_bbox`):
/// how many of `boxes` overlap `bbox` with positive area (touching edges do
/// not count). Orientation is ignored.
pub fn count_overlapping(bbox: &Bbox, boxes: &[Bbox]) -> usize {
    let (ax1, ax2) = (bbox.xmin(), bbox.xmax());
    let (ay1, ay2) = (bbox.ymin(), bbox.ymax());
    boxes
        .iter()
        .filter(|b| !(b.xmax() <= ax1 || b.ymax() <= ay1 || b.xmin() >= ax2 || b.ymin() >= ay2))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn both_directions_at_once_equal_each_alone() {
        let points = [
            p(-0.0, 3.0),
            p(f64::NAN, 1.0),
            p(2.0, -1.0),
            p(0.0, f64::INFINITY),
            p(0.5, 0.25),
        ];
        let mut both = DataLimits::NULL;
        both.update_from_points(&points, true, true);
        let mut apart = DataLimits::NULL;
        apart.update_from_points(&points, true, false);
        apart.update_from_points(&points, false, true);
        assert_eq!(both, apart);
        assert_eq!(
            (both.bbox.x0(), both.bbox.y1(), both.minpos),
            (-0.0, 3.0, p(0.5, 0.25))
        );
    }

    #[test]
    fn points_update_only_the_requested_axes() {
        let mut limits = DataLimits::NULL;
        limits.update_from_points(&[p(1.0, 2.0), p(f64::NAN, 10.0), p(-3.0, -1.0)], true, true);
        assert_eq!(limits.bbox, Bbox::from_extents(-3.0, -1.0, 1.0, 2.0));
        assert_eq!(limits.minpos, p(1.0, 2.0));
        limits.update_from_points(&[p(100.0, 100.0)], false, true);
        assert_eq!(limits.bbox.x1(), 1.0, "x untouched");
        assert_eq!(limits.bbox.y1(), 100.0);
    }

    #[test]
    fn minpos_tracks_smallest_positive_values() {
        let path = Path::polyline(vec![
            p(-1.0, 3.0),
            p(2.0, 0.5),
            p(f64::NAN, 0.1),
            p(0.0, 7.0),
        ]);
        let limits = path.data_limits();
        assert_eq!(limits.bbox, Bbox::from_extents(-1.0, 0.5, 2.0, 7.0));
        assert_eq!(limits.minpos, p(2.0, 0.5));
        let negative = Path::polyline(vec![p(-1.0, -2.0), p(-0.5, -1.0)]);
        assert_eq!(
            negative.data_limits().minpos,
            p(f64::INFINITY, f64::INFINITY)
        );
    }

    #[test]
    fn updates_respect_axis_flags() {
        let mut limits = DataLimits::NULL;
        limits.update_from_path(&Path::polyline(vec![p(1.0, 1.0), p(2.0, 3.0)]), true, false);
        assert_eq!(limits.bbox.x0(), 1.0);
        assert!(limits.bbox.y0().is_infinite());
        limits.update_from_path(&Path::polyline(vec![p(5.0, 4.0)]), true, true);
        assert_eq!(limits.bbox, Bbox::from_extents(1.0, 4.0, 5.0, 4.0));
    }

    #[test]
    fn transformed_updates_skip_closures() {
        let square = Path::closed(vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(9.0, 9.0)]);
        let mut limits = DataLimits::NULL;
        limits.update_from_transformed_path(&square, &Affine2D::scaling(2.0, 2.0));
        assert_eq!(limits.bbox, Bbox::from_extents(0.0, 0.0, 2.0, 2.0));
        assert_eq!(limits.minpos, p(2.0, 2.0));
    }

    #[test]
    fn overlap_counting() {
        let a = Bbox::from_extents(0.0, 0.0, 1.0, 1.0);
        let boxes = [
            Bbox::from_extents(0.5, 0.5, 2.0, 2.0),
            Bbox::from_extents(1.0, 1.0, 2.0, 2.0),
            Bbox::from_extents(2.0, 2.0, 0.9, 0.9),
        ];
        assert_eq!(count_overlapping(&a, &boxes), 2);
    }
}
