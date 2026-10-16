//! Bézier curve utilities: evaluation, subdivision, extrema and flattening.

use crate::Point;

/// A Bézier curve of degree 1 (line), 2 (quadratic) or 3 (cubic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Bezier {
    /// Straight segment.
    Line(Point, Point),
    /// Quadratic curve with one control point.
    Quad(Point, Point, Point),
    /// Cubic curve with two control points.
    Cubic(Point, Point, Point, Point),
}

impl Bezier {
    /// First point of the curve.
    pub const fn start(&self) -> Point {
        match *self {
            Self::Line(p0, _) | Self::Quad(p0, _, _) | Self::Cubic(p0, _, _, _) => p0,
        }
    }

    /// Last point of the curve.
    pub const fn end(&self) -> Point {
        match *self {
            Self::Line(_, p1) | Self::Quad(_, _, p1) | Self::Cubic(_, _, _, p1) => p1,
        }
    }

    /// The control polygon (all defining points, start to end).
    pub fn control_points(&self) -> Vec<Point> {
        match *self {
            Self::Line(a, b) => vec![a, b],
            Self::Quad(a, b, c) => vec![a, b, c],
            Self::Cubic(a, b, c, d) => vec![a, b, c, d],
        }
    }

    /// Point at parameter `t ∈ [0, 1]` (de Casteljau, numerically stable).
    pub fn eval(&self, t: f64) -> Point {
        match *self {
            Self::Line(a, b) => a.lerp(b, t),
            Self::Quad(a, b, c) => a.lerp(b, t).lerp(b.lerp(c, t), t),
            Self::Cubic(a, b, c, d) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let cd = c.lerp(d, t);
                ab.lerp(bc, t).lerp(bc.lerp(cd, t), t)
            }
        }
    }

    /// First derivative with respect to `t`.
    pub fn derivative(&self, t: f64) -> Point {
        match *self {
            Self::Line(a, b) => b - a,
            Self::Quad(a, b, c) => ((b - a).lerp(c - b, t)) * 2.0,
            Self::Cubic(a, b, c, d) => {
                let (u, v, w) = (b - a, c - b, d - c);
                u.lerp(v, t).lerp(v.lerp(w, t), t) * 3.0
            }
        }
    }

    /// Splits the curve at `t` into two curves of the same degree.
    pub fn split(&self, t: f64) -> (Self, Self) {
        match *self {
            Self::Line(a, b) => {
                let m = a.lerp(b, t);
                (Self::Line(a, m), Self::Line(m, b))
            }
            Self::Quad(a, b, c) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let m = ab.lerp(bc, t);
                (Self::Quad(a, ab, m), Self::Quad(m, bc, c))
            }
            Self::Cubic(a, b, c, d) => {
                let ab = a.lerp(b, t);
                let bc = b.lerp(c, t);
                let cd = c.lerp(d, t);
                let abc = ab.lerp(bc, t);
                let bcd = bc.lerp(cd, t);
                let m = abc.lerp(bcd, t);
                (Self::Cubic(a, ab, abc, m), Self::Cubic(m, bcd, cd, d))
            }
        }
    }

    /// Parameters in the open interval `(0, 1)` where `dx/dt` or `dy/dt`
    /// vanishes, sorted ascending. Together with the end points these are
    /// the only candidates for the curve's axis-aligned extremes.
    pub fn axis_aligned_extrema(&self) -> Vec<f64> {
        let mut roots = Vec::new();
        match *self {
            Self::Line(..) => {}
            Self::Quad(a, b, c) => {
                for (p0, p1, p2) in [(a.x, b.x, c.x), (a.y, b.y, c.y)] {
                    // d/dt = 2[(p1 - p0) + t (p0 - 2 p1 + p2)]
                    let denom = p0 - 2.0 * p1 + p2;
                    if denom != 0.0 {
                        roots.push((p0 - p1) / denom);
                    }
                }
            }
            Self::Cubic(a, b, c, d) => {
                for (p0, p1, p2, p3) in [(a.x, b.x, c.x, d.x), (a.y, b.y, c.y, d.y)] {
                    // d/dt ∝ qa t² + qb t + qc
                    let qa = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
                    let qb = 2.0 * (p0 - 2.0 * p1 + p2);
                    let qc = p1 - p0;
                    roots.extend(quadratic_roots(qa, qb, qc));
                }
            }
        }
        roots.retain(|t| *t > 0.0 && *t < 1.0);
        roots.sort_by(f64::total_cmp);
        roots.dedup();
        roots
    }

    /// Approximates the curve by line segments whose deviation from the true
    /// curve is at most `tolerance`, appending the points after the start
    /// point (the end point is always included).
    pub fn flatten_into(&self, tolerance: f64, out: &mut Vec<Point>) {
        let segments = self.segments_for_tolerance(tolerance);
        let step = 1.0 / segments as f64;
        out.extend((1..segments).map(|i| self.eval(i as f64 * step)));
        out.push(self.end());
    }

    /// Number of uniform parameter steps that keeps the chord deviation
    /// below `tolerance`, using the classic second-difference bound.
    fn segments_for_tolerance(&self, tolerance: f64) -> usize {
        let tolerance = tolerance.max(1e-9);
        let dd = match *self {
            Self::Line(..) => return 1,
            Self::Quad(a, b, c) => (a - b * 2.0 + c).length() * 2.0,
            Self::Cubic(a, b, c, d) => {
                let d1 = (a - b * 2.0 + c).length();
                let d2 = (b - c * 2.0 + d).length();
                d1.max(d2) * 6.0
            }
        };
        let n = (dd / (8.0 * tolerance)).sqrt().ceil();
        if n.is_finite() {
            (n as usize).clamp(1, 10_000)
        } else {
            1
        }
    }
}

