//! Estimation of nodal gradients for the cubic interpolator (matplotlib's
//! `_DOF_estimator_geom` and `_DOF_estimator_min_E`), and their conversion
//! to element degrees of freedom.

use std::f64::consts::PI;

use super::hct::{self, J0_TO_J1, J0_TO_J2, Mat2, safe_inv22};

/// A compressed, rescaled triangulation with nodal values.
#[derive(Clone, Copy, Debug)]
pub(super) struct Mesh<'a> {
    /// Node coordinates.
    pub(super) pts: &'a [[f64; 2]],
    /// Triangles (anticlockwise node indices).
    pub(super) tris: &'a [[usize; 3]],
    /// Nodal values.
    pub(super) z: &'a [f64],
}

impl Mesh<'_> {
    fn corners(&self, t: usize) -> [[f64; 2]; 3] {
        self.tris[t].map(|v| self.pts[v])
    }
}

/// The Jacobian at apex 0: rows are the edges `p1 - p0` and `p2 - p0`.
pub(super) fn jacobian([p0, p1, p2]: [[f64; 2]; 3]) -> Mat2 {
    [
        [p1[0] - p0[0], p1[1] - p0[1]],
        [p2[0] - p0[0], p2[1] - p0[1]],
    ]
}

/// Element degrees of freedom from nodal values and gradients.
pub(super) fn element_dofs(mesh: Mesh<'_>, dz: &[[f64; 2]]) -> Vec<[f64; 9]> {
    (0..mesh.tris.len())
        .map(|t| {
            let j = jacobian(mesh.corners(t));
            let js = [j, hct::mat2_mul(J0_TO_J1, j), hct::mat2_mul(J0_TO_J2, j)];
            let mut dof = [0.0; 9];
            for (apex, (&node, jac)) in mesh.tris[t].iter().zip(js).enumerate() {
                let g = dz[node];
                dof[3 * apex] = mesh.z[node];
                dof[3 * apex + 1] = jac[0][0] * g[0] + jac[0][1] * g[1];
                dof[3 * apex + 2] = jac[1][0] * g[0] + jac[1][1] * g[1];
            }
            dof
        })
        .collect()
}

/// Nodal gradients as angle-weighted averages of the triangles' linear
/// gradients ('geom').
pub(super) fn geom_gradients(mesh: Mesh<'_>) -> Vec<[f64; 2]> {
    let n = mesh.pts.len();
    let mut weight_sum = vec![0.0; n];
    let mut dfx_sum = vec![0.0; n];
    let mut dfy_sum = vec![0.0; n];
    for t in 0..mesh.tris.len() {
        let corners = mesh.corners(t);
        let weights = angle_weights(corners);
        let grad = linear_gradient(corners, mesh.tris[t].map(|v| mesh.z[v]));
        for (apex, &node) in mesh.tris[t].iter().enumerate() {
            weight_sum[node] += weights[apex];
            dfx_sum[node] += weights[apex] * grad[0];
            dfy_sum[node] += weights[apex] * grad[1];
        }
    }
    (0..n)
        .map(|i| [dfx_sum[i] / weight_sum[i], dfy_sum[i] / weight_sum[i]])
        .collect()
}

/// Per-apex weights proportional to the apex angle up to `pi / 2`, zero
/// for degenerate (0 or `pi`) angles.
fn angle_weights(corners: [[f64; 2]; 3]) -> [f64; 3] {
    std::array::from_fn(|i| {
        let (p0, p1, p2) = (corners[i], corners[(i + 1) % 3], corners[(i + 2) % 3]);
        let alpha1 = (p1[1] - p0[1]).atan2(p1[0] - p0[0]);
        let alpha2 = (p2[1] - p0[1]).atan2(p2[0] - p0[0]);
        let angle = ((alpha2 - alpha1) / PI).rem_euclid(1.0).abs();
        0.5 - (angle - 0.5).abs()
    })
}

