//! The VP8 key-frame header (libwebp's `vp8_dec.c`, `quant_dec.c` and
//! `tree_dec.c`; RFC 6386 sections 9 and 19.2): size, segments, loop
//! filter, partitions, quantisers and token probabilities.

use super::bool_decoder::BoolDecoder;
use super::tables::{AC_QUANT, COEFF_PROBS, COEFF_UPDATE_PROBS, DC_QUANT};
use crate::webp::DecodeError;

/// The segments a frame may divide its macroblocks into.
pub(super) const SEGMENTS: usize = 4;
/// The bytes of the frame tag and the key-frame start code and size.
const FRAME_HEADER_SIZE: usize = 10;
/// The key-frame start code.
const START_CODE: [u8; 3] = [0x9D, 0x01, 0x2A];

/// The token probabilities, by block type, band, context and tree node.
pub(super) type CoeffProbs = [[[[u8; 11]; 3]; 8]; 4];

/// The dequantisation factors of one segment: DC and AC, for luma, the
/// luma DC transform and chroma.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Quantizers {
    pub(super) y1: [i32; 2],
    pub(super) y2: [i32; 2],
    pub(super) uv: [i32; 2],
}

/// The loop filter of a segment and kind of macroblock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct FilterStrength {
    /// The edge limit; 0 turns filtering off.
    pub(super) limit: i32,
    /// The interior limit.
    pub(super) interior: i32,
    /// The high-edge-variance threshold.
    pub(super) hev_threshold: i32,
    /// Whether the edges inside the macroblock are filtered.
    pub(super) inner: bool,
}

/// Which loop filter a frame uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FilterKind {
    /// No filtering (level 0).
    None,
    /// The simple filter, on luma only.
    Simple,
    /// The normal filter.
    Normal,
}

/// A parsed key-frame header.
#[derive(Debug)]
pub(super) struct FrameHeader<'a> {
    /// The picture size in pixels.
    pub(super) width: usize,
    pub(super) height: usize,
    /// Whether macroblocks carry a segment.
    pub(super) segment_map: bool,
    /// The probabilities of the segment tree.
    pub(super) segment_probs: [u8; 3],
    /// The dequantisation factors of each segment.
    pub(super) quantizers: [Quantizers; SEGMENTS],
    /// The loop filter.
    pub(super) filter: FilterKind,
    /// The filter strength of each segment, for 16×16 and 4×4 prediction.
    pub(super) strengths: [[FilterStrength; 2]; SEGMENTS],
    /// The token probabilities.
    pub(super) coeff_probs: Box<CoeffProbs>,
    /// The probability that a macroblock has no coefficients, if signalled.
    pub(super) skip_prob: Option<u8>,
    /// The token partitions; macroblock row `y` uses partition
    /// `y % partitions.len()`.
    pub(super) partitions: Vec<&'a [u8]>,
}

/// The width and height a VP8 bitstream declares, checked as libwebp's
/// `VP8GetInfo` does.
pub(crate) fn read_info(data: &[u8]) -> Result<(u32, u32), DecodeError> {
    if data.len() < FRAME_HEADER_SIZE {
        return Err(DecodeError::Truncated);
    }
    let tag = u32::from(data[0]) | (u32::from(data[1]) << 8) | (u32::from(data[2]) << 16);
    let key_frame = tag & 1 == 0;
    let profile = (tag >> 1) & 7;
    let shown = (tag >> 4) & 1 == 1;
    let first_partition = (tag >> 5) as usize;
    let width = u32::from(u16::from_le_bytes([data[6], data[7]]) & 0x3FFF);
    let height = u32::from(u16::from_le_bytes([data[8], data[9]]) & 0x3FFF);
    let valid = data[3..6] == START_CODE
        && key_frame
        && profile <= 3
        && shown
        && first_partition < data.len()
        && width > 0
        && height > 0;
    if valid {
        Ok((width, height))
    } else {
        Err(DecodeError::LossyHeader)
    }
}

