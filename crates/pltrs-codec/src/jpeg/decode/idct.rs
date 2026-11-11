//! Dequantization and the accurate integer inverse DCT, libjpeg-turbo's
//! `jpeg_idct_islow` (Loeffler, Ligtenberg and Moschytz), and the
//! component planes it produces.
//!
//! The arithmetic follows the SIMD form of the routine that Pillow's
//! libjpeg-turbo runs on the processor ([`Simd`]). On x86-64
//! (`jsimd_idct_islow_avx2`, like its SSE2 twin): products of coefficients
//! and quantization steps, and the sums the SIMD code forms before
//! multiplying, wrap to 16 bits, the multiplications accumulate in 32
//! bits, and each pass saturates its outputs to 16 bits. On arm64
//! (`jsimd_idct_islow_neon`) the sums are the same and the passes narrow
//! them otherwise: the first keeps the low 16 bits, the second takes the
//! high half before it rounds and saturates. For every block a real
//! encoder produces either is the C routine's result exactly; for corrupt
//! coefficients it is still Pillow's on that processor.

use super::frame::{Block, Component};
use crate::jpeg::tables::{
    CONST_BITS, FIX_0_298631336, FIX_0_390180644, FIX_0_541196100, FIX_0_765366865,
    FIX_0_899976223, FIX_1_175875602, FIX_1_501321110, FIX_1_847759065, FIX_1_961570560,
    FIX_2_053119869, FIX_2_562915447, FIX_3_072711026, PASS1_BITS,
};

/// `a · ca + b · cb` of 16-bit values in 32 bits (`pmaddwd`; the
/// constants are small enough that the sum cannot overflow).
fn madd(a: i16, ca: i32, b: i16, cb: i32) -> i32 {
    i32::from(a) * ca + i32::from(b) * cb
}

/// The SIMD form of the routine: how each pass narrows its 32-bit sums.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Simd {
    /// libjpeg-turbo's AVX2 and SSE2 routines.
    X86,
    /// Its Neon routine.
    Neon,
}

impl Simd {
    /// The form libjpeg-turbo runs on this processor.
    const HOST: Self = if cfg!(target_arch = "aarch64") {
        Self::Neon
    } else {
        Self::X86
    };

    /// A sum of the first pass, descaled by `2^(CONST_BITS - PASS1_BITS)`
    /// and rounded half up.
    fn first(self, x: i32) -> i16 {
        let shift = CONST_BITS - PASS1_BITS;
        match self {
            // `paddd`, `psrad`, then `packssdw`: the rounding wraps, the
            // result saturates.
            Self::X86 => {
                let value = x.wrapping_add(1 << (shift - 1)) >> shift;
                value.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
            }
            // `vrshrn_n_s32`: rounded without wrapping, the low half kept.
            Self::Neon => ((i64::from(x) + (1 << (shift - 1))) >> shift) as i16,
        }
    }

    /// A sum of the second pass as a sample: descaled by
    /// `2^(CONST_BITS + PASS1_BITS + 3)`, rounded half up, saturated to a
    /// signed byte and centred.
    fn second(self, x: i32) -> u8 {
        let shift = CONST_BITS + PASS1_BITS + 3;
        let value = match self {
            // As the first pass, then `packsswb`.
            Self::X86 => (x.wrapping_add(1 << (shift - 1)) >> shift)
                .clamp(i32::from(i16::MIN), i32::from(i16::MAX)),
            // `vaddhn_s32` keeps the high half of the sum, which
            // `vqrshrn_n_s16` rounds without wrapping.
            Self::Neon => ((x >> 16) + (1 << (shift - 17))) >> (shift - 16),
        };
        (value.clamp(-128, 127) + 128) as u8
    }
}

