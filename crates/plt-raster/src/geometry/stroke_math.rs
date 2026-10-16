//! Caps and joins — a port of Agg's `math_stroke`.
//!
//! Offsets are computed with the same floating-point expressions in the
//! same order as the C++ code so outlines match vertex for vertex.

use std::f64::consts::PI;

use plt_core::Point;

use super::vertex_seq::{VERTEX_DIST_EPSILON, VertexDist};
use crate::style::{InnerJoin, LineCap, LineJoin, StrokeStyle};

const INTERSECTION_EPSILON: f64 = 1.0e-30;

/// Agg's `cross_product`: which side of the line `1 → 2` the point lies on.
fn cross_product(x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> f64 {
    (x - x2) * (y2 - y1) - (y - y2) * (x2 - x1)
}

/// Intersection of lines `a → b` and `c → d` (Agg's `calc_intersection`).
#[allow(clippy::too_many_arguments)] // Mirrors Agg's signature one to one.
pub(crate) fn calc_intersection(
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    cx: f64,
    cy: f64,
    dx: f64,
    dy: f64,
) -> Option<(f64, f64)> {
    let num = (ay - cy) * (dx - cx) - (ax - cx) * (dy - cy);
    let den = (bx - ax) * (dy - cy) - (by - ay) * (dx - cx);
    if den.abs() < INTERSECTION_EPSILON {
        return None;
    }
    let r = num / den;
    Some((ax + r * (bx - ax), ay + r * (by - ay)))
}

fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    (dx * dx + dy * dy).sqrt()
}

/// Precomputed stroke parameters.
#[derive(Clone, Debug)]
pub(super) struct StrokeMath {
    width: f64,
    width_abs: f64,
    width_eps: f64,
    width_sign: f64,
    miter_limit: f64,
    inner_miter_limit: f64,
    approx_scale: f64,
    cap: LineCap,
    join: LineJoin,
    inner_join: InnerJoin,
}

impl StrokeMath {
    pub(super) fn new(style: &StrokeStyle) -> Self {
        let width = style.width * 0.5;
        Self {
            width,
            width_abs: width.abs(),
            width_eps: width / 1024.0,
            width_sign: if width < 0.0 { -1.0 } else { 1.0 },
            miter_limit: style.miter_limit,
            inner_miter_limit: style.inner_miter_limit,
            approx_scale: style.approximation_scale,
            cap: style.cap,
            join: style.join,
            inner_join: style.inner_join,
        }
    }

    /// Angular step for round caps and joins.
    fn arc_step(&self) -> f64 {
        (self.width_abs / (self.width_abs + 0.125 / self.approx_scale)).acos() * 2.0
    }

    #[allow(clippy::too_many_arguments)] // Mirrors Agg's signature one to one.
    fn calc_arc(
        &self,
        out: &mut Vec<Point>,
        x: f64,
        y: f64,
        dx1: f64,
        dy1: f64,
        dx2: f64,
        dy2: f64,
    ) {
        let mut a1 = (dy1 * self.width_sign).atan2(dx1 * self.width_sign);
        let mut a2 = (dy2 * self.width_sign).atan2(dx2 * self.width_sign);
        let da = self.arc_step();
        out.push(Point::new(x + dx1, y + dy1));
        if self.width_sign > 0.0 {
            if a1 > a2 {
                a2 += 2.0 * PI;
            }
            let n = ((a2 - a1) / da) as i32;
            let da = (a2 - a1) / f64::from(n + 1);
            a1 += da;
            for _ in 0..n {
                out.push(Point::new(
                    x + a1.cos() * self.width,
                    y + a1.sin() * self.width,
                ));
                a1 += da;
            }
        } else {
            if a1 < a2 {
                a2 -= 2.0 * PI;
            }
            let n = ((a1 - a2) / da) as i32;
            let da = (a1 - a2) / f64::from(n + 1);
            a1 -= da;
            for _ in 0..n {
                out.push(Point::new(
                    x + a1.cos() * self.width,
                    y + a1.sin() * self.width,
                ));
                a1 -= da;
            }
        }
        out.push(Point::new(x + dx2, y + dy2));
    }

