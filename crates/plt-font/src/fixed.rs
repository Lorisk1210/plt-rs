//! FreeType-compatible fixed-point arithmetic.
//!
//! matplotlib measures text with FreeType and HarfBuzz; reproducing their
//! widths to the 1/64 pixel requires the same rounding, so these helpers are
//! transliterations of `FT_MulFix`, `FT_MulDiv` and `FT_DivFix`.

/// `FT_MulFix`: `a * b / 65536`, rounded half away from zero.
pub(crate) fn mul_fix(a: i64, b: i64) -> i64 {
    let ab = a * b;
    (ab + 0x8000 - i64::from(ab < 0)) >> 16
}

/// `FT_MulDiv`: `a * b / c`, rounded half away from zero on magnitudes.
pub(crate) fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    let negative = (a < 0) ^ (b < 0) ^ (c < 0);
    let (a, b, c) = (a.unsigned_abs(), b.unsigned_abs(), c.unsigned_abs());
    let magnitude = (a * b + (c >> 1))
        .checked_div(c)
        .map_or(0x7FFF_FFFF, |q| q as i64);
    if negative { -magnitude } else { magnitude }
}

/// `FT_DivFix`: `a * 65536 / b`, rounded half away from zero on magnitudes.
pub(crate) fn div_fix(a: i64, b: i64) -> i64 {
    let negative = (a < 0) ^ (b < 0);
    let (a, b) = (a.unsigned_abs(), b.unsigned_abs());
    let magnitude = ((a << 16) + (b >> 1))
        .checked_div(b)
        .map_or(0x7FFF_FFFF, |q| q as i64);
    if negative { -magnitude } else { magnitude }
}

/// `FT_PIX_ROUND`: rounds a 26.6 value to the nearest whole pixel.
pub(crate) const fn pix_round(value: i64) -> i64 {
    (value + 32) & !63
}

/// `FT_Hypot` on 16.16 values (an integer square root of the sum of squares).
pub(crate) fn hypot(x: i64, y: i64) -> i64 {
    let sum = (x * x + y * y) as f64;
    sum.sqrt().round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_fix_rounds_half_away_from_zero() {
        assert_eq!(mul_fix(3, 0x8000), 2);
        assert_eq!(mul_fix(-3, 0x8000), -2);
        assert_eq!(mul_fix(1, 0x8000), 1);
        assert_eq!(mul_fix(-1, 0x8000), -1);
        assert_eq!(mul_fix(627, 204_800), 1959);
        assert_eq!(mul_fix(-29, 24_576), -11);
    }

    #[test]
    fn mul_div_and_div_fix() {
        assert_eq!(mul_div(1401, 24_576, 64), 537_984);
        assert_eq!(mul_div(-7, 1, 2), -4);
        assert_eq!(mul_div(5, 5, 0), 0x7FFF_FFFF);
        assert_eq!(div_fix(889, 2048), 28_448);
        assert_eq!(div_fix(768, 2048), 24_576);
        assert_eq!(div_fix(-768, 2048), -24_576);
    }

    #[test]
    fn pixel_rounding_and_hypot() {
        assert_eq!(pix_round(31), 0);
        assert_eq!(pix_round(32), 64);
        assert_eq!(pix_round(-33), -64);
        assert_eq!(hypot(0x1_0000, 0), 0x1_0000);
        assert_eq!(hypot(3 << 16, 4 << 16), 5 << 16);
    }
}
