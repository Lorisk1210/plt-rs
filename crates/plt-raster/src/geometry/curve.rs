//! Bézier flattening — a port of Agg's `curve3_div` / `curve4_div` with
//! the defaults matplotlib uses through `conv_curve` (approximation scale
//! 1, angle tolerance 0, cusp limit 0).
//!
//! Curves are subdivided recursively at their midpoints until the control
//! polygon deviates from the chord by at most `0.5 / approximation_scale`
//! pixels.

use plt_core::{Path, Point, Segment};

use super::PathSink;

const COLLINEARITY_EPSILON: f64 = 1e-30;
const RECURSION_LIMIT: u32 = 32;

fn sq_distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Flattens curves into polylines and feeds whole paths to a [`PathSink`].
#[derive(Clone, Debug)]
pub(crate) struct CurveFlattener {
    distance_tolerance_square: f64,
    points: Vec<Point>,
}

impl Default for CurveFlattener {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl CurveFlattener {
    /// A flattener with the given approximation scale (1 = Agg default).
    pub(crate) fn new(approximation_scale: f64) -> Self {
        let tolerance = 0.5 / approximation_scale;
        Self {
            distance_tolerance_square: tolerance * tolerance,
            points: Vec::new(),
        }
    }

    /// Sends `path` to `sink`, replacing curves by line segments exactly as
    /// Agg's `conv_curve` does. After a close the current point returns to
    /// the start of the sub-path.
    pub(crate) fn feed<S: PathSink + ?Sized>(&mut self, path: &Path, sink: &mut S) {
        let mut current = Point::ORIGIN;
        let mut start = Point::ORIGIN;
        for segment in path.segments() {
            match segment {
                Segment::MoveTo(p) => {
                    sink.move_to(p);
                    current = p;
                    start = p;
                }
                Segment::LineTo(p) => {
                    sink.line_to(p);
                    current = p;
                }
                Segment::QuadTo(c, p) => {
                    for &q in self.quad(current, c, p) {
                        sink.line_to(q);
                    }
                    current = p;
                }
                Segment::CubicTo(c1, c2, p) => {
                    for &q in self.cubic(current, c1, c2, p) {
                        sink.line_to(q);
                    }
                    current = p;
                }
                Segment::Close => {
                    sink.close();
                    current = start;
                }
            }
        }
    }

    /// The flattened quadratic without its start point (ends with `p3`).
    /// Non-finite input degenerates to the straight segment.
    pub(crate) fn quad(&mut self, p1: Point, p2: Point, p3: Point) -> &[Point] {
        self.points.clear();
        self.points.push(p1);
        if p1.is_finite() && p2.is_finite() && p3.is_finite() {
            self.recursive_quad(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, 0);
        }
        self.points.push(p3);
        &self.points[1..]
    }

    /// The flattened cubic without its start point (ends with `p4`).
    /// Non-finite input degenerates to the straight segment.
    pub(crate) fn cubic(&mut self, p1: Point, p2: Point, p3: Point, p4: Point) -> &[Point] {
        self.points.clear();
        self.points.push(p1);
        if p1.is_finite() && p2.is_finite() && p3.is_finite() && p4.is_finite() {
            let c = [p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p4.x, p4.y];
            self.recursive_cubic(c, 0);
        }
        self.points.push(p4);
        &self.points[1..]
    }

    #[allow(clippy::too_many_arguments)] // Mirrors Agg's recursion one to one.
    fn recursive_quad(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, level: u32) {
        if level > RECURSION_LIMIT {
            return;
        }
        let x12 = f64::midpoint(x1, x2);
        let y12 = f64::midpoint(y1, y2);
        let x23 = f64::midpoint(x2, x3);
        let y23 = f64::midpoint(y2, y3);
        let x123 = f64::midpoint(x12, x23);
        let y123 = f64::midpoint(y12, y23);

        let dx = x3 - x1;
        let dy = y3 - y1;
        let d = ((x2 - x3) * dy - (y2 - y3) * dx).abs();
        let tol = self.distance_tolerance_square;
        if d > COLLINEARITY_EPSILON {
            if d * d <= tol * (dx * dx + dy * dy) {
                self.points.push(Point::new(x123, y123));
                return;
            }
        } else {
            let da = dx * dx + dy * dy;
            let d = if da == 0.0 {
                sq_distance(x1, y1, x2, y2)
            } else {
                let t = ((x2 - x1) * dx + (y2 - y1) * dy) / da;
                if t > 0.0 && t < 1.0 {
                    // Simple collinear case 1---2---3: keep the end points.
                    return;
                }
                if t <= 0.0 {
                    sq_distance(x2, y2, x1, y1)
                } else if t >= 1.0 {
                    sq_distance(x2, y2, x3, y3)
                } else {
                    sq_distance(x2, y2, x1 + t * dx, y1 + t * dy)
                }
            };
            if d < tol {
                self.points.push(Point::new(x2, y2));
                return;
            }
        }
        self.recursive_quad(x1, y1, x12, y12, x123, y123, level + 1);
        self.recursive_quad(x123, y123, x23, y23, x3, y3, level + 1);
    }

