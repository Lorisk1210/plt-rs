//! FreeType-compatible fixed-point arithmetic.
//!
//! matplotlib measures and hints text with FreeType and HarfBuzz;
//! reproducing their widths and grid-fitted outlines to the 1/64 pixel
//! requires the same rounding, so these helpers are transliterations of
//! `FT_MulFix`, `FT_MulDiv`, `FT_DivFix`, the TrueType interpreter's 2.14
//! products and FreeType's vector length and normalisation. Like FreeType
//! on 64-bit platforms they compute in 64 bits and wrap on overflow, which
//! only hostile font data reaches.

/// `FT_MulFix`: `a * b / 65536`, rounded half away from zero.
pub(crate) const fn mul_fix(a: i64, b: i64) -> i64 {
    let ab = a.wrapping_mul(b);
    ab.wrapping_add(0x8000 + (ab >> 63)) >> 16
}

/// Splits a value into its magnitude and whether it is negative
/// (`FT_MOVE_SIGN`).
const fn magnitude(value: i64) -> (u64, bool) {
    (value.unsigned_abs(), value < 0)
}

/// Applies a sign computed from the operands' signs (`NEG_LONG` for
/// negative results).
const fn signed(value: u64, negative: bool) -> i64 {
    if negative {
        (value as i64).wrapping_neg()
    } else {
        value as i64
    }
}

/// `FT_MulDiv`: `a * b / c`, rounded half away from zero on magnitudes;
/// `0x7FFF_FFFF` when `c` is zero.
pub(crate) const fn mul_div(a: i64, b: i64, c: i64) -> i64 {
    let ((a, sa), (b, sb), (c, sc)) = (magnitude(a), magnitude(b), magnitude(c));
    let d = match a.wrapping_mul(b).wrapping_add(c >> 1).checked_div(c) {
        Some(d) => d,
        None => 0x7FFF_FFFF,
    };
    signed(d, sa ^ sb ^ sc)
}

/// `FT_MulDiv_No_Round`: `a * b / c` truncated on magnitudes;
/// `0x7FFF_FFFF` when `c` is zero.
pub(crate) const fn mul_div_no_round(a: i64, b: i64, c: i64) -> i64 {
    let ((a, sa), (b, sb), (c, sc)) = (magnitude(a), magnitude(b), magnitude(c));
    let d = match a.wrapping_mul(b).checked_div(c) {
        Some(d) => d,
        None => 0x7FFF_FFFF,
    };
    signed(d, sa ^ sb ^ sc)
}

/// `FT_DivFix`: `a * 65536 / b`, rounded half away from zero on
/// magnitudes; `0x7FFF_FFFF` when `b` is zero.
pub(crate) const fn div_fix(a: i64, b: i64) -> i64 {
    let ((a, sa), (b, sb)) = (magnitude(a), magnitude(b));
    let q = match (a << 16).wrapping_add(b >> 1).checked_div(b) {
        Some(q) => q,
        None => 0x7FFF_FFFF,
    };
    signed(q, sa ^ sb)
}

/// `FT_PIX_ROUND`: rounds a 26.6 value to the nearest whole pixel.
pub(crate) const fn pix_round(value: i64) -> i64 {
    value.wrapping_add(32) & !63
}

/// `FT_PIX_FLOOR`: rounds a 26.6 value down to a whole pixel.
pub(crate) const fn pix_floor(value: i64) -> i64 {
    value & !63
}

/// `FT_PIX_CEIL`: rounds a 26.6 value up to a whole pixel.
pub(crate) const fn pix_ceil(value: i64) -> i64 {
    value.wrapping_add(63) & !63
}

/// `TT_MulFix14`: `a * b / 16384` for a 2.14 `b`, rounded half away from
/// zero.
pub(crate) const fn mul_fix14(a: i64, b: i64) -> i64 {
    let ab = a.wrapping_mul(b);
    ab.wrapping_add(0x2000 + (ab >> 63)) >> 14
}

