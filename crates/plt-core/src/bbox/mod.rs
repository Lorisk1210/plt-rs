//! Axis-aligned bounding boxes with matplotlib semantics.

mod anchor;
mod derived;

pub use anchor::Anchor;

use crate::Point;

/// An axis-aligned box spanned by two corner points.
///
/// Like matplotlib's `Bbox`, the box is *not* normalised: `x0` may be greater
/// than `x1` (an inverted axis), in which case [`width`](Self::width) is
/// negative. Use [`xmin`](Self::xmin)/[`xmax`](Self::xmax) for ordered
/// bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bbox {
    /// First corner, usually the lower-left.
    pub p0: Point,
    /// Second corner, usually the upper-right.
    pub p1: Point,
}

impl Bbox {
    /// The unit box from `(0, 0)` to `(1, 1)`.
    pub const UNIT: Self = Self::from_extents(0.0, 0.0, 1.0, 1.0);

    /// The "null" box `[[inf, inf], [-inf, -inf]]`, the identity for
    /// [`union`](Self::union) and for incremental data-limit updates.
    pub const NULL: Self = Self::from_extents(
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );

    /// Creates a box from its two corners.
    pub const fn new(p0: Point, p1: Point) -> Self {
        Self { p0, p1 }
    }

    /// Creates a box from `x0, y0, x1, y1`.
    pub const fn from_extents(x0: f64, y0: f64, x1: f64, y1: f64) -> Self {
        Self::new(Point::new(x0, y0), Point::new(x1, y1))
    }

    /// Creates a box from its lower-left corner and (possibly negative) size.
    pub const fn from_bounds(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::from_extents(x, y, x + width, y + height)
    }

    /// The smallest box containing every point, or [`Bbox::NULL`] when empty.
    pub fn from_points<I: IntoIterator<Item = Point>>(points: I) -> Self {
        points
            .into_iter()
            .fold(Self::NULL, |b, p| Self::new(b.p0.min(p), b.p1.max(p)))
    }

