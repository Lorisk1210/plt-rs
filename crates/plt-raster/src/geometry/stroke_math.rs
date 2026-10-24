//! Caps and joins — a port of Agg's `math_stroke`.
//!
//! Offsets are computed with the same floating-point expressions in the
//! same order as the C++ code so outlines match vertex for vertex.

use std::f64::consts::PI;

use plt_core::Point;
use plt_core::path::outline::{
    VERTEX_DIST_EPSILON, VertexDist, calc_distance, calc_intersection, cross_product,
};

use crate::style::{InnerJoin, LineCap, LineJoin, StrokeStyle};

/// The most intermediate points of a round cap or join. A stroke as wide as
/// the largest canvas (2²³ pixels) needs about 6 400 for a half turn.
const MAX_ARC_POINTS: i32 = 1 << 13;

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

    /// Agg's `int(angle / step)` intermediate points of a round cap or join.
    ///
    /// Huge widths or approximation scales make the step zero; C++ leaves
    /// the conversion of the infinite quotient undefined, Rust saturates it
    /// to `i32::MAX` points. The count is capped at [`MAX_ARC_POINTS`].
    fn arc_points(angle: f64, step: f64) -> i32 {
        ((angle / step) as i32).min(MAX_ARC_POINTS)
    }

    /// Agg's `calc_arc`: the round join or cap around `center` from the
    /// offset `(dx1, dy1)` to the offset `(dx2, dy2)`.
    fn calc_arc(
        &self,
        out: &mut Vec<Point>,
        center: Point,
        (dx1, dy1): (f64, f64),
        (dx2, dy2): (f64, f64),
    ) {
        let Point { x, y } = center;
        let mut a1 = (dy1 * self.width_sign).atan2(dx1 * self.width_sign);
        let mut a2 = (dy2 * self.width_sign).atan2(dx2 * self.width_sign);
        let da = self.arc_step();
        out.push(Point::new(x + dx1, y + dy1));
        if self.width_sign > 0.0 {
            if a1 > a2 {
                a2 += 2.0 * PI;
            }
            let n = Self::arc_points(a2 - a1, da);
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
            let n = Self::arc_points(a1 - a2, da);
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

    /// Agg's `calc_miter`: the miter at `v1` of the offsets `dx1..dy2` of
    /// the segments `v0 → v1` and `v1 → v2`, `join` deciding what a miter
    /// longer than `mlimit` widths becomes.
    fn calc_miter(
        &self,
        out: &mut Vec<Point>,
        [v0, v1, v2]: [&VertexDist; 3],
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
        if let Some(p) = calc_intersection(
            Point::new(v0.x + dx1, v0.y - dy1),
            Point::new(v1.x + dx1, v1.y - dy1),
            Point::new(v1.x + dx2, v1.y - dy2),
            Point::new(v2.x + dx2, v2.y - dy2),
        ) {
            xi = p.x;
            yi = p.y;
            di = calc_distance(v1.point(), p);
            if di <= lim {
                out.push(Point::new(xi, yi));
                limit_exceeded = false;
            }
            intersection_failed = false;
        } else {
            // Collinear points: either the path continues straight on (one
            // offset point suffices) or it turns back on itself.
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
        match join {
            LineJoin::MiterRevert => {
                out.push(Point::new(v1.x + dx1, v1.y - dy1));
                out.push(Point::new(v1.x + dx2, v1.y - dy2));
            }
            LineJoin::MiterRound => self.calc_arc(out, v1.point(), (dx1, -dy1), (dx2, -dy2)),
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
            let n = Self::arc_points(PI, da);
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

        let cp = cross_product(v0.point(), v1.point(), v2.point());
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
                    self.calc_miter(
                        out,
                        [v0, v1, v2],
                        offsets,
                        LineJoin::MiterRevert,
                        limit,
                        0.0,
                    );
                }
                InnerJoin::Jag | InnerJoin::Round => {
                    let d = (dx1 - dx2) * (dx1 - dx2) + (dy1 - dy2) * (dy1 - dy2);
                    if d < len1 * len1 && d < len2 * len2 {
                        self.calc_miter(
                            out,
                            [v0, v1, v2],
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
                        self.calc_arc(out, v1.point(), (dx2, -dy2), (dx1, -dy1));
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
            let first = Point::new(v1.x + dx1, v1.y - dy1);
            out.push(
                calc_intersection(
                    Point::new(v0.x + dx1, v0.y - dy1),
                    first,
                    Point::new(v1.x + dx2, v1.y - dy2),
                    Point::new(v2.x + dx2, v2.y - dy2),
                )
                .unwrap_or(first),
            );
            return;
        }

        match self.join {
            LineJoin::Miter | LineJoin::MiterRevert | LineJoin::MiterRound => {
                self.calc_miter(
                    out,
                    [v0, v1, v2],
                    offsets,
                    self.join,
                    self.miter_limit,
                    dbevel,
                );
            }
            LineJoin::Round => self.calc_arc(out, v1.point(), (dx1, -dy1), (dx2, -dy2)),
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
    fn round_caps_and_joins_stay_bounded() {
        // A zero angular step would ask for i32::MAX points per arc.
        let mut out = Vec::new();
        let mut huge = StrokeStyle::new(1e300)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Round);
        let m = StrokeMath::new(&huge);
        m.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out.len(), MAX_ARC_POINTS as usize + 2);
        huge.width = 2.0;
        huge.approximation_scale = 1e300;
        let m = StrokeMath::new(&huge);
        let (a, b, c) = (vd(0.0, 0.0), vd(10.0, 0.0), vd(10.0, -10.0));
        m.calc_join(&mut out, &a, &b, &c, 10.0, 10.0);
        assert!(out.len() <= MAX_ARC_POINTS as usize + 2);
        huge.approximation_scale = f64::NAN;
        StrokeMath::new(&huge).calc_cap(&mut out, &a, &b, 10.0);
        assert_eq!(out.len(), 2);
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
}