/// `TT_DotFix14`: `(ax * bx + ay * by) / 16384` for a 2.14 vector `b`,
/// rounded half away from zero.
pub(crate) const fn dot_fix14(ax: i64, ay: i64, bx: i64, by: i64) -> i64 {
    let c = ax.wrapping_mul(bx).wrapping_add(ay.wrapping_mul(by));
    c.wrapping_add(0x2000 + (c >> 63)) >> 14
}

/// `FT_MSB`: index of the most significant set bit (0 for zero).
const fn msb(value: u32) -> u32 {
    31u32.saturating_sub(value.leading_zeros())
}

/// `FT_Vector_NormLen`'s unit vector: `(x, y)` (truncated to 32 bits, as
/// FreeType does) scaled to length 0x10000 by Newton iterations on a
/// prenormalised integer vector. A component that is zero stays zero.
pub(crate) fn normalize(x: i64, y: i64) -> (i64, i64) {
    let (x_, y_) = (x as i32, y as i32);
    let (mut ux, sx) = (x_.unsigned_abs(), x_ < 0);
    let (mut uy, sy) = (y_.unsigned_abs(), y_ < 0);
    let apply = |value: u32, negative: bool| {
        if negative {
            -i64::from(value)
        } else {
            i64::from(value)
        }
    };
    if ux == 0 {
        return (x, if uy > 0 { apply(0x10000, sy) } else { y });
    }
    if uy == 0 {
        return (apply(0x10000, sx), y);
    }
    let length = |ux: u32, uy: u32| {
        if ux > uy {
            ux.wrapping_add(uy >> 1)
        } else {
            uy.wrapping_add(ux >> 1)
        }
    };
    let l = length(ux, uy);
    let mut shift = 31 - msb(l) as i32;
    shift -= 15 + i32::from(l >= (0xAAAA_AAAA_u32 >> shift));
    if shift > 0 {
        ux <<= shift;
        uy <<= shift;
    } else {
        ux >>= -shift;
        uy >>= -shift;
    }
    let mut b = 0x10000 - length(ux, uy) as i32;
    let (x_, y_) = (ux as i32, uy as i32);
    let (mut u, mut v);
    // Newton's iterations converge in a handful of steps; the cap only
    // guards against hostile inputs.
    let mut iterations = 0;
    loop {
        u = x_.wrapping_add(x_.wrapping_mul(b) >> 16) as u32;
        v = y_.wrapping_add(y_.wrapping_mul(b) >> 16) as u32;
        let mut z =
            (u.wrapping_mul(u).wrapping_add(v.wrapping_mul(v)) as i32).wrapping_neg() / 0x200;
        z = z.wrapping_mul(b.wrapping_add(0x10000) >> 8) / 0x10000;
        b = b.wrapping_add(z);
        iterations += 1;
        if z <= 0 || iterations >= 64 {
            break;
        }
    }
    (apply(u, sx), apply(v, sy))
}

/// CORDIC arctangents for the pseudo-rotations of `FT_Vector_Length`.
const ARCTAN: [i64; 22] = [
    1_740_967, 919_879, 466_945, 234_379, 117_304, 58_666, 29_335, 14_668, 7_334, 3_667, 1_833,
    917, 458, 229, 115, 57, 29, 14, 7, 4, 2, 1,
];

