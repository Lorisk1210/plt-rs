//! Boxes derived from a box: padding, scaling, anchoring, splitting and
//! transforming (matplotlib's `BboxBase` methods returning a new `Bbox`).

use super::{Anchor, Bbox};
use crate::numeric::{py_max, py_min};
use crate::{Affine2D, Point};

impl Bbox {
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

    /// Splits the box at fractional x positions (matplotlib's `splitx`).
    ///
    /// Every edge, including the outer ones, is `x0 + f * width` for
    /// `f` in `[0, *fractions, 1]`, so fractions outside `[0, 1]` or out of
    /// order produce overlapping or inverted pieces.
    pub fn split_x(&self, fractions: &[f64]) -> Vec<Self> {
        let edges = Self::split_edges(self.x0(), self.x1(), fractions);
        edges
            .windows(2)
            .map(|w| Self::from_extents(w[0], self.y0(), w[1], self.y1()))
            .collect()
    }

    /// Splits the box at fractional y positions (matplotlib's `splity`; see
    /// [`split_x`](Self::split_x)).
    pub fn split_y(&self, fractions: &[f64]) -> Vec<Self> {
        let edges = Self::split_edges(self.y0(), self.y1(), fractions);
        edges
            .windows(2)
            .map(|w| Self::from_extents(self.x0(), w[0], self.x1(), w[1]))
            .collect()
    }

    fn split_edges(start: f64, end: f64, fractions: &[f64]) -> Vec<f64> {
        let span = end - start;
        std::iter::once(0.0)
            .chain(fractions.iter().copied())
            .chain(std::iter::once(1.0))
            .map(|f| start + f * span)
            .collect()
    }

    /// Transforms the box by an affine map.
    ///
    /// Matches matplotlib's `Bbox.transformed`: the lower-left, upper-left and
    /// lower-right corners are mapped, and the result spans from the image of
    /// the lower-left corner to `(lr.x, ul.y)`. For rotations this is not the
    /// enclosing box — use [`transformed_enclosing`](Self::transformed_enclosing).
    #[must_use]
    pub fn transformed(&self, t: &Affine2D) -> Self {
        let ll = t.transform_point(self.p0);
        let ul = t.transform_point(Point::new(self.p0.x, self.p1.y));
        let lr = t.transform_point(Point::new(self.p1.x, self.p0.y));
        Self::new(ll, Point::new(lr.x, ul.y))
    }

    /// The box enclosing all four transformed corners, keeping the input's
    /// orientation (matplotlib's `TransformedBbox`).
    #[must_use]
    pub fn transformed_enclosing(&self, t: &Affine2D) -> Self {
        self.mapped_enclosing(|p| t.transform_point(p))
    }

    /// The box enclosing all four corners mapped by `map`, keeping the
    /// input's orientation (matplotlib's `TransformedBbox` through any
    /// transform, which maps only the corners).
    ///
    /// ```
    /// use pltrs_core::{Bbox, Point};
    /// let squared = Bbox::from_extents(1.0, 2.0, 3.0, 4.0)
    ///     .mapped_enclosing(|p| Point::new(p.x * p.x, p.y * p.y));
    /// assert_eq!(squared, Bbox::from_extents(1.0, 4.0, 9.0, 16.0));
    /// ```
    #[must_use]
    pub fn mapped_enclosing(&self, map: impl Fn(Point) -> Point) -> Self {
        let corners = [
            self.p0,
            Point::new(self.p1.x, self.p0.y),
            Point::new(self.p0.x, self.p1.y),
            self.p1,
        ]
        .map(map);
        // Python's builtin `min` and `max` of the corners, so a NaN
        // behaves according to its position.
        let span = |coord: fn(&Point) -> f64, flip: bool| {
            let [first, rest @ ..] = corners.map(|p| coord(&p));
            let lo = rest.iter().fold(first, |current, &v| py_min(current, v));
            let hi = rest.iter().fold(first, |current, &v| py_max(current, v));
            if flip { (hi, lo) } else { (lo, hi) }
        };
        let (x0, x1) = span(|p| p.x, self.p0.x > self.p1.x);
        let (y0, y1) = span(|p| p.y, self.p0.y > self.p1.y);
        Self::from_extents(x0, y0, x1, y1)
    }

