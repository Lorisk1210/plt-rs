//! Path/path and path/rectangle intersection tests (matplotlib's
//! `_path.path_intersects_path` and `path_intersects_rectangle`).

use super::Path;
use super::convert::VertexSource;
use crate::{Affine2D, Bbox, Point};

/// matplotlib's `isclose` (relative 1e-10, absolute 1e-13).
fn isclose(a: f64, b: f64) -> bool {
    (a - b).abs() <= (1e-10 * a.abs().max(b.abs())).max(1e-13)
}

fn squared_length(a: Point, b: Point) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

/// matplotlib's `segments_intersect` for `p1 → p2` and `p3 → p4`, with
/// tolerant end-point and collinearity checks.
#[expect(
    clippy::float_cmp,
    reason = "matplotlib detects vertical segments exactly"
)]
fn segments_intersect(p1: Point, p2: Point, p3: Point, p4: Point) -> bool {
    let (x1, y1, x2, y2) = (p1.x, p1.y, p2.x, p2.y);
    let (x3, y3, x4, y4) = (p3.x, p3.y, p4.x, p4.y);
    let den = ((y4 - y3) * (x2 - x1)) - ((x4 - x3) * (y2 - y1));
    if isclose(den, 0.0) {
        // Parallel: only collinear segments can meet, by overlapping.
        let t_area = (x2 * y3 - x3 * y2) - x1 * (y3 - y2) + y1 * (x3 - x2);
        if !isclose(t_area, 0.0) {
            return false;
        }
        let overlap = |a1: f64, a2: f64, b1: f64, b2: f64| {
            (a1.min(a2) <= b1.min(b2) && b1.min(b2) <= a1.max(a2))
                || (b1.min(b2) <= a1.min(a2) && a1.min(a2) <= b1.max(b2))
        };
        return if x1 == x2 && x2 == x3 {
            overlap(y1, y2, y3, y4)
        } else {
            overlap(x1, x2, x3, x4)
        };
    }
    let n1 = ((x4 - x3) * (y1 - y3)) - ((y4 - y3) * (x1 - x3));
    let n2 = ((x2 - x1) * (y1 - y3)) - ((y2 - y1) * (x1 - x3));
    let u1 = n1 / den;
    let u2 = n2 / den;
    let in_unit = |u: f64| (u > 0.0 || isclose(u, 0.0)) && (u < 1.0 || isclose(u, 1.0));
    in_unit(u1) && in_unit(u2)
}

/// Whether `a → b` meets the rectangle centred at `c` of size `w × h`
/// (matplotlib's `segment_intersects_rectangle`).
fn segment_intersects_rectangle(a: Point, b: Point, c: Point, w: f64, h: f64) -> bool {
    (a.x + b.x - 2.0 * c.x).abs() < w + (a.x - b.x).abs()
        && (a.y + b.y - 2.0 * c.y).abs() < h + (a.y - b.y).abs()
        && 2.0 * ((a.x - c.x) * (a.y - b.y) - (a.y - c.y) * (a.x - b.x)).abs()
            < w * (a.y - b.y).abs() + h * (a.x - b.x).abs()
}

