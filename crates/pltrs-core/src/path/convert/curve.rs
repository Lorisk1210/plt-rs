//! Bézier flattening (Agg's `conv_curve` with `curve3_div` / `curve4_div`
//! at matplotlib's settings: approximation scale 1, angle tolerance 0,
//! cusp limit 0).

use super::source::next_point;
use super::{Vertex, VertexSource};
use crate::Point;
use crate::numeric::{difference_of_products, fused, sum_of_products};
use crate::path::PathCode;

const COLLINEARITY_EPSILON: f64 = 1e-30;
const RECURSION_LIMIT: u32 = 32;
/// Subdivision steps allowed per curve. Agg terminates far earlier for
/// every curve within a billion pixels of the canvas; only curves reaching
/// farther and NaN-poisoned ones (which make Agg recurse ~2^33 times)
/// exhaust it, and each that does costs the whole budget.
const NODE_BUDGET: usize = 1 << 16;
/// `(0.5 / approximation_scale)²`: the squared flatness tolerance in pixels.
const DISTANCE_TOLERANCE_SQ: f64 = 0.25;

/// Replaces curves by line segments; see [`VertexSource::flatten_curves`].
///
/// Each curve is subdivided recursively at its midpoints until its control
/// polygon is within half a unit of the chord, exactly as Agg does, and the
/// interior points are emitted as [`PathCode::LineTo`] vertices ending at
/// the curve's end point. A curve starts at the previous vertex, whatever
/// its code (after a closure, at the closure's stored coordinates).
///
/// Curves on which Agg would effectively never finish (NaN coordinates
/// defeat every flatness test, so Agg recurses about 2^33 times) become a
/// single line to their end point.
#[derive(Clone, Debug)]
pub struct CurveFlattener<I> {
    source: I,
    enabled: bool,
    last: Point,
    points: Vec<Point>,
    pending: usize,
}

impl<I: VertexSource> CurveFlattener<I> {
    pub(super) const fn new(source: I, enabled: bool) -> Self {
        Self {
            source,
            enabled,
            last: Point::ORIGIN,
            points: Vec::new(),
            pending: 0,
        }
    }

    /// Emits the next queued interior point of the current curve.
    fn emit_pending(&mut self) -> Option<Vertex> {
        let point = *self.points.get(self.pending)?;
        self.pending += 1;
        self.last = point;
        Some(Vertex::line_to(point))
    }
}

impl<I: VertexSource> Iterator for CurveFlattener<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        if !self.enabled {
            return self.source.next();
        }
        if let Some(vertex) = self.emit_pending() {
            return Some(vertex);
        }
        let vertex = self.source.next()?;
        match vertex.code {
            PathCode::Curve3 => {
                let end = next_point(&mut self.source);
                flatten_quad(self.last, vertex.point, end, &mut self.points);
            }
            PathCode::Curve4 => {
                let control = next_point(&mut self.source);
                let end = next_point(&mut self.source);
                flatten_cubic(self.last, vertex.point, control, end, &mut self.points);
            }
            PathCode::MoveTo | PathCode::LineTo | PathCode::ClosePoly => {
                self.last = vertex.point;
                return Some(vertex);
            }
        }
        // The first flattened point is the curve's start: skip it.
        self.pending = 1;
        self.emit_pending()
    }
}

impl<I: VertexSource> VertexSource for CurveFlattener<I> {
    fn rewind(&mut self) {
        self.source.rewind();
        self.last = Point::ORIGIN;
        self.points.clear();
        self.pending = 0;
    }
}

/// Agg's `curve3_div`: the flattened quadratic from `p1` (inclusive) to
/// `p3` (inclusive) into `out`, which is cleared first.
fn flatten_quad(p1: Point, p2: Point, p3: Point, out: &mut Vec<Point>) {
    let mut subdivider = Subdivider::start(out, p1);
    subdivider.quad([p1, p2, p3], 0);
    subdivider.finish(p3);
}