/// Parses the header of a key frame, returning it and the decoder of the
/// first partition, positioned at the macroblock modes.
pub(super) fn parse(data: &[u8]) -> Result<(FrameHeader<'_>, BoolDecoder<'_>), DecodeError> {
    let (width, height) = read_info(data)?;
    let first_size = (u32::from_le_bytes([data[0], data[1], data[2], 0]) >> 5) as usize;
    let rest = &data[FRAME_HEADER_SIZE..];
    if first_size > rest.len() {
        return Err(DecodeError::Truncated);
    }
    let (first, tokens) = rest.split_at(first_size);
    let mut reader = BoolDecoder::new(first);
    // The colour space and clamping type change nothing in decoding.
    reader.literal(2);
    let segments = SegmentHeader::read(&mut reader);
    let filter = FilterHeader::read(&mut reader);
    if reader.is_eof() {
        return Err(DecodeError::LossyHeader);
    }
    let partitions = split_partitions(&mut reader, tokens)?;
    let quantizers = read_quantizers(&mut reader, &segments);
    // The flag that would keep the updated probabilities for later frames.
    reader.flag();
    let coeff_probs = read_coeff_probs(&mut reader);
    let skip_prob = reader.flag().then(|| reader.literal(8) as u8);
    let header = FrameHeader {
        width: width as usize,
        height: height as usize,
        segment_map: segments.update_map,
        segment_probs: segments.tree_probs,
        quantizers,
        filter: filter.kind(),
        strengths: filter.strengths(&segments),
        coeff_probs,
        skip_prob,
        partitions,
    };
    Ok((header, reader))
}

/// The segment header: whether segments are used and their adjustments.
#[derive(Debug)]
struct SegmentHeader {
    enabled: bool,
    update_map: bool,
    absolute: bool,
    quantizer: [i32; SEGMENTS],
    filter_level: [i32; SEGMENTS],
    tree_probs: [u8; 3],
}

impl SegmentHeader {
    fn read(reader: &mut BoolDecoder<'_>) -> Self {
        let mut header = Self {
            enabled: reader.flag(),
            update_map: false,
            absolute: true,
            quantizer: [0; SEGMENTS],
            filter_level: [0; SEGMENTS],
            tree_probs: [255; 3],
        };
        if !header.enabled {
            return header;
        }
        header.update_map = reader.flag();
        if reader.flag() {
            header.absolute = reader.flag();
            header.quantizer = std::array::from_fn(|_| reader.optional_signed(7));
            header.filter_level = std::array::from_fn(|_| reader.optional_signed(6));
        }
        if header.update_map {
            header.tree_probs = std::array::from_fn(|_| {
                if reader.flag() {
                    reader.literal(8) as u8
                } else {
                    255
                }
            });
        }
        header
    }
}

/// The loop-filter header.
#[derive(Debug)]
struct FilterHeader {
    simple: bool,
    level: i32,
    sharpness: i32,
    /// The adjustments of the intra reference frame and of 4×4 prediction,
    /// when deltas are on.
    deltas: Option<(i32, i32)>,
}

impl FilterHeader {
    fn read(reader: &mut BoolDecoder<'_>) -> Self {
        let simple = reader.flag();
        let level = reader.literal(6) as i32;
        let sharpness = reader.literal(3) as i32;
        let deltas = reader.flag().then(|| {
            let mut reference = [0; 4];
            let mut mode = [0; 4];
            if reader.flag() {
                for delta in reference.iter_mut().chain(&mut mode) {
                    if reader.flag() {
                        *delta = reader.signed_literal(6);
                    }
                }
            }
            (reference[0], mode[0])
        });
        Self {
            simple,
            level,
            sharpness,
            deltas,
        }
    }

    fn kind(&self) -> FilterKind {
        match (self.level, self.simple) {
            (0, _) => FilterKind::None,
            (_, true) => FilterKind::Simple,
            (_, false) => FilterKind::Normal,
        }
    }

    /// The strengths of each segment (libwebp's
    /// `PrecomputeFilterStrengths`).
    fn strengths(&self, segments: &SegmentHeader) -> [[FilterStrength; 2]; SEGMENTS] {
        std::array::from_fn(|segment| {
            let base = if segments.enabled {
                segments.filter_level[segment] + if segments.absolute { 0 } else { self.level }
            } else {
                self.level
            };
            std::array::from_fn(|intra4| {
                let mut level = base;
                if let Some((reference, mode)) = self.deltas {
                    level += reference + if intra4 == 1 { mode } else { 0 };
                }
                self.strength(level.clamp(0, 63), intra4 == 1)
            })
        })
    }

    fn strength(&self, level: i32, inner: bool) -> FilterStrength {
        if level == 0 {
            return FilterStrength {
                inner,
                ..FilterStrength::default()
            };
        }
        let mut interior = level;
        if self.sharpness > 0 {
            interior >>= if self.sharpness > 4 { 2 } else { 1 };
            interior = interior.min(9 - self.sharpness);
        }
        let interior = interior.max(1);
        FilterStrength {
            limit: 2 * level + interior,
            interior,
            hev_threshold: match level {
                40.. => 2,
                15.. => 1,
                _ => 0,
            },
            inner,
        }
    }
}

