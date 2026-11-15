//! Affine transformations of the plane.

use crate::Point;
use crate::numeric::{
    ACCELERATE_BLAS, CONTRACTION, Contraction, difference_of_products, sum_of_products,
};

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
    ///
    /// matplotlib multiplies the two 3×3 matrices with numpy's `dot`,
    /// whose BLAS sums each entry's products in order: rounded one by one
    /// here, and with fused multiply-adds where it is Apple's Accelerate
    /// (`numeric::ACCELERATE_BLAS`).
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        let dot = |x: f64, p: f64, y: f64, q: f64| {
            if ACCELERATE_BLAS {
                y.mul_add(q, x * p)
            } else {
                x * p + y * q
            }
        };
        Self::from_values(
            dot(next.a, self.a, next.c, self.b),
            dot(next.b, self.a, next.d, self.b),
            dot(next.a, self.c, next.c, self.d),
            dot(next.b, self.c, next.d, self.d),
            dot(next.a, self.e, next.c, self.f) + next.e,
            dot(next.b, self.e, next.d, self.f) + next.f,
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

    /// Composition as Agg's `trans_affine::multiply` (its `*=`) computes
    /// it: [`then`](Self::then) with the products matplotlib's build of
    /// Agg fuses (`numeric::fused`). The renderers' transforms are composed
    /// this way in matplotlib's C++; its Python transforms are numpy's
    /// products, which [`then`](Self::then) is.
    #[must_use]
    pub fn agg_then(self, next: Self) -> Self {
        Self::from_values(
            sum_of_products(self.a, next.a, self.b, next.c),
            sum_of_products(self.a, next.b, self.b, next.d),
            sum_of_products(self.c, next.a, self.d, next.c),
            sum_of_products(self.c, next.b, self.d, next.d),
            sum_of_products(self.e, next.a, self.f, next.c) + next.e,
            sum_of_products(self.e, next.b, self.f, next.d) + next.f,
        )
    }

    /// One over the determinant of the linear part (Agg's
    /// `trans_affine::determinant_reciprocal`): infinite or NaN for a
    /// singular transform.
    pub fn agg_determinant_reciprocal(&self) -> f64 {
        1.0 / difference_of_products(self.a, self.d, self.b, self.c)
    }

    /// The inverse as Agg's `trans_affine::invert` computes it: by the
    /// reciprocal of the determinant, whatever its value (a singular
    /// transform inverts to infinities and NaN, as in Agg).
    #[must_use]
    pub fn agg_inverted(&self) -> Self {
        let d = self.agg_determinant_reciprocal();
        let a = self.d * d;
        let d_new = self.a * d;
        let b = -self.b * d;
        let c = -self.c * d;
        Self::from_values(
            a,
            b,
            c,
            d_new,
            difference_of_products(-self.e, a, self.f, c),
            difference_of_products(-self.e, b, self.f, d_new),
        )
    }

    /// Applies the transformation as Agg's `trans_affine::transform` does
    /// (`conv_transform`, the span interpolators): where matplotlib's build
    /// of Agg fuses, `x * a` is fused into `x * a + y * c`, which shows
    /// once the transform rotates or shears.
    pub fn agg_transform_point(&self, p: Point) -> Point {
        Point::new(
            sum_of_products(p.x, self.a, p.y, self.c) + self.e,
            sum_of_products(p.x, self.b, p.y, self.d) + self.f,
        )
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

    /// Applies the transformation to a point as matplotlib's
    /// `_path.affine_transform` does, which every affine transform of
    /// matplotlib's Python maps arrays with (`Affine2D.transform`,
    /// `transform_path`).
    #[inline]
    pub fn transform_point(&self, p: Point) -> Point {
        self.transform_point_as(CONTRACTION, p)
    }

    /// [`transform_point`](Self::transform_point) as the build of
    /// `_path.affine_transform` by `contraction`'s compiler computes it.
    /// The function keeps each product in a variable (`t0 = sx * x;
    /// t1 = shx * y; t = t0 + t1 + tx`): clang rounds both, GCC fuses the
    /// first into their sum, across the statements (its `widening_mul`
    /// GIMPLE). That is Agg's `trans_affine::transform` as clang builds it,
    /// so under GCC this equals [`agg_transform_point`](Self::agg_transform_point)
    /// bit for bit, and the callers mirroring either agree.
    #[inline]
    pub fn transform_point_as(&self, contraction: Contraction, p: Point) -> Point {
        match contraction {
            Contraction::Gcc => Point::new(
                contraction.fused(self.a, p.x, self.c * p.y) + self.e,
                contraction.fused(self.b, p.x, self.d * p.y) + self.f,
            ),
            Contraction::Unfused | Contraction::Clang => Point::new(
                self.a * p.x + self.c * p.y + self.e,
                self.b * p.x + self.d * p.y + self.f,
            ),
        }
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
    fn points_map_as_each_compilers_affine_transform() {
        // 0.1 * 10 is 1 + 2^-54 before rounding: GCC fuses it into the sum
        // with the second product, clang and the x86-64 build round it.
        let t = Affine2D::from_values(0.1, 0.0, 1.0, 1.0, 0.0, 0.0);
        let p = Point::new(10.0, -1.0);
        assert_eq!(t.transform_point_as(Contraction::Gcc, p).x, 2f64.powi(-54));
        assert_eq!(t.transform_point_as(Contraction::Clang, p).x, 0.0);
        assert_eq!(t.transform_point_as(Contraction::Unfused, p).x, 0.0);
        assert_eq!(t.transform_point(p), t.transform_point_as(CONTRACTION, p));
        if CONTRACTION == Contraction::Gcc {
            assert_eq!(t.transform_point(p), t.agg_transform_point(p));
        }
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