    /// The box rotated by `radians` about the origin, as the bounding box
    /// of its finite rotated corners (matplotlib's `Bbox.rotated`), or
    /// [`Bbox::NULL`] when no rotated corner is finite.
    #[must_use]
    pub fn rotated(&self, radians: f64) -> Self {
        let rotation = Affine2D::rotation(radians);
        let corners = self.corners().map(|p| rotation.transform_point(p));
        Self::from_points(corners.into_iter().filter(|p| p.is_finite()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(b.split_y(&[]), vec![b]);
    }

    #[test]
    fn split_edges_are_computed_from_the_first_corner() {
        // matplotlib's last edge is x0 + 1 * width, which need not be x1.
        let b = Bbox::from_extents(0.2, 0.0, 0.9, 1.0);
        let last = b.split_x(&[0.5])[1];
        assert_eq!(last.x1(), 0.2 + (0.9 - 0.2));
        assert_ne!(last.x1(), 0.9);
        let inverted = Bbox::from_extents(5.0, 4.0, 1.0, 2.0).split_y(&[0.75, 0.25]);
        assert_eq!(inverted[1], Bbox::from_extents(5.0, 2.5, 1.0, 3.5));
    }

    #[test]
    fn rotation_bounds_finite_corners() {
        let quarter = Bbox::UNIT.rotated(std::f64::consts::FRAC_PI_2);
        assert!((quarter.x0() + 1.0).abs() < 1e-15 && (quarter.y1() - 1.0).abs() < 1e-15);
        let half_infinite = Bbox::from_extents(0.0, 0.0, f64::INFINITY, 1.0).rotated(0.0);
        assert_eq!(half_infinite, Bbox::from_extents(0.0, 0.0, 0.0, 1.0));
        let all_infinite = Bbox::from_extents(f64::INFINITY, 0.0, f64::INFINITY, 1.0);
        assert!(all_infinite.rotated(0.3).is_null());
    }

    #[test]
    fn transformed_matches_matplotlib_corner_rule() {
        let b = Bbox::from_extents(0.0, 0.0, 1.0, 2.0);
        let t = Affine2D::scaling(2.0, -1.0).translate(1.0, 0.0);
        assert_eq!(b.transformed(&t), Bbox::from_extents(1.0, 0.0, 3.0, -2.0));
    }

    #[test]
    fn enclosing_box_bounds_all_corners_in_the_input_orientation() {
        let rotation = Affine2D::identity().rotate_deg(45.0);
        let enclosing = Bbox::UNIT.transformed_enclosing(&rotation);
        assert!((enclosing.x0() + 0.5_f64.sqrt()).abs() < 1e-15);
        assert!((enclosing.y1() - 2.0_f64.sqrt()).abs() < 1e-15);
        let inverted = Bbox::from_extents(4.0, 3.0, 1.0, 2.0);
        assert_eq!(
            inverted.transformed_enclosing(&Affine2D::scaling(2.0, 2.0)),
            Bbox::from_extents(8.0, 6.0, 2.0, 4.0)
        );
        // The first corner maps to (NaN, 0 * NaN), and Python's min/max
        // keep a leading NaN.
        let nan = Bbox::from_extents(f64::NAN, 0.0, 1.0, 1.0);
        let scaled = nan.transformed_enclosing(&Affine2D::scaling(2.0, 3.0));
        assert!(
            [scaled.x0(), scaled.y0(), scaled.x1(), scaled.y1()]
                .iter()
                .all(|v| v.is_nan())
        );
        // A trailing NaN is skipped instead.
        let late_nan = Bbox::from_extents(0.0, 0.0, 1.0, f64::NAN);
        let shifted = late_nan.transformed_enclosing(&Affine2D::translation(1.0, 0.0));
        assert_eq!(shifted.interval_x(), (1.0, 2.0));
    }
}