/// The eight sums of one pass over eight inputs (a column, then a row),
/// before they are descaled.
fn pass(input: [i16; 8]) -> [i32; 8] {
    let [in0, in1, in2, in3, in4, in5, in6, in7] = input;
    // Even part: the rotator is sqrt(2) · c(−6).
    let tmp3 = madd(in2, FIX_0_541196100 + FIX_0_765366865, in6, FIX_0_541196100);
    let tmp2 = madd(in2, FIX_0_541196100, in6, FIX_0_541196100 - FIX_1_847759065);
    let tmp0 = i32::from(in0.wrapping_add(in4)) << CONST_BITS;
    let tmp1 = i32::from(in0.wrapping_sub(in4)) << CONST_BITS;
    let (tmp10, tmp13) = (tmp0.wrapping_add(tmp3), tmp0.wrapping_sub(tmp3));
    let (tmp11, tmp12) = (tmp1.wrapping_add(tmp2), tmp1.wrapping_sub(tmp2));

    // Odd part, with z5 folded into z3 and z4.
    let (z3, z4) = (in7.wrapping_add(in3), in5.wrapping_add(in1));
    let z3_out = madd(z3, FIX_1_175875602 - FIX_1_961570560, z4, FIX_1_175875602);
    let z4_out = madd(z3, FIX_1_175875602, z4, FIX_1_175875602 - FIX_0_390180644);
    let odd0 = madd(
        in7,
        FIX_0_298631336 - FIX_0_899976223,
        in1,
        -FIX_0_899976223,
    );
    let odd1 = madd(
        in5,
        FIX_2_053119869 - FIX_2_562915447,
        in3,
        -FIX_2_562915447,
    );
    let odd2 = madd(
        in5,
        -FIX_2_562915447,
        in3,
        FIX_3_072711026 - FIX_2_562915447,
    );
    let odd3 = madd(
        in7,
        -FIX_0_899976223,
        in1,
        FIX_1_501321110 - FIX_0_899976223,
    );
    let (odd0, odd1) = (odd0.wrapping_add(z3_out), odd1.wrapping_add(z4_out));
    let (odd2, odd3) = (odd2.wrapping_add(z3_out), odd3.wrapping_add(z4_out));

    [
        tmp10.wrapping_add(odd3),
        tmp11.wrapping_add(odd2),
        tmp12.wrapping_add(odd1),
        tmp13.wrapping_add(odd0),
        tmp13.wrapping_sub(odd0),
        tmp12.wrapping_sub(odd1),
        tmp11.wrapping_sub(odd2),
        tmp10.wrapping_sub(odd3),
    ]
}

/// The 8×8 samples (row-major) of a block of quantized coefficients and
/// its quantization table (both in natural order).
pub(super) fn inverse_dct(block: &Block, quant: &[u16; 64]) -> [u8; 64] {
    inverse_dct_as(Simd::HOST, block, quant)
}

/// [`inverse_dct`] in the form `simd`.
fn inverse_dct_as(simd: Simd, block: &Block, quant: &[u16; 64]) -> [u8; 64] {
    // Dequantization keeps the low 16 bits (`pmullw`).
    let dequantized: [i16; 64] = std::array::from_fn(|i| block[i].wrapping_mul(quant[i] as i16));
    // Pass 1: columns, scaled up by 2^PASS1_BITS. A block without AC
    // terms below its first row takes the shortcut of shifting its first
    // row in 16 bits (the Neon routine decides that for each half of the
    // block, and its full pass gives those columns the same).
    let mut workspace = [0_i16; 64];
    if block[8..].iter().all(|&c| c == 0) {
        for (column, &dc) in dequantized[..8].iter().enumerate() {
            let value = dc.wrapping_shl(PASS1_BITS);
            for row in 0..8 {
                workspace[row * 8 + column] = value;
            }
        }
    } else {
        for column in 0..8 {
            let input = std::array::from_fn(|row| dequantized[row * 8 + column]);
            for (row, sum) in pass(input).into_iter().enumerate() {
                workspace[row * 8 + column] = simd.first(sum);
            }
        }
    }
    // Pass 2: rows, descaled by 8 · 2^PASS1_BITS, saturated to signed
    // bytes and centred.
    let mut samples = [0; 64];
    let rows = workspace.as_chunks::<8>().0;
    for (&input, out) in rows.iter().zip(samples.as_chunks_mut::<8>().0) {
        for (sample, sum) in out.iter_mut().zip(pass(input)) {
            *sample = simd.second(sum);
        }
    }
    samples
}

/// A component's samples after the inverse DCT: whole blocks, `stride`
/// samples per row, of which the first `width × height` are the image's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Plane {
    /// The component's samples across.
    pub(super) width: usize,
    /// The component's samples down.
    pub(super) height: usize,
    /// Samples per row of `samples`.
    pub(super) stride: usize,
    /// The samples, row by row.
    pub(super) samples: Vec<u8>,
}

impl Plane {
    /// Row `y`.
    pub(super) fn row(&self, y: usize) -> &[u8] {
        &self.samples[y * self.stride..y * self.stride + self.width]
    }
}

