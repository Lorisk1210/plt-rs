//! The inverse transforms of the specification (section 7.13), bit for
//! bit: the encoder reconstructs exactly what a decoder will, so that
//! later blocks predict from the same samples.

use super::{Kind, TxType};

/// `Cos128_Lookup`: `4096 · cos(angle · π / 128)` for angles 0 to 64.
const COS128: [i32; 65] = [
    4096, 4095, 4091, 4085, 4076, 4065, 4052, 4036, 4017, 3996, 3973, 3948, 3920, 3889, 3857, 3822,
    3784, 3745, 3703, 3659, 3612, 3564, 3513, 3461, 3406, 3349, 3290, 3229, 3166, 3102, 3035, 2967,
    2896, 2824, 2751, 2675, 2598, 2520, 2440, 2359, 2276, 2191, 2106, 2019, 1931, 1842, 1751, 1660,
    1567, 1474, 1380, 1285, 1189, 1092, 995, 897, 799, 700, 601, 501, 401, 301, 201, 101, 0,
];

/// The intermediate range of the row transforms at 8 bits (`BitDepth + 8`).
const ROW_CLAMP_BITS: u32 = 16;
/// The range between the row and column transforms (`Max(BitDepth + 6, 16)`).
const COL_CLAMP_BITS: u32 = 16;

/// `Round2`: `x / 2^n` rounded half up.
pub(crate) fn round2(x: i64, n: u32) -> i64 {
    if n == 0 { x } else { (x + (1 << (n - 1))) >> n }
}

/// `cos128` of any integer angle.
fn cos128(angle: i32) -> i64 {
    let a = (angle & 255) as usize;
    i64::from(match a {
        0..=64 => COS128[a],
        65..=128 => -COS128[128 - a],
        129..=192 => -COS128[a - 128],
        _ => COS128[256 - a],
    })
}

/// `sin128(angle)`, which is `cos128(angle - 64)`.
fn sin128(angle: i32) -> i64 {
    cos128(angle - 64)
}

/// The bit reversal of the `bits` low bits of `x` (`brev`).
fn brev(bits: u32, x: usize) -> usize {
    (0..bits).fold(0, |t, i| t | (((x >> i) & 1) << (bits - 1 - i)))
}

/// The array of a 1D transform, with the butterflies of section 7.13.2.1.
struct Lanes<'a> {
    t: &'a mut [i32],
    /// The intermediate range `r`, in bits.
    range: u32,
}

impl Lanes<'_> {
    /// `B(a, b, angle, flip, r)`: a rotation, then a swap when `flip`.
    fn b(&mut self, a: usize, b: usize, angle: usize, flip: bool) {
        let angle = angle as i32;
        let (ta, tb) = (i64::from(self.t[a]), i64::from(self.t[b]));
        let x = ta * cos128(angle) - tb * sin128(angle);
        let y = ta * sin128(angle) + tb * cos128(angle);
        self.t[a] = round2(x, 12) as i32;
        self.t[b] = round2(y, 12) as i32;
        if flip {
            self.t.swap(a, b);
        }
    }

    /// `H(a, b, flip, r)`: sum and difference, clamped to `r` bits.
    fn h(&mut self, a: usize, b: usize, flip: bool) {
        let (a, b) = if flip { (b, a) } else { (a, b) };
        let limit = 1i64 << (self.range - 1);
        let clamp = |v: i64| v.clamp(-limit, limit - 1) as i32;
        let (x, y) = (i64::from(self.t[a]), i64::from(self.t[b]));
        self.t[a] = clamp(x + y);
        self.t[b] = clamp(x - y);
    }
}

