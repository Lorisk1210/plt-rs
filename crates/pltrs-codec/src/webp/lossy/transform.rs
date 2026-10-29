//! VP8's inverse transforms (libwebp's `dsp/dec.c`; RFC 6386 section 14):
//! the 4×4 DCT that adds residuals to a predicted block, and the
//! Walsh–Hadamard transform of the luma DC coefficients.

/// `a · (1 + 20091 / 65536)`: `a · √2 · cos(π/8)` in fixed point. The
/// product wraps like libwebp's `int` arithmetic, which only the
/// coefficients of corrupt streams make overflow.
fn mul1(a: i32) -> i32 {
    (a.wrapping_mul(20091) >> 16) + a
}

/// `a · 35468 / 65536`: `a · √2 · sin(π/8)` in fixed point, wrapping as
/// [`mul1`] does.
fn mul2(a: i32) -> i32 {
    a.wrapping_mul(35468) >> 16
}

/// Inverts the DCT of the 16 coefficients `input` (raster order) and adds
/// the result to the 4×4 block at `origin` of `block` (rows `stride`
/// apart), clamping to 0–255.
pub(super) fn add_inverse_dct(input: &[i16], block: &mut [u8], origin: usize, stride: usize) {
    let coeff = |i: usize| i32::from(input[i]);
    let mut columns = [0i32; 16];
    for i in 0..4 {
        let a = coeff(i) + coeff(8 + i);
        let b = coeff(i) - coeff(8 + i);
        let c = mul2(coeff(4 + i)) - mul1(coeff(12 + i));
        let d = mul1(coeff(4 + i)) + mul2(coeff(12 + i));
        columns[4 * i..4 * i + 4].copy_from_slice(&[a + d, b + c, b - c, a - d]);
    }
    for row in 0..4 {
        let dc = columns[row] + 4;
        let a = dc + columns[8 + row];
        let b = dc - columns[8 + row];
        let c = mul2(columns[4 + row]) - mul1(columns[12 + row]);
        let d = mul1(columns[4 + row]) + mul2(columns[12 + row]);
        let start = origin + row * stride;
        for (pixel, value) in block[start..start + 4]
            .iter_mut()
            .zip([a + d, b + c, b - c, a - d])
        {
            *pixel = (i32::from(*pixel) + (value >> 3)).clamp(0, 255) as u8;
        }
    }
}

/// Inverts the Walsh–Hadamard transform of the luma DC coefficients,
/// writing each block's DC coefficient (`out[16 · n]` for block `n`).
pub(super) fn inverse_wht(input: &[i16; 16], out: &mut [i16]) {
    let coeff = |i: usize| i32::from(input[i]);
    let mut temp = [0i32; 16];
    for i in 0..4 {
        let a0 = coeff(i) + coeff(12 + i);
        let a1 = coeff(4 + i) + coeff(8 + i);
        let a2 = coeff(4 + i) - coeff(8 + i);
        let a3 = coeff(i) - coeff(12 + i);
        temp[i] = a0 + a1;
        temp[8 + i] = a0 - a1;
        temp[4 + i] = a3 + a2;
        temp[12 + i] = a3 - a2;
    }
    for i in 0..4 {
        let dc = temp[4 * i] + 3;
        let a0 = dc + temp[4 * i + 3];
        let a1 = temp[4 * i + 1] + temp[4 * i + 2];
        let a2 = temp[4 * i + 1] - temp[4 * i + 2];
        let a3 = dc - temp[4 * i + 3];
        for (k, value) in [a0 + a1, a3 + a2, a0 - a1, a3 - a2].into_iter().enumerate() {
            out[(4 * i + k) * 16] = (value >> 3) as i16;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dc_coefficient_shifts_the_whole_block() {
        let mut coeffs = [0i16; 16];
        coeffs[0] = 80;
        let mut block = [100u8; 4 * 6];
        add_inverse_dct(&coeffs, &mut block, 1, 6);
        // (80 + 4) >> 3 = 10 everywhere in the 4×4 block.
        assert_eq!(&block[1..5], &[110; 4]);
        assert_eq!(&block[19..23], &[110; 4]);
        assert_eq!(block[0], 100, "outside the block");
        let mut dark = [3u8; 16];
        coeffs[0] = -80;
        add_inverse_dct(&coeffs, &mut dark, 0, 4);
        assert_eq!(dark, [0; 16], "clamped at zero");
    }

    #[test]
    fn extreme_coefficients_wrap_instead_of_overflowing() {
        // Found by fuzzing: corrupt streams decode to coefficients whose
        // fixed-point products overflow `i32`.
        for value in [i16::MAX, i16::MIN] {
            let mut block = [128u8; 16];
            add_inverse_dct(&[value; 16], &mut block, 0, 4);
            let mut out = [0i16; 256];
            inverse_wht(&[value; 16], &mut out);
        }
    }

    #[test]
    fn a_lone_wht_dc_spreads_evenly() {
        let mut input = [0i16; 16];
        input[0] = 85;
        let mut out = [0i16; 256];
        inverse_wht(&input, &mut out);
        // (85 + 3) >> 3 = 11 for every block.
        assert!((0..16).all(|n| out[16 * n] == 11));
    }
}
