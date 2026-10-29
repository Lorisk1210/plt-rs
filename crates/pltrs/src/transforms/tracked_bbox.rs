//! matplotlib's mutable `Bbox`: a box that grows with data updates.

use pltrs_core::path::DataLimits;
use pltrs_core::{Bbox, Path, Point};

/// A bounding box updated in place from data, as matplotlib's `Bbox`
/// behaves when used for data limits.
///
/// Besides its points it carries the smallest positive coordinates seen
/// (`minpos`, for logarithmic autoscaling), the default for whether an
/// update discards the current bounds (`ignore`, initially `true`), and
/// the points it was created with (for [`mutated_x`](Self::mutated_x)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackedBbox {
    limits: DataLimits,
    original: Bbox,
    ignore: bool,
}

impl TrackedBbox {
    /// A box with the given points, infinite `minpos` and the ignore flag
    /// set (matplotlib's `Bbox(points)`).
    pub const fn new(bbox: Bbox) -> Self {
        Self {
            limits: DataLimits {
                bbox,
                minpos: DataLimits::NULL.minpos,
            },
            original: bbox,
            ignore: true,
        }
    }

    /// The current points.
    pub const fn bbox(&self) -> Bbox {
        self.limits.bbox
    }

    /// The smallest positive x and y seen (`inf` when there were none).
    pub const fn minpos(&self) -> Point {
        self.limits.minpos
    }

    /// Overrides `minpos` (matplotlib's `minpos` setter).
    pub const fn set_minpos(&mut self, minpos: Point) {
        self.limits.minpos = minpos;
    }

    /// A snapshot of the points and `minpos` (matplotlib's `frozen`).
    pub const fn limits(&self) -> DataLimits {
        self.limits
    }

    /// Sets whether updates that do not say otherwise discard the current
    /// bounds (matplotlib's `Bbox.ignore`).
    pub const fn set_ignore(&mut self, ignore: bool) {
        self.ignore = ignore;
    }

    /// Grows the box to the points with two finite coordinates (matplotlib's
    /// `update_from_data_xy`).
    ///
    /// `ignore` (defaulting to the stored flag) starts from the null box
    /// and infinite `minpos` instead of the current state. Only the axes
    /// selected by `update_x`/`update_y` are written back, so an ignoring
    /// x-only update keeps the old y bounds. No points, or no axis
    /// selected, leaves the box untouched.
    pub fn update_from_points(
        &mut self,
        points: &[Point],
        ignore: Option<bool>,
        update_x: bool,
        update_y: bool,
    ) {
        if points.is_empty() || !(update_x || update_y) {
            return;
        }
        let mut next = if ignore.unwrap_or(self.ignore) {
            DataLimits::NULL
        } else {
            self.limits
        };
        next.update_from_points(points, update_x, update_y);
        let (current, updated) = (&mut self.limits, next);
        if update_x {
            current.bbox.p0.x = updated.bbox.p0.x;
            current.bbox.p1.x = updated.bbox.p1.x;
            current.minpos.x = updated.minpos.x;
        }
        if update_y {
            current.bbox.p0.y = updated.bbox.p0.y;
            current.bbox.p1.y = updated.bbox.p1.y;
            current.minpos.y = updated.minpos.y;
        }
    }

    /// [`update_from_points`](Self::update_from_points) with every vertex
    /// of `path`; codes are ignored (matplotlib's `update_from_path`).
    pub fn update_from_path(
        &mut self,
        path: &Path,
        ignore: Option<bool>,
        update_x: bool,
        update_y: bool,
    ) {
        self.update_from_points(path.vertices(), ignore, update_x, update_y);
    }

    /// Grows only the x bounds to the finite values (matplotlib's
    /// `update_from_data_x`).
    pub fn update_from_x(&mut self, xs: &[f64], ignore: Option<bool>) {
        let points: Vec<Point> = xs.iter().map(|&x| Point::new(x, x)).collect();
        self.update_from_points(&points, ignore, true, false);
    }

    /// Grows only the y bounds to the finite values (matplotlib's
    /// `update_from_data_y`).
    pub fn update_from_y(&mut self, ys: &[f64], ignore: Option<bool>) {
        let points: Vec<Point> = ys.iter().map(|&y| Point::new(y, y)).collect();
        self.update_from_points(&points, ignore, false, true);
    }