/// The inverse DCT of `2^n` values (section 7.13.2.3), `n` from 2 to 6.
#[expect(
    clippy::too_many_lines,
    reason = "the specification's 31 ordered steps, transcribed one for one"
)]
pub(crate) fn dct(t: &mut [i32], n: u32, range: u32) {
    let size = 1 << n;
    let copy: Vec<i32> = t[..size].to_vec();
    for (i, v) in t[..size].iter_mut().enumerate() {
        *v = copy[brev(n, i)];
    }
    let mut l = Lanes { t, range };
    if n == 6 {
        for i in 0..16 {
            l.b(32 + i, 63 - i, 63 - 4 * brev(4, i), false);
        }
    }
    if n >= 5 {
        for i in 0..8 {
            l.b(16 + i, 31 - i, 6 + (brev(3, 7 - i) << 3), false);
        }
    }
    if n == 6 {
        for i in 0..16 {
            l.h(32 + i * 2, 33 + i * 2, i & 1 == 1);
        }
    }
    if n >= 4 {
        for i in 0..4 {
            l.b(8 + i, 15 - i, 12 + (brev(2, 3 - i) << 4), false);
        }
    }
    if n >= 5 {
        for i in 0..8 {
            l.h(16 + 2 * i, 17 + 2 * i, i & 1 == 1);
        }
    }
    if n == 6 {
        for i in 0..4 {
            for j in 0..2 {
                l.b(
                    62 - i * 4 - j,
                    33 + i * 4 + j,
                    60 - 16 * brev(2, i) + 64 * j,
                    true,
                );
            }
        }
    }
    if n >= 3 {
        for i in 0..2 {
            l.b(4 + i, 7 - i, 56 - 32 * i, false);
        }
    }
    if n >= 4 {
        for i in 0..4 {
            l.h(8 + 2 * i, 9 + 2 * i, i & 1 == 1);
        }
    }
    if n >= 5 {
        for i in 0..2 {
            for j in 0..2 {
                l.b(
                    30 - 4 * i - j,
                    17 + 4 * i + j,
                    24 + (j << 6) + ((1 - i) << 5),
                    true,
                );
            }
        }
    }
    if n == 6 {
        for i in 0..8 {
            for j in 0..2 {
                l.h(32 + i * 4 + j, 35 + i * 4 - j, i & 1 == 1);
            }
        }
    }
    for i in 0..2 {
        l.b(2 * i, 2 * i + 1, 32 + 16 * i, i == 0);
    }
    if n >= 3 {
        for i in 0..2 {
            l.h(4 + 2 * i, 5 + 2 * i, i == 1);
        }
    }
    if n >= 4 {
        for i in 0..2 {
            l.b(14 - i, 9 + i, 48 + 64 * i, true);
        }
    }
    if n >= 5 {
        for i in 0..4 {
            for j in 0..2 {
                l.h(16 + 4 * i + j, 19 + 4 * i - j, i & 1 == 1);
            }
        }
    }
    if n == 6 {
        for i in 0..2 {
            for j in 0..4 {
                l.b(
                    61 - i * 8 - j,
                    34 + i * 8 + j,
                    56 - i * 32 + (j >> 1) * 64,
                    true,
                );
            }
        }
    }
    for i in 0..2 {
        l.h(i, 3 - i, false);
    }
    if n >= 3 {
        l.b(6, 5, 32, true);
    }
    if n >= 4 {
        for i in 0..2 {
            for j in 0..2 {
                l.h(8 + 4 * i + j, 11 + 4 * i - j, i == 1);
            }
        }
    }
    if n >= 5 {
        for i in 0..4 {
            l.b(29 - i, 18 + i, 48 + (i >> 1) * 64, true);
        }
    }
    if n == 6 {
        for i in 0..4 {
            for j in 0..4 {
                l.h(32 + 8 * i + j, 39 + 8 * i - j, i & 1 == 1);
            }
        }
    }
    if n >= 3 {
        for i in 0..4 {
            l.h(i, 7 - i, false);
        }
    }
    if n >= 4 {
        for i in 0..2 {
            l.b(13 - i, 10 + i, 32, true);
        }
    }
    if n >= 5 {
        for i in 0..2 {
            for j in 0..4 {
                l.h(16 + i * 8 + j, 23 + i * 8 - j, i == 1);
            }
        }
    }
    if n == 6 {
        for i in 0..8 {
            l.b(59 - i, 36 + i, if i < 4 { 48 } else { 112 }, true);
        }
    }
    if n >= 4 {
        for i in 0..8 {
            l.h(i, 15 - i, false);
        }
    }
    if n >= 5 {
        for i in 0..4 {
            l.b(27 - i, 20 + i, 32, true);
        }
    }
    if n == 6 {
        for i in 0..8 {
            l.h(32 + i, 47 - i, false);
            l.h(48 + i, 63 - i, true);
        }
    }
    if n >= 5 {
        for i in 0..16 {
            l.h(i, 31 - i, false);
        }
    }
    if n == 6 {
        for i in 0..8 {
            l.b(55 - i, 40 + i, 32, true);
        }
        for i in 0..32 {
            l.h(i, 63 - i, false);
        }
    }
}

/// The inverse ADST4 (section 7.13.2.6).
fn adst4(t: &mut [i32]) {
    const SINPI_1_9: i64 = 1321;
    const SINPI_2_9: i64 = 2482;
    const SINPI_3_9: i64 = 3344;
    const SINPI_4_9: i64 = 3803;
    let [t0, t1, t2, t3] = [t[0], t[1], t[2], t[3]].map(i64::from);
    let mut s0 = SINPI_1_9 * t0;
    let mut s1 = SINPI_2_9 * t0;
    let s2 = SINPI_3_9 * t1;
    let s3 = SINPI_4_9 * t2;
    let s4 = SINPI_1_9 * t2;
    let s5 = SINPI_2_9 * t3;
    let s6 = SINPI_4_9 * t3;
    let b7 = t0 - t2 + t3;
    s0 += s3;
    s1 -= s4;
    let s3 = s2;
    let s2 = SINPI_3_9 * b7;
    s0 += s5;
    s1 -= s6;
    let x = [s0 + s3, s1 + s3, s2, s0 + s1 - s3];
    for (v, x) in t.iter_mut().zip(x) {
        *v = round2(x, 12) as i32;
    }
}

