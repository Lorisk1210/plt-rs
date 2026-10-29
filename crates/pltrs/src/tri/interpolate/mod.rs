//! Interpolation of nodal values over a triangulation.

mod cubic;
mod dof;
mod hct;

pub(crate) use cubic::ptp;
pub use cubic::{CubicKind, CubicTriInterpolator};

use super::{TriError, TriFinder, Triangulation};

/// Interpolates a field given at the points of a triangulation
/// (matplotlib's `TriInterpolator`).
///
/// Queries outside the unmasked triangles give `None`; results are NaN
/// where the field itself is undefined (NaN nodal values).
pub trait TriInterpolator {
    /// The point locator used for queries.
    fn finder(&self) -> &TriFinder<'_>;

    /// The interpolant of triangle `tri` evaluated at `(x, y)`.
    ///
    /// Like matplotlib, masked triangles are not rejected: a linear
    /// interpolator evaluates their zero plane and a cubic one the patch of
    /// the last unmasked triangle.
    fn value_in(&self, tri: usize, x: f64, y: f64) -> f64;

    /// The gradient `[dz/dx, dz/dy]` of the interpolant of triangle `tri`
    /// at `(x, y)` (masked triangles as for [`TriInterpolator::value_in`]).
    fn gradient_in(&self, tri: usize, x: f64, y: f64) -> [f64; 2];

    /// The interpolated value at `(x, y)`.
    fn value(&self, x: f64, y: f64) -> Option<f64> {
        let tri = self.finder().find(x, y)?;
        Some(self.value_in(tri, x, y))
    }

    /// The interpolated gradient `[dz/dx, dz/dy]` at `(x, y)`.
    fn gradient(&self, x: f64, y: f64) -> Option<[f64; 2]> {
        let tri = self.finder().find(x, y)?;
        Some(self.gradient_in(tri, x, y))
    }
}

/// Piecewise-linear interpolation: each triangle is the plane through its
/// vertices (matplotlib's `LinearTriInterpolator`).
#[derive(Clone, Debug)]
pub struct LinearTriInterpolator<'a> {
    finder: TriFinder<'a>,
    planes: Vec<[f64; 3]>,
}

impl<'a> LinearTriInterpolator<'a> {
    /// Interpolates `z` (one value per point) over `tri`.
    pub fn new(tri: &'a Triangulation, z: &[f64]) -> Result<Self, TriError> {
        let planes = tri.plane_coefficients(z)?;
        Ok(Self {
            finder: tri.trifinder()?,
            planes,
        })
    }
}

impl TriInterpolator for LinearTriInterpolator<'_> {
    fn finder(&self) -> &TriFinder<'_> {
        &self.finder
    }

    fn value_in(&self, tri: usize, x: f64, y: f64) -> f64 {
        let [a, b, c] = self.planes[tri];
        a * x + b * y + c
    }

    fn gradient_in(&self, tri: usize, _x: f64, _y: f64) -> [f64; 2] {
        let [a, b, _] = self.planes[tri];
        [a, b]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_interpolation_and_gradient() {
        let tri = Triangulation::new(
            &[0.0, 1.0, 0.0, 1.0],
            &[0.0, 0.0, 1.0, 1.0],
            vec![[0, 1, 3], [0, 3, 2]],
        )
        .unwrap();
        let interp = LinearTriInterpolator::new(&tri, &[1.0, 2.0, 3.0, 5.0]).unwrap();
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-14);
        assert!(close(interp.value(0.75, 0.25), 2.5));
        let [gx, gy] = interp.gradient(0.75, 0.25).unwrap();
        assert!(close(Some(gx), 1.0) && close(Some(gy), 3.0));
        assert!(close(interp.value(0.25, 0.75), 3.0));
        assert_eq!(interp.value(-0.1, 0.5), None);
        assert!(LinearTriInterpolator::new(&tri, &[1.0]).is_err());
    }
}