    /// First corner x.
    pub const fn x0(&self) -> f64 {
        self.p0.x
    }
    /// First corner y.
    pub const fn y0(&self) -> f64 {
        self.p0.y
    }
    /// Second corner x.
    pub const fn x1(&self) -> f64 {
        self.p1.x
    }
    /// Second corner y.
    pub const fn y1(&self) -> f64 {
        self.p1.y
    }
    /// Smaller x bound.
    pub fn xmin(&self) -> f64 {
        nan_min(self.p0.x, self.p1.x)
    }
    /// Larger x bound.
    pub fn xmax(&self) -> f64 {
        nan_max(self.p0.x, self.p1.x)
    }
    /// Smaller y bound.
    pub fn ymin(&self) -> f64 {
        nan_min(self.p0.y, self.p1.y)
    }
    /// Larger y bound.
    pub fn ymax(&self) -> f64 {
        nan_max(self.p0.y, self.p1.y)
    }
    /// Signed width `x1 - x0`.
    pub fn width(&self) -> f64 {
        self.p1.x - self.p0.x
    }
    /// Signed height `y1 - y0`.
    pub fn height(&self) -> f64 {
        self.p1.y - self.p0.y
    }
    /// `(x0, y0, width, height)`.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        (self.p0.x, self.p0.y, self.width(), self.height())
    }
    /// `(x0, y0, x1, y1)`.
    pub const fn extents(&self) -> (f64, f64, f64, f64) {
        (self.p0.x, self.p0.y, self.p1.x, self.p1.y)
    }
    /// `(x0, x1)`.
    pub const fn interval_x(&self) -> (f64, f64) {
        (self.p0.x, self.p1.x)
    }
    /// `(y0, y1)`.
    pub const fn interval_y(&self) -> (f64, f64) {
        (self.p0.y, self.p1.y)
    }
    /// Midpoint of the box.
    pub fn center(&self) -> Point {
        self.p0.lerp(self.p1, 0.5)
    }

    /// The four corners: `(x0, y0)`, `(x0, y1)`, `(x1, y0)`, `(x1, y1)`.
    pub const fn corners(&self) -> [Point; 4] {
        [
            self.p0,
            Point::new(self.p0.x, self.p1.y),
            Point::new(self.p1.x, self.p0.y),
            self.p1,
        ]
    }

    /// Returns `true` when all coordinates are finite.
    pub fn is_finite(&self) -> bool {
        self.p0.is_finite() && self.p1.is_finite()
    }

    /// Returns `true` for the empty [`Bbox::NULL`]-like state (no point added).
    pub fn is_null(&self) -> bool {
        self.p0.x > self.p1.x && self.p0.x.is_infinite()
    }

    /// Whether `x` lies in the closed x interval (in either orientation).
    pub fn contains_x(&self, x: f64) -> bool {
        let (x0, x1) = self.interval_x();
        (x0 <= x && x <= x1) || (x0 >= x && x >= x1)
    }

    /// Whether `y` lies in the closed y interval (in either orientation).
    pub fn contains_y(&self, y: f64) -> bool {
        let (y0, y1) = self.interval_y();
        (y0 <= y && y <= y1) || (y0 >= y && y >= y1)
    }

    /// Whether the point lies in the closed box.
    pub fn contains(&self, p: Point) -> bool {
        self.contains_x(p.x) && self.contains_y(p.y)
    }

    /// Whether `x` lies strictly inside the x interval.
    pub fn fully_contains_x(&self, x: f64) -> bool {
        let (x0, x1) = self.interval_x();
        (x0 < x && x < x1) || (x0 > x && x > x1)
    }

    /// Whether `y` lies strictly inside the y interval.
    pub fn fully_contains_y(&self, y: f64) -> bool {
        let (y0, y1) = self.interval_y();
        (y0 < y && y < y1) || (y0 > y && y > y1)
    }

    /// Whether the point lies strictly inside the box.
    pub fn fully_contains(&self, p: Point) -> bool {
        self.fully_contains_x(p.x) && self.fully_contains_y(p.y)
    }

    /// The extents `(x0, y0, x1, y1)` with each axis swapped into
    /// increasing order when `x1 < x0` (`y1 < y0`), the way matplotlib's
    /// overlap tests sort boxes: a NaN coordinate never triggers the swap.
    fn sorted_extents(&self) -> (f64, f64, f64, f64) {
        let sort = |lo: f64, hi: f64| if hi < lo { (hi, lo) } else { (lo, hi) };
        let (x0, x1) = sort(self.p0.x, self.p1.x);
        let (y0, y1) = sort(self.p0.y, self.p1.y);
        (x0, y0, x1, y1)
    }

    /// Whether the two (closed) boxes intersect, regardless of orientation;
    /// touching edges overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        let (ax0, ay0, ax1, ay1) = self.sorted_extents();
        let (bx0, by0, bx1, by1) = other.sorted_extents();
        ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
    }

    /// Whether the two (open) boxes intersect, regardless of orientation.
    pub fn fully_overlaps(&self, other: &Self) -> bool {
        let (ax0, ay0, ax1, ay1) = self.sorted_extents();
        let (bx0, by0, bx1, by1) = other.sorted_extents();
        ax0 < bx1 && bx0 < ax1 && ay0 < by1 && by0 < ay1
    }

    /// The ordered box containing both boxes.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self::from_extents(
            nan_min(self.xmin(), other.xmin()),
            nan_min(self.ymin(), other.ymin()),
            nan_max(self.xmax(), other.xmax()),
            nan_max(self.ymax(), other.ymax()),
        )
    }

    /// The ordered box containing every box, or `None` when there are none
    /// (matplotlib's `Bbox.union`). NaN coordinates propagate, and a null
    /// box makes the union the whole plane (its ordered bounds are
    /// `-inf..inf`).
    pub fn union_all<'a, I: IntoIterator<Item = &'a Self>>(boxes: I) -> Option<Self> {
        let mut boxes = boxes.into_iter();
        let first = boxes.next()?;
        Some(boxes.fold(first.union(first), |union, b| union.union(b)))
    }

    /// The ordered intersection, or `None` when the boxes are disjoint.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let x0 = nan_max(self.xmin(), other.xmin());
        let x1 = nan_min(self.xmax(), other.xmax());
        let y0 = nan_max(self.ymin(), other.ymin());
        let y1 = nan_min(self.ymax(), other.ymax());
        (x0 <= x1 && y0 <= y1).then(|| Self::from_extents(x0, y0, x1, y1))
    }

    /// How many points lie strictly inside the box (non-finite points never do).
    pub fn count_contains(&self, points: &[Point]) -> usize {
        let (lo, hi) = (
            Point::new(self.xmin(), self.ymin()),
            Point::new(self.xmax(), self.ymax()),
        );
        points
            .iter()
            .filter(|p| lo.x < p.x && p.x < hi.x && lo.y < p.y && p.y < hi.y)
            .count()
    }

    /// How many boxes strictly overlap this one (matplotlib's
    /// `count_overlaps`, C++ `count_bboxes_overlapping_bbox`): touching
    /// edges do not count, and since comparisons with NaN fail, a box
    /// counts unless one of its non-NaN edges proves it disjoint.
    pub fn count_overlaps(&self, others: &[Self]) -> usize {
        let (ax0, ay0, ax1, ay1) = self.sorted_extents();
        others
            .iter()
            .filter(|b| {
                let (bx0, by0, bx1, by1) = b.sorted_extents();
                !(bx1 <= ax0 || by1 <= ay0 || bx0 >= ax1 || by0 >= ay1)
            })
            .count()
    }

    /// Whether the box has a positive extent in some direction and a finite
    /// size (matplotlib's private `Bbox._is_finite`).
    pub fn has_finite_extent(&self) -> bool {
        let (w, h) = (self.width(), self.height());
        (w > 0.0 || h > 0.0) && w < f64::INFINITY && h < f64::INFINITY
    }

    /// Extends the (ordered) box to include `p`; non-finite points are ignored.
    #[must_use]
    pub fn including(&self, p: Point) -> Self {
        if p.is_finite() {
            Self::new(self.p0.min(p), self.p1.max(p))
        } else {
            *self
        }
    }
}

