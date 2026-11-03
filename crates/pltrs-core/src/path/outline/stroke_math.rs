//! Caps and joins: a port of Agg's `math_stroke`, which pltrs-raster's
//! stroker and the containment outlines of [`super::Outline`] share.
//!
//! Offsets are computed with the same floating-point expressions in the
//! same order as the C++ code so outlines match vertex for vertex.

use std::cell::Cell;
use std::f64::consts::PI;

use super::math::{calc_distance, calc_intersection, cross_product};
use super::vertex_seq::{VERTEX_DIST_EPSILON, VertexDist};
use crate::Point;

/// The most intermediate points of a round cap or join. A stroke as wide as
/// the largest canvas (2²³ pixels) needs about 6 400 for a half turn.
const MAX_ARC_POINTS: i32 = 1 << 13;

/// Steps of round caps and joins below which short arcs are recognised
/// without arc tangents (their tangent must stay finite).
const SHORT_ARC_LIMIT: f64 = 1.5;

/// The fraction of the step below which an arc counts as short.
const SHORT_ARC_MARGIN: f64 = 1.0 - 1e-9;

/// Shape of the ends of open sub-paths and dashes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineCap {
    /// The stroke ends exactly at the end point.
    #[default]
    Butt,
    /// The stroke extends half a width beyond the end point (matplotlib's
    /// `"projecting"`).
    Square,
    /// A semicircle centred on the end point.
    Round,
}

/// Shape of the outer corner where two segments meet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LineJoin {
    /// Agg's `miter_join`: a miter that is cut off at the miter limit.
    #[default]
    Miter,
    /// Agg's `miter_join_revert`: a miter that falls back to a bevel
    /// beyond the limit, as in SVG/PDF. matplotlib's `"miter"` uses this.
    MiterRevert,
    /// A circular arc.
    Round,
    /// A straight cut across the corner.
    Bevel,
    /// A miter that falls back to a round join beyond the limit.
    MiterRound,
}

/// Shape of the inner corner where two segments meet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InnerJoin {
    /// A straight cut.
    Bevel,
    /// A miter limited by the inner miter limit (Agg's default, used by
    /// matplotlib).
    #[default]
    Miter,
    /// A notch back to the vertex for very short segments.
    Jag,
    /// A rounded notch for very short segments.
    Round,
}

/// How the ends and corners of a stroke are shaped (the settings of Agg's
/// `math_stroke` other than the width). The default is Agg's: butt caps,
/// miter joins limited to 4 half widths, inner miters limited to 1.01.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeShape {
    /// End caps.
    pub cap: LineCap,
    /// Outer joins.
    pub join: LineJoin,
    /// Miter length limit, in multiples of half the width.
    pub miter_limit: f64,
    /// Inner joins.
    pub inner_join: InnerJoin,
    /// Inner miter limit.
    pub inner_miter_limit: f64,
    /// Scale for the number of segments in round caps and joins.
    pub approximation_scale: f64,
}

impl Default for StrokeShape {
    fn default() -> Self {
        Self {
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            inner_join: InnerJoin::Miter,
            inner_miter_limit: 1.01,
            approximation_scale: 1.0,
        }
    }
}

/// The caps and joins of a stroke of a given width and shape.
#[derive(Clone, Debug)]
pub struct StrokeMath {
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
    /// The angular step of round caps and joins.
    arc_step: f64,
    /// `tan` of a hair less than [`arc_step`](Self::arc_step), when that
    /// is below a right angle: arcs turning through less need no
    /// intermediate points (see [`StrokeMath::arc_is_short`]).
    short_arc_tan: Option<f64>,
    /// The last angle of a round join and the bits of its arguments:
    /// consecutive joins share an offset, the end of one starting the
    /// next, so each angle is computed once.
    last_angle: Cell<Option<(u64, u64, f64)>>,
}