    /// Whether either x bound differs from the original points (a NaN
    /// bound always counts as changed).
    pub fn mutated_x(&self) -> bool {
        self.limits.bbox.interval_x() != self.original.interval_x()
    }

    /// Whether either y bound differs from the original points (a NaN
    /// bound always counts as changed).
    pub fn mutated_y(&self) -> bool {
        self.limits.bbox.interval_y() != self.original.interval_y()
    }

    /// Whether any bound differs from the original points.
    pub fn mutated(&self) -> bool {
        self.mutated_x() || self.mutated_y()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INF: f64 = f64::INFINITY;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn data() -> [Point; 3] {
        [p(1.0, 2.0), p(3.0, -1.0), p(-0.5, 4.0)]
    }

    #[test]
    fn fresh_boxes_ignore_their_points() {
        let mut b = TrackedBbox::new(Bbox::UNIT);
        assert_eq!(b.minpos(), p(INF, INF));
        assert!(!b.mutated());
        b.update_from_points(&data(), None, true, true);
        assert_eq!(b.bbox(), Bbox::from_extents(-0.5, -1.0, 3.0, 4.0));
        assert_eq!(b.minpos(), p(1.0, 2.0));
        assert!(b.mutated_x() && b.mutated_y());
    }

    #[test]
    fn including_keeps_the_current_bounds_and_orientation() {
        let mut b = TrackedBbox::new(Bbox::from_extents(5.0, 5.0, -5.0, -5.0));
        b.update_from_points(&[p(2.0, 3.0)], Some(false), true, true);
        assert_eq!(b.bbox(), Bbox::from_extents(2.0, 3.0, 2.0, 3.0));
        let mut b = TrackedBbox::new(Bbox::UNIT);
        b.set_ignore(false);
        b.update_from_points(&[p(2.0, 3.0)], None, true, true);
        assert_eq!(b.bbox(), Bbox::from_extents(0.0, 0.0, 2.0, 3.0));
    }

    #[test]
    fn single_axis_updates_keep_the_other_axis() {
        let mut b = TrackedBbox::new(Bbox::UNIT);
        b.update_from_points(&data(), Some(true), true, false);
        assert_eq!(b.bbox(), Bbox::from_extents(-0.5, 0.0, 3.0, 1.0));
        assert_eq!(b.minpos(), p(1.0, INF));
        assert!(b.mutated_x() && !b.mutated_y());
        b.update_from_y(&[0.5, 9.0], Some(false));
        assert_eq!(b.bbox(), Bbox::from_extents(-0.5, 0.0, 3.0, 9.0));
        assert_eq!(b.minpos(), p(1.0, 0.5));
    }

    #[test]
    fn empty_and_non_finite_input() {
        let mut b = TrackedBbox::new(Bbox::UNIT);
        b.update_from_points(&[], Some(true), true, true);
        b.update_from_points(&data(), Some(true), false, false);
        assert_eq!(b.bbox(), Bbox::UNIT, "no-ops");
        b.update_from_points(&[p(f64::NAN, 1.0), p(INF, 2.0)], Some(true), true, true);
        assert_eq!(b.bbox(), Bbox::NULL, "only finite points count");
        b.update_from_x(&[3.0, f64::NAN, 7.5], Some(false));
        assert_eq!(b.bbox().interval_x(), (3.0, 7.5));
    }

    #[test]
    fn explicit_minpos_and_snapshots() {
        let mut b = TrackedBbox::new(Bbox::UNIT);
        b.set_minpos(p(0.5, 0.25));
        b.update_from_points(&[p(0.25, 8.0), p(2.0, 0.125)], Some(false), true, true);
        assert_eq!(b.minpos(), p(0.25, 0.125));
        let frozen = b.limits();
        b.update_from_points(&[p(-1.0, -1.0)], Some(true), true, true);
        assert_eq!(frozen.bbox, Bbox::from_extents(0.0, 0.0, 2.0, 8.0));
        assert_eq!(b.minpos(), p(INF, INF));
    }

    #[test]
    fn paths_contribute_every_vertex() {
        let path = Path::closed(vec![p(0.0, 0.0), p(2.0, 1.0), p(1.0, 3.0), p(9.0, 9.0)]);
        let mut b = TrackedBbox::new(Bbox::NULL);
        b.update_from_path(&path, Some(true), true, true);
        assert_eq!(b.bbox(), Bbox::from_extents(0.0, 0.0, 9.0, 9.0));
    }
}
