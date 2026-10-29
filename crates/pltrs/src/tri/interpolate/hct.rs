//! The reduced Hsieh–Clough–Tocher triangular element (matplotlib's
//! `_ReducedHCT_Element`): explicit cubic shape functions on the three
//! sub-triangles, their derivatives, and the element bending stiffness.
//!
//! Shape functions follow Bernadou & Hassan, "Basis functions for general
//! Hsieh-Clough-Tocher triangles, complete or reduced" (1981). An element
//! has 9 degrees of freedom: at each apex `i`, the value and the
//! derivatives along the two adjacent edges (`dof[3i..3i + 3]`).

/// A 2×2 matrix, row-major.
pub(super) type Mat2 = [[f64; 2]; 2];

const M: [[f64; 10]; 9] = [
    [0.00, 0.00, 0.00, 4.50, 4.50, 0.00, 0.00, 0.00, 0.00, 0.00],
    [-0.25, 0.00, 0.00, 0.50, 1.25, 0.00, 0.00, 0.00, 0.00, 0.00],
    [-0.25, 0.00, 0.00, 1.25, 0.50, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.50, 1.00, 0.00, -1.50, 0.00, 3.00, 3.00, 0.00, 0.00, 3.00],
    [0.00, 0.00, 0.00, -0.25, 0.25, 0.00, 1.00, 0.00, 0.00, 0.50],
    [0.25, 0.00, 0.00, -0.50, -0.25, 1.00, 0.00, 0.00, 0.00, 1.00],
    [0.50, 0.00, 1.00, 0.00, -1.50, 0.00, 0.00, 3.00, 3.00, 3.00],
    [0.25, 0.00, 0.00, -0.25, -0.50, 0.00, 0.00, 0.00, 1.00, 1.00],
    [0.00, 0.00, 0.00, 0.25, -0.25, 0.00, 0.00, 1.00, 0.00, 0.50],
];
const M0: [[f64; 10]; 9] = [
    [0.00; 10],
    [0.00; 10],
    [0.00; 10],
    [-1.00, 0.00, 0.00, 1.50, 1.50, 0.00, 0.00, 0.00, 0.00, -3.00],
    [-0.50, 0.00, 0.00, 0.75, 0.75, 0.00, 0.00, 0.00, 0.00, -1.50],
    [0.00; 10],
    [1.00, 0.00, 0.00, -1.50, -1.50, 0.00, 0.00, 0.00, 0.00, 3.00],
    [0.00; 10],
    [0.50, 0.00, 0.00, -0.75, -0.75, 0.00, 0.00, 0.00, 0.00, 1.50],
];
const M1: [[f64; 10]; 9] = [
    [-0.50, 0.00, 0.00, 1.50, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
    [-0.25, 0.00, 0.00, 0.75, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
    [0.00; 10],
    [0.00; 10],
    [0.50, 0.00, 0.00, -1.50, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.25, 0.00, 0.00, -0.75, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
];
const M2: [[f64; 10]; 9] = [
    [0.50, 0.00, 0.00, 0.00, -1.50, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.25, 0.00, 0.00, 0.00, -0.75, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
    [-0.50, 0.00, 0.00, 0.00, 1.50, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
    [-0.25, 0.00, 0.00, 0.00, 0.75, 0.00, 0.00, 0.00, 0.00, 0.00],
    [0.00; 10],
    [0.00; 10],
    [0.00; 10],
];

/// Rotates gradient components into the first apex's basis, per
/// sub-triangle (2 rows each).
const ROTATE_DV: [[f64; 2]; 6] = [
    [1.0, 0.0],
    [0.0, 1.0],
    [0.0, 1.0],
    [-1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 0.0],
];

/// Rotates Hessian components into the first apex's basis, per
/// sub-triangle (3 rows each).
const ROTATE_D2V: [[f64; 3]; 9] = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 1.0],
    [0.0, -2.0, -1.0],
    [1.0, 1.0, 1.0],
    [1.0, 0.0, 0.0],
    [-2.0, 0.0, -1.0],
];

/// Gauss points (barycentric) integrating P2 exactly on each sub-triangle.
const GAUSS_POINTS: [[f64; 3]; 9] = [
    [13.0 / 18.0, 4.0 / 18.0, 1.0 / 18.0],
    [4.0 / 18.0, 13.0 / 18.0, 1.0 / 18.0],
    [7.0 / 18.0, 7.0 / 18.0, 4.0 / 18.0],
    [1.0 / 18.0, 13.0 / 18.0, 4.0 / 18.0],
    [1.0 / 18.0, 4.0 / 18.0, 13.0 / 18.0],
    [4.0 / 18.0, 7.0 / 18.0, 7.0 / 18.0],
    [4.0 / 18.0, 1.0 / 18.0, 13.0 / 18.0],
    [13.0 / 18.0, 1.0 / 18.0, 4.0 / 18.0],
    [7.0 / 18.0, 4.0 / 18.0, 7.0 / 18.0],
];
const GAUSS_WEIGHT: f64 = 1.0 / 9.0;

/// Maps the Jacobian at apex 0 to the Jacobians at apexes 1 and 2.
pub(super) const J0_TO_J1: Mat2 = [[-1.0, 1.0], [-1.0, 0.0]];
/// See [`J0_TO_J1`].
pub(super) const J0_TO_J2: Mat2 = [[0.0, -1.0], [1.0, -1.0]];

/// The eccentricity parameters of a triangle (matplotlib's
/// `_compute_tri_eccentricities`).
pub(super) fn eccentricities([p0, p1, p2]: [[f64; 2]; 3]) -> [f64; 3] {
    let sq = |a: [f64; 2], b: [f64; 2]| {
        let d = [a[0] - b[0], a[1] - b[1]];
        d[0] * d[0] + d[1] * d[1]
    };
    let (dot_a, dot_b, dot_c) = (sq(p2, p1), sq(p0, p2), sq(p1, p0));
    [
        (dot_c - dot_b) / dot_a,
        (dot_a - dot_c) / dot_b,
        (dot_b - dot_a) / dot_c,
    ]
}

/// The interpolated value at barycentric coordinates `alpha`.
pub(super) fn value(alpha: [f64; 3], ecc: [f64; 3], dof: &[f64; 9]) -> f64 {
    let (sub, [x, y, z], e) = sub_triangle(alpha, ecc);
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let v = [
        xx * x,
        yy * y,
        zz * z,
        xx * z,
        xx * y,
        yy * x,
        yy * z,
        zz * y,
        zz * x,
        x * y * z,
    ];
    let prod = combine(e, |m| mat_vec(m, &v));
    let s = roll(&prod, sub);
    dot9(dof, &s)
}

/// The gradient `[dz/dx, dz/dy]` (in the coordinates `jacobian` refers to)
/// at barycentric coordinates `alpha`.
pub(super) fn gradient(alpha: [f64; 3], jacobian: Mat2, ecc: [f64; 3], dof: &[f64; 9]) -> [f64; 2] {
    let (sub, [x, y, z], e) = sub_triangle(alpha, ecc);
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let raw: [[f64; 2]; 10] = [
        [-3.0 * xx, -3.0 * xx],
        [3.0 * yy, 0.0],
        [0.0, 3.0 * zz],
        [-2.0 * x * z, -2.0 * x * z + xx],
        [-2.0 * x * y + xx, -2.0 * x * y],
        [2.0 * x * y - yy, -yy],
        [2.0 * y * z, yy],
        [zz, 2.0 * y * z],
        [-zz, 2.0 * x * z - zz],
        [x * z - y * z, x * y - y * z],
    ];
    let block = [ROTATE_DV[2 * sub], ROTATE_DV[2 * sub + 1]];
    let dv = raw.map(|row| {
        [
            row[0] * block[0][0] + row[1] * block[1][0],
            row[0] * block[0][1] + row[1] * block[1][1],
        ]
    });
    let prod = combine(e, |m| mat_cols(m, &dv));
    let ds = roll(&prod, sub);
    let df = [0, 1].map(|c| (0..9).fold(0.0, |acc, k| acc + dof[k] * ds[k][c]));
    let inv = safe_inv22(jacobian);
    [
        inv[0][0] * df[0] + inv[0][1] * df[1],
        inv[1][0] * df[0] + inv[1][1] * df[1],
    ]
}

/// Second derivatives of the 9 shape functions (rows) in the covariant
/// coordinates of the first apex basis, at barycentric `alpha`.
fn shape_hessians(alpha: [f64; 3], ecc: [f64; 3]) -> [[f64; 3]; 9] {
    let (sub, [x, y, z], e) = sub_triangle(alpha, ecc);
    let raw: [[f64; 3]; 10] = [
        [6.0 * x, 6.0 * x, 6.0 * x],
        [6.0 * y, 0.0, 0.0],
        [0.0, 6.0 * z, 0.0],
        [2.0 * z, 2.0 * z - 4.0 * x, 2.0 * z - 2.0 * x],
        [2.0 * y - 4.0 * x, 2.0 * y, 2.0 * y - 2.0 * x],
        [2.0 * x - 4.0 * y, 0.0, -2.0 * y],
        [2.0 * z, 0.0, 2.0 * y],
        [0.0, 2.0 * y, 2.0 * z],
        [0.0, 2.0 * x - 4.0 * z, -2.0 * z],
        [-2.0 * z, -2.0 * y, x - y - z],
    ];
    let block = [
        ROTATE_D2V[3 * sub],
        ROTATE_D2V[3 * sub + 1],
        ROTATE_D2V[3 * sub + 2],
    ];
    let d2v = raw.map(|row| {
        [0, 1, 2].map(|c| row[0] * block[0][c] + row[1] * block[1][c] + row[2] * block[2][c])
    });
    let prod = combine(e, |m| mat_cols(m, &d2v));
    roll(&prod, sub)
}

/// The element stiffness matrix for the curvature energy, in global nodal
/// degrees of freedom (matplotlib's `get_bending_matrices`).
pub(super) fn bending_matrix(jacobian: Mat2, ecc: [f64; 3]) -> [[f64; 9]; 9] {
    let j1 = mat2_mul(J0_TO_J1, jacobian);
    let j2 = mat2_mul(J0_TO_J2, jacobian);
    let mut dof_rot = [[0.0; 9]; 9];
    for (apex, j) in [jacobian, j1, j2].into_iter().enumerate() {
        let base = 3 * apex;
        dof_rot[base][base] = 1.0;
        for r in 0..2 {
            for c in 0..2 {
                dof_rot[base + 1 + r][base + 1 + c] = j[r][c];
            }
        }
    }
    let (h_rot, area) = hessian_rotation(jacobian);
    let mut k = [[0.0; 9]; 9];
    for point in GAUSS_POINTS {
        let d2s = shape_hessians(point, ecc);
        let d2x =
            d2s.map(|row| [0, 1, 2].map(|c| (0..3).fold(0.0, |acc, i| acc + row[i] * h_rot[i][c])));
        // d2x @ E with E = diag(1, 1, 2).
        let weighted = d2x.map(|row| [row[0], row[1], 2.0 * row[2]]);
        for r in 0..9 {
            for c in 0..9 {
                let term = (0..3).fold(0.0, |acc, i| acc + weighted[r][i] * d2x[c][i]);
                k[r][c] += GAUSS_WEIGHT * term;
            }
        }
    }
    let rotated = mat9_mul(&mat9_mul(&transpose9(&dof_rot), &k), &dof_rot);
    rotated.map(|row| row.map(|v| area * v))
}

/// The rotation of Hessian components from the first apex basis to global
/// coordinates, and the triangle area `det(J) / 2`.
fn hessian_rotation(jacobian: Mat2) -> ([[f64; 3]; 3], f64) {
    let inv = safe_inv22(jacobian);
    let (i00, i11, i10, i01) = (inv[0][0], inv[1][1], inv[1][0], inv[0][1]);
    let h = [
        [i00 * i00, i10 * i10, i00 * i10],
        [i01 * i01, i11 * i11, i01 * i11],
        [2.0 * i00 * i01, 2.0 * i11 * i10, i00 * i11 + i10 * i01],
    ];
    let area = 0.5 * (jacobian[0][0] * jacobian[1][1] - jacobian[0][1] * jacobian[1][0]);
    (h, area)
}

/// Picks the sub-triangle (the apex with the smallest barycentric
/// coordinate) and rotates the coordinates and eccentricities to it.
fn sub_triangle(alpha: [f64; 3], ecc: [f64; 3]) -> (usize, [f64; 3], [f64; 3]) {
    let sub = (1..3).fold(0, |best, i| if alpha[i] < alpha[best] { i } else { best });
    let rot = |v: [f64; 3]| [v[sub], v[(sub + 1) % 3], v[(sub + 2) % 3]];
    (sub, rot(alpha), rot(ecc))
}

/// `M v + e0 M0 v + e1 M1 v + e2 M2 v` with `apply(m) = m v`.
fn combine<T: Copy + Combine>(e: [f64; 3], apply: impl Fn(&[[f64; 10]; 9]) -> [T; 9]) -> [T; 9] {
    let mut prod = apply(&M);
    for (factor, m) in e.into_iter().zip([&M0, &M1, &M2]) {
        let term = apply(m);
        for (p, t) in prod.iter_mut().zip(term) {
            p.add_scaled(factor, t);
        }
    }
    prod
}

/// Element-wise `self += factor * other`.
trait Combine {
    fn add_scaled(&mut self, factor: f64, other: Self);
}

impl Combine for f64 {
    fn add_scaled(&mut self, factor: f64, other: Self) {
        *self += factor * other;
    }
}

impl<const N: usize> Combine for [f64; N] {
    fn add_scaled(&mut self, factor: f64, other: Self) {
        for (a, b) in self.iter_mut().zip(other) {
            *a += factor * b;
        }
    }
}

/// Rows rolled down by `3 * sub` (numpy `roll` along axis 0).
fn roll<T: Copy>(rows: &[T; 9], sub: usize) -> [T; 9] {
    std::array::from_fn(|i| rows[(i + 9 - 3 * sub) % 9])
}

fn mat_vec(m: &[[f64; 10]; 9], v: &[f64; 10]) -> [f64; 9] {
    m.map(|row| row.iter().zip(v).fold(0.0, |acc, (a, b)| acc + a * b))
}

fn mat_cols<const N: usize>(m: &[[f64; 10]; 9], v: &[[f64; N]; 10]) -> [[f64; N]; 9] {
    m.map(|row| std::array::from_fn(|c| (0..10).fold(0.0, |acc, k| acc + row[k] * v[k][c])))
}

fn dot9(a: &[f64; 9], b: &[f64; 9]) -> f64 {
    a.iter().zip(b).fold(0.0, |acc, (x, y)| acc + x * y)
}

/// `a @ b` for 2×2 matrices.
pub(super) fn mat2_mul(a: Mat2, b: Mat2) -> Mat2 {
    std::array::from_fn(|r| std::array::from_fn(|c| a[r][0] * b[0][c] + a[r][1] * b[1][c]))
}

fn mat9_mul(a: &[[f64; 9]; 9], b: &[[f64; 9]; 9]) -> [[f64; 9]; 9] {
    std::array::from_fn(|r| {
        std::array::from_fn(|c| (0..9).fold(0.0, |acc, k| acc + a[r][k] * b[k][c]))
    })
}

fn transpose9(a: &[[f64; 9]; 9]) -> [[f64; 9]; 9] {
    std::array::from_fn(|r| std::array::from_fn(|c| a[c][r]))
}

/// The inverse of a 2×2 matrix, or zeros if it is (nearly) singular
/// (matplotlib's `_safe_inv22_vectorized`).
pub(super) fn safe_inv22(m: Mat2) -> Mat2 {
    let prod1 = m[0][0] * m[1][1];
    let delta = prod1 - m[0][1] * m[1][0];
    let inv = if delta.abs() > 1e-8 * prod1.abs() {
        1.0 / delta
    } else {
        0.0
    };
    [
        [m[1][1] * inv, -m[0][1] * inv],
        [-m[1][0] * inv, m[0][0] * inv],
    ]
}

/// The inverse of a symmetric 2×2 matrix, or its Moore–Penrose
/// pseudo-inverse if it is rank deficient (`_pseudo_inv22sym_vectorized`).
pub(super) fn pseudo_inv22sym(m: Mat2) -> Mat2 {
    let prod1 = m[0][0] * m[1][1];
    let delta = prod1 - m[0][1] * m[1][0];
    if delta.abs() > 1e-8 * prod1.abs() {
        [
            [m[1][1] / delta, -m[0][1] / delta],
            [-m[1][0] / delta, m[0][0] / delta],
        ]
    } else {
        let trace = m[0][0] + m[1][1];
        let zero = if trace.abs() < 1e-8 { 1.0 } else { 0.0 };
        let factor = (1.0 - zero) / (trace * trace + zero);
        m.map(|row| row.map(|v| v * factor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_functions_interpolate_apex_values() {
        let ecc = [0.0; 3];
        for apex in 0..3 {
            let mut alpha = [0.0; 3];
            alpha[apex] = 1.0;
            let mut dof = [0.0; 9];
            dof[3 * apex] = 1.0;
            assert!((value(alpha, ecc, &dof) - 1.0).abs() < 1e-14);
        }
        // A constant field: all value dofs 1, all derivative dofs 0.
        let dof = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        assert!((value([0.2, 0.3, 0.5], [0.1, -0.2, 0.3], &dof) - 1.0).abs() < 1e-14);
    }

    #[test]
    fn bending_matrix_annihilates_linear_fields() {
        let jacobian = [[1.0, 0.2], [0.3, 1.1]];
        let k = bending_matrix(jacobian, [0.1, -0.05, 0.2]);
        // z = x in global dofs (value, dz/dx, dz/dy per apex); apexes at
        // x = 0, J[0][0] and J[1][0].
        let dof = [0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.3, 1.0, 0.0];
        for row in k {
            assert!(dot9(&row, &dof).abs() < 1e-12);
        }
    }

    #[test]
    fn inverses() {
        let m = [[2.0, 1.0], [1.0, 3.0]];
        let inv = safe_inv22(m);
        let id = mat2_mul(m, inv);
        assert!((id[0][0] - 1.0).abs() < 1e-15 && id[0][1].abs() < 1e-15);
        assert_eq!(
            safe_inv22([[1.0, 2.0], [2.0, 4.0]]),
            [[0.0, -0.0], [-0.0, 0.0]]
        );
        let rank1 = pseudo_inv22sym([[1.0, 1.0], [1.0, 1.0]]);
        assert_eq!(rank1, [[0.25, 0.25], [0.25, 0.25]]);
    }
}
