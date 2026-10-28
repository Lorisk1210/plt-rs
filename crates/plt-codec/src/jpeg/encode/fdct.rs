//! The forward DCT and quantization of 8×8 blocks, as libjpeg-turbo does
//! them by default: the accurate integer DCT (`jpeg_fdct_islow`, after
//! Loeffler, Ligtenberg and Moschytz) and division by reciprocal
//! multiplication (`compute_reciprocal` and `quantize`).

use crate::jpeg::tables::{
    CONST_BITS, FIX_0_298631336, FIX_0_390180644, FIX_0_541196100, FIX_0_765366865,
    FIX_0_899976223, FIX_1_175875602, FIX_1_501321110, FIX_1_847759065, FIX_1_961570560,
    FIX_2_053119869, FIX_2_562915447, FIX_3_072711026, PASS1_BITS,
};

/// `x / 2^n`, rounded half up.
fn descale(x: i32, n: u32) -> i32 {
    (x + (1 << (n - 1))) >> n
}

/// One butterfly pass over eight values `at(0..8)` of `data`; `even` and
/// `odd` finish the even- and odd-indexed outputs.
fn pass(data: &mut [i32; 64], at: impl Fn(usize) -> usize, even: impl Fn(i32) -> i32, odd: u32) {
    let v = |i: usize| data[at(i)];
    let (tmp0, tmp7) = (v(0) + v(7), v(0) - v(7));
    let (tmp1, tmp6) = (v(1) + v(6), v(1) - v(6));
    let (tmp2, tmp5) = (v(2) + v(5), v(2) - v(5));
    let (tmp3, tmp4) = (v(3) + v(4), v(3) - v(4));

    let (tmp10, tmp13) = (tmp0 + tmp3, tmp0 - tmp3);
    let (tmp11, tmp12) = (tmp1 + tmp2, tmp1 - tmp2);
    let z1 = (tmp12 + tmp13) * FIX_0_541196100;
    let out_even = [
        even(tmp10 + tmp11),
        descale(z1 + tmp13 * FIX_0_765366865, odd),
        even(tmp10 - tmp11),
        descale(z1 - tmp12 * FIX_1_847759065, odd),
    ];

    let (z1, z2) = (tmp4 + tmp7, tmp5 + tmp6);
    let (z3, z4) = (tmp4 + tmp6, tmp5 + tmp7);
    let z5 = (z3 + z4) * FIX_1_175875602;
    let (tmp4, tmp5) = (tmp4 * FIX_0_298631336, tmp5 * FIX_2_053119869);
    let (tmp6, tmp7) = (tmp6 * FIX_3_072711026, tmp7 * FIX_1_501321110);
    let (z1, z2) = (-z1 * FIX_0_899976223, -z2 * FIX_2_562915447);
    let z3 = -z3 * FIX_1_961570560 + z5;
    let z4 = -z4 * FIX_0_390180644 + z5;
    let out_odd = [
        descale(tmp7 + z1 + z4, odd),
        descale(tmp6 + z2 + z3, odd),
        descale(tmp5 + z2 + z4, odd),
        descale(tmp4 + z1 + z3, odd),
    ];

    for (k, (e, o)) in out_even.into_iter().zip(out_odd).enumerate() {
        data[at(2 * k)] = e;
        data[at(2 * k + 1)] = o;
    }
}

/// The DCT of a block of level-shifted samples (row-major), scaled up by
/// 8 as libjpeg leaves it.
pub(super) fn forward_dct(samples: &[i32; 64]) -> [i32; 64] {
    let mut data = *samples;
    for row in 0..8 {
        pass(
            &mut data,
            |i| row * 8 + i,
            |x| x << PASS1_BITS,
            CONST_BITS - PASS1_BITS,
        );
    }
    for column in 0..8 {
        pass(
            &mut data,
            |i| i * 8 + column,
            |x| descale(x, PASS1_BITS),
            CONST_BITS + PASS1_BITS,
        );
    }
    data
}

