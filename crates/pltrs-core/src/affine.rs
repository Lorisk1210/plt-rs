//! Affine transformations of the plane.

use crate::Point;

/// An affine transformation `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
///
/// The coefficient naming follows PostScript/PDF/SVG (and matplotlib's
/// `Affine2D.to_values()`), i.e. the matrix is
///
/// ```text
/// | a c e |
/// | b d f |
/// | 0 0 1 |
/// ```
///
/// Builder methods such as [`scale`](Self::scale) and
/// [`rotate`](Self::rotate) *append* an operation, exactly like matplotlib:
/// the result applies `self` first and the new operation afterwards.
///
/// ```
/// use pltrs_core::{Affine2D, Point};
/// let t = Affine2D::identity().scale(2.0, 2.0).translate(1.0, 0.0);
/// assert_eq!(t.transform_point(Point::new(1.0, 1.0)), Point::new(3.0, 2.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine2D {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
}

crate::archive_struct!(Affine2D { a, b, c, d, e, f });

impl Default for Affine2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Affine2D {
    /// The identity transformation.
    pub const IDENTITY: Self = Self::from_values(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    /// Returns the identity transformation.
    pub const fn identity() -> Self {
        Self::IDENTITY
    }

    /// Creates a transformation from its six coefficients (see the type docs).
    pub const fn from_values(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// Creates a transformation from the top two rows of a 3×3 matrix.
    pub const fn from_rows(rows: [[f64; 3]; 2]) -> Self {
        let [[a, c, e], [b, d, f]] = rows;
        Self::from_values(a, b, c, d, e, f)
    }

    /// A pure translation.
    pub const fn translation(tx: f64, ty: f64) -> Self {
        Self::from_values(1.0, 0.0, 0.0, 1.0, tx, ty)
    }

    /// A pure scaling about the origin (matplotlib's
    /// `Affine2D().scale(sx, sy)`: the identity scaled element by element,
    /// so a negative factor leaves negative zeros in its row).
    pub const fn scaling(sx: f64, sy: f64) -> Self {
        Self::from_values(sx, 0.0 * sy, 0.0 * sx, sy, 0.0 * sx, 0.0 * sy)
    }

    /// A counter-clockwise rotation about the origin by `theta` radians.
    pub fn rotation(theta: f64) -> Self {
        Self::IDENTITY.rotate(theta)
    }

    /// The coefficients `[a, b, c, d, e, f]`.
    pub const fn values(&self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// The full homogeneous 3×3 matrix, row-major.
    pub const fn matrix(&self) -> [[f64; 3]; 3] {
        [
            [self.a, self.c, self.e],
            [self.b, self.d, self.f],
            [0.0, 0.0, 1.0],
        ]
    }

    /// Appends a translation.
    #[must_use]
    pub fn translate(self, tx: f64, ty: f64) -> Self {
        Self {
            e: self.e + tx,
            f: self.f + ty,
            ..self
        }
    }

    /// Appends a scaling about the origin.
    #[must_use]
    pub fn scale(self, sx: f64, sy: f64) -> Self {
        Self::from_values(
            self.a * sx,
            self.b * sy,
            self.c * sx,
            self.d * sy,
            self.e * sx,
            self.f * sy,
        )
    }

    /// Appends a counter-clockwise rotation by `theta` radians about the origin.
    #[must_use]
    pub fn rotate(self, theta: f64) -> Self {
        let (sin, cos) = theta.sin_cos();
        Self::from_values(
            cos * self.a - sin * self.b,
            sin * self.a + cos * self.b,
            cos * self.c - sin * self.d,
            sin * self.c + cos * self.d,
            cos * self.e - sin * self.f,
            sin * self.e + cos * self.f,
        )
    }

    /// Appends a counter-clockwise rotation by `degrees` about the origin.
    #[must_use]
    pub fn rotate_deg(self, degrees: f64) -> Self {
        self.rotate(degrees.to_radians())
    }

    /// Appends a rotation by `theta` radians about the point `(x, y)`.
    #[must_use]
    pub fn rotate_around(self, x: f64, y: f64, theta: f64) -> Self {
        self.translate(-x, -y).rotate(theta).translate(x, y)
    }

    /// Appends a rotation by `degrees` about the point `(x, y)`.
    #[must_use]
    pub fn rotate_deg_around(self, x: f64, y: f64, degrees: f64) -> Self {
        self.rotate_around(x, y, degrees.to_radians())
    }

    /// Appends a skew; the shear angles are in radians.
    #[must_use]
    pub fn skew(self, x_shear: f64, y_shear: f64) -> Self {
        let rx = x_shear.tan();
        let ry = y_shear.tan();
        Self::from_values(
            self.a + rx * self.b,
            self.b + ry * self.a,
            self.c + rx * self.d,
            self.d + ry * self.c,
            self.e + rx * self.f,
            self.f + ry * self.e,
        )
    }

    /// Appends a skew; the shear angles are in degrees.
    #[must_use]
    pub fn skew_deg(self, x_shear: f64, y_shear: f64) -> Self {
        self.skew(x_shear.to_radians(), y_shear.to_radians())
    }

    /// Composition: the result applies `self` first, then `next`
    /// (matplotlib's `self + next`).
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        Self::from_values(
            next.a * self.a + next.c * self.b,
            next.b * self.a + next.d * self.b,
            next.a * self.c + next.c * self.d,
            next.b * self.c + next.d * self.d,
            next.a * self.e + next.c * self.f + next.e,
            next.b * self.e + next.d * self.f + next.f,
        )
    }

    /// Determinant of the linear part.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// The inverse transformation, or `None` when it is singular.
    ///
    /// It is computed as numpy's `linalg.inv` computes it, which
    /// matplotlib's `Affine2D.inverted` calls, so that points mapped back
    /// land on matplotlib's bits: LAPACK's LU decomposition of the 3×3
    /// matrix with partial pivoting, then the triangular solves, with the
    /// reciprocals of the pivots multiplied in and the back substitution
    /// fused (`OpenBLAS`'s kernels). Non-finite matrices invert to
    /// non-finite results, as with numpy.
    ///
    /// ```
    /// use pltrs_core::{Affine2D, Point};
    /// let to_display = Affine2D::from_values(496.0, 0.0, 0.0, 369.6, 80.0, 52.8);
    /// let back = to_display.inverted().expect("invertible");
    /// assert_eq!(back.transform_point(Point::new(80.0, 52.8)), Point::new(0.0, 0.0));
    /// assert!(Affine2D::scaling(0.0, 1.0).inverted().is_none());
    /// ```
    pub fn inverted(&self) -> Option<Self> {
        // The rows of [[a, c, e], [b, d, f], [0, 0, 1]], the one with the
        // larger first entry on top; the third row of the factors stays
        // that of the identity.
        let swapped = self.b.abs() > self.a.abs();
        let (top, bottom) = if swapped {
            ([self.b, self.d, self.f], [self.a, self.c, self.e])
        } else {
            ([self.a, self.c, self.e], [self.b, self.d, self.f])
        };
        if top[0] == 0.0 {
            return None;
        }
        let multiplier = bottom[0] * (1.0 / top[0]);
        let u11 = bottom[1] - multiplier * top[1];
        let u12 = bottom[2] - multiplier * top[2];
        if u11 == 0.0 {
            return None;
        }
        // Column `j` of the inverse solves `L U x = P e_j`; only its first
        // two entries are kept (the third is that of the identity).
        let solve = |p0: f64, p1: f64, p2: f64| {
            let b1 = p1 - multiplier * p0;
            let b0 = (-p2).mul_add(top[2], p0);
            let b1 = (-p2).mul_add(u12, b1) * (1.0 / u11);
            let b0 = (-b1).mul_add(top[1], b0) * (1.0 / top[0]);
            (b0, b1)
        };
        let (first, second) = if swapped { (0.0, 1.0) } else { (1.0, 0.0) };
        let (a, b) = solve(first, second, 0.0);
        let (c, d) = solve(second, first, 0.0);
        let (e, f) = solve(0.0, 0.0, 1.0);
        Some(Self::from_values(a, b, c, d, e, f))
    }

    /// Returns `true` for the exact identity.
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// Returns `true` when the transform has no rotation or shear, so the
    /// x and y axes map independently.
    pub fn is_separable(&self) -> bool {
        self.b == 0.0 && self.c == 0.0
    }

    /// Translation part `(e, f)`.
    pub const fn translation_part(&self) -> Point {
        Point::new(self.e, self.f)
    }

    /// Applies the transformation to a point.
    pub fn transform_point(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Applies only the linear part (no translation), as for a displacement.
    pub fn transform_vector(&self, v: Point) -> Point {
        Point::new(self.a * v.x + self.c * v.y, self.b * v.x + self.d * v.y)
    }

    /// Transforms every point in place.
    pub fn transform_in_place(&self, points: &mut [Point]) {
        for p in points {
            *p = self.transform_point(*p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn approx(a: Point, b: Point) -> bool {
        (a - b).length() < 1e-12
    }

    #[test]
    fn append_order_matches_matplotlib() {
        // matplotlib: Affine2D().scale(2).rotate_deg(90) maps (1, 0) -> (0, 2).
        let t = Affine2D::identity().scale(2.0, 2.0).rotate_deg(90.0);
        assert!(approx(
            t.transform_point(Point::new(1.0, 0.0)),
            Point::new(0.0, 2.0)
        ));
        let t = Affine2D::identity().rotate(FRAC_PI_2).translate(1.0, 0.0);
        assert!(approx(
            t.transform_point(Point::new(1.0, 0.0)),
            Point::new(1.0, 1.0)
        ));
    }

    #[test]
    fn then_composes_left_to_right() {
        let first = Affine2D::scaling(2.0, 3.0);
        let second = Affine2D::translation(1.0, 1.0);
        let p = Point::new(1.0, 1.0);
        assert_eq!(first.then(second).transform_point(p), Point::new(3.0, 4.0));
        assert_eq!(second.then(first).transform_point(p), Point::new(4.0, 6.0));
    }

    #[test]
    fn builder_equals_composition() {
        let built = Affine2D::identity()
            .scale(2.0, 0.5)
            .rotate(0.3)
            .translate(4.0, -1.0);
        let composed = Affine2D::scaling(2.0, 0.5)
            .then(Affine2D::rotation(0.3))
            .then(Affine2D::translation(4.0, -1.0));
        for (x, y) in built.values().into_iter().zip(composed.values()) {
            assert!((x - y).abs() < 1e-12);
        }
    }

    /// numpy's inverses of a display transform, of a matrix it pivots
    /// and of a rotation, bit for bit.
    #[test]
    fn inverses_are_numpys() {
        let bits = |values: [u64; 6]| {
            let [a, b, c, d, e, f] = values.map(f64::from_bits);
            Affine2D::from_values(a, b, c, d, e, f)
        };
        let cases = [
            (
                [
                    0x407f_0000_0000_0000,
                    0,
                    0,
                    0x4077_1999_9999_999a,
                    0x4054_0000_0000_0000,
                    0x404a_6666_6666_6666,
                ],
                [
                    0x3f60_8421_0842_1084,
                    0,
                    0,
                    0x3f66_2a1c_d058_a873,
                    0xbfc4_a529_4a52_94a5,
                    0xbfc2_4924_9249_2492,
                ],
            ),
            (
                [
                    0x3fd3_3333_3333_3333,
                    0x4000_0000_0000_0000,
                    0xbff8_0000_0000_0000,
                    0x3fe6_6666_6666_6666,
                    0x4024_0000_0000_0000,
                    0xc010_0000_0000_0000,
                ],
                [
                    0x3fcb_e9ab_76d4_2316,
                    0xbfe3_f00c_c297_86c7,
                    0x3fdd_e813_23e3_4a2b,
                    0x3fb7_ecdc_1cb5_d4ef,
                    0xbfd3_f00c_c297_86be,
                    0x401a_6add_b508_c5c7,
                ],
            ),
            (
                [
                    0x4006_1aff_ade4_ba3a,
                    0x3ff2_b12c_2ee0_8d90,
                    0xbfe8_ec3a_e92b_676b,
                    0x3ffd_7954_e7db_a2f8,
                    0x405e_d999_9999_999a,
                    0xc04c_5999_9999_999a,
                ],
                [
                    0x3fd3_a638_9a92_6ca5,
                    0xbfc8_ec3a_e92b_676a,
                    0x3fc0_9d7c_9b72_44f1,
                    0x3fdd_7954_e7db_a2f8,
                    0xc03e_86bb_bdf9_e823,
                    0x4049_11d1_01e6_9d3d,
                ],
            ),
        ];
        for (matrix, inverse) in cases {
            assert_eq!(bits(matrix).inverted(), Some(bits(inverse)));
        }
        let singular = Affine2D::from_values(1.0, 2.0, 2.0, 4.0, 0.0, 0.0);
        assert!(singular.inverted().is_none());
        let nan = Affine2D::from_values(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0).inverted();
        assert!(nan.is_some_and(|inv| inv.values()[0].is_nan()));
    }

    #[test]
    fn inverse_round_trips() {
        let t = Affine2D::identity()
            .skew_deg(10.0, 20.0)
            .rotate_deg(33.0)
            .translate(5.0, 7.0);
        let inv = t.inverted().expect("invertible");
        let p = Point::new(-3.0, 8.5);
        assert!(approx(inv.transform_point(t.transform_point(p)), p));
        assert!(Affine2D::scaling(0.0, 1.0).inverted().is_none());
    }

    #[test]
    fn rotate_around_keeps_centre_fixed() {
        let t = Affine2D::identity().rotate_deg_around(2.0, 3.0, 45.0);
        assert!(approx(
            t.transform_point(Point::new(2.0, 3.0)),
            Point::new(2.0, 3.0)
        ));
    }

    #[test]
    fn vectors_ignore_translation() {
        let t = Affine2D::translation(10.0, 10.0).scale(2.0, 2.0);
        assert_eq!(
            t.transform_vector(Point::new(1.0, 1.0)),
            Point::new(2.0, 2.0)
        );
    }
}
