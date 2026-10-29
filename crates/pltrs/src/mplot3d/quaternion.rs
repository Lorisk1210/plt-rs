//! Quaternions for rotating the view of a 3-D axes with the mouse
//! (matplotlib's `_Quaternion`), with the same operation order so that
//! dragged views match matplotlib's to the last bit.

use super::math::{Vec3, cross, dot, norm};

/// A quaternion: a scalar part and a vector part along i, j and k.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Quaternion {
    pub(crate) scalar: f64,
    pub(crate) vector: Vec3,
}

impl Quaternion {
    /// The quaternion `scalar + vector`.
    pub(crate) const fn new(scalar: f64, vector: Vec3) -> Self {
        Self { scalar, vector }
    }

    /// The product `self · other`.
    #[must_use]
    pub(crate) fn mul(self, other: Self) -> Self {
        let c = cross(self.vector, other.vector);
        let vector =
            [0, 1, 2].map(|i| self.scalar * other.vector[i] + self.vector[i] * other.scalar + c[i]);
        Self::new(
            self.scalar * other.scalar - dot(self.vector, other.vector),
            vector,
        )
    }

    /// The shortest rotation taking direction `from` to direction `to`
    /// (`sqrt(to · from')`, normalised); antiparallel directions turn about
    /// the basis vector most perpendicular to `from`.
    pub(crate) fn rotate_from_to(from: Vec3, to: Vec3) -> Self {
        let k = cross(from, to);
        let nk = norm(k);
        let th = nk.atan2(dot(from, to)) / 2.0;
        if nk != 0.0 {
            let (sin, cos) = th.sin_cos();
            return Self::new(cos, k.map(|c| c * sin / nk));
        }
        if dot(from, to) >= 0.0 {
            return Self::new(1.0, [0.0; 3]);
        }
        let squares = from.map(|c| c * c);
        let mut basis = [0.0; 3];
        let smallest = (0..3)
            .min_by(|&a, &b| squares[a].total_cmp(&squares[b]))
            .unwrap_or(0);
        basis[smallest] = 1.0;
        let k = cross(from, basis);
        let nk = norm(k);
        Self::new(0.0, k.map(|c| c / nk))
    }

    /// The rotation of the view angles (radians), Tait-Bryan angles about
    /// −z, y′ and x″: `exp(roll/2 e_x) exp(elev/2 e_y) exp(−azim/2 e_z)`.
    pub(crate) fn from_cardan_angles(elev: f64, azim: f64, roll: f64) -> Self {
        let (sa, ca) = (azim / 2.0).sin_cos();
        let (se, ce) = (elev / 2.0).sin_cos();
        let (sr, cr) = (roll / 2.0).sin_cos();
        Self::new(
            ca * ce * cr + sa * se * sr,
            [
                ca * ce * sr - sa * se * cr,
                ca * se * cr + sa * ce * sr,
                ca * se * sr - sa * ce * cr,
            ],
        )
    }

    /// The view angles `(elev, azim, roll)` in radians of this rotation
    /// (the inverse of [`Quaternion::from_cardan_angles`], whatever the
    /// norm).
    pub(crate) fn as_cardan_angles(self) -> (f64, f64, f64) {
        let qw = self.scalar;
        let [qx, qy, qz] = self.vector;
        let azim = (2.0 * (-qw * qz + qx * qy)).atan2(qw * qw + qx * qx - qy * qy - qz * qz);
        let norm = qw * qw + qx * qx + qy * qy + qz * qz;
        let elev = (2.0 * (qw * qy + qz * qx) / norm).clamp(-1.0, 1.0).asin();
        let roll = (2.0 * (qw * qx - qy * qz)).atan2(qw * qw - qx * qx - qy * qy + qz * qz);
        (elev, azim, roll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn products_follow_hamilton() {
        let i = Quaternion::new(0.0, [1.0, 0.0, 0.0]);
        let j = Quaternion::new(0.0, [0.0, 1.0, 0.0]);
        assert_eq!(i.mul(j), Quaternion::new(0.0, [0.0, 0.0, 1.0]), "ij = k");
        assert_eq!(i.mul(i), Quaternion::new(-1.0, [0.0; 3]), "i² = −1");
    }

    #[test]
    fn cardan_angles_round_trip() {
        for (elev, azim, roll) in [(0.5, -1.0, 0.2), (-1.2, 2.5, -3.0), (0.0, 0.0, 0.0)] {
            let (e, a, r) = Quaternion::from_cardan_angles(elev, azim, roll).as_cardan_angles();
            assert!(
                close(e, elev) && close(a, azim) && close(r, roll),
                "{e} {a} {r}"
            );
        }
    }

    #[test]
    fn rotations_between_directions() {
        let q = Quaternion::rotate_from_to([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        let half = std::f64::consts::FRAC_PI_4;
        assert!(close(q.scalar, half.cos()) && close(q.vector[2], half.sin()));
        let same = Quaternion::rotate_from_to([0.0, 0.0, 2.0], [0.0, 0.0, 1.0]);
        assert_eq!(same, Quaternion::new(1.0, [0.0; 3]));
        let opposite = Quaternion::rotate_from_to([1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]);
        assert_eq!(opposite.scalar, 0.0);
        assert!(close(norm(opposite.vector), 1.0));
        assert!(close(dot(opposite.vector, [1.0, 0.0, 0.0]), 0.0));
    }
}