impl StrokeMath {
    /// `math_stroke::width` and the shape's settings: offsets are half of
    /// `width`, on the right of the direction of travel for positive
    /// widths.
    pub fn new(width: f64, shape: &StrokeShape) -> Self {
        let width = width * 0.5;
        let width_abs = width.abs();
        let approx_scale = shape.approximation_scale;
        let arc_step = (width_abs / (width_abs + 0.125 / approx_scale)).acos() * 2.0;
        Self {
            width,
            width_abs,
            width_eps: width / 1024.0,
            width_sign: if width < 0.0 { -1.0 } else { 1.0 },
            miter_limit: shape.miter_limit,
            inner_miter_limit: shape.inner_miter_limit,
            approx_scale,
            cap: shape.cap,
            join: shape.join,
            inner_join: shape.inner_join,
            arc_step,
            short_arc_tan: (arc_step < SHORT_ARC_LIMIT)
                .then(|| (arc_step * SHORT_ARC_MARGIN).tan()),
            last_angle: Cell::new(None),
        }
    }

    /// Whether the arc from the offset `u` to the offset `v` turns, in the
    /// direction of the width's sign, through clearly more than nothing and
    /// clearly less than one [`arc_step`](Self::arc_step), so that Agg's
    /// `calc_arc` puts no point between its ends.
    ///
    /// The turn is measured by its cross and dot products, which differ
    /// from the difference of the two angles `calc_arc` takes by a few
    /// rounding errors: the margins (a millionth of a radian, a billionth
    /// of the step) are far wider, and an arc within them is computed in
    /// full. That spares the arc tangents of the short turns of finely
    /// flattened curves.
    fn arc_is_short(&self, (ux, uy): (f64, f64), (vx, vy): (f64, f64)) -> bool {
        let Some(limit) = self.short_arc_tan else {
            return false;
        };
        let turn = (ux * vy - uy * vx) * self.width_sign;
        let dot = ux * vx + uy * vy;
        dot > 0.0 && turn > dot * 1e-6 && turn < dot * limit
    }