/// Division of DCT outputs by one quantization step through a reciprocal
/// (libjpeg-turbo's `compute_reciprocal`).
#[derive(Clone, Copy, Debug)]
struct Reciprocal {
    multiplier: u32,
    correction: u32,
    shift: u32,
}

impl Reciprocal {
    /// The reciprocal of `divisor`, the quantization step times 8.
    fn new(divisor: u32) -> Self {
        let bits = divisor.ilog2();
        let mut shift = 16 + bits;
        let mut multiplier = (1_u32 << shift) / divisor;
        let remainder = (1_u32 << shift) % divisor;
        let mut correction = divisor / 2;
        if remainder == 0 {
            multiplier >>= 1;
            shift -= 1;
        } else if remainder <= divisor / 2 {
            correction += 1;
        } else {
            multiplier += 1;
        }
        Self {
            multiplier,
            correction,
            shift,
        }
    }

    /// `value / divisor`, rounded to nearest with the reciprocal's bias.
    fn divide(self, value: i32) -> i32 {
        let magnitude = value.unsigned_abs();
        let quotient = ((magnitude + self.correction) * self.multiplier) >> self.shift;
        let quotient = i32::try_from(quotient).expect("a quotient below the dividend");
        if value < 0 { -quotient } else { quotient }
    }
}

/// Quantizes DCT outputs with one table.
#[derive(Clone, Debug)]
pub(super) struct Quantizer {
    reciprocals: [Reciprocal; 64],
}

impl Quantizer {
    /// The quantizer for `table` (natural order).
    pub(super) fn new(table: &[u16; 64]) -> Self {
        Self {
            reciprocals: table.map(|step| Reciprocal::new(u32::from(step) << 3)),
        }
    }

    /// The quantized coefficients of a DCT output, in natural order.
    pub(super) fn quantize(&self, coefficients: &[i32; 64]) -> [i32; 64] {
        std::array::from_fn(|i| self.reciprocals[i].divide(coefficients[i]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_blocks_have_only_a_dc_term() {
        let dct = forward_dct(&[10; 64]);
        assert_eq!(dct[0], 10 * 64, "the sum, scaled by 8 overall");
        assert!(dct[1..].iter().all(|&c| c == 0));
    }

    #[test]
    fn the_dct_matches_the_float_transform() {
        let samples: [i32; 64] = std::array::from_fn(|i| {
            let (x, y) = (i % 8, i / 8);
            i32::try_from((x * 37 + y * 91 + x * y * 13) % 256).unwrap() - 128
        });
        let dct = forward_dct(&samples);
        for v in 0..8 {
            for u in 0..8 {
                let c = |k: usize| if k == 0 { 0.5_f64.sqrt() } else { 1.0 };
                let mut sum = 0.0;
                for (i, &s) in samples.iter().enumerate() {
                    let (x, y) = ((i % 8) as f64, (i / 8) as f64);
                    let pi = std::f64::consts::PI;
                    sum += f64::from(s)
                        * ((2.0 * x + 1.0) * u as f64 * pi / 16.0).cos()
                        * ((2.0 * y + 1.0) * v as f64 * pi / 16.0).cos();
                }
                // libjpeg's scaling: 8 × the orthonormal transform.
                let expected = 2.0 * c(u) * c(v) * sum;
                let got = f64::from(dct[v * 8 + u]);
                assert!(
                    (got - expected).abs() <= 2.0,
                    "({u}, {v}): {got} vs {expected}"
                );
            }
        }
    }

    #[test]
    fn reciprocals_round_like_division() {
        for step in 1..=255_u16 {
            let quantizer = Quantizer::new(&[step; 64]);
            for value in (-2048 * 8..=2048 * 8).step_by(5) {
                let divisor = i32::from(step) * 8;
                let got = quantizer.reciprocals[0].divide(value);
                let exact = f64::from(value) / f64::from(divisor);
                assert!(
                    (f64::from(got) - exact).abs() <= 0.5 + 1e-9,
                    "{value} / {divisor}: {got}"
                );
            }
        }
    }
}