/// The gradient of the linear interpolant of one triangle (zero for flat
/// triangles).
fn linear_gradient([p0, p1, p2]: [[f64; 2]; 3], [f0, f1, f2]: [f64; 3]) -> [f64; 2] {
    let dm = [
        [p1[0] - p0[0], p2[0] - p0[0]],
        [p1[1] - p0[1], p2[1] - p0[1]],
    ];
    let inv = safe_inv22(dm);
    let (dz1, dz2) = (f1 - f0, f2 - f0);
    [
        dz1 * inv[0][0] + dz2 * inv[1][0],
        dz1 * inv[0][1] + dz2 * inv[1][1],
    ]
}

/// Nodal gradients minimising the bending energy of the HCT interpolant
/// (`min_E`), solved by Jacobi-preconditioned conjugate gradients from the
/// 'geom' estimate. If the solver does not improve on its starting point
/// (e.g. NaN values), the 'geom' estimate is returned.
pub(super) fn min_energy_gradients(mesh: Mesh<'_>, eccs: &[[f64; 3]]) -> Vec<[f64; 2]> {
    let start: Vec<f64> = geom_gradients(mesh).into_iter().flatten().collect();
    let (matrix, rhs) = stiffness_system(mesh, eccs);
    let (solution, err) = conjugate_gradient(&matrix, &rhs, start.clone(), 1e-10, 1000);
    let start_err = residual_norm(&matrix, &start, &rhs);
    let best = if start_err < err { start } else { solution };
    best.as_chunks::<2>().0.to_vec()
}

const FREE_DOFS: [usize; 6] = [1, 2, 4, 5, 7, 8];
const VALUE_DOFS: [usize; 3] = [0, 3, 6];

/// The free-dof stiffness matrix `Kff` and force vector `Ff = -Kfc Uc`.
fn stiffness_system(mesh: Mesh<'_>, eccs: &[[f64; 3]]) -> (SparseMatrix, Vec<f64>) {
    let n = 2 * mesh.pts.len();
    let mut entries = Vec::with_capacity(36 * mesh.tris.len());
    let mut force = vec![0.0; n];
    for (t, &ecc) in eccs.iter().enumerate() {
        let k = hct::bending_matrix(jacobian(mesh.corners(t)), ecc);
        let nodes = mesh.tris[t];
        let index = |dof: usize| 2 * nodes[dof / 3] + dof % 3 - 1;
        for &i in &FREE_DOFS {
            for &j in &FREE_DOFS {
                entries.push((index(i), index(j), k[i][j]));
            }
        }
        for &i in &FREE_DOFS {
            let kfc = VALUE_DOFS
                .iter()
                .zip(nodes)
                .fold(0.0, |acc, (&c, node)| acc + k[i][c] * mesh.z[node]);
            force[index(i)] += -kfc;
        }
    }
    (SparseMatrix::compress(n, entries), force)
}

/// A square sparse matrix with summed entries sorted by (column, row), as
/// matplotlib's `_Sparse_Matrix_coo.compress_csc` leaves them.
#[derive(Clone, Debug)]
struct SparseMatrix {
    n: usize,
    entries: Vec<(usize, usize, f64)>,
}

impl SparseMatrix {
    fn compress(n: usize, mut entries: Vec<(usize, usize, f64)>) -> Self {
        entries.sort_by_key(|&(row, col, _)| (col, row));
        let mut merged: Vec<(usize, usize, f64)> = Vec::with_capacity(entries.len());
        for (row, col, value) in entries {
            match merged.last_mut() {
                Some(last) if last.0 == row && last.1 == col => last.2 += value,
                _ => merged.push((row, col, value)),
            }
        }
        Self { n, entries: merged }
    }