impl Path {
    /// The untransformed, NaN-free, flattened stream walked by the
    /// intersection tests.
    fn intersection_stream(&self) -> impl VertexSource + '_ {
        self.iter_vertices()
            .remove_nans(true, self.codes.is_some())
            .flatten_curves(true)
    }

    /// matplotlib's `Path.intersects_path`.
    ///
    /// Both paths are cleaned of non-finite vertices, flattened and walked
    /// as single polylines: moves and the stored coordinates of closures act
    /// as ordinary segment end points, and (nearly) zero-length segments are
    /// skipped. With `filled`, a path containing the other also counts.
    pub fn intersects_path(&self, other: &Self, filled: bool) -> bool {
        self.outlines_intersect(other)
            || (filled && (self.contains_path(other, None) || other.contains_path(self, None)))
    }

    fn outlines_intersect(&self, other: &Self) -> bool {
        if self.len() < 2 || other.len() < 2 {
            return false;
        }
        let mut first = self.intersection_stream();
        let mut second = other.intersection_stream();
        let mut p11 = first.next().map_or(Point::ORIGIN, |v| v.point);
        for v in first {
            let p12 = v.point;
            if isclose(squared_length(p11, p12), 0.0) {
                continue;
            }
            second.rewind();
            let mut p21 = second.next().map_or(Point::ORIGIN, |v| v.point);
            for w in second.by_ref() {
                let p22 = w.point;
                if isclose(squared_length(p21, p22), 0.0) {
                    continue;
                }
                if segments_intersect(p11, p12, p21, p22) {
                    return true;
                }
                p21 = p22;
            }
            p11 = p12;
        }
        false
    }

    /// matplotlib's `Path.intersects_bbox`: whether any (flattened,
    /// NaN-free) segment of the path meets the box, or the path's first
    /// vertex lies in it; with `filled`, also whether the path contains the
    /// box's centre.
    pub fn intersects_bbox(&self, bbox: &Bbox, filled: bool) -> bool {
        if self.is_empty() {
            return false;
        }
        let c = Point::new(
            f64::midpoint(bbox.x0(), bbox.x1()),
            f64::midpoint(bbox.y0(), bbox.y1()),
        );
        let w = (bbox.x0() - bbox.x1()).abs();
        let h = (bbox.y0() - bbox.y1()).abs();
        let mut stream = self.intersection_stream();
        let mut a = stream.next().map_or(Point::ORIGIN, |v| v.point);
        if 2.0 * (a.x - c.x).abs() <= w && 2.0 * (a.y - c.y).abs() <= h {
            return true;
        }
        for v in stream {
            if segment_intersects_rectangle(a, v.point, c, w, h) {
                return true;
            }
            a = v.point;
        }
        filled && self.contains_point(c, Some(&Affine2D::IDENTITY), 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn square_at(x: f64, y: f64, s: f64) -> Path {
        Path::polygon([p(x, y), p(x + s, y), p(x + s, y + s), p(x, y + s)])
    }

    #[test]
    fn segment_tests() {
        assert!(segments_intersect(
            p(0.0, 0.0),
            p(2.0, 2.0),
            p(0.0, 2.0),
            p(2.0, 0.0)
        ));
        assert!(segments_intersect(
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 2.0)
        ));
        assert!(!segments_intersect(
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(0.0, 1.0),
            p(1.0, 1.0)
        ));
        assert!(segments_intersect(
            p(0.0, 0.0),
            p(2.0, 0.0),
            p(1.0, 0.0),
            p(3.0, 0.0)
        ));
        assert!(!segments_intersect(
            p(0.0, 0.0),
            p(0.0, 1.0),
            p(0.0, 2.0),
            p(0.0, 3.0)
        ));
    }

    #[test]
    fn filled_containment_counts_as_intersection() {
        let outer = square_at(0.0, 0.0, 1.0);
        let inner = square_at(0.25, 0.25, 0.5);
        assert!(!outer.intersects_path(&inner, false));
        assert!(outer.intersects_path(&inner, true));
        assert!(inner.intersects_path(&outer, true));
        assert!(!outer.intersects_path(&square_at(2.0, 2.0, 1.0), true));
        assert!(outer.intersects_path(&square_at(0.5, 0.5, 1.0), false));
    }

    #[test]
    fn bbox_tests() {
        let sq = square_at(0.0, 0.0, 1.0);
        let inner = Bbox::from_extents(0.25, 0.25, 0.75, 0.75);
        assert!(sq.intersects_bbox(&inner, true));
        assert!(!sq.intersects_bbox(&inner, false));
        assert!(sq.intersects_bbox(&Bbox::from_extents(0.5, 0.5, 2.0, 2.0), false));
        assert!(!sq.intersects_bbox(&Bbox::from_extents(2.0, 2.0, 3.0, 3.0), true));
        assert!(!Path::default().intersects_bbox(&inner, true));
    }
}
