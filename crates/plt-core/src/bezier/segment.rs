//! matplotlib's `BezierSegment`: a Bézier curve of any degree in any
//! dimension, with its polynomial form and axis-aligned extrema.

use std::fmt;

use crate::Point;

/// Why Bézier control points or a curve split were rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BezierError {
    /// No control points were given (or they have no coordinates).
    Empty,
    /// The control points do not all have the same dimension.
    RaggedControlPoints,
    /// Both ends of the curve lie on the same side of the closed path
    /// (matplotlib's `NonIntersectingPathException`).
    NotIntersecting,
    /// The two lines are parallel, so they have no single intersection.
    ParallelLines,
}

impl fmt::Display for BezierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "a Bézier segment needs at least one control point",
            Self::RaggedControlPoints => "all control points must have the same dimension",
            Self::NotIntersecting => "both points are on the same side of the closed path",
            Self::ParallelLines => "the lines do not intersect",
        })
    }
}

impl std::error::Error for BezierError {}

/// A d-dimensional Bézier curve given by its control points (matplotlib's
/// `BezierSegment`).
#[derive(Clone, Debug, PartialEq)]
pub struct BezierSegment {
    dimension: usize,
    /// Control points, row after row.
    coordinates: Vec<f64>,
}

/// `C(n, k)` as a float (exact for the degrees used here).
fn binomial(n: usize, k: usize) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

impl BezierSegment {
    /// A curve through control points of `dimension` coordinates each,
    /// stored row after row in `coordinates`.
    pub fn new(dimension: usize, coordinates: Vec<f64>) -> Result<Self, BezierError> {
        if dimension == 0 || coordinates.is_empty() {
            return Err(BezierError::Empty);
        }
        if !coordinates.len().is_multiple_of(dimension) {
            return Err(BezierError::RaggedControlPoints);
        }
        Ok(Self {
            dimension,
            coordinates,
        })
    }

    /// A curve from rows of coordinates (all of the same length).
    pub fn from_rows<R: AsRef<[f64]>>(rows: &[R]) -> Result<Self, BezierError> {
        let dimension = rows.first().ok_or(BezierError::Empty)?.as_ref().len();
        if rows.iter().any(|r| r.as_ref().len() != dimension) {
            return Err(BezierError::RaggedControlPoints);
        }
        Self::new(
            dimension,
            rows.iter()
                .flat_map(|r| r.as_ref().iter().copied())
                .collect(),
        )
    }

    /// A planar curve.
    pub fn from_points(points: &[Point]) -> Result<Self, BezierError> {
        Self::new(2, points.iter().flat_map(|p| [p.x, p.y]).collect())
    }

    /// Number of coordinates per control point.
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// The degree: one less than the number of control points.
    pub fn degree(&self) -> usize {
        self.coordinates.len() / self.dimension - 1
    }

    /// The control points, one slice of coordinates each.
    pub fn control_points(&self) -> impl ExactSizeIterator<Item = &[f64]> {
        self.coordinates.chunks_exact(self.dimension)
    }

    /// The control points of a planar curve (`None` in other dimensions).
    pub fn points(&self) -> Option<Vec<Point>> {
        (self.dimension == 2).then(|| {
            self.control_points()
                .map(|c| Point::new(c[0], c[1]))
                .collect()
        })
    }

    /// The point at parameter `t`, computed like matplotlib's `__call__`:
    /// `Σ C(n, i) (1 - t)^(n - i) tⁱ Pᵢ`.
    pub fn eval(&self, t: f64) -> Vec<f64> {
        let n = self.degree();
        let mut out = vec![0.0; self.dimension];
        for (i, point) in self.control_points().enumerate() {
            let weight = (1.0 - t).powi((n - i) as i32) * t.powi(i as i32);
            let coeff = binomial(n, i);
            for (o, c) in out.iter_mut().zip(point) {
                *o += weight * (c * coeff);
            }
        }
        out
    }

    /// The first two coordinates of [`eval`](Self::eval) as a point.
    pub fn point_at(&self, t: f64) -> Point {
        let v = self.eval(t);
        Point::new(v[0], v.get(1).copied().unwrap_or(0.0))
    }

