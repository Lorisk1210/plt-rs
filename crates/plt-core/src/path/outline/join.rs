//! The part of Agg's `math_stroke` that matplotlib's containment tests use:
//! miter joins, inner miter joins and butt caps (Agg's defaults).

use super::math::{calc_distance, calc_intersection, cross_product};
use super::vertex_seq::{VERTEX_DIST_EPSILON, VertexDist};
use crate::Point;

/// Agg's default miter limit.
const MITER_LIMIT: f64 = 4.0;
/// Agg's default inner miter limit.
const INNER_MITER_LIMIT: f64 = 1.01;

/// What a miter beyond its limit becomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MiterOverflow {
    /// Agg's `miter_join`: a clipped miter.
    Clip,
    /// Agg's `miter_join_revert`: a bevel.
    Bevel,
}

/// Offset geometry for a given (signed) line width.
#[derive(Clone, Copy, Debug)]
pub(super) struct JoinMath {
    width: f64,
    width_abs: f64,
    width_sign: f64,
}

impl JoinMath {
    /// `math_stroke::width`: offsets are half the given width, on the right
    /// of the direction of travel for positive widths.
    pub(super) fn new(width: f64) -> Self {
        let width = width * 0.5;
        Self {
            width,
            width_abs: width.abs(),
            width_sign: if width < 0.0 { -1.0 } else { 1.0 },
        }
    }

    /// `calc_cap` for butt caps: the two offset points at `v0` of the
    /// segment `v0 → v1` of length `len`.
    pub(super) fn butt_cap(
        &self,
        out: &mut Vec<Point>,
        v0: &VertexDist,
        v1: &VertexDist,
        len: f64,
    ) {
        out.clear();
        let dx1 = (v1.y - v0.y) / len * self.width;
        let dy1 = (v1.x - v0.x) / len * self.width;
        out.push(Point::new(v0.x - dx1, v0.y + dy1));
        out.push(Point::new(v0.x + dx1, v0.y - dy1));
    }

    /// `calc_join` with miter outer joins and inner miters, at `v1` between
    /// `v0 → v1` (length `len1`) and `v1 → v2` (length `len2`).
    pub(super) fn join(
        &self,
        out: &mut Vec<Point>,
        v0: &VertexDist,
        v1: &VertexDist,
        v2: &VertexDist,
        len1: f64,
        len2: f64,
    ) {
        let dx1 = self.width * (v1.y - v0.y) / len1;
        let dy1 = self.width * (v1.x - v0.x) / len1;
        let dx2 = self.width * (v2.y - v1.y) / len2;
        let dy2 = self.width * (v2.x - v1.x) / len2;
        let offsets = [dx1, dy1, dx2, dy2];
        out.clear();
        let cp = cross_product(v0.point(), v1.point(), v2.point());
        if (cp > VERTEX_DIST_EPSILON && self.width > 0.0)
            || (cp < -VERTEX_DIST_EPSILON && self.width < 0.0)
        {
            let limit = (len1.min(len2) / self.width_abs).max(INNER_MITER_LIMIT);
            self.miter(out, [v0, v1, v2], offsets, MiterOverflow::Bevel, limit, 0.0);
        } else {
            let dx = f64::midpoint(dx1, dx2);
            let dy = f64::midpoint(dy1, dy2);
            let dbevel = (dx * dx + dy * dy).sqrt();
            self.miter(
                out,
                [v0, v1, v2],
                offsets,
                MiterOverflow::Clip,
                MITER_LIMIT,
                dbevel,
            );
        }
    }

    /// `calc_miter`.
    fn miter(
        &self,
        out: &mut Vec<Point>,
        [v0, v1, v2]: [&VertexDist; 3],
        [dx1, dy1, dx2, dy2]: [f64; 4],
        overflow: MiterOverflow,
        limit: f64,
        dbevel: f64,
    ) {
        let lim = self.width_abs * limit;
        let mut limit_exceeded = true;
        let mut intersection = None;
        let mut di = 1.0;
        if let Some(xi) = calc_intersection(
            Point::new(v0.x + dx1, v0.y - dy1),
            Point::new(v1.x + dx1, v1.y - dy1),
            Point::new(v1.x + dx2, v1.y - dy2),
            Point::new(v2.x + dx2, v2.y - dy2),
        ) {
            di = calc_distance(v1.point(), xi);
            if di <= lim {
                out.push(xi);
                limit_exceeded = false;
            }
            intersection = Some(xi);
        } else {
            // Collinear: either the path continues straight on (one offset
            // point suffices) or it turns back on itself.
            let p = Point::new(v1.x + dx1, v1.y - dy1);
            if (cross_product(v0.point(), v1.point(), p) < 0.0)
                == (cross_product(v1.point(), v2.point(), p) < 0.0)
            {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                limit_exceeded = false;
            }
        }
        if !limit_exceeded {
            return;
        }
        match (overflow, intersection) {
            (MiterOverflow::Bevel, _) => {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                out.push(Point::new(v1.x + dx2, v1.y - dy2));
            }
            (MiterOverflow::Clip, None) => {
                let mlimit = limit * self.width_sign;
                out.push(Point::new(
                    v1.x + dx1 + dy1 * mlimit,
                    v1.y - dy1 + dx1 * mlimit,
                ));
                out.push(Point::new(
                    v1.x + dx2 - dy2 * mlimit,
                    v1.y - dy2 - dx2 * mlimit,
                ));
            }
            (MiterOverflow::Clip, Some(xi)) => {
                let x1 = v1.x + dx1;
                let y1 = v1.y - dy1;
                let x2 = v1.x + dx2;
                let y2 = v1.y - dy2;
                let di = (lim - dbevel) / (di - dbevel);
                out.push(Point::new(x1 + (xi.x - x1) * di, y1 + (xi.y - y1) * di));
                out.push(Point::new(x2 + (xi.x - x2) * di, y2 + (xi.y - y2) * di));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vd(x: f64, y: f64) -> VertexDist {
        VertexDist::new(Point::new(x, y))
    }

    #[test]
    fn caps_and_right_angle_joins() {
        let m = JoinMath::new(2.0);
        let mut out = Vec::new();
        m.butt_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out, vec![Point::new(0.0, 1.0), Point::new(0.0, -1.0)]);
        let (a, b) = (vd(0.0, 0.0), vd(10.0, 0.0));
        m.join(&mut out, &a, &b, &vd(10.0, 10.0), 10.0, 10.0);
        assert_eq!(out, vec![Point::new(11.0, -1.0)]);
        m.join(&mut out, &a, &b, &vd(10.0, -10.0), 10.0, 10.0);
        assert_eq!(out, vec![Point::new(9.0, -1.0)]);
    }

    #[test]
    fn sharp_outer_miters_are_clipped() {
        let m = JoinMath::new(2.0);
        let mut out = Vec::new();
        let len2 = 101f64.sqrt();
        m.join(
            &mut out,
            &vd(0.0, 0.0),
            &vd(10.0, 0.0),
            &vd(0.0, 1.0),
            10.0,
            len2,
        );
        assert_eq!(out.len(), 2);
    }
}
