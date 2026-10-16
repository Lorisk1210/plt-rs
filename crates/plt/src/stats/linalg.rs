//! Determinant and inverse of small dense matrices by LU decomposition with
//! partial pivoting (LAPACK `getrf`/`getri` semantics: a matrix is singular
//! exactly when a pivot is zero).

use crate::numeric::Grid;

/// `P·A = L·U` of a square matrix, packed LAPACK-style.
#[derive(Clone, Debug)]
pub(crate) struct Lu {
    /// Unit-lower `L` below the diagonal, `U` on and above it.
    packed: Grid<f64>,
    /// Row `i` of `P·A` is row `perm[i]` of `A`.
    perm: Vec<usize>,
    /// Number of row swaps.
    swaps: usize,
}

impl Lu {
    /// Decomposes `a`; `None` if a pivot is exactly zero (singular).
    pub(crate) fn new(a: &Grid<f64>) -> Option<Self> {
        let n = a.rows();
        let mut m = a.clone();
        let mut perm: Vec<usize> = (0..n).collect();
        let mut swaps = 0;
        for k in 0..n {
            let pivot_row = (k..n)
                .max_by(|&i, &j| m[(i, k)].abs().total_cmp(&m[(j, k)].abs()).then(j.cmp(&i)))?;
            if m[(pivot_row, k)] == 0.0 {
                return None;
            }
            if pivot_row != k {
                for c in 0..n {
                    let tmp = m[(k, c)];
                    m[(k, c)] = m[(pivot_row, c)];
                    m[(pivot_row, c)] = tmp;
                }
                perm.swap(k, pivot_row);
                swaps += 1;
            }
            let reciprocal = 1.0 / m[(k, k)];
            for r in k + 1..n {
                let factor = m[(r, k)] * reciprocal;
                m[(r, k)] = factor;
                for c in k + 1..n {
                    m[(r, c)] -= factor * m[(k, c)];
                }
            }
        }
        Some(Self {
            packed: m,
            perm,
            swaps,
        })
    }

    /// The determinant: the product of `U`'s diagonal, sign-corrected.
    pub(crate) fn determinant(&self) -> f64 {
        let product = (0..self.packed.rows()).fold(1.0, |acc, i| acc * self.packed[(i, i)]);
        if self.swaps % 2 == 1 {
            -product
        } else {
            product
        }
    }

    /// The inverse, solving `A·X = I` column by column.
    pub(crate) fn inverse(&self) -> Grid<f64> {
        let n = self.packed.rows();
        let mut inv = Grid::filled(n, n, 0.0);
        for col in 0..n {
            let mut x: Vec<f64> = self
                .perm
                .iter()
                .map(|&p| if p == col { 1.0 } else { 0.0 })
                .collect();
            for r in 0..n {
                for c in 0..r {
                    x[r] -= self.packed[(r, c)] * x[c];
                }
            }
            for r in (0..n).rev() {
                for c in r + 1..n {
                    x[r] -= self.packed[(r, c)] * x[c];
                }
                x[r] /= self.packed[(r, r)];
            }
            for (r, v) in x.into_iter().enumerate() {
                inv[(r, col)] = v;
            }
        }
        inv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinant_and_inverse() {
        let a = Grid::from_rows([[4.0, 3.0], [6.0, 3.0]]).unwrap();
        let lu = Lu::new(&a).unwrap();
        assert!((lu.determinant() + 6.0).abs() < 1e-14);
        let inv = lu.inverse();
        for (got, want) in inv.as_slice().iter().zip([-0.5, 0.5, 1.0, -2.0 / 3.0]) {
            assert!((got - want).abs() < 1e-15, "{got} vs {want}");
        }
    }

    #[test]
    fn three_by_three() {
        let a = Grid::from_rows([[2.0, 0.0, 1.0], [1.0, 3.0, 2.0], [1.0, 1.0, 2.0]]).unwrap();
        let lu = Lu::new(&a).unwrap();
        assert!((lu.determinant() - 6.0).abs() < 1e-14);
        let inv = lu.inverse();
        let adjugate = [4.0, 1.0, -3.0, 0.0, 3.0, -3.0, -2.0, -2.0, 6.0];
        for (got, want) in inv.as_slice().iter().zip(adjugate) {
            assert!((got - want / 6.0).abs() < 1e-14, "{got} vs {want}");
        }
    }

    #[test]
    fn singular_matrices_have_no_decomposition() {
        assert!(Lu::new(&Grid::from_rows([[1.0, 2.0], [2.0, 4.0]]).unwrap()).is_none());
        assert!(Lu::new(&Grid::from_rows([[0.0]]).unwrap()).is_none());
    }
}
