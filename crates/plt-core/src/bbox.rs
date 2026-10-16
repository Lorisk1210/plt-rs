//! Axis-aligned bounding boxes with matplotlib semantics.

use crate::{Affine2D, Point};

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

/// Where to place a shrunken box inside its container (see [`Bbox::anchored`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// Centre.
    C,
    /// Lower left.
    SW,
    /// Bottom centre.
    S,
    /// Lower right.
    SE,
    /// Right centre.
    E,
    /// Upper right.
    NE,
    /// Top centre.
    N,
    /// Upper left.
    NW,
    /// Left centre.
    W,
    /// Fractional position `(cx, cy)` with `(0, 0)` = lower left.
    Fraction(f64, f64),
}

impl Anchor {
    /// The fractional `(cx, cy)` position this anchor stands for.
    pub const fn coefficients(self) -> (f64, f64) {
        match self {
            Self::C => (0.5, 0.5),
            Self::SW => (0.0, 0.0),
            Self::S => (0.5, 0.0),
            Self::SE => (1.0, 0.0),
            Self::E => (1.0, 0.5),
            Self::NE => (1.0, 1.0),
            Self::N => (0.5, 1.0),
            Self::NW => (0.0, 1.0),
            Self::W => (0.0, 0.5),
            Self::Fraction(cx, cy) => (cx, cy),
        }
    }

