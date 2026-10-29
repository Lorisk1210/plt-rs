//! Agg's fixed-point constants, rounding helpers and affine arithmetic.
//!
//! Bit-exact agreement with matplotlib depends on reproducing Agg's integer
//! conversions *exactly*, including their behaviour on x86-64 for values
//! that do not fit in an `int`, and on evaluating affine maps with Agg's
//! operation order (which differs from `pltrs_core::Affine2D::inverted`).

use pltrs_core::Affine2D;

/// `image_subpixel_shift`: sample coordinates carry 8 fractional bits.
pub(super) const SUBPIXEL_SHIFT: u32 = 8;
/// `image_subpixel_scale`.
pub(super) const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;
/// `image_subpixel_mask`.
pub(super) const SUBPIXEL_MASK: i32 = SUBPIXEL_SCALE - 1;
/// `image_filter_shift`: filter weights carry 14 fractional bits.
pub(super) const FILTER_SHIFT: u32 = 14;
/// `image_filter_scale`: the fixed-point value of a weight of 1.
pub(super) const FILTER_SCALE: i32 = 1 << FILTER_SHIFT;

/// `int(v)` as compiled for x86-64 (`cvttsd2si`): truncation toward zero,
/// with NaN and out-of-range inputs producing `i32::MIN` ("integer
/// indefinite") instead of Rust's saturation.
pub(super) fn trunc(v: f64) -> i32 {
    if v.is_nan() || v >= 2_147_483_648.0 || v <= -2_147_483_649.0 {
        i32::MIN
    } else {
        v as i32
    }
}

/// Agg's `iround`: round half away from zero by biasing then truncating.
pub(super) fn iround(v: f64) -> i32 {
    trunc(if v < 0.0 { v - 0.5 } else { v + 0.5 })
}

/// Agg's `uround`, only ever applied to small non-negative values.
pub(super) fn uround(v: f64) -> i32 {
    trunc(v + 0.5)
}

/// An affine map stored and evaluated exactly like Agg's `trans_affine`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct AggAffine {
    pub(super) sx: f64,
    pub(super) shy: f64,
    pub(super) shx: f64,
    pub(super) sy: f64,
    pub(super) tx: f64,
    pub(super) ty: f64,
}

impl AggAffine {
    /// The identity map.
    pub(super) const IDENTITY: Self = Self {
        sx: 1.0,
        shy: 0.0,
        shx: 0.0,
        sy: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Converts a matplotlib-style affine (`convert_trans_affine`).
    pub(super) fn from_affine(affine: &Affine2D) -> Self {
        let [a, b, c, d, e, f] = affine.values();
        Self {
            sx: a,
            shy: b,
            shx: c,
            sy: d,
            tx: e,
            ty: f,
        }
    }

    /// `trans_affine::transform`.
    pub(super) fn transform(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x * self.sx + y * self.shx + self.tx,
            x * self.shy + y * self.sy + self.ty,
        )
    }

    /// `trans_affine::invert`, including its use of the determinant's
    /// reciprocal and its update order.
    pub(super) fn inverted(&self) -> Self {
        let d = 1.0 / (self.sx * self.sy - self.shy * self.shx);
        let t0 = self.sy * d;
        let sy = self.sx * d;
        let shy = -self.shy * d;
        let shx = -self.shx * d;
        let t4 = -self.tx * t0 - self.ty * shx;
        let ty = -self.tx * shy - self.ty * sy;
        Self {
            sx: t0,
            shy,
            shx,
            sy,
            tx: t4,
            ty,
        }
    }

    /// `trans_affine::scaling_abs`: the length of each output row, used to
    /// estimate how many source pixels one destination pixel spans.
    pub(super) fn scaling_abs(&self) -> (f64, f64) {
        (
            (self.sx * self.sx + self.shx * self.shx).sqrt(),
            (self.shy * self.shy + self.sy * self.sy).sqrt(),
        )
    }

    /// Whether every coefficient is finite.
    pub(super) fn is_finite(&self) -> bool {
        [self.sx, self.shy, self.shx, self.sy, self.tx, self.ty]
            .iter()
            .all(|v| v.is_finite())
    }

    /// Whether the map is a pure (possibly flipped) translation, for which
    /// matplotlib always falls back to nearest-neighbour sampling.
    pub(super) fn is_unit_scale_translation(&self) -> bool {
        // Exact comparisons mirror `fabs(sx) == 1.0` in the C++ source.
        #[expect(clippy::float_cmp, reason = "matplotlib compares exactly")]
        let unit = self.sx.abs() == 1.0 && self.sy.abs() == 1.0;
        unit && !self.has_shear()
    }

    /// Whether the off-diagonal (rotation/shear) terms are non-zero.
    pub(super) fn has_shear(&self) -> bool {
        self.shx != 0.0 || self.shy != 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trunc_mirrors_x86_integer_indefinite() {
        assert_eq!(trunc(2.9), 2);
        assert_eq!(trunc(-2.9), -2);
        assert_eq!(trunc(f64::NAN), i32::MIN);
        assert_eq!(trunc(3.0e9), i32::MIN);
        assert_eq!(trunc(-3.0e9), i32::MIN);
        assert_eq!(trunc(f64::INFINITY), i32::MIN);
    }

    #[test]
    fn iround_rounds_half_away_from_zero() {
        assert_eq!(iround(0.5), 1);
        assert_eq!(iround(-0.5), -1);
        assert_eq!(iround(1.49), 1);
        assert_eq!(iround(-1.51), -2);
        assert_eq!(uround(2.5), 3);
    }

    #[test]
    fn inversion_round_trips() {
        let t = AggAffine::from_affine(
            &Affine2D::identity()
                .scale(2.0, -3.0)
                .rotate_deg(30.0)
                .translate(4.0, 5.0),
        );
        let inv = t.inverted();
        let (x, y) = t.transform(1.25, -0.5);
        let (bx, by) = inv.transform(x, y);
        assert!((bx - 1.25).abs() < 1e-12 && (by + 0.5).abs() < 1e-12);
    }

    #[test]
    fn unit_scale_detection() {
        let flip = AggAffine::from_affine(&Affine2D::from_values(1.0, 0.0, 0.0, -1.0, 0.0, 7.0));
        assert!(flip.is_unit_scale_translation());
        let scaled = AggAffine::from_affine(&Affine2D::scaling(2.0, 1.0));
        assert!(!scaled.is_unit_scale_translation());
        let rotated = AggAffine::from_affine(&Affine2D::rotation(0.1));
        assert!(rotated.has_shear() && !rotated.is_unit_scale_translation());
    }
}