    #[allow(clippy::too_many_arguments)] // Mirrors Agg's signature one to one.
    fn calc_miter(
        &self,
        out: &mut Vec<Point>,
        v0: &VertexDist,
        v1: &VertexDist,
        v2: &VertexDist,
        [dx1, dy1, dx2, dy2]: [f64; 4],
        join: LineJoin,
        mlimit: f64,
        dbevel: f64,
    ) {
        let lim = self.width_abs * mlimit;
        let mut limit_exceeded = true;
        let mut intersection_failed = true;
        let mut xi = v1.x;
        let mut yi = v1.y;
        let mut di = 1.0;
        if let Some((x, y)) = calc_intersection(
            v0.x + dx1,
            v0.y - dy1,
            v1.x + dx1,
            v1.y - dy1,
            v1.x + dx2,
            v1.y - dy2,
            v2.x + dx2,
            v2.y - dy2,
        ) {
            xi = x;
            yi = y;
            di = distance(v1.x, v1.y, xi, yi);
            if di <= lim {
                out.push(Point::new(xi, yi));
                limit_exceeded = false;
            }
            intersection_failed = false;
        } else {
            // Collinear points: either the path continues straight on (one
            // offset point suffices) or it turns back on itself.
            let x2 = v1.x + dx1;
            let y2 = v1.y - dy1;
            if (cross_product(v0.x, v0.y, v1.x, v1.y, x2, y2) < 0.0)
                == (cross_product(v1.x, v1.y, v2.x, v2.y, x2, y2) < 0.0)
            {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                limit_exceeded = false;
            }
        }
        if !limit_exceeded {
            return;
        }
        match join {
            LineJoin::MiterRevert => {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                out.push(Point::new(v1.x + dx2, v1.y - dy2));
            }
            LineJoin::MiterRound => self.calc_arc(out, v1.x, v1.y, dx1, -dy1, dx2, -dy2),
            _ => {
                if intersection_failed {
                    let mlimit = mlimit * self.width_sign;
                    out.push(Point::new(
                        v1.x + dx1 + dy1 * mlimit,
                        v1.y - dy1 + dx1 * mlimit,
                    ));
                    out.push(Point::new(
                        v1.x + dx2 - dy2 * mlimit,
                        v1.y - dy2 - dx2 * mlimit,
                    ));
                } else {
                    let x1 = v1.x + dx1;
                    let y1 = v1.y - dy1;
                    let x2 = v1.x + dx2;
                    let y2 = v1.y - dy2;
                    let di = (lim - dbevel) / (di - dbevel);
                    out.push(Point::new(x1 + (xi - x1) * di, y1 + (yi - y1) * di));
                    out.push(Point::new(x2 + (xi - x2) * di, y2 + (yi - y2) * di));
                }
            }
        }
    }

    /// The cap at `v0` of the segment `v0 → v1` of length `len`.
    pub(super) fn calc_cap(
        &self,
        out: &mut Vec<Point>,
        v0: &VertexDist,
        v1: &VertexDist,
        len: f64,
    ) {
        out.clear();
        let mut dx1 = (v1.y - v0.y) / len;
        let mut dy1 = (v1.x - v0.x) / len;
        dx1 *= self.width;
        dy1 *= self.width;
        if self.cap == LineCap::Round {
            let da = self.arc_step();
            let n = (PI / da) as i32;
            let da = PI / f64::from(n + 1);
            out.push(Point::new(v0.x - dx1, v0.y + dy1));
            if self.width_sign > 0.0 {
                let mut a1 = dy1.atan2(-dx1) + da;
                for _ in 0..n {
                    out.push(Point::new(
                        v0.x + a1.cos() * self.width,
                        v0.y + a1.sin() * self.width,
                    ));
                    a1 += da;
                }
            } else {
                let mut a1 = (-dy1).atan2(dx1) - da;
                for _ in 0..n {
                    out.push(Point::new(
                        v0.x + a1.cos() * self.width,
                        v0.y + a1.sin() * self.width,
                    ));
                    a1 -= da;
                }
            }
            out.push(Point::new(v0.x + dx1, v0.y - dy1));
        } else {
            let (dx2, dy2) = if self.cap == LineCap::Square {
                (dy1 * self.width_sign, dx1 * self.width_sign)
            } else {
                (0.0, 0.0)
            };
            out.push(Point::new(v0.x - dx1 - dx2, v0.y + dy1 - dy2));
            out.push(Point::new(v0.x + dx1 - dx2, v0.y - dy1 - dy2));
        }
    }