/// The ADST input permutation (section 7.13.2.4).
fn adst_input_permutation(t: &mut [i32], n: u32) {
    let n0 = 1 << n;
    let copy: Vec<i32> = t[..n0].to_vec();
    for (i, v) in t[..n0].iter_mut().enumerate() {
        *v = copy[if i & 1 == 1 { i - 1 } else { n0 - i - 1 }];
    }
}

/// The ADST output permutation (section 7.13.2.5).
fn adst_output_permutation(t: &mut [i32], n: u32) {
    let n0 = 1 << n;
    let copy: Vec<i32> = t[..n0].to_vec();
    for (i, v) in t[..n0].iter_mut().enumerate() {
        let a = (i >> 3) & 1;
        let b = ((i >> 2) & 1) ^ ((i >> 3) & 1);
        let c = ((i >> 1) & 1) ^ ((i >> 2) & 1);
        let d = (i & 1) ^ ((i >> 1) & 1);
        let idx = ((d << 3) | (c << 2) | (b << 1) | a) >> (4 - n);
        *v = if i & 1 == 1 { -copy[idx] } else { copy[idx] };
    }
}

/// The inverse ADST8 (section 7.13.2.7).
fn adst8(t: &mut [i32], range: u32) {
    adst_input_permutation(t, 3);
    let mut l = Lanes { t, range };
    for i in 0..4 {
        l.b(2 * i, 2 * i + 1, 60 - 16 * i, true);
    }
    for i in 0..4 {
        l.h(i, 4 + i, false);
    }
    for i in 0..2 {
        l.b(4 + 3 * i, 5 + i, 48 - 32 * i, true);
    }
    for i in 0..2 {
        for j in 0..2 {
            l.h(4 * j + i, 2 + 4 * j + i, false);
        }
    }
    for i in 0..2 {
        l.b(2 + 4 * i, 3 + 4 * i, 32, true);
    }
    adst_output_permutation(t, 3);
}

/// The inverse ADST16 (section 7.13.2.8).
fn adst16(t: &mut [i32], range: u32) {
    adst_input_permutation(t, 4);
    let mut l = Lanes { t, range };
    for i in 0..8 {
        l.b(2 * i, 2 * i + 1, 62 - 8 * i, true);
    }
    for i in 0..8 {
        l.h(i, 8 + i, false);
    }
    for i in 0..2 {
        l.b(8 + 2 * i, 9 + 2 * i, 56 - 32 * i, true);
        l.b(13 + 2 * i, 12 + 2 * i, 8 + 32 * i, true);
    }
    for i in 0..4 {
        for j in 0..2 {
            l.h(8 * j + i, 4 + 8 * j + i, false);
        }
    }
    for i in 0..2 {
        for j in 0..2 {
            l.b(4 + 8 * j + 3 * i, 5 + 8 * j + i, 48 - 32 * i, true);
        }
    }
    for i in 0..2 {
        for j in 0..4 {
            l.h(4 * j + i, 2 + 4 * j + i, false);
        }
    }
    for i in 0..4 {
        l.b(2 + 4 * i, 3 + 4 * i, 32, true);
    }
    adst_output_permutation(t, 4);
}

/// The inverse ADST of `2^n` values, `n` from 2 to 4 (section 7.13.2.9).
pub(crate) fn adst(t: &mut [i32], n: u32, range: u32) {
    match n {
        2 => adst4(t),
        3 => adst8(t, range),
        _ => adst16(t, range),
    }
}

/// The inverse identity transform of `2^n` values, `n` from 2 to 5
/// (sections 7.13.2.11 to 7.13.2.15).
pub(crate) fn identity(t: &mut [i32], n: u32) {
    for v in &mut t[..1 << n] {
        let x = i64::from(*v);
        *v = match n {
            2 => round2(x * 5793, 12),
            3 => x * 2,
            4 => round2(x * 11586, 12),
            _ => x * 4,
        } as i32;
    }
}