    /// The power-basis coefficients `Cⱼ` with `B(t) = Σ Cⱼ tʲ`, one row per
    /// power (matplotlib's `polynomial_coefficients`).
    pub fn polynomial_coefficients(&self) -> Vec<Vec<f64>> {
        let n = self.degree();
        let points: Vec<&[f64]> = self.control_points().collect();
        (0..=n)
            .map(|j| {
                let mut row = vec![0.0; self.dimension];
                for (i, point) in points.iter().enumerate().take(j + 1) {
                    let sign = if (i + j) % 2 == 0 { 1.0 } else { -1.0 };
                    let m = binomial(n, j) * sign * binomial(j, i);
                    for (r, c) in row.iter_mut().zip(*point) {
                        *r += m * c;
                    }
                }
                row
            })
            .collect()
    }

    /// The parameters in `[0, 1]` where a coordinate's derivative vanishes,
    /// with the coordinate (dimension) index of each: matplotlib's
    /// `axis_aligned_extrema`, grouped by dimension, ascending within one.
    pub fn axis_aligned_extrema(&self) -> (Vec<usize>, Vec<f64>) {
        let n = self.degree();
        let mut dims = Vec::new();
        let mut roots = Vec::new();
        if n <= 1 {
            return (dims, roots);
        }
        let coefficients = self.polynomial_coefficients();
        for d in 0..self.dimension {
            let derivative: Vec<f64> = coefficients[1..]
                .iter()
                .enumerate()
                .map(|(k, row)| (k + 1) as f64 * row[d])
                .collect();
            for root in real_roots_in_01(&derivative) {
                dims.push(d);
                roots.push(root);
            }
        }
        (dims, roots)
    }

    /// Splits the curve at `t` with de Casteljau's algorithm (matplotlib's
    /// `split_de_casteljau`): the halves before and after `t`.
    pub fn split(&self, t: f64) -> (Self, Self) {
        let d = self.dimension;
        let mut level = self.coordinates.clone();
        let mut left = level[..d].to_vec();
        let mut right_rev = level[level.len() - d..].to_vec();
        while level.len() > d {
            level = level
                .chunks_exact(d)
                .zip(level.chunks_exact(d).skip(1))
                .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| a * (1.0 - t) + b * t))
                .collect();
            left.extend_from_slice(&level[..d]);
            right_rev.extend_from_slice(&level[level.len() - d..]);
        }
        let right = right_rev.chunks_exact(d).rev().flatten().copied().collect();
        (
            Self {
                dimension: d,
                coordinates: left,
            },
            Self {
                dimension: d,
                coordinates: right,
            },
        )
    }
}

/// matplotlib's `_real_roots_in_01`: the sorted real roots in `[0, 1]` of
/// `Σ cₖ tᵏ` (ascending coefficients), ignoring near-zero leading terms.
fn real_roots_in_01(coefficients: &[f64]) -> Vec<f64> {
    let degree = coefficients
        .iter()
        .rposition(|c| c.abs() >= 1e-12)
        .unwrap_or(0);
    let c = &coefficients[..=degree];
    let mut roots = match degree {
        0 => Vec::new(),
        1 => vec![-c[0] / c[1]],
        2 => quadratic_roots(c[0], c[1], c[2]),
        _ => {
            const EPS: f64 = 1e-10;
            polynomial_roots(c, -EPS, 1.0 + EPS)
                .into_iter()
                .map(|r| r.clamp(0.0, 1.0))
                .collect()
        }
    };
    roots.retain(|r| (0.0..=1.0).contains(r));
    roots.sort_by(f64::total_cmp);
    roots
}

/// matplotlib's `_quadratic_roots_in_01` before the range filter: the
/// numerically stable quadratic formula.
fn quadratic_roots(c0: f64, c1: f64, c2: f64) -> Vec<f64> {
    let disc = c1 * c1 - 4.0 * c2 * c0;
    if disc < 0.0 {
        return Vec::new();
    }
    let sqrt_disc = disc.sqrt();
    let q = if c1 >= 0.0 {
        -0.5 * (c1 + sqrt_disc)
    } else {
        -0.5 * (c1 - sqrt_disc)
    };
    let mut roots = Vec::with_capacity(2);
    if q.abs() > 1e-12 {
        roots.push(c0 / q);
    }
    if c2.abs() > 1e-12 {
        roots.push(q / c2);
    }
    roots
}