    /// `y.atan2(x)`, remembered for the next join.
    fn angle(&self, y: f64, x: f64) -> f64 {
        let key = (y.to_bits(), x.to_bits());
        if let Some((last_y, last_x, angle)) = self.last_angle.get()
            && (last_y, last_x) == key
        {
            return angle;
        }
        let angle = y.atan2(x);
        self.last_angle.set(Some((key.0, key.1, angle)));
        angle
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
        if self.arc_is_short((dx1, dy1), (dx2, dy2)) {
            out.push(Point::new(x + dx1, y + dy1));
            out.push(Point::new(x + dx2, y + dy2));
            return;
        }
        let mut a1 = self.angle(dy1 * self.width_sign, dx1 * self.width_sign);
        let mut a2 = self.angle(dy2 * self.width_sign, dx2 * self.width_sign);
        let da = self.arc_step;
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

    /// Agg's `calc_cap`: the cap at `v0` of the segment `v0 → v1` of
    /// length `len`, replacing the points of `out`.
    pub fn calc_cap(&self, out: &mut Vec<Point>, v0: &VertexDist, v1: &VertexDist, len: f64) {
        out.clear();
        let mut dx1 = (v1.y - v0.y) / len;
        let mut dy1 = (v1.x - v0.x) / len;
        dx1 *= self.width;
        dy1 *= self.width;
        if self.cap == LineCap::Round {
            let da = self.arc_step;
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

    /// Agg's `calc_join`: the join at `v1` between segments `v0 → v1`
    /// (length `len1`) and `v1 → v2` (length `len2`), replacing the points
    /// of `out`.
    pub fn calc_join(
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

    fn math(width: f64, shape: StrokeShape) -> StrokeMath {
        StrokeMath::new(width, &shape)
    }

    fn shaped(cap: LineCap, join: LineJoin) -> StrokeShape {
        StrokeShape {
            cap,
            join,
            ..StrokeShape::default()
        }
    }

    #[test]
    fn butt_and_square_caps() {
        let mut out = Vec::new();
        let butt = math(2.0, StrokeShape::default());
        butt.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out, vec![Point::new(0.0, 1.0), Point::new(0.0, -1.0)]);
        let square = math(2.0, shaped(LineCap::Square, LineJoin::Miter));
        square.calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out, vec![Point::new(-1.0, 1.0), Point::new(-1.0, -1.0)]);
    }

    #[test]
    fn round_cap_is_a_semicircle() {
        let mut out = Vec::new();
        let m = math(20.0, shaped(LineCap::Round, LineJoin::Miter));
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
        let mut round = shaped(LineCap::Round, LineJoin::Round);
        math(1e300, round).calc_cap(&mut out, &vd(0.0, 0.0), &vd(10.0, 0.0), 10.0);
        assert_eq!(out.len(), MAX_ARC_POINTS as usize + 2);
        round.approximation_scale = 1e300;
        let (a, b, c) = (vd(0.0, 0.0), vd(10.0, 0.0), vd(10.0, -10.0));
        math(2.0, round).calc_join(&mut out, &a, &b, &c, 10.0, 10.0);
        assert!(out.len() <= MAX_ARC_POINTS as usize + 2);
        round.approximation_scale = f64::NAN;
        math(2.0, round).calc_cap(&mut out, &a, &b, 10.0);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn short_arcs_match_the_arc_tangent_computation() {
        // Turns sweeping across the step (and across the ±π cut of the arc
        // tangent), for both signs of the width: the shortcut must give the
        // points the full computation gives.
        for width in [2.08, -2.08, 0.6, 15.0] {
            let fast = math(width, shaped(LineCap::Butt, LineJoin::Round));
            let full = StrokeMath {
                short_arc_tan: None,
                ..fast.clone()
            };
            let (mut a, mut b) = (Vec::new(), Vec::new());
            for i in 0..2000 {
                let start = f64::from(i) * 0.0137 - 3.3;
                let turn = (f64::from(i % 97) - 48.0) * fast.arc_step / 40.0;
                let r = width.abs() * 0.5;
                let u = (r * start.cos(), r * start.sin());
                let v = (r * (start + turn).cos(), r * (start + turn).sin());
                fast.calc_arc(&mut a, Point::new(1.5, -2.5), u, v);
                full.calc_arc(&mut b, Point::new(1.5, -2.5), u, v);
                assert_eq!(a, b, "width {width}, start {start}, turn {turn}");
                a.clear();
                b.clear();
            }
        }
    }

    #[test]
    fn right_angle_miter() {
        let mut out = Vec::new();
        let m = math(2.0, StrokeShape::default());
        let (a, b) = (vd(0.0, 0.0), vd(10.0, 0.0));
        // Turning towards +y puts the offset side (-y) on the outside.
        m.calc_join(&mut out, &a, &b, &vd(10.0, 10.0), 10.0, 10.0);
        assert_eq!(out, vec![Point::new(11.0, -1.0)]);
        // The opposite turn is an inner join, mitered at the offset lines' crossing.
        m.calc_join(&mut out, &a, &b, &vd(10.0, -10.0), 10.0, 10.0);
        assert_eq!(out, vec![Point::new(9.0, -1.0)]);
    }

    #[test]
    fn sharp_miters_are_clipped_or_revert_to_bevels() {
        let mut out = Vec::new();
        let (a, b, c) = (vd(0.0, 0.0), vd(10.0, 0.0), vd(0.0, 1.0));
        let len2 = 101f64.sqrt();
        math(2.0, StrokeShape::default()).calc_join(&mut out, &a, &b, &c, 10.0, len2);
        assert_eq!(out.len(), 2, "Agg's default clips the miter");
        let revert = StrokeShape {
            miter_limit: 1.0,
            ..shaped(LineCap::Butt, LineJoin::MiterRevert)
        };
        math(2.0, revert).calc_join(&mut out, &a, &b, &c, 10.0, len2);
        assert_eq!(out.len(), 2, "a bevel");
        assert_eq!(out[0], Point::new(10.0, -1.0));
    }
}
