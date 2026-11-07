//! The font matrix of PostScript fonts (Type 1 and CFF) as FreeType keeps
//! it: normalized so that the vertical scale is one unit per em, in 16.16,
//! with an offset in whole font units, and applied to glyph outlines in
//! font units before they are scaled.
//!
//! FreeType's handling of the matrix is copyright David Turner, Robert
//! Wilhelm and Werner Lemberg, and ported here under the FreeType License
//! (FTL).

use crate::fixed::{mul_div, mul_fix};
use crate::outline::Outline;

/// A normalized font matrix and offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FontTransform {
    pub(crate) xx: i64,
    pub(crate) yx: i64,
    pub(crate) xy: i64,
    pub(crate) yy: i64,
    pub(crate) offset: (i64, i64),
}

impl FontTransform {
    /// A Type 1 font without `/FontMatrix`: FreeType's matrix stays zero.
    pub(crate) const ZERO: Self = Self {
        xx: 0,
        yx: 0,
        xy: 0,
        yy: 0,
        offset: (0, 0),
    };

    /// The identity, which leaves outlines alone.
    pub(crate) const IDENTITY: Self = Self {
        xx: 0x10000,
        yx: 0,
        xy: 0,
        yy: 0x10000,
        offset: (0, 0),
    };

    pub(crate) fn is_identity(&self) -> bool {
        self.xx == 0x10000 && self.yy == 0x10000 && self.xy == 0 && self.yx == 0
    }

    /// Applies the matrix to an outline in integer units
    /// (`FT_Outline_Transform`) and returns the advance it scales
    /// (`FT_MulFix` by `xx`); the identity changes neither.
    pub(crate) fn apply(&self, outline: &mut Outline, advance: i64) -> i64 {
        if self.is_identity() {
            return advance;
        }
        for point in &mut outline.points {
            let (x, y) = (point.x as i64, point.y as i64);
            point.x = (mul_fix(x, self.xx) + mul_fix(y, self.xy)) as f64;
            point.y = (mul_fix(x, self.yx) + mul_fix(y, self.yy)) as f64;
        }
        mul_fix(advance, self.xx)
    }

    /// `FT_Matrix_Multiply_Scaled`: this matrix times `other`, whose
    /// values carry a factor of `scaling`.
    pub(crate) fn multiply_scaled(&self, other: &Self, scaling: i64) -> Self {
        let val = 0x10000 * scaling;
        let product = |a: i64, b: i64, c: i64, d: i64| mul_div(a, b, val) + mul_div(c, d, val);
        Self {
            xx: product(self.xx, other.xx, self.xy, other.yx),
            xy: product(self.xx, other.xy, self.xy, other.yy),
            yx: product(self.yx, other.xx, self.yy, other.yx),
            yy: product(self.yx, other.xy, self.yy, other.yy),
            offset: other.offset,
        }
    }

    /// `FT_Vector_Transform_Scaled`: `vector` through this matrix, whose
    /// values carry a factor of `scaling`.
    pub(crate) fn transform_scaled(&self, (x, y): (i64, i64), scaling: i64) -> (i64, i64) {
        let val = 0x10000 * scaling;
        (
            mul_div(x, self.xx, val) + mul_div(y, self.xy, val),
            mul_div(x, self.yx, val) + mul_div(y, self.yy, val),
        )
    }
}

/// `FT_Matrix_Check`: whether a matrix is invertible enough to use.
pub(crate) fn matrix_is_usable(xx: i64, xy: i64, yx: i64, yy: i64) -> bool {
    let val = xx.abs() | xy.abs() | yx.abs() | yy.abs();
    if val == 0 || val > 0x7FFF_FFFF {
        return false;
    }
    let shift = i64::from(63 - val.leading_zeros()) - 12;
    let [xx, xy, yx, yy] = if shift > 0 {
        [xx >> shift, xy >> shift, yx >> shift, yy >> shift]
    } else {
        [xx, xy, yx, yy]
    };
    let temp1 = 32 * (xx * yy - xy * yx).unsigned_abs();
    let temp2 = (xx * xx) as u64 + (xy * xy) as u64 + (yx * yx) as u64 + (yy * yy) as u64;
    temp1 > temp2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::PointKind;

    #[test]
    fn transforms() {
        assert!(!matrix_is_usable(0x10000, 0x10000, 0x10000, 0x10000));
        assert!(matrix_is_usable(0x10000, 0, 0, 0x10000));
        assert!(FontTransform::IDENTITY.is_identity() && !FontTransform::ZERO.is_identity());
        let slant = FontTransform {
            xy: 0x4000,
            ..FontTransform::IDENTITY
        };
        let mut outline = Outline::default();
        outline.push(10.0, 100.0, PointKind::OnCurve);
        assert_eq!(slant.apply(&mut outline, 500), 500);
        assert_eq!((outline.points[0].x, outline.points[0].y), (35.0, 100.0));
        let half = FontTransform {
            xx: 0x8000,
            yy: 0x8000,
            ..FontTransform::IDENTITY
        };
        // Both carry a factor of 1000: the product is a quarter.
        let thousand = FontTransform {
            xx: 1000 << 16,
            yy: 1000 << 16,
            ..FontTransform::IDENTITY
        };
        let product = thousand.multiply_scaled(&half, 1000);
        assert_eq!((product.xx, product.yy), (0x8000, 0x8000));
        assert_eq!(half.transform_scaled((2000, 10), 1), (1000, 5));
    }
}