fn horner(c: &[f64], t: f64) -> f64 {
    c.iter().rev().fold(0.0, |acc, k| acc * t + k)
}

/// The real roots of `Σ cₖ tᵏ` (degree ≥ 1) in `[lo, hi]`, located between
/// the roots of the derivative and refined by bisection. (matplotlib calls
/// `numpy.roots` here; both agree to ~1e-12 on simple roots.)
fn polynomial_roots(c: &[f64], lo: f64, hi: f64) -> Vec<f64> {
    if c.len() == 2 {
        let r = -c[0] / c[1];
        return if (lo..=hi).contains(&r) {
            vec![r]
        } else {
            Vec::new()
        };
    }
    let derivative: Vec<f64> = c[1..]
        .iter()
        .enumerate()
        .map(|(k, v)| (k + 1) as f64 * v)
        .collect();
    let mut knots = vec![lo];
    knots.extend(polynomial_roots(&derivative, lo, hi));
    knots.push(hi);
    let mut roots = Vec::new();
    for pair in knots.windows(2) {
        let (mut a, mut b) = (pair[0], pair[1]);
        let (fa, fb) = (horner(c, a), horner(c, b));
        if fa == 0.0 {
            roots.push(a);
            continue;
        }
        if fa.signum() == fb.signum() {
            continue;
        }
        while b - a > f64::EPSILON * b.abs().max(1.0) {
            let m = f64::midpoint(a, b);
            if horner(c, m).signum() == fa.signum() {
                a = m;
            } else {
                b = m;
            }
        }
        roots.push(f64::midpoint(a, b));
    }
    if horner(c, hi) == 0.0 {
        roots.push(hi);
    }
    roots.dedup();
    roots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cubic() -> BezierSegment {
        BezierSegment::from_rows(&[[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]).expect("valid")
    }

    #[test]
    fn evaluation_and_properties() {
        let c = cubic();
        assert_eq!((c.degree(), c.dimension()), (3, 2));
        assert_eq!(c.eval(0.5), vec![0.5, 0.75]);
        assert_eq!(c.point_at(0.0), Point::new(0.0, 0.0));
        assert_eq!(
            c.polynomial_coefficients(),
            vec![
                vec![0.0, 0.0],
                vec![0.0, 3.0],
                vec![3.0, -3.0],
                vec![-2.0, 0.0]
            ]
        );
        assert_eq!(
            BezierSegment::from_rows::<[f64; 2]>(&[]),
            Err(BezierError::Empty)
        );
        assert_eq!(
            BezierSegment::from_rows(&[vec![0.0, 0.0], vec![1.0]]),
            Err(BezierError::RaggedControlPoints)
        );
    }

    #[test]
    fn extrema_like_matplotlib() {
        // x' = 6t(1 - t) vanishes at both ends, y' at the middle.
        assert_eq!(
            cubic().axis_aligned_extrema(),
            (vec![0, 0, 1], vec![0.0, 1.0, 0.5])
        );
        let line = BezierSegment::from_rows(&[[0.0, 0.0], [4.0, 2.0]]).expect("valid");
        assert_eq!(line.axis_aligned_extrema(), (vec![], vec![]));
        let quartic = BezierSegment::from_rows(&[
            [0.0, 0.0],
            [1.0, 3.0],
            [2.0, -2.0],
            [3.0, 2.0],
            [4.0, 0.0],
        ])
        .expect("valid");
        let (dims, roots) = quartic.axis_aligned_extrema();
        assert_eq!(dims, vec![1, 1, 1]);
        for (got, want) in roots.iter().zip([
            0.197_868_915_336_342_77,
            0.582_517_508_272_926_4,
            0.813_363_576_390_730_6,
        ]) {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn de_casteljau_split() {
        let (left, right) = cubic().split(0.5);
        assert_eq!(left.points().expect("2-D")[3], Point::new(0.5, 0.75));
        assert_eq!(right.points().expect("2-D")[0], Point::new(0.5, 0.75));
        assert_eq!(right.points().expect("2-D")[3], Point::new(1.0, 0.0));
        let point = BezierSegment::from_rows(&[[1.5, -2.0]]).expect("valid");
        assert_eq!(point.split(0.3), (point.clone(), point));
    }
}
