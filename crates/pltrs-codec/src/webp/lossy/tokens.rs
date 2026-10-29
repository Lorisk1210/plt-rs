//! The residual coefficients of a macroblock (libwebp's `GetCoeffs` and
//! `ParseResiduals`; RFC 6386 section 13): tokens decoded with the
//! probabilities of each block type, band and context, dequantised into
//! zigzag positions.

use super::bool_decoder::BoolDecoder;
use super::header::{CoeffProbs, Quantizers};
use super::transform::inverse_wht;

/// The coefficients of a macroblock: 16 luma blocks, then 4 U and 4 V
/// blocks, 16 coefficients each in raster order.
pub(super) type Coefficients = [i16; 384];

/// The band of each coefficient position (and of the position after the
/// last, for its context).
const BANDS: [usize; 17] = [0, 1, 2, 3, 6, 4, 5, 6, 6, 6, 6, 6, 6, 6, 6, 7, 0];
/// The raster position of each coefficient in zigzag order.
const ZIGZAG: [usize; 16] = [0, 1, 4, 8, 5, 2, 3, 6, 9, 12, 13, 10, 7, 11, 14, 15];
/// The probabilities of the extra bits of the large-value categories.
const CATEGORY_PROBS: [&[u8]; 4] = [
    &[173, 148, 140],
    &[176, 155, 140, 135],
    &[180, 157, 141, 134, 130],
    &[254, 254, 243, 230, 196, 177, 153, 140, 133, 130, 129],
];

/// The block types indexing the token probabilities.
#[derive(Clone, Copy, Debug)]
enum BlockType {
    /// Luma AC coefficients after a separate DC transform.
    LumaAc = 0,
    /// The luma DC transform.
    LumaDc = 1,
    /// Chroma.
    Chroma = 2,
    /// Luma with its DC coefficient.
    Luma = 3,
}

/// Whether the blocks bordering a macroblock had non-zero coefficients:
/// on its bottom edge (for the macroblock below) or right edge (for the
/// one to its right).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct NonZero {
    /// Bits 0–3: the four luma blocks; 4–5: U; 6–7: V.
    blocks: u32,
    /// The luma DC transform.
    dc: bool,
}

impl NonZero {
    /// Clears the context of a macroblock without coefficients; the DC
    /// context belongs to 16×16-predicted macroblocks only.
    pub(super) fn skip(&mut self, intra16: bool) {
        self.blocks = 0;
        if intra16 {
            self.dc = false;
        }
    }
}

/// Which coefficients of a macroblock are non-zero.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Residuals {
    /// Two bits per luma block, the first block highest: 3 for more than
    /// three coefficients, 2 for two or three, 1 for a DC coefficient only,
    /// 0 for none.
    pub(super) luma: u32,
    /// The same for U (bits 0–7) and V (bits 8–15).
    pub(super) chroma: u32,
}

impl Residuals {
    /// Whether the macroblock has no coefficient at all.
    pub(super) fn is_empty(self) -> bool {
        self.luma == 0 && self.chroma == 0
    }
}

/// Decodes a large value: categories of growing ranges with extra bits.
fn large_value(reader: &mut BoolDecoder<'_>, p: &[u8; 11]) -> i32 {
    if !reader.bit(p[3]) {
        if reader.bit(p[4]) {
            3 + i32::from(reader.bit(p[5]))
        } else {
            2
        }
    } else if !reader.bit(p[6]) {
        if reader.bit(p[7]) {
            7 + 2 * i32::from(reader.bit(165)) + i32::from(reader.bit(145))
        } else {
            5 + i32::from(reader.bit(159))
        }
    } else {
        let high = usize::from(reader.bit(p[8]));
        let low = usize::from(reader.bit(p[9 + high]));
        let category = 2 * high + low;
        let extra = CATEGORY_PROBS[category]
            .iter()
            .fold(0, |v, &prob| 2 * v + i32::from(reader.bit(prob)));
        extra + 3 + (8 << category)
    }
}

/// Decodes the tokens of one block from position `first`, writing the
/// dequantised coefficients to `out`; returns the position after the last
/// non-zero coefficient (0 for none).
fn block_coefficients(
    reader: &mut BoolDecoder<'_>,
    probs: &[[[u8; 11]; 3]; 8],
    context: usize,
    dequant: [i32; 2],
    first: usize,
    out: &mut [i16],
) -> usize {
    let mut p = &probs[BANDS[first]][context];
    let mut n = first;
    while n < 16 {
        if !reader.bit(p[0]) {
            return n;
        }
        while !reader.bit(p[1]) {
            n += 1;
            if n == 16 {
                return 16;
            }
            p = &probs[BANDS[n]][0];
        }
        // The next position's probabilities, by this token's size.
        let next = &probs[BANDS[n + 1]];
        let v = if reader.bit(p[2]) {
            let v = large_value(reader, p);
            p = &next[2];
            v
        } else {
            p = &next[1];
            1
        };
        let factor = dequant[usize::from(n > 0)];
        out[ZIGZAG[n]] = (reader.signed(v) * factor) as i16;
        n += 1;
    }
    16
}