/// `FT_Vector_Length` (`FT_Hypot`): the length of a 16.16 vector by
/// CORDIC pseudo-polarisation, exactly as FreeType computes it.
pub(crate) fn vector_length(x: i64, y: i64) -> i64 {
    /// The highest bit of prenormalised components (`FT_TRIG_SAFE_MSB`).
    const SAFE_MSB: i32 = 29;
    if x == 0 {
        return y.wrapping_abs();
    }
    if y == 0 {
        return x.wrapping_abs();
    }
    // ft_trig_prenorm
    let bits = msb((x.wrapping_abs() | y.wrapping_abs()) as u32) as i32;
    let (mut x, mut y, shift) = if bits <= SAFE_MSB {
        let shift = SAFE_MSB - bits;
        (
            ((x as u64) << shift) as i64,
            ((y as u64) << shift) as i64,
            shift,
        )
    } else {
        let shift = bits - SAFE_MSB;
        (x >> shift, y >> shift, -shift)
    };
    // ft_trig_pseudo_polarize: only the length is needed.
    if y > x {
        if y > -x {
            (x, y) = (y, -x);
        } else {
            (x, y) = (-x, -y);
        }
    } else if y < -x {
        (x, y) = (-y, x);
    }
    for (i, b) in (1..=ARCTAN.len()).zip((0..).map(|k| 1i64 << k)) {
        let (dx, dy) = ((y + b) >> i, (x + b) >> i);
        if y > 0 {
            (x, y) = (x + dx, y - dy);
        } else {
            (x, y) = (x - dx, y + dy);
        }
    }
    // ft_trig_downscale
    let (value, negative) = magnitude(x);
    let scaled = ((u128::from(value) * 0xDBD9_5B16 + 0x4000_0000) >> 32) as i64;
    let x = if negative { -scaled } else { scaled };
    if shift > 0 {
        (x + (1 << (shift - 1))) >> shift
    } else {
        i64::from((x as u32) << -shift)
    }
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
        // Overflow wraps like FreeType's 64-bit arithmetic.
        let _ = mul_fix(i64::MAX, i64::MAX);
    }

    #[test]
    fn mul_div_and_div_fix() {
        assert_eq!(mul_div(1401, 24_576, 64), 537_984);
        assert_eq!(mul_div(-7, 1, 2), -4);
        assert_eq!(mul_div(5, 5, 0), 0x7FFF_FFFF);
        assert_eq!(mul_div_no_round(-7, 1, 2), -3);
        assert_eq!(mul_div_no_round(7, 64, 2), 224);
        assert_eq!(div_fix(889, 2048), 28_448);
        assert_eq!(div_fix(768, 2048), 24_576);
        assert_eq!(div_fix(-768, 2048), -24_576);
        assert_eq!(div_fix(1, 0), 0x7FFF_FFFF);
        let _ = (mul_div(i64::MIN, i64::MIN, 1), div_fix(i64::MIN, -1));
    }

    #[test]
    fn pixel_rounding() {
        assert_eq!(pix_round(31), 0);
        assert_eq!(pix_round(32), 64);
        assert_eq!(pix_round(-33), -64);
        assert_eq!((pix_floor(-1), pix_floor(63)), (-64, 0));
        assert_eq!((pix_ceil(1), pix_ceil(-63)), (64, 0));
    }

    #[test]
    fn two_dot_fourteen_products() {
        assert_eq!(mul_fix14(100, 0x4000), 100);
        assert_eq!(mul_fix14(-3, 0x2000), -2);
        assert_eq!(dot_fix14(64, 64, 0x2D41, 0x2D41), 91);
    }

    #[test]
    fn normalized_vectors_have_unit_length() {
        assert_eq!(normalize(0x4000, 0), (0x10000, 0));
        assert_eq!(normalize(0, -5), (0, -0x10000));
        assert_eq!(normalize(0, 0), (0, 0));
        let (x, y) = normalize(3, 4);
        assert!(
            (x - 39_322).abs() <= 1 && (y - 52_429).abs() <= 1,
            "{x} {y}"
        );
        let (x, y) = normalize(-1000, 1000);
        assert!(
            (x + 46_341).abs() <= 1 && (y - 46_341).abs() <= 1,
            "{x} {y}"
        );
        for (vx, vy) in [
            (i64::MAX, 1),
            (i64::MIN, i64::MIN),
            (1, -1),
            (0x7FFF_FFFF, 3),
        ] {
            let _ = normalize(vx, vy);
        }
    }

    #[test]
    fn cordic_lengths() {
        assert_eq!(vector_length(0x1_0000, 0), 0x1_0000);
        assert_eq!(vector_length(0, -7), 7);
        assert_eq!(vector_length(3 << 16, 4 << 16), 5 << 16);
        assert_eq!(vector_length(0x4000 << 2, 0x4000 << 2), 92_682);
        assert_eq!(vector_length(-3, 4), 5);
    }
}