    /// Parses matplotlib's anchor codes (`"C"`, `"SW"`, ..., `"W"`).
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "C" => Self::C,
            "SW" => Self::SW,
            "S" => Self::S,
            "SE" => Self::SE,
            "E" => Self::E,
            "NE" => Self::NE,
            "N" => Self::N,
            "NW" => Self::NW,
            "W" => Self::W,
            _ => return None,
        })
    }
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
        self.p0.x.min(self.p1.x)
    }
    /// Larger x bound.
    pub fn xmax(&self) -> f64 {
        self.p0.x.max(self.p1.x)
    }
    /// Smaller y bound.
    pub fn ymin(&self) -> f64 {
        self.p0.y.min(self.p1.y)
    }
    /// Larger y bound.
    pub fn ymax(&self) -> f64 {
        self.p0.y.max(self.p1.y)
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

    /// Whether the two (closed) boxes intersect, regardless of orientation.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.xmin() <= other.xmax()
            && other.xmin() <= self.xmax()
            && self.ymin() <= other.ymax()
            && other.ymin() <= self.ymax()
    }

    /// Whether the two (open) boxes intersect, regardless of orientation.
    pub fn fully_overlaps(&self, other: &Self) -> bool {
        self.xmin() < other.xmax()
            && other.xmin() < self.xmax()
            && self.ymin() < other.ymax()
            && other.ymin() < self.ymax()
    }

    /// The ordered box containing both boxes.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        Self::from_extents(
            self.xmin().min(other.xmin()),
            self.ymin().min(other.ymin()),
            self.xmax().max(other.xmax()),
            self.ymax().max(other.ymax()),
        )
    }

    /// The ordered intersection, or `None` when the boxes are disjoint.
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let x0 = self.xmin().max(other.xmin());
        let x1 = self.xmax().min(other.xmax());
        let y0 = self.ymin().max(other.ymin());
        let y1 = self.ymax().min(other.ymax());
        (x0 <= x1 && y0 <= y1).then(|| Self::from_extents(x0, y0, x1, y1))
    }

    /// Grows the box by `dx` on the left/right and `dy` on the bottom/top
    /// (matplotlib's `padded`).
    #[must_use]
    pub fn padded(&self, dx: f64, dy: f64) -> Self {
        Self::new(self.p0 - Point::new(dx, dy), self.p1 + Point::new(dx, dy))
    }

    /// Scales the box about its centre by `sw` horizontally and `sh` vertically.
    #[must_use]
    pub fn expanded(&self, sw: f64, sh: f64) -> Self {
        let delta = Point::new(
            (sw * self.width() - self.width()) / 2.0,
            (sh * self.height() - self.height()) / 2.0,
        );
        Self::new(self.p0 - delta, self.p1 + delta)
    }

    /// Keeps the first corner and scales the size by `(mx, my)`.
    #[must_use]
    pub fn shrunk(&self, mx: f64, my: f64) -> Self {
        Self::new(
            self.p0,
            self.p0 + Point::new(mx * self.width(), my * self.height()),
        )
    }

    /// The largest box with aspect ratio `box_aspect` (height / width, in a
    /// space whose height-to-width ratio is `fig_aspect`) that fits inside
    /// `container` (defaults to `self`). The result keeps `self`'s first
    /// corner. Returns `None` unless both aspects are positive.
    pub fn shrunk_to_aspect(
        &self,
        box_aspect: f64,
        container: Option<&Self>,
        fig_aspect: f64,
    ) -> Option<Self> {
        if box_aspect <= 0.0 || fig_aspect <= 0.0 {
            return None;
        }
        let container = container.unwrap_or(self);
        let (w, h) = (container.width(), container.height());
        let fitted_h = w * box_aspect / fig_aspect;
        let (width, height) = if fitted_h <= h {
            (w, fitted_h)
        } else {
            (h * fig_aspect / box_aspect, h)
        };
        Some(Self::new(self.p0, self.p0 + Point::new(width, height)))
    }

    /// Moves the box (keeping its size) to `anchor` within `container`.
    #[must_use]
    pub fn anchored(&self, anchor: Anchor, container: &Self) -> Self {
        let (l, b, w, h) = container.bounds();
        let (left, bottom, width, height) = self.bounds();
        let (cx, cy) = anchor.coefficients();
        self.translated(
            (l + cx * (w - width)) - left,
            (b + cy * (h - height)) - bottom,
        )
    }

    /// The box shifted by `(tx, ty)`.
    #[must_use]
    pub fn translated(&self, tx: f64, ty: f64) -> Self {
        let delta = Point::new(tx, ty);
        Self::new(self.p0 + delta, self.p1 + delta)
    }

    /// Splits the box at fractional x positions (`fractions` in `[0, 1]`).
    pub fn split_x(&self, fractions: &[f64]) -> Vec<Self> {
        let edges = Self::split_edges(self.x0(), self.x1(), fractions);
        edges
            .windows(2)
            .map(|w| Self::from_extents(w[0], self.y0(), w[1], self.y1()))
            .collect()
    }

    /// Splits the box at fractional y positions (`fractions` in `[0, 1]`).
    pub fn split_y(&self, fractions: &[f64]) -> Vec<Self> {
        let edges = Self::split_edges(self.y0(), self.y1(), fractions);
        edges
            .windows(2)
            .map(|w| Self::from_extents(self.x0(), w[0], self.x1(), w[1]))
            .collect()
    }

    fn split_edges(start: f64, end: f64, fractions: &[f64]) -> Vec<f64> {
        let span = end - start;
        std::iter::once(start)
            .chain(fractions.iter().map(|f| start + f * span))
            .chain(std::iter::once(end))
            .collect()
    }

    /// Transforms the box by an affine map.
    ///
    /// Matches matplotlib's `Bbox.transformed`: the lower-left, upper-left and
    /// lower-right corners are mapped, and the result spans from the image of
    /// the lower-left corner to `(lr.x, ul.y)`. For rotations this is not the
    /// enclosing box — use [`transformed_hull`](Self::transformed_hull).
    #[must_use]
    pub fn transformed(&self, t: &Affine2D) -> Self {
        let ll = t.transform_point(self.p0);
        let ul = t.transform_point(Point::new(self.p0.x, self.p1.y));
        let lr = t.transform_point(Point::new(self.p1.x, self.p0.y));
        Self::new(ll, Point::new(lr.x, ul.y))
    }

    /// The ordered box enclosing all four transformed corners.
    #[must_use]
    pub fn transformed_hull(&self, t: &Affine2D) -> Self {
        Self::from_points(self.corners().map(|p| t.transform_point(p)))
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
    fn anchoring_and_shrinking() {
        let container = Bbox::from_bounds(0.0, 0.0, 10.0, 10.0);
        let small = Bbox::from_bounds(0.0, 0.0, 2.0, 4.0);
        assert_eq!(
            small.anchored(Anchor::C, &container),
            Bbox::from_bounds(4.0, 3.0, 2.0, 4.0)
        );
        assert_eq!(
            small.anchored(Anchor::NE, &container),
            Bbox::from_bounds(8.0, 6.0, 2.0, 4.0)
        );
        assert_eq!(
            container.shrunk(0.5, 0.25),
            Bbox::from_bounds(0.0, 0.0, 5.0, 2.5)
        );
    }

    #[test]
    fn expanded_and_padded() {
        let b = Bbox::from_bounds(0.0, 0.0, 2.0, 2.0);
        assert_eq!(
            b.expanded(2.0, 1.0),
            Bbox::from_extents(-1.0, 0.0, 3.0, 2.0)
        );
        assert_eq!(b.padded(1.0, 0.5), Bbox::from_extents(-1.0, -0.5, 3.0, 2.5));
    }

    #[test]
    fn shrink_to_aspect() {
        let b = Bbox::from_bounds(1.0, 1.0, 4.0, 2.0);
        assert_eq!(
            b.shrunk_to_aspect(1.0, None, 1.0),
            Some(Bbox::from_bounds(1.0, 1.0, 2.0, 2.0))
        );
        assert_eq!(
            b.shrunk_to_aspect(0.25, None, 1.0),
            Some(Bbox::from_bounds(1.0, 1.0, 4.0, 1.0))
        );
        assert_eq!(b.shrunk_to_aspect(0.0, None, 1.0), None);
    }

    #[test]
    fn split() {
        let b = Bbox::from_extents(0.0, 0.0, 10.0, 1.0);
        let parts = b.split_x(&[0.2, 0.5]);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], Bbox::from_extents(2.0, 0.0, 5.0, 1.0));
    }

    #[test]
    fn overlap_tests_ignore_orientation() {
        let a = Bbox::from_extents(2.0, 0.0, 0.0, 2.0);
        let b = Bbox::from_extents(2.0, 2.0, 3.0, 3.0);
        assert!(a.overlaps(&b));
        assert!(!a.fully_overlaps(&b));
    }

    #[test]
    fn points_and_null() {
        assert!(Bbox::NULL.is_null());
        let b = Bbox::from_points([Point::new(1.0, 5.0), Point::new(-1.0, 2.0)]);
        assert_eq!(b, Bbox::from_extents(-1.0, 2.0, 1.0, 5.0));
        assert_eq!(b.including(Point::new(f64::NAN, 0.0)), b);
    }

    #[test]
    fn transformed_matches_matplotlib_corner_rule() {
        let b = Bbox::from_extents(0.0, 0.0, 1.0, 2.0);
        let t = Affine2D::scaling(2.0, -1.0).translate(1.0, 0.0);
        assert_eq!(b.transformed(&t), Bbox::from_extents(1.0, 0.0, 3.0, -2.0));
    }
}