/// The inverse Walsh-Hadamard transform of 4 values (section 7.13.2.10).
fn walsh_hadamard(t: &mut [i32], shift: u32) {
    let mut a = t[0] >> shift;
    let mut c = t[1] >> shift;
    let mut d = t[2] >> shift;
    let mut b = t[3] >> shift;
    a += c;
    d -= b;
    let e = (a - d) >> 1;
    b = e - b;
    c = e - c;
    a -= b;
    d += c;
    t[..4].copy_from_slice(&[a, b, c, d]);
}

/// A 1D inverse transform of `kind` over `2^n` values.
fn transform_1d(t: &mut [i32], kind: Kind, n: u32, range: u32) {
    match kind {
        Kind::Dct => dct(t, n, range),
        Kind::Adst => adst(t, n, range),
        Kind::Identity => identity(t, n),
    }
}

/// The 2D inverse transform (section 7.13.3) of a square block of
/// `2^log2` samples a side: `dequant` holds the dequantized coefficients
/// row by row, `Min(32, side)` to a row; the residual comes out row by
/// row, `side` to a row. `lossless` selects the Walsh-Hadamard transform
/// of lossless 4×4 blocks.
pub(crate) fn inverse_2d(dequant: &[i32], log2: u32, tx_type: TxType, lossless: bool) -> Vec<i32> {
    let side = 1usize << log2;
    let coded = side.min(32);
    let row_shift = if lossless {
        0
    } else {
        [0, 1, 2, 2, 2][log2 as usize - 2]
    };
    let col_shift = if lossless { 0 } else { 4 };
    let col_limit = 1i32 << (COL_CLAMP_BITS - 1);
    let mut residual = vec![0i32; side * side];
    let mut t = vec![0i32; side];
    for (i, row) in residual.chunks_exact_mut(side).enumerate().take(coded) {
        t.fill(0);
        t[..coded].copy_from_slice(&dequant[i * coded..(i + 1) * coded]);
        if lossless {
            walsh_hadamard(&mut t, 2);
        } else {
            transform_1d(&mut t, tx_type.row(), log2, ROW_CLAMP_BITS);
        }
        for (r, &v) in row.iter_mut().zip(&t) {
            *r = (round2(i64::from(v), row_shift) as i32).clamp(-col_limit, col_limit - 1);
        }
    }
    for j in 0..side {
        for (i, v) in t.iter_mut().enumerate() {
            *v = residual[i * side + j];
        }
        if lossless {
            walsh_hadamard(&mut t, 0);
        } else {
            transform_1d(&mut t, tx_type.column(), log2, COL_CLAMP_BITS);
        }
        for (i, &v) in t.iter().enumerate() {
            residual[i * side + j] = round2(i64::from(v), col_shift) as i32;
        }
    }
    residual
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_reversal() {
        assert_eq!(brev(3, 0b001), 0b100);
        assert_eq!(brev(4, 0b0110), 0b0110);
        assert_eq!(
            (0..8).map(|i| brev(3, i)).collect::<Vec<_>>(),
            [0, 4, 2, 6, 1, 5, 3, 7]
        );
    }

    #[test]
    fn cosines_cover_the_circle() {
        assert_eq!(cos128(0), 4096);
        assert_eq!(cos128(64), 0);
        assert_eq!(cos128(128), -4096);
        assert_eq!(cos128(192), 0);
        assert_eq!(cos128(-32), 2896);
        assert_eq!(sin128(32), 2896);
        assert_eq!(sin128(64), 4096);
    }

    #[test]
    fn rounding_is_half_up() {
        assert_eq!(round2(5, 1), 3);
        assert_eq!(round2(-5, 1), -2);
        assert_eq!(round2(-6, 2), -1);
        assert_eq!(round2(7, 0), 7);
    }

    #[test]
    fn a_dc_coefficient_spreads_evenly() {
        // 4×4: the row gives 512 · 2896 / 4096 = 362 in each sample, the
        // column 362 · 2896 / 4096 = 256 (rounded), shifted by 4.
        let mut dequant = vec![0; 16];
        dequant[0] = 512;
        let residual = inverse_2d(&dequant, 2, TxType::DctDct, false);
        assert!(residual.iter().all(|&r| r == 16), "{residual:?}");
        // 64×64: 800 → 566, shifted by 2 to 142, then 142 → 100 and 6.
        let mut dequant = vec![0; 32 * 32];
        dequant[0] = 800;
        let residual = inverse_2d(&dequant, 6, TxType::DctDct, false);
        assert!(residual.iter().all(|&r| r == 6), "{:?}", &residual[..8]);
    }

    #[test]
    fn walsh_hadamard_of_a_dc_value() {
        let mut dequant = vec![0; 16];
        dequant[0] = 4 * 4;
        let residual = inverse_2d(&dequant, 2, TxType::DctDct, true);
        assert!(residual.iter().all(|&r| r == 1), "{residual:?}");
    }
}