/// `min` that propagates NaN, like numpy.
fn nan_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// `max` that propagates NaN, like numpy.
fn nan_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverted_boxes_keep_orientation() {
        let b = Bbox::from_extents(3.0, 0.0, 1.0, 2.0);
        assert_eq!(b.width(), -2.0);
        assert_eq!((b.xmin(), b.xmax()), (1.0, 3.0));
        assert!(b.contains(Point::new(2.0, 1.0)));
        assert!(b.fully_contains(Point::new(2.0, 1.0)));
        assert!(!b.fully_contains(Point::new(3.0, 1.0)));
    }

    #[test]
    fn union_and_intersection() {
        let a = Bbox::from_extents(0.0, 0.0, 2.0, 2.0);
        let b = Bbox::from_extents(1.0, 1.0, 3.0, 4.0);
        assert_eq!(a.union(&b), Bbox::from_extents(0.0, 0.0, 3.0, 4.0));
        assert_eq!(
            a.intersection(&b),
            Some(Bbox::from_extents(1.0, 1.0, 2.0, 2.0))
        );
        assert_eq!(
            a.intersection(&Bbox::from_extents(5.0, 5.0, 6.0, 6.0)),
            None
        );
        // As in matplotlib, the null box's ordered bounds are (-inf, inf).
        assert_eq!(
            Bbox::NULL.union(&a),
            Bbox::from_extents(-f64::INFINITY, -f64::INFINITY, f64::INFINITY, f64::INFINITY)
        );
    }

    #[test]
    fn overlap_tests_ignore_orientation() {
        let a = Bbox::from_extents(2.0, 0.0, 0.0, 2.0);
        let b = Bbox::from_extents(2.0, 2.0, 3.0, 3.0);
        assert!(a.overlaps(&b));
        assert!(!a.fully_overlaps(&b));
        let nan = Bbox::from_extents(f64::NAN, 0.0, 1.0, 1.0);
        assert!(!a.overlaps(&nan) && !a.fully_overlaps(&nan));
        assert!(a.overlaps(&Bbox::NULL), "the null box sorts to the plane");
    }

    #[test]
    fn overlap_counting_sorts_like_matplotlib() {
        let a = Bbox::from_extents(0.0, 0.0, 2.0, 2.0);
        let touching = Bbox::from_extents(2.0, 0.0, 3.0, 2.0);
        let inverted = Bbox::from_extents(3.0, 3.0, 1.0, 1.0);
        assert_eq!(a.count_overlaps(&[]), 0);
        assert_eq!(a.count_overlaps(&[touching, inverted, Bbox::NULL]), 2);
        // A NaN edge never proves disjointness, but the other edges still can.
        let nan_inside = Bbox::from_extents(f64::NAN, 0.0, 1.0, 1.0);
        let nan_left = Bbox::from_extents(f64::NAN, 0.0, -5.0, 1.0);
        assert_eq!(a.count_overlaps(&[nan_inside, nan_left]), 1);
    }

    #[test]
    fn union_of_many() {
        assert_eq!(Bbox::union_all(&[]), None);
        let inverted = Bbox::from_extents(1.0, 1.0, 0.0, 0.0);
        assert_eq!(Bbox::union_all(&[inverted]), Some(Bbox::UNIT));
        let boxes = [Bbox::UNIT, Bbox::from_extents(2.0, 3.0, 4.0, 5.0)];
        assert_eq!(
            Bbox::union_all(&boxes),
            Some(Bbox::from_extents(0.0, 0.0, 4.0, 5.0))
        );
        let with_nan = [Bbox::UNIT, Bbox::from_extents(f64::NAN, 0.0, 2.0, 2.0)];
        let union = Bbox::union_all(&with_nan).expect("non-empty");
        assert!(union.x0().is_nan() && union.x1().is_nan());
        assert_eq!((union.y0(), union.y1()), (0.0, 2.0));
    }

    #[test]
    fn points_and_null() {
        assert!(Bbox::NULL.is_null());
        let b = Bbox::from_points([Point::new(1.0, 5.0), Point::new(-1.0, 2.0)]);
        assert_eq!(b, Bbox::from_extents(-1.0, 2.0, 1.0, 5.0));
        assert_eq!(b.including(Point::new(f64::NAN, 0.0)), b);
    }
}