/// The two-bit code of a block with `count` coefficients whose DC is
/// `dc_nonzero`.
fn block_code(count: usize, dc_nonzero: bool) -> u32 {
    match count {
        4.. => 3,
        2..=3 => 2,
        _ => u32::from(dc_nonzero),
    }
}

/// Decodes the coefficients of a macroblock into `coeffs`, updating the
/// non-zero contexts of the macroblock above (`top`) and to the left.
pub(super) fn read_residuals(
    reader: &mut BoolDecoder<'_>,
    probs: &CoeffProbs,
    quant: &Quantizers,
    intra16: bool,
    top: &mut NonZero,
    left: &mut NonZero,
    coeffs: &mut Coefficients,
) -> Residuals {
    coeffs.fill(0);
    let (first, luma_type) = if intra16 {
        let mut dc = [0i16; 16];
        let context = usize::from(top.dc) + usize::from(left.dc);
        let probs_dc = &probs[BlockType::LumaDc as usize];
        let count = block_coefficients(reader, probs_dc, context, quant.y2, 0, &mut dc);
        top.dc = count > 0;
        left.dc = top.dc;
        if count > 1 {
            inverse_wht(&dc, coeffs);
        } else {
            let dc0 = (i32::from(dc[0]) + 3) >> 3;
            coeffs
                .iter_mut()
                .step_by(16)
                .take(16)
                .for_each(|c| *c = dc0 as i16);
        }
        (1, BlockType::LumaAc)
    } else {
        (0, BlockType::Luma)
    };
    let luma_probs = &probs[luma_type as usize];
    let mut top_bits = top.blocks & 0x0F;
    let mut left_bits = left.blocks & 0x0F;
    let mut luma = 0u32;
    for y in 0..4 {
        let mut l = left_bits & 1;
        let mut codes = 0u32;
        for x in 0..4 {
            let block = &mut coeffs[(y * 4 + x) * 16..][..16];
            let context = (l + (top_bits & 1)) as usize;
            let count = block_coefficients(reader, luma_probs, context, quant.y1, first, block);
            l = u32::from(count > first);
            top_bits = (top_bits >> 1) | (l << 7);
            codes = (codes << 2) | block_code(count, block[0] != 0);
        }
        top_bits >>= 4;
        left_bits = (left_bits >> 1) | (l << 7);
        luma = (luma << 8) | codes;
    }
    let mut out_top = top_bits;
    let mut out_left = left_bits >> 4;
    let mut chroma = 0u32;
    let chroma_probs = &probs[BlockType::Chroma as usize];
    for (plane, channel) in [0u32, 2].into_iter().enumerate() {
        let mut codes = 0u32;
        let mut top_bits = top.blocks >> (4 + channel);
        let mut left_bits = left.blocks >> (4 + channel);
        for y in 0..2 {
            let mut l = left_bits & 1;
            for x in 0..2 {
                let index = 16 + plane * 4 + y * 2 + x;
                let block = &mut coeffs[index * 16..][..16];
                let context = (l + (top_bits & 1)) as usize;
                let count = block_coefficients(reader, chroma_probs, context, quant.uv, 0, block);
                l = u32::from(count > 0);
                top_bits = (top_bits >> 1) | (l << 3);
                codes = (codes << 2) | block_code(count, block[0] != 0);
            }
            top_bits >>= 2;
            left_bits = (left_bits >> 1) | (l << 5);
        }
        chroma |= codes << (4 * channel);
        out_top |= (top_bits << 4) << channel;
        out_left |= (left_bits & 0xF0) << channel;
    }
    top.blocks = out_top;
    left.blocks = out_left;
    Residuals { luma, chroma }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_codes_classify_coefficient_counts() {
        assert_eq!(block_code(16, false), 3);
        assert_eq!(block_code(3, false), 2);
        assert_eq!(block_code(1, true), 1);
        assert_eq!(block_code(1, false), 0);
        assert_eq!(block_code(0, false), 0);
    }

    #[test]
    fn skipped_macroblocks_keep_the_dc_context_of_4x4_prediction() {
        let mut context = NonZero {
            blocks: 0xFF,
            dc: true,
        };
        context.skip(false);
        assert_eq!((context.blocks, context.dc), (0, true));
        context.skip(true);
        assert!(!context.dc);
    }
}
