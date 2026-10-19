//! Decoding a VP8 key frame into Y, U and V planes (libwebp's
//! `ParseFrame`, `ParseIntraMode` and `ReconstructRow`): macroblock modes
//! from the first partition, coefficients from the token partitions,
//! prediction from the unfiltered neighbours, then the loop filter over
//! the whole frame.

use super::bool_decoder::BoolDecoder;
use super::filter::{Plane, filter_macroblock};
use super::header::{self, FilterStrength, FrameHeader};
use super::predict::{BlockMode, Edges, STRIDE, predict_block, predict_subblock};
use super::tables::SUBBLOCK_MODE_PROBS;
use super::tokens::{Coefficients, NonZero, read_residuals};
use super::transform::add_inverse_dct;
use crate::webp::DecodeError;

/// Where the luma block starts in its work buffer: one row of top
/// neighbours, and room for the left column.
const WORK_ORIGIN: usize = STRIDE + 8;
/// The luma work buffer: the top row and 16 rows.
const LUMA_WORK: usize = STRIDE * 17;
/// A chroma work buffer: the top row and 8 rows.
const CHROMA_WORK: usize = STRIDE * 9;
/// The value of missing top neighbours.
const ABOVE_FRAME: u8 = 127;
/// The value of missing left neighbours.
const LEFT_OF_FRAME: u8 = 129;

/// A decoded frame: planes padded to whole macroblocks.
#[derive(Debug)]
pub(super) struct Frame {
    pub(super) width: usize,
    pub(super) height: usize,
    /// Y, then U and V at half resolution.
    pub(super) planes: [Plane; 3],
}

/// How a macroblock is predicted.
#[derive(Clone, Copy, Debug)]
enum LumaModes {
    /// One prediction for the 16×16 block.
    Whole(BlockMode),
    /// One subblock mode per 4×4 block, in raster order.
    Subblocks([u8; 16]),
}

/// The header fields of a macroblock, from the first partition.
#[derive(Clone, Copy, Debug)]
struct MacroblockModes {
    segment: usize,
    skip: bool,
    luma: LumaModes,
    chroma: BlockMode,
}

/// Decodes the VP8 key frame in `data`.
pub(super) fn decode(data: &[u8]) -> Result<Frame, DecodeError> {
    let (header, mut modes_reader) = header::parse(data)?;
    let mut decoder = FrameDecoder::new(&header);
    let mut partitions: Vec<BoolDecoder<'_>> = header
        .partitions
        .iter()
        .map(|partition| BoolDecoder::new(partition))
        .collect();
    let mut strengths = Vec::with_capacity(decoder.mb_w * decoder.mb_h);
    for mb_y in 0..decoder.mb_h {
        let row = decoder.read_modes(&mut modes_reader, &header);
        if modes_reader.is_eof() {
            return Err(DecodeError::Truncated);
        }
        let tokens = &mut partitions[mb_y % header.partitions.len()];
        let mut left = NonZero::default();
        for (mb_x, modes) in row.iter().enumerate() {
            let strength =
                decoder.decode_macroblock(tokens, &header, modes, &mut left, (mb_x, mb_y));
            if tokens.is_eof() {
                return Err(DecodeError::Truncated);
            }
            strengths.push(strength);
        }
    }
    let mut planes = decoder.planes;
    for (index, strength) in strengths.iter().enumerate() {
        let position = (index % decoder.mb_w, index / decoder.mb_w);
        filter_macroblock(&mut planes, header.filter, strength, position);
    }
    Ok(Frame {
        width: header.width,
        height: header.height,
        planes,
    })
}

/// The state of a frame being decoded.
#[derive(Debug)]
struct FrameDecoder {
    mb_w: usize,
    mb_h: usize,
    planes: [Plane; 3],
    /// The subblock modes along the bottom of the previous macroblock row.
    top_modes: Vec<u8>,
    /// The non-zero coefficients along the bottom of that row.
    top_nonzero: Vec<NonZero>,
    coeffs: Box<Coefficients>,
    work: [Vec<u8>; 3],
}