/// Agg's `curve4_div`: the flattened cubic from `p1` (inclusive) to `p4`
/// (inclusive) into `out`, which is cleared first.
fn flatten_cubic(p1: Point, p2: Point, p3: Point, p4: Point, out: &mut Vec<Point>) {
    let mut subdivider = Subdivider::start(out, p1);
    subdivider.cubic([p1, p2, p3, p4], 0);
    subdivider.finish(p4);
}

/// The recursive midpoint subdivision of Agg's `curve3_div` / `curve4_div`.
struct Subdivider<'a> {
    out: &'a mut Vec<Point>,
    nodes: usize,
}

impl<'a> Subdivider<'a> {
    fn start(out: &'a mut Vec<Point>, first: Point) -> Self {
        out.clear();
        out.push(first);
        Self { out, nodes: 0 }
    }

    /// Whether to subdivide at `level` (counting the step).
    fn enter(&mut self, level: u32) -> bool {
        if level > RECURSION_LIMIT || self.nodes >= NODE_BUDGET {
            return false;
        }
        self.nodes += 1;
        true
    }

    fn finish(self, last: Point) {
        if self.nodes >= NODE_BUDGET {
            self.out.truncate(1);
        }
        self.out.push(last);
    }

    fn quad(&mut self, [p1, p2, p3]: [Point; 3], level: u32) {
        if !self.enter(level) {
            return;
        }
        let p12 = p1.midpoint(p2);
        let p23 = p2.midpoint(p3);
        let p123 = p12.midpoint(p23);

        let dx = p3.x - p1.x;
        let dy = p3.y - p1.y;
        let d = difference_of_products(p2.x - p3.x, dy, p2.y - p3.y, dx).abs();
        if d > COLLINEARITY_EPSILON {
            if d * d <= DISTANCE_TOLERANCE_SQ * sum_of_products(dx, dx, dy, dy) {
                self.out.push(p123);
                return;
            }
        } else {
            let da = sum_of_products(dx, dx, dy, dy);
            let d = if da == 0.0 {
                sq_distance(p1, p2)
            } else {
                let t = sum_of_products(p2.x - p1.x, dx, p2.y - p1.y, dy) / da;
                if t > 0.0 && t < 1.0 {
                    // Simple collinear case 1---2---3: the end points suffice.
                    return;
                }
                chord_distance(p2, p1, p3, Point::new(dx, dy), t)
            };
            if d < DISTANCE_TOLERANCE_SQ {
                self.out.push(p2);
                return;
            }
        }
        self.quad([p1, p12, p123], level + 1);
        self.quad([p123, p23, p3], level + 1);
    }

    fn cubic(&mut self, [p1, p2, p3, p4]: [Point; 4], level: u32) {
        if !self.enter(level) {
            return;
        }
        let p12 = p1.midpoint(p2);
        let p23 = p2.midpoint(p3);
        let p34 = p3.midpoint(p4);
        let p123 = p12.midpoint(p23);
        let p234 = p23.midpoint(p34);
        let p1234 = p123.midpoint(p234);

        let dx = p4.x - p1.x;
        let dy = p4.y - p1.y;
        let d2 = difference_of_products(p2.x - p4.x, dy, p2.y - p4.y, dx).abs();
        let d3 = difference_of_products(p3.x - p4.x, dy, p3.y - p4.y, dx).abs();
        let chord_sq = sum_of_products(dx, dx, dy, dy);
        let flat_enough = |d: f64| d * d <= DISTANCE_TOLERANCE_SQ * chord_sq;
        match (d2 > COLLINEARITY_EPSILON, d3 > COLLINEARITY_EPSILON) {
            (false, false) => {
                // All collinear, or p1 == p4.
                let (d2, d3) = if chord_sq == 0.0 {
                    (sq_distance(p1, p2), sq_distance(p4, p3))
                } else {
                    let k = 1.0 / chord_sq;
                    let t2 = k * sum_of_products(p2.x - p1.x, dx, p2.y - p1.y, dy);
                    let t3 = k * sum_of_products(p3.x - p1.x, dx, p3.y - p1.y, dy);
                    if t2 > 0.0 && t2 < 1.0 && t3 > 0.0 && t3 < 1.0 {
                        // Simple collinear case 1---2---3---4.
                        return;
                    }
                    let d = Point::new(dx, dy);
                    (
                        chord_distance(p2, p1, p4, d, t2),
                        chord_distance(p3, p1, p4, d, t3),
                    )
                };
                if d2 > d3 {
                    if d2 < DISTANCE_TOLERANCE_SQ {
                        self.out.push(p2);
                        return;
                    }
                } else if d3 < DISTANCE_TOLERANCE_SQ {
                    self.out.push(p3);
                    return;
                }
            }
            (false, true) => {
                if flat_enough(d3) {
                    self.out.push(p23);
                    return;
                }
            }
            (true, false) => {
                if flat_enough(d2) {
                    self.out.push(p23);
                    return;
                }
            }
            (true, true) => {
                if flat_enough(d2 + d3) {
                    self.out.push(p23);
                    return;
                }
            }
        }
        self.cubic([p1, p12, p123, p1234], level + 1);
        self.cubic([p1234, p234, p34, p4], level + 1);
    }
}