    fn recursive_cubic(&mut self, c: [f64; 8], level: u32) {
        if level > RECURSION_LIMIT {
            return;
        }
        let [x1, y1, x2, y2, x3, y3, x4, y4] = c;
        let x12 = f64::midpoint(x1, x2);
        let y12 = f64::midpoint(y1, y2);
        let x23 = f64::midpoint(x2, x3);
        let y23 = f64::midpoint(y2, y3);
        let x34 = f64::midpoint(x3, x4);
        let y34 = f64::midpoint(y3, y4);
        let x123 = f64::midpoint(x12, x23);
        let y123 = f64::midpoint(y12, y23);
        let x234 = f64::midpoint(x23, x34);
        let y234 = f64::midpoint(y23, y34);
        let x1234 = f64::midpoint(x123, x234);
        let y1234 = f64::midpoint(y123, y234);

        let dx = x4 - x1;
        let dy = y4 - y1;
        let d2 = ((x2 - x4) * dy - (y2 - y4) * dx).abs();
        let d3 = ((x3 - x4) * dy - (y3 - y4) * dx).abs();
        let tol = self.distance_tolerance_square;
        let mid = Point::new(x23, y23);
        match (d2 > COLLINEARITY_EPSILON, d3 > COLLINEARITY_EPSILON) {
            (false, false) => {
                // All collinear, or p1 == p4.
                let (d2, d3) = if dx * dx + dy * dy == 0.0 {
                    (sq_distance(x1, y1, x2, y2), sq_distance(x4, y4, x3, y3))
                } else {
                    let k = 1.0 / (dx * dx + dy * dy);
                    let t2 = k * ((x2 - x1) * dx + (y2 - y1) * dy);
                    let t3 = k * ((x3 - x1) * dx + (y3 - y1) * dy);
                    if t2 > 0.0 && t2 < 1.0 && t3 > 0.0 && t3 < 1.0 {
                        // Simple collinear case 1---2---3---4.
                        return;
                    }
                    let project = |t: f64, x: f64, y: f64| {
                        if t <= 0.0 {
                            sq_distance(x, y, x1, y1)
                        } else if t >= 1.0 {
                            sq_distance(x, y, x4, y4)
                        } else {
                            sq_distance(x, y, x1 + t * dx, y1 + t * dy)
                        }
                    };
                    (project(t2, x2, y2), project(t3, x3, y3))
                };
                if d2 > d3 {
                    if d2 < tol {
                        self.points.push(Point::new(x2, y2));
                        return;
                    }
                } else if d3 < tol {
                    self.points.push(Point::new(x3, y3));
                    return;
                }
            }
            (false, true) => {
                // p1, p2, p4 collinear; p3 is significant.
                if d3 * d3 <= tol * (dx * dx + dy * dy) {
                    self.points.push(mid);
                    return;
                }
            }
            (true, false) => {
                // p1, p3, p4 collinear; p2 is significant.
                if d2 * d2 <= tol * (dx * dx + dy * dy) {
                    self.points.push(mid);
                    return;
                }
            }
            (true, true) => {
                if (d2 + d3) * (d2 + d3) <= tol * (dx * dx + dy * dy) {
                    self.points.push(mid);
                    return;
                }
            }
        }
        self.recursive_cubic([x1, y1, x12, y12, x123, y123, x1234, y1234], level + 1);
        self.recursive_cubic([x1234, y1234, x234, y234, x34, y34, x4, y4], level + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_core::bezier::Bezier;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn straight_curves_collapse_to_their_end() {
        let mut f = CurveFlattener::default();
        assert_eq!(
            f.quad(p(0.0, 0.0), p(1.0, 1.0), p(2.0, 2.0)),
            &[p(2.0, 2.0)]
        );
        assert_eq!(
            f.cubic(p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)),
            &[p(3.0, 0.0)]
        );
    }

    #[test]
    fn flattened_cubic_stays_within_tolerance() {
        let mut f = CurveFlattener::default();
        let curve = Bezier::Cubic(p(0.0, 0.0), p(0.0, 80.0), p(80.0, 80.0), p(80.0, 0.0));
        let pts = f
            .cubic(curve.start(), p(0.0, 80.0), p(80.0, 80.0), curve.end())
            .to_vec();
        assert!(pts.len() > 8);
        assert_eq!(*pts.last().unwrap(), curve.end());
        // Every sample of the curve is close to the polyline.
        let mut poly = vec![curve.start()];
        poly.extend(pts);
        for i in 0..=200 {
            let q = curve.eval(f64::from(i) / 200.0);
            let dist = poly
                .windows(2)
                .map(|w| distance_to_segment(q, w[0], w[1]))
                .fold(f64::INFINITY, f64::min);
            assert!(dist < 0.5, "deviation {dist}");
        }
    }

    #[test]
    fn non_finite_curves_degenerate_to_lines() {
        let mut f = CurveFlattener::default();
        assert_eq!(
            f.quad(p(0.0, 0.0), p(f64::NAN, 1.0), p(2.0, 0.0)),
            &[p(2.0, 0.0)]
        );
        let end = p(1.0, 1.0);
        assert_eq!(
            f.cubic(p(0.0, 0.0), p(f64::INFINITY, 0.0), p(0.0, 0.0), end),
            &[end]
        );
    }

    fn distance_to_segment(q: Point, a: Point, b: Point) -> f64 {
        let ab = b - a;
        let t = ((q - a).dot(ab) / ab.dot(ab)).clamp(0.0, 1.0);
        q.distance(a + ab * t)
    }
}