impl FrameDecoder {
    fn new(header: &FrameHeader<'_>) -> Self {
        let mb_w = header.width.div_ceil(16);
        let mb_h = header.height.div_ceil(16);
        let plane = |size: usize| Plane {
            data: vec![0; mb_w * size * mb_h * size],
            stride: mb_w * size,
        };
        Self {
            mb_w,
            mb_h,
            planes: [plane(16), plane(8), plane(8)],
            top_modes: vec![0; 4 * mb_w],
            top_nonzero: vec![NonZero::default(); mb_w],
            coeffs: Box::new([0; 384]),
            work: [
                vec![0; LUMA_WORK],
                vec![0; CHROMA_WORK],
                vec![0; CHROMA_WORK],
            ],
        }
    }

    /// Reads the modes of a macroblock row.
    fn read_modes(
        &mut self,
        reader: &mut BoolDecoder<'_>,
        header: &FrameHeader<'_>,
    ) -> Vec<MacroblockModes> {
        let mut left = [0u8; 4];
        (0..self.mb_w)
            .map(|mb_x| {
                let top = &mut self.top_modes[4 * mb_x..4 * mb_x + 4];
                read_macroblock_modes(reader, header, top, &mut left)
            })
            .collect()
    }

    /// Decodes and reconstructs a macroblock, returning its loop filter.
    fn decode_macroblock(
        &mut self,
        tokens: &mut BoolDecoder<'_>,
        header: &FrameHeader<'_>,
        modes: &MacroblockModes,
        left: &mut NonZero,
        position: (usize, usize),
    ) -> FilterStrength {
        let intra16 = matches!(modes.luma, LumaModes::Whole(_));
        let top = &mut self.top_nonzero[position.0];
        let empty = if modes.skip {
            top.skip(intra16);
            left.skip(intra16);
            self.coeffs.fill(0);
            true
        } else {
            let quant = &header.quantizers[modes.segment];
            read_residuals(
                tokens,
                &header.coeff_probs,
                quant,
                intra16,
                top,
                left,
                &mut self.coeffs,
            )
            .is_empty()
        };
        self.reconstruct(modes, position);
        let mut strength = header.strengths[modes.segment][usize::from(!intra16)];
        strength.inner |= !empty;
        strength
    }

    /// Predicts the macroblock at `(mb_x, mb_y)` from its unfiltered
    /// neighbours, adds its residuals and stores it in the planes.
    fn reconstruct(&mut self, modes: &MacroblockModes, (mb_x, mb_y): (usize, usize)) {
        let edges = Edges {
            top: mb_y > 0,
            left: mb_x > 0,
        };
        let last_column = mb_x + 1 == self.mb_w;
        let [luma_work, u_work, v_work] = &mut self.work;
        load_borders(luma_work, &self.planes[0], 16, (mb_x, mb_y), last_column);
        match modes.luma {
            LumaModes::Whole(mode) => {
                predict_block(luma_work, WORK_ORIGIN, 16, mode, edges);
                for n in 0..16 {
                    add_residual(luma_work, &self.coeffs[n * 16..][..16], n % 4, n / 4);
                }
            }
            LumaModes::Subblocks(subblock_modes) => {
                // The blocks of the right column take the macroblock's
                // top-right pixels as theirs.
                for row in [3, 7, 11] {
                    let above_right = WORK_ORIGIN - STRIDE + 16;
                    luma_work.copy_within(
                        above_right..above_right + 4,
                        WORK_ORIGIN + row * STRIDE + 16,
                    );
                }
                for (n, &mode) in subblock_modes.iter().enumerate() {
                    let at = WORK_ORIGIN + (n / 4) * 4 * STRIDE + (n % 4) * 4;
                    predict_subblock(luma_work, at, mode);
                    add_residual(luma_work, &self.coeffs[n * 16..][..16], n % 4, n / 4);
                }
            }
        }
        for (plane, work, first_block) in [(1, &mut *u_work, 16), (2, &mut *v_work, 20)] {
            load_borders(work, &self.planes[plane], 8, (mb_x, mb_y), last_column);
            predict_block(work, WORK_ORIGIN, 8, modes.chroma, edges);
            for n in 0..4 {
                let coeffs = &self.coeffs[(first_block + n) * 16..][..16];
                add_residual(work, coeffs, n % 2, n / 2);
            }
        }
        for ((plane, work), size) in self.planes.iter_mut().zip(&self.work).zip([16, 8, 8]) {
            let origin = mb_y * size * plane.stride + mb_x * size;
            for row in 0..size {
                let start = WORK_ORIGIN + row * STRIDE;
                plane.data[origin + row * plane.stride..][..size]
                    .copy_from_slice(&work[start..start + size]);
            }
        }
    }
}

