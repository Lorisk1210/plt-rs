//! Homogeneous 3-D projection math (matplotlib's `proj3d`).
//!
//! Points are row-free `[x, y, z]` arrays and matrices act on homogeneous
//! column vectors, exactly as in matplotlib, so that the projection of a
//! point is `M · [x, y, z, 1]` followed by the perspective divide.

/// A point or direction in 3-D space.
pub type Vec3 = [f64; 3];

/// `a - b`.
pub(crate) fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `a + b`.
pub(crate) fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a * s`.
pub(crate) fn scaled(a: Vec3, s: f64) -> Vec3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// The dot product.
pub(crate) fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The cross product (numpy's `cross`).
pub(crate) fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The Euclidean length (numpy's `linalg.norm`).
pub(crate) fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// `a` divided by its length.
fn normalized(a: Vec3) -> Vec3 {
    let n = norm(a);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// A 4×4 matrix acting on homogeneous column vectors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix4([[f64; 4]; 4]);

impl Matrix4 {
    /// The identity.
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// A matrix from its rows.
    pub const fn from_rows(rows: [[f64; 4]; 4]) -> Self {
        Self(rows)
    }

    /// The rows.
    pub const fn rows(&self) -> &[[f64; 4]; 4] {
        &self.0
    }

    /// The matrix product `self · other` (numpy's `dot`).
    #[must_use]
    pub fn dot(&self, other: &Self) -> Self {
        let mut out = [[0.0; 4]; 4];
        for (row, out_row) in self.0.iter().zip(&mut out) {
            for (j, cell) in out_row.iter_mut().enumerate() {
                *cell = (0..4).map(|k| row[k] * other.0[k][j]).sum();
            }
        }
        Self(out)
    }

    /// `self · v` for a homogeneous vector.
    pub fn transform(&self, v: [f64; 4]) -> [f64; 4] {
        self.0
            .map(|row| row[0] * v[0] + row[1] * v[1] + row[2] * v[2] + row[3] * v[3])
    }

    /// The inverse matrix, by Gauss-Jordan elimination with partial
    /// pivoting, or `None` when the matrix is singular.
    pub fn inverse(&self) -> Option<Self> {
        let mut a = self.0;
        let mut inv = Self::IDENTITY.0;
        for col in 0..4 {
            let pivot = (col..4).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
            if a[pivot][col] == 0.0 || !a[pivot][col].is_finite() {
                return None;
            }
            a.swap(col, pivot);
            inv.swap(col, pivot);
            let p = a[col][col];
            for k in 0..4 {
                a[col][k] /= p;
                inv[col][k] /= p;
            }
            for row in (0..4).filter(|&row| row != col) {
                let factor = a[row][col];
                for k in 0..4 {
                    a[row][k] -= factor * a[col][k];
                    inv[row][k] -= factor * inv[col][k];
                }
            }
        }
        Some(Self(inv))
    }

    /// Projects a point: `self · [x, y, z, 1]` divided by its last
    /// component (matplotlib's `proj_transform`).
    pub fn project(&self, p: Vec3) -> Vec3 {
        let [x, y, z, w] = self.transform([p[0], p[1], p[2], 1.0]);
        [x / w, y / w, z / w]
    }
}

/// Maps the box `limits = [xmin, xmax, ymin, ymax, zmin, zmax]` onto
/// `[0, aspect[i]]` along each axis (matplotlib's `world_transformation`).
pub fn world_transformation(limits: [f64; 6], aspect: Vec3) -> Matrix4 {
    let [xmin, xmax, ymin, ymax, zmin, zmax] = limits;
    let dx = (xmax - xmin) / aspect[0];
    let dy = (ymax - ymin) / aspect[1];
    let dz = (zmax - zmin) / aspect[2];
    Matrix4([
        [1.0 / dx, 0.0, 0.0, -xmin / dx],
        [0.0, 1.0 / dy, 0.0, -ymin / dy],
        [0.0, 0.0, 1.0 / dz, -zmin / dz],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// The rotation by `angle` radians about the direction `v`
/// (matplotlib's `_rotation_about_vector`).
fn rotation_about_vector(v: Vec3, angle: f64) -> [Vec3; 3] {
    let [vx, vy, vz] = normalized(v);
    let (s, c) = angle.sin_cos();
    // More stable than `1 - c` for small angles.
    let t = 2.0 * (angle / 2.0).sin().powi(2);
    [
        [t * vx * vx + c, t * vx * vy - vz * s, t * vx * vz + vy * s],
        [t * vy * vx + vz * s, t * vy * vy + c, t * vy * vz - vx * s],
        [t * vz * vx - vy * s, t * vz * vy + vx * s, t * vz * vz + c],
    ]
}

/// The unit vectors pointing right (`u`), up (`v`) and out of the screen
/// (`w`) for an eye at `eye` looking at `center` with `vertical` up,
/// rolled by `roll` radians (matplotlib's `_view_axes`).
pub fn view_axes(eye: Vec3, center: Vec3, vertical: Vec3, roll: f64) -> [Vec3; 3] {
    let w = normalized(sub(eye, center));
    let u = normalized(cross(vertical, w));
    let v = cross(w, u);
    if roll == 0.0 {
        return [u, v, w];
    }
    // A positive rotation of the camera is a negative rotation of the world.
    let r = rotation_about_vector(w, -roll);
    let rotate = |a: Vec3| r.map(|row| dot(row, a));
    [rotate(u), rotate(v), w]
}

/// The world-to-view transformation for the view axes `u, v, w` and the
/// eye position (matplotlib's `_view_transformation_uvw`).
pub fn view_transformation(u: Vec3, v: Vec3, w: Vec3, eye: Vec3) -> Matrix4 {
    let mut rotation = Matrix4::IDENTITY;
    for (row, axis) in rotation.0.iter_mut().zip([u, v, w]) {
        row[..3].copy_from_slice(&axis);
    }
    let mut translation = Matrix4::IDENTITY;
    for (row, e) in translation.0.iter_mut().zip(eye) {
        row[3] = -e;
    }
    rotation.dot(&translation)
}

/// The perspective projection between the planes `zfront` and `zback`
/// (matplotlib's `_persp_transformation`).
pub fn persp_transformation(zfront: f64, zback: f64, focal_length: f64) -> Matrix4 {
    let e = focal_length;
    let b = (zfront + zback) / (zfront - zback);
    let c = -2.0 * (zfront * zback) / (zfront - zback);
    Matrix4([
        [e, 0.0, 0.0, 0.0],
        [0.0, e, 0.0, 0.0],
        [0.0, 0.0, b, c],
        [0.0, 0.0, -1.0, 0.0],
    ])
}

/// The orthographic projection between the planes `zfront` and `zback`
/// (matplotlib's `_ortho_transformation`; the projected `w` is
/// `zback - zfront`, not 1).
pub fn ortho_transformation(zfront: f64, zback: f64) -> Matrix4 {
    let a = -(zfront + zback);
    let b = -(zfront - zback);
    Matrix4([
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 2.0, 0.0, 0.0],
        [0.0, 0.0, -2.0, 0.0],
        [0.0, 0.0, a, b],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverses_undo_the_matrix() {
        let m = Matrix4::from_rows([
            [2.0, 0.0, 1.0, 3.0],
            [0.0, 1.0, 0.0, -1.0],
            [1.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ]);
        let product = m.dot(&m.inverse().unwrap());
        for (i, row) in product.rows().iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((cell - expected).abs() < 1e-12, "({i}, {j}): {cell}");
            }
        }
        assert_eq!(Matrix4::from_rows([[0.0; 4]; 4]).inverse(), None);
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn world_transformation_maps_limits_to_the_aspect_box() {
        let m = world_transformation([-1.0, 1.0, 0.0, 4.0, 2.0, 3.0], [2.0, 1.0, 0.5]);
        assert_eq!(m.project([-1.0, 0.0, 2.0]), [0.0, 0.0, 0.0]);
        let far = m.project([1.0, 4.0, 3.0]);
        assert!(close(far[0], 2.0) && close(far[1], 1.0) && close(far[2], 0.5));
    }

    #[test]
    fn view_axes_are_orthonormal_and_roll_rotates_u_and_v() {
        let eye = [10.0, -5.0, 6.0];
        let [u, v, w] = view_axes(eye, [0.5, 0.5, 0.4], [0.0, 0.0, 1.0], 0.0);
        for (a, b) in [(u, v), (v, w), (u, w)] {
            assert!(dot(a, b).abs() < 1e-12);
        }
        assert!(close(norm(u), 1.0) && close(norm(v), 1.0) && close(norm(w), 1.0));
        let [ur, vr, wr] = view_axes(eye, [0.5, 0.5, 0.4], [0.0, 0.0, 1.0], 0.3);
        assert_eq!(w, wr);
        assert!(close(dot(u, ur), 0.3_f64.cos()) && close(dot(v, vr), 0.3_f64.cos()));
    }

    #[test]
    fn view_transformation_moves_the_eye_to_the_origin() {
        let eye = [3.0, 4.0, 5.0];
        let [u, v, w] = view_axes(eye, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0], 0.0);
        let m = view_transformation(u, v, w, eye);
        let at_eye = m.project(eye);
        assert!(at_eye.iter().all(|c| c.abs() < 1e-12));
        let center = m.project([0.0, 0.0, 0.0]);
        assert!(close(center[2], -norm(eye)), "the target lies along -w");
    }

    #[test]
    fn projections_match_matplotlib() {
        let persp = persp_transformation(-10.0, 10.0, 1.0);
        assert_eq!(persp.rows()[2][3], -10.0);
        assert_eq!(persp.rows()[3][2], -1.0);
        let ortho = ortho_transformation(-10.0, 10.0);
        assert_eq!(ortho.rows()[3], [0.0, 0.0, 0.0, 20.0]);
        let p = ortho.project([1.0, 2.0, 3.0]);
        assert!(close(p[0], 0.1) && close(p[1], 0.2) && close(p[2], -0.3));
    }

    #[test]
    fn matrix_product_and_identity() {
        let m = world_transformation([0.0, 2.0, 0.0, 2.0, 0.0, 2.0], [1.0, 1.0, 1.0]);
        assert_eq!(m.dot(&Matrix4::IDENTITY), m);
        assert_eq!(Matrix4::IDENTITY.dot(&m), m);
        assert_eq!(cross([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), [0.0, 0.0, 1.0]);
        assert_eq!(
            add([1.0, 2.0, 3.0], scaled([1.0, 1.0, 1.0], 2.0)),
            [3.0, 4.0, 5.0]
        );
    }
}