/// Real roots of `a t² + b t + c = 0`, degrading gracefully to linear.
fn quadratic_roots(a: f64, b: f64, c: f64) -> Vec<f64> {
    const EPS: f64 = 1e-12;
    if a.abs() < EPS {
        return if b.abs() < EPS {
            Vec::new()
        } else {
            vec![-c / b]
        };
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return Vec::new();
    }
    let sq = disc.sqrt();
    // Numerically stable form avoiding cancellation.
    let q = -0.5 * (b + b.signum() * sq);
    if q == 0.0 {
        return vec![0.0];
    }
    vec![q / a, c / q]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn eval_endpoints_and_midpoint() {
        let c = Bezier::Cubic(p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0));
        assert_eq!(c.eval(0.0), p(0.0, 0.0));
        assert_eq!(c.eval(1.0), p(1.0, 0.0));
        assert_eq!(c.eval(0.5), p(0.5, 0.75));
    }

    #[test]
    fn split_reconstructs_curve() {
        let c = Bezier::Cubic(p(0.0, 0.0), p(1.0, 3.0), p(2.0, -1.0), p(4.0, 1.0));
        let (left, right) = c.split(0.3);
        for i in 0..=10 {
            let t = f64::from(i) / 10.0;
            assert!((left.eval(t) - c.eval(0.3 * t)).length() < 1e-12);
            assert!((right.eval(t) - c.eval(0.3 + 0.7 * t)).length() < 1e-12);
        }
    }

    #[test]
    fn extrema_of_symmetric_arch() {
        let c = Bezier::Cubic(p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0));
        assert_eq!(c.axis_aligned_extrema(), vec![0.5]);
        let q = Bezier::Quad(p(0.0, 0.0), p(1.0, 2.0), p(2.0, 0.0));
        assert_eq!(q.axis_aligned_extrema(), vec![0.5]);
    }

    #[test]
    fn derivative_matches_finite_difference() {
        let c = Bezier::Cubic(p(0.0, 0.0), p(1.0, 3.0), p(2.0, -1.0), p(4.0, 1.0));
        let h = 1e-6;
        let numeric = (c.eval(0.4 + h) - c.eval(0.4 - h)) / (2.0 * h);
        assert!((numeric - c.derivative(0.4)).length() < 1e-6);
    }

    #[test]
    fn flattening_respects_tolerance() {
        let c = Bezier::Cubic(p(0.0, 0.0), p(0.0, 100.0), p(100.0, 100.0), p(100.0, 0.0));
        let mut pts = vec![c.start()];
        c.flatten_into(0.1, &mut pts);
        assert_eq!(*pts.last().expect("non-empty"), c.end());
        // Every chord midpoint stays close to the curve.
        let n = pts.len() - 1;
        for i in 0..n {
            let t = (i as f64 + 0.5) / n as f64;
            let mid = pts[i].lerp(pts[i + 1], 0.5);
            assert!((mid - c.eval(t)).length() < 0.25);
        }
    }
}