/// Adds the residual of 4×4 block `(x, y)` of a work buffer.
fn add_residual(work: &mut [u8], coeffs: &[i16], x: usize, y: usize) {
    add_inverse_dct(coeffs, work, WORK_ORIGIN + y * 4 * STRIDE + x * 4, STRIDE);
}

/// Fills the neighbours of the `size`-pixel block at macroblock
/// `(mb_x, mb_y)` in its work buffer from the unfiltered plane: the row
/// above (for luma with four pixels above-right) and the column to the
/// left, with libwebp's values outside the frame.
fn load_borders(
    work: &mut [u8],
    plane: &Plane,
    size: usize,
    (mb_x, mb_y): (usize, usize),
    last_column: bool,
) {
    let (x0, y0) = (mb_x * size, mb_y * size);
    let pixel = |x: usize, y: usize| plane.data[y * plane.stride + x];
    for row in 0..size {
        work[WORK_ORIGIN + row * STRIDE - 1] = if mb_x == 0 {
            LEFT_OF_FRAME
        } else {
            pixel(x0 - 1, y0 + row)
        };
    }
    let above = WORK_ORIGIN - STRIDE;
    work[above - 1] = match (mb_y, mb_x) {
        (0, _) => ABOVE_FRAME,
        (_, 0) => LEFT_OF_FRAME,
        _ => pixel(x0 - 1, y0 - 1),
    };
    let width = if size == 16 { 20 } else { size };
    for col in 0..width {
        work[above + col] = if mb_y == 0 {
            ABOVE_FRAME
        } else if col < size || !last_column {
            pixel(x0 + col, y0 - 1)
        } else {
            pixel(x0 + size - 1, y0 - 1)
        };
    }
}

/// Reads the segment, skip flag and prediction modes of a macroblock,
/// updating the subblock modes along its top and left edges.
fn read_macroblock_modes(
    reader: &mut BoolDecoder<'_>,
    header: &FrameHeader<'_>,
    top: &mut [u8],
    left: &mut [u8; 4],
) -> MacroblockModes {
    let probs = header.segment_probs;
    let segment = if !header.segment_map {
        0
    } else if reader.bit(probs[0]) {
        2 + usize::from(reader.bit(probs[2]))
    } else {
        usize::from(reader.bit(probs[1]))
    };
    let skip = header.skip_prob.is_some_and(|prob| reader.bit(prob));
    let luma = if reader.bit(145) {
        let mode = if reader.bit(156) {
            if reader.bit(128) { 1 } else { 3 }
        } else if reader.bit(163) {
            2
        } else {
            0
        };
        top.fill(mode);
        left.fill(mode);
        LumaModes::Whole(BlockMode::from_index(mode))
    } else {
        let mut modes = [0u8; 16];
        for y in 0..4 {
            let mut mode = left[y];
            for x in 0..4 {
                mode = read_subblock_mode(
                    reader,
                    &SUBBLOCK_MODE_PROBS[usize::from(top[x])][usize::from(mode)],
                );
                top[x] = mode;
            }
            modes[4 * y..4 * y + 4].copy_from_slice(top);
            left[y] = mode;
        }
        LumaModes::Subblocks(modes)
    };
    let chroma = if !reader.bit(142) {
        BlockMode::Dc
    } else if !reader.bit(114) {
        BlockMode::Vertical
    } else if reader.bit(183) {
        BlockMode::TrueMotion
    } else {
        BlockMode::Horizontal
    };
    MacroblockModes {
        segment,
        skip,
        luma,
        chroma,
    }
}

/// Reads a subblock mode through its tree.
fn read_subblock_mode(reader: &mut BoolDecoder<'_>, p: &[u8; 9]) -> u8 {
    if !reader.bit(p[0]) {
        0
    } else if !reader.bit(p[1]) {
        1
    } else if !reader.bit(p[2]) {
        2
    } else if !reader.bit(p[3]) {
        if !reader.bit(p[4]) {
            3
        } else if !reader.bit(p[5]) {
            4
        } else {
            5
        }
    } else if !reader.bit(p[6]) {
        6
    } else if !reader.bit(p[7]) {
        7
    } else if !reader.bit(p[8]) {
        8
    } else {
        9
    }
}