    /// The join at `v1` between segments `v0 → v1` (length `len1`) and
    /// `v1 → v2` (length `len2`).
    pub(super) fn calc_join(
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

        let cp = cross_product(v0.x, v0.y, v1.x, v1.y, v2.x, v2.y);
        if (cp > VERTEX_DIST_EPSILON && self.width > 0.0)
            || (cp < -VERTEX_DIST_EPSILON && self.width < 0.0)
        {
            // Inner join.
            let limit = (len1.min(len2) / self.width_abs).max(self.inner_miter_limit);
            match self.inner_join {
                InnerJoin::Bevel => {
                    out.push(Point::new(v1.x + dx1, v1.y - dy1));
                    out.push(Point::new(v1.x + dx2, v1.y - dy2));
                }
                InnerJoin::Miter => {
                    self.calc_miter(out, v0, v1, v2, offsets, LineJoin::MiterRevert, limit, 0.0);
                }
                InnerJoin::Jag | InnerJoin::Round => {
                    let d = (dx1 - dx2) * (dx1 - dx2) + (dy1 - dy2) * (dy1 - dy2);
                    if d < len1 * len1 && d < len2 * len2 {
                        self.calc_miter(
                            out,
                            v0,
                            v1,
                            v2,
                            offsets,
                            LineJoin::MiterRevert,
                            limit,
                            0.0,
                        );
                    } else if self.inner_join == InnerJoin::Jag {
                        out.push(Point::new(v1.x + dx1, v1.y - dy1));
                        out.push(Point::new(v1.x, v1.y));
                        out.push(Point::new(v1.x + dx2, v1.y - dy2));
                    } else {
                        out.push(Point::new(v1.x + dx1, v1.y - dy1));
                        out.push(Point::new(v1.x, v1.y));
                        self.calc_arc(out, v1.x, v1.y, dx2, -dy2, dx1, -dy1);
                        out.push(Point::new(v1.x, v1.y));
                        out.push(Point::new(v1.x + dx2, v1.y - dy2));
                    }
                }
            }
            return;
        }

        // Outer join: distance from v1 to the middle of the bevel line.
        let dx = f64::midpoint(dx1, dx2);
        let dy = f64::midpoint(dy1, dy2);
        let dbevel = (dx * dx + dy * dy).sqrt();

        if matches!(self.join, LineJoin::Round | LineJoin::Bevel)
            && self.approx_scale * (self.width_abs - dbevel) < self.width_eps
        {
            // Nearly collinear: a single miter point is indistinguishable.
            match calc_intersection(
                v0.x + dx1,
                v0.y - dy1,
                v1.x + dx1,
                v1.y - dy1,
                v1.x + dx2,
                v1.y - dy2,
                v2.x + dx2,
                v2.y - dy2,
            ) {
                Some((x, y)) => out.push(Point::new(x, y)),
                None => out.push(Point::new(v1.x + dx1, v1.y - dy1)),
            }
            return;
        }

        match self.join {
            LineJoin::Miter | LineJoin::MiterRevert | LineJoin::MiterRound => {
                self.calc_miter(
                    out,
                    v0,
                    v1,
                    v2,
                    offsets,
                    self.join,
                    self.miter_limit,
                    dbevel,
                );
            }
            LineJoin::Round => self.calc_arc(out, v1.x, v1.y, dx1, -dy1, dx2, -dy2),
            LineJoin::Bevel => {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                out.push(Point::new(v1.x + dx2, v1.y - dy2));
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
    fn butt_and_square_caps() {
        let mut out = Vec::new();
        let butt = StrokeMath::new(&StrokeStyle::new(2.0));
        butt.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out, vec![Point::new(0.0, 1.0), Point::new(0.0, -1.0)]);
        let square = StrokeMath::new(&StrokeStyle::new(2.0).with_cap(LineCap::Square));
        square.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out, vec![Point::new(-1.0, 1.0), Point::new(-1.0, -1.0)]);
    }

    #[test]
    fn round_cap_is_a_semicircle() {
        let mut out = Vec::new();
        let m = StrokeMath::new(&StrokeStyle::new(20.0).with_cap(LineCap::Round));
        m.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert!(out.len() > 4);
        for p in &out {
            assert!((p.length() - 10.0).abs() < 1e-9);
            assert!(p.x <= 1e-9);
        }
    }

    #[test]
    fn right_angle_miter() {
        let mut out = Vec::new();
        let m = StrokeMath::new(&StrokeStyle::new(2.0));
        // Turning towards +y puts the offset side (-y) on the outside.
        m.calc_join(
            &mut out,
            &vd(0.0, 0.0),
            &vd(10.0, 0.0),
            &vd(10.0, 10.0),
            10.0,
            10.0,
        );
        assert_eq!(out, vec![Point::new(11.0, -1.0)]);
        // The opposite turn is an inner join, mitered at the offset lines' crossing.
        m.calc_join(
            &mut out,
            &vd(0.0, 0.0),
            &vd(10.0, 0.0),
            &vd(10.0, -10.0),
            10.0,
            10.0,
        );
        assert_eq!(out, vec![Point::new(9.0, -1.0)]);
    }

    #[test]
    fn sharp_miter_reverts_to_bevel() {
        let mut out = Vec::new();
        let m = StrokeMath::new(
            &StrokeStyle::new(2.0)
                .with_join(LineJoin::MiterRevert)
                .with_miter_limit(1.0),
        );
        m.calc_join(
            &mut out,
            &vd(0.0, 0.0),
            &vd(10.0, 0.0),
            &vd(0.0, 1.0),
            10.0,
            101f64.sqrt(),
        );
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn intersection() {
        assert_eq!(
            calc_intersection(0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 2.0, 0.0),
            Some((1.0, 1.0))
        );
        assert_eq!(
            calc_intersection(0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0),
            None
        );
    }
}