    fn dot(&self, v: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; self.n];
        for &(row, col, value) in &self.entries {
            out[row] += value * v[col];
        }
        out
    }

    fn diagonal(&self) -> Vec<f64> {
        let mut diag = vec![0.0; self.n];
        for &(row, col, value) in &self.entries {
            if row == col {
                diag[row] = value;
            }
        }
        diag
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn residual_norm(a: &SparseMatrix, x: &[f64], b: &[f64]) -> f64 {
    a.dot(x)
        .iter()
        .zip(b)
        .map(|(ax, b)| (ax - b) * (ax - b))
        .sum::<f64>()
        .sqrt()
}

/// matplotlib's `_cg`: Jacobi-preconditioned conjugate gradients following
/// C. T. Kelley; returns the solution and its residual norm.
fn conjugate_gradient(
    a: &SparseMatrix,
    b: &[f64],
    mut x: Vec<f64>,
    tol: f64,
    max_iter: usize,
) -> (Vec<f64>, f64) {
    let b_norm = dot(b, b).sqrt();
    let kvec: Vec<f64> = a.diagonal().into_iter().map(|d| d.max(1e-6)).collect();
    let mut r: Vec<f64> = b.iter().zip(a.dot(&x)).map(|(b, ax)| b - ax).collect();
    let mut w: Vec<f64> = r.iter().zip(&kvec).map(|(r, k)| r / k).collect();
    let mut p = vec![0.0; b.len()];
    let mut beta = 0.0;
    let mut rho = dot(&r, &w);
    let mut k = 0;
    while rho.abs().sqrt() > tol * b_norm && k < max_iter {
        for (p, w) in p.iter_mut().zip(&w) {
            *p = w + beta * *p;
        }
        let z = a.dot(&p);
        let alpha = rho / dot(&p, &z);
        for (r, z) in r.iter_mut().zip(&z) {
            *r -= alpha * z;
        }
        for ((w, r), k) in w.iter_mut().zip(&r).zip(&kvec) {
            *w = r / k;
        }
        let rho_old = rho;
        rho = dot(&r, &w);
        for (x, p) in x.iter_mut().zip(&p) {
            *x += alpha * p;
        }
        beta = rho / rho_old;
        k += 1;
    }
    let err = residual_norm(a, &x, b);
    (x, err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conjugate_gradient_solves_spd_systems() {
        let a = SparseMatrix::compress(
            2,
            vec![
                (0, 0, 4.0),
                (0, 1, 1.0),
                (1, 0, 1.0),
                (1, 1, 2.0),
                (1, 1, 1.0),
            ],
        );
        let (x, err) = conjugate_gradient(&a, &[1.0, 2.0], vec![0.0, 0.0], 1e-12, 100);
        assert!(err < 1e-10);
        assert!((4.0 * x[0] + x[1] - 1.0).abs() < 1e-10);
        assert!((x[0] + 3.0 * x[1] - 2.0).abs() < 1e-10);
    }

    /// The matrix as dense rows.
    fn dense(a: &SparseMatrix) -> Vec<Vec<f64>> {
        let mut rows = vec![vec![0.0; a.n]; a.n];
        for &(row, col, value) in &a.entries {
            rows[row][col] += value;
        }
        rows
    }

    /// The 2-D Poisson matrix of an `n × m` grid as COO entries, built as
    /// matplotlib's `test_triinterpcubic_cg_solver` does (zeros left in the
    /// first off-diagonals at the row ends).
    fn poisson(n: usize, m: usize) -> Vec<(usize, usize, f64)> {
        let l = n * m;
        let mut entries: Vec<(usize, usize, f64)> = (0..l).map(|k| (k, k, 4.0)).collect();
        let end_of_row = |k: usize| if k % m == m - 1 { 0.0 } else { -1.0 };
        entries.extend((0..l - 1).map(|k| (k, k + 1, end_of_row(k))));
        entries.extend((0..l - 1).map(|k| (k + 1, k, end_of_row(k))));
        entries.extend((0..l - n).map(|k| (k, k + n, -1.0)));
        entries.extend((0..l - n).map(|k| (k + n, k, -1.0)));
        entries
    }

    /// Solves `a x = e_i` for every basis vector from `start` and checks
    /// the solution to six decimals.
    fn solves_every_basis_vector(a: &SparseMatrix, start: f64) {
        let rows = dense(a);
        for i in 0..a.n {
            let b: Vec<f64> = (0..a.n).map(|k| f64::from(u8::from(k == i))).collect();
            let (x, _) = conjugate_gradient(a, &b, vec![start; a.n], 1e-10, 1000);
            for (row, expected) in rows.iter().zip(&b) {
                assert!((dot(row, &x) - expected).abs() < 1e-6, "basis vector {i}");
            }
        }
    }

    #[test]
    fn conjugate_gradient_solves_poisson_systems() {
        // matplotlib's test_triinterpcubic_cg_solver, first case.
        solves_every_basis_vector(&SparseMatrix::compress(48, poisson(12, 4)), 0.0);
    }

    #[test]
    fn conjugate_gradient_solves_systems_with_empty_diagonal_terms() {
        // Two rows and columns with a zero diagonal, linked to their
        // neighbours (test_triinterpcubic_cg_solver, second case).
        let shift = |k: usize| k + usize::from(k >= 12) + usize::from(k >= 49);
        let mut entries: Vec<_> = poisson(12, 4)
            .into_iter()
            .map(|(row, col, value)| (shift(row), shift(col), value))
            .collect();
        entries.extend([(12, 11, 1.0), (11, 12, 1.0), (49, 48, 1.0), (48, 49, 1.0)]);
        solves_every_basis_vector(&SparseMatrix::compress(50, entries), 1.0);
    }

    #[test]
    fn compression_sums_duplicate_entries() {
        let rows = [0, 1, 2, 0, 0, 1, 1, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1];
        let cols = [0, 1, 2, 1, 1, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2];
        let entries = rows
            .into_iter()
            .zip(cols)
            .map(|(r, c)| (r, c, 1.0))
            .collect();
        assert_eq!(
            dense(&SparseMatrix::compress(3, entries)),
            [[1.0, 2.0, 0.0], [2.0, 1.0, 5.0], [0.0, 5.0, 1.0]]
        );
    }

    #[test]
    fn angle_weights_sum_to_one_or_twice_the_obtuse_apex() {
        // matplotlib's test_triinterpcubic_geom_weights: a triangle's
        // weights sum to 1 when its angles are acute, else to twice the
        // weight of its obtuse apex, however the figure is rotated.
        let (ax, ay) = (0.0, 1.687);
        let points = [[ax, ay], [0.5 * ax, -ay], [0.0, 0.0], [1.0, 0.0]];
        for step in 0..14 {
            let theta = 2.0 * PI * f64::from(step) / 13.0;
            let (sin, cos) = theta.sin_cos();
            let rotated = points.map(|[x, y]| [cos * x + sin * y, -sin * x + cos * y]);
            for tri in [[0, 2, 3], [1, 3, 2]] {
                let weights = angle_weights(tri.map(|v| rotated[v]));
                let sum: f64 = weights.iter().sum();
                let closest = weights
                    .iter()
                    .map(|w| (sum - 2.0 * w).abs())
                    .fold((sum - 1.0).abs(), f64::min);
                assert!(closest < 1e-6, "rotation {step}: weights {weights:?}");
            }
        }
    }

    #[test]
    fn geom_gradients_of_a_linear_field() {
        let pts = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let tris = [[0, 1, 3], [0, 3, 2]];
        let z: Vec<f64> = pts.iter().map(|p| 2.0 * p[0] - p[1]).collect();
        let mesh = Mesh {
            pts: &pts,
            tris: &tris,
            z: &z,
        };
        for g in geom_gradients(mesh) {
            assert!((g[0] - 2.0).abs() < 1e-14 && (g[1] + 1.0).abs() < 1e-14);
        }
        let eccs: Vec<[f64; 3]> = (0..2)
            .map(|t| hct::eccentricities(mesh.corners(t)))
            .collect();
        for g in min_energy_gradients(mesh, &eccs) {
            assert!((g[0] - 2.0).abs() < 1e-9 && (g[1] + 1.0).abs() < 1e-9);
        }
    }
}