/// The plane of a decoded component: the inverse DCT of each block
/// covering its samples (libjpeg skips the MCU padding blocks), as
/// `block_at(row, column)` gives them.
pub(super) fn plane(component: &Component, block_at: impl Fn(usize, usize) -> Block) -> Plane {
    let quant = component.quant.unwrap_or([0; 64]);
    let stride = component.width_in_blocks * 8;
    let mut samples = vec![0; stride * component.height_in_blocks * 8];
    for block_row in 0..component.height_in_blocks {
        for block_column in 0..component.width_in_blocks {
            let block = inverse_dct(&block_at(block_row, block_column), &quant);
            for (y, row) in block.as_chunks::<8>().0.iter().enumerate() {
                let start = (block_row * 8 + y) * stride + block_column * 8;
                samples[start..start + 8].copy_from_slice(row);
            }
        }
    }
    Plane {
        width: component.sample_width,
        height: component.sample_height,
        stride,
        samples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dc_only_block_is_flat() {
        let mut block = [0; 64];
        block[0] = 10;
        // 10 · 8 / 8 = 10 above the centre.
        assert_eq!(inverse_dct(&block, &[8; 64]), [138; 64]);
        block[0] = -100;
        assert_eq!(inverse_dct(&block, &[16; 64]), [0; 64], "saturates");
    }

    #[test]
    fn the_idct_matches_the_float_transform() {
        let block: Block =
            std::array::from_fn(|i| ((i * 37 % 23) as i16 - 11) * (64 - i as i16) / 16);
        let quant = [3; 64];
        let samples = inverse_dct(&block, &quant);
        for y in 0..8 {
            for x in 0..8 {
                let mut sum = 0.0;
                for v in 0..8 {
                    for u in 0..8 {
                        let c = |k: usize| if k == 0 { 0.5_f64.sqrt() } else { 1.0 };
                        let pi = std::f64::consts::PI;
                        let coefficient = f64::from(block[v * 8 + u]) * 3.0;
                        sum += c(u)
                            * c(v)
                            * coefficient
                            * ((2.0 * x as f64 + 1.0) * u as f64 * pi / 16.0).cos()
                            * ((2.0 * y as f64 + 1.0) * v as f64 * pi / 16.0).cos();
                    }
                }
                let expected = (sum / 4.0 + 128.0).clamp(0.0, 255.0);
                let got = f64::from(samples[y * 8 + x]);
                assert!(
                    (got - expected).abs() <= 1.0,
                    "({x}, {y}): {got} vs {expected}"
                );
            }
        }
    }

    /// Where the two SIMD forms part: sums beyond what a real encoder
    /// produces.
    #[test]
    fn the_simd_forms_narrow_out_of_range_sums_their_own_way() {
        // In range, both round half up.
        for x in [0, 1023, 1024, -1024, -1025, 5_000_000, -5_000_000] {
            assert_eq!(Simd::X86.first(x), Simd::Neon.first(x), "{x}");
        }
        for x in [0, 131_071, 131_072, -131_072, -131_073, 20_000_000] {
            assert_eq!(Simd::X86.second(x), Simd::Neon.second(x), "{x}");
        }
        // The first pass saturates on x86 and keeps the low half on arm64.
        assert_eq!(Simd::X86.first(1 << 27), i16::MAX);
        assert_eq!(Simd::Neon.first(1 << 27), 0);
        assert_eq!(Simd::Neon.first((1 << 27) + (3 << 11)), 3);
        // Rounding the largest sums wraps on x86 only.
        assert_eq!(Simd::X86.first(i32::MAX), i16::MIN);
        assert_eq!(Simd::Neon.first(i32::MAX), 0);
        assert_eq!(Simd::X86.second(i32::MAX), 0);
        assert_eq!(Simd::Neon.second(i32::MAX), 255);
        // A block of large coefficients decodes differently.
        let block: Block = std::array::from_fn(|i| if i % 9 == 0 { 3000 } else { -700 });
        let quant = [40; 64];
        assert_ne!(
            inverse_dct_as(Simd::X86, &block, &quant),
            inverse_dct_as(Simd::Neon, &block, &quant)
        );
        // One a real encoder produces does not.
        let block: Block = std::array::from_fn(|i| if i < 6 { 20 - 3 * i as i16 } else { 0 });
        assert_eq!(
            inverse_dct_as(Simd::X86, &block, &[8; 64]),
            inverse_dct_as(Simd::Neon, &block, &[8; 64])
        );
    }

    #[test]
    fn extreme_coefficients_do_not_overflow() {
        let block = [i16::MIN; 64];
        let samples = inverse_dct_as(Simd::X86, &block, &[u16::MAX; 64]);
        assert!(samples.iter().all(|&s| s == 0 || s == 255));
        // The Neon form wraps where that one saturates: any sample may
        // come out, but none panics.
        let wrapped = inverse_dct_as(Simd::Neon, &block, &[u16::MAX; 64]);
        assert_eq!(wrapped.len(), 64);
    }
}