/// Splits the token partitions after the first partition: their count,
/// then the sizes of all but the last in three bytes each. Sizes past the
/// data are cut short; the last partition must not be empty.
fn split_partitions<'a>(
    reader: &mut BoolDecoder<'_>,
    data: &'a [u8],
) -> Result<Vec<&'a [u8]>, DecodeError> {
    let count = 1usize << reader.literal(2);
    let table_size = 3 * (count - 1);
    if data.len() < table_size {
        return Err(DecodeError::InvalidPartitions);
    }
    let (sizes, mut rest) = data.split_at(table_size);
    let mut partitions = Vec::with_capacity(count);
    for size in sizes.as_chunks::<3>().0 {
        let size = u32::from_le_bytes([size[0], size[1], size[2], 0]) as usize;
        let (partition, after) = rest.split_at(size.min(rest.len()));
        partitions.push(partition);
        rest = after;
    }
    if rest.is_empty() {
        return Err(DecodeError::InvalidPartitions);
    }
    partitions.push(rest);
    Ok(partitions)
}

/// Reads the base quantiser and its deltas, and the factors of each
/// segment (libwebp's `VP8ParseQuant`).
fn read_quantizers(
    reader: &mut BoolDecoder<'_>,
    segments: &SegmentHeader,
) -> [Quantizers; SEGMENTS] {
    let base = reader.literal(7) as i32;
    let [y1_dc, y2_dc, y2_ac, uv_dc, uv_ac] = std::array::from_fn(|_| reader.optional_signed(4));
    let dc = |q: i32, max: i32| i32::from(DC_QUANT[q.clamp(0, max) as usize]);
    let ac = |q: i32| i32::from(AC_QUANT[q.clamp(0, 127) as usize]);
    let factors = |q: i32| Quantizers {
        y1: [dc(q + y1_dc, 127), ac(q)],
        // `x · 155 / 100`, as libwebp computes it, at least 8.
        y2: [
            dc(q + y2_dc, 127) * 2,
            ((ac(q + y2_ac) * 101_581) >> 16).max(8),
        ],
        uv: [dc(q + uv_dc, 117), ac(q + uv_ac)],
    };
    if segments.enabled {
        std::array::from_fn(|segment| {
            let q = segments.quantizer[segment] + if segments.absolute { 0 } else { base };
            factors(q)
        })
    } else {
        [factors(base); SEGMENTS]
    }
}

/// Reads the updates of the token probabilities over their defaults.
fn read_coeff_probs(reader: &mut BoolDecoder<'_>) -> Box<CoeffProbs> {
    let mut probs = Box::new(COEFF_PROBS);
    for (plane, updates) in probs.iter_mut().zip(&COEFF_UPDATE_PROBS) {
        for (band, updates) in plane.iter_mut().zip(updates) {
            for (context, updates) in band.iter_mut().zip(updates) {
                for (prob, &update) in context.iter_mut().zip(updates) {
                    if reader.bit(update) {
                        *prob = reader.literal(8) as u8;
                    }
                }
            }
        }
    }
    probs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame tag for a shown key frame with a first partition of `size`
    /// bytes, the start code and a `width × height` size.
    fn tag(size: u32, width: u16, height: u16) -> Vec<u8> {
        let bits = (size << 5) | (1 << 4);
        let mut data = bits.to_le_bytes()[..3].to_vec();
        data.extend(START_CODE);
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
        data
    }

    #[test]
    fn frame_tags_are_checked_like_libwebp() {
        let mut data = tag(1, 3, 2);
        data.push(0);
        assert_eq!(read_info(&data), Ok((3, 2)));
        assert_eq!(read_info(&data[..9]), Err(DecodeError::Truncated));
        let mut inter = data.clone();
        inter[0] |= 1;
        assert_eq!(read_info(&inter), Err(DecodeError::LossyHeader));
        let mut empty = tag(1, 0, 2);
        empty.push(0);
        assert_eq!(read_info(&empty), Err(DecodeError::LossyHeader));
        let too_long = tag(64, 3, 2);
        assert_eq!(read_info(&too_long), Err(DecodeError::LossyHeader));
    }

    #[test]
    fn filter_strengths_follow_sharpness() {
        let header = FilterHeader {
            simple: false,
            level: 20,
            sharpness: 5,
            deltas: Some((2, -30)),
        };
        let segments = SegmentHeader {
            enabled: false,
            update_map: false,
            absolute: true,
            quantizer: [0; SEGMENTS],
            filter_level: [0; SEGMENTS],
            tree_probs: [255; 3],
        };
        let [intra16, intra4] = header.strengths(&segments)[0];
        // Level 22, interior 22 >> 2 = 5, capped at 9 - 5 = 4.
        assert_eq!(
            intra16,
            FilterStrength {
                limit: 48,
                interior: 4,
                hev_threshold: 1,
                inner: false
            }
        );
        // Level 22 - 30 clamps to 0: no filtering.
        assert_eq!(intra4.limit, 0);
        assert!(intra4.inner);
    }
}