fn sq_distance(a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    sum_of_products(dx, dx, dy, dy)
}

/// Squared distance from `p` to the chord `a → a + d` at the projection
/// parameter `t` (clamped to the end points like Agg).
fn chord_distance(p: Point, a: Point, b: Point, d: Point, t: f64) -> f64 {
    if t <= 0.0 {
        sq_distance(p, a)
    } else if t >= 1.0 {
        sq_distance(p, b)
    } else {
        sq_distance(p, Point::new(fused(t, d.x, a.x), fused(t, d.y, a.y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Path;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn straight_curves_collapse() {
        let mut out = Vec::new();
        flatten_quad(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0), &mut out);
        assert_eq!(out, vec![p(0.0, 0.0), p(2.0, 2.0)]);
        flatten_cubic(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0), &mut out);
        assert_eq!(out, vec![p(0.0, 0.0), p(3.0, 0.0)]);
    }

    #[test]
    fn curves_become_lines_ending_at_the_end_point() {
        let path = Path::circle(Point::ORIGIN, 100.0);
        let out: Vec<Vertex> = path.iter_vertices().flatten_curves(true).collect();
        assert!(out.len() > 50);
        assert_eq!(out[0], Vertex::move_to(p(0.0, -100.0)));
        assert!(
            out[1..out.len() - 1]
                .iter()
                .all(|v| v.code == PathCode::LineTo)
        );
        assert_eq!(out[out.len() - 2].point, p(0.0, -100.0));
        assert_eq!(out[out.len() - 1].code, PathCode::ClosePoly);
        for v in &out[..out.len() - 1] {
            assert!((v.point.length() - 100.0).abs() < 0.5);
        }
    }

    #[test]
    fn nan_poisoned_curves_degenerate_to_lines() {
        let mut out = Vec::new();
        flatten_cubic(
            p(f64::NAN, 0.0),
            p(1.0, 1.0),
            p(1.0, 1.0),
            p(2.0, 0.0),
            &mut out,
        );
        assert_eq!(out.len(), 2);
        assert_eq!(out[1], p(2.0, 0.0));
    }

    #[test]
    fn infinite_starts_terminate_like_agg() {
        // Agg's flatness test passes at once with infinite distances.
        let mut out = Vec::new();
        flatten_cubic(
            p(0.0, f64::INFINITY),
            p(1.0, 1.0),
            p(3.0, 2.0),
            p(2.0, 0.0),
            &mut out,
        );
        assert_eq!(out, vec![p(0.0, f64::INFINITY), p(2.0, 1.5), p(2.0, 0.0)]);
    }

    #[test]
    fn disabled_keeps_curves() {
        let path = Path::unit_circle();
        assert_eq!(
            path.iter_vertices().flatten_curves(false).count(),
            path.len()
        );
    }
}
