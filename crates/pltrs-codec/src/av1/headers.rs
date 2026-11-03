//! The OBUs around the tile data: the sequence header of a still picture
//! (`reduced_still_picture_header`), the uncompressed header of its key
//! frame, the uniform tile layout, and OBU framing (sections 5.3 to 5.9).

use super::ColorDescription;
use super::bit_writer::{BitWriter, leb128};

/// `OBU_SEQUENCE_HEADER`.
pub(crate) const OBU_SEQUENCE_HEADER: u8 = 1;
/// `OBU_FRAME`: a frame header and its tile group.
pub(crate) const OBU_FRAME: u8 = 6;

/// `MAX_TILE_WIDTH` in 64×64 superblocks.
const MAX_TILE_WIDTH_SB: usize = 4096 >> 6;
/// `MAX_TILE_AREA` in 64×64 superblocks.
const MAX_TILE_AREA_SB: usize = (4096 * 2304) >> 12;
/// `MAX_TILE_COLS` and `MAX_TILE_ROWS`.
const MAX_TILES_ACROSS: usize = 64;
/// The bytes of each tile size field (`tile_size_bytes_minus_1 + 1`).
pub(crate) const TILE_SIZE_BYTES: usize = 4;

/// An OBU of `kind` with its size field.
pub(crate) fn obu(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut out = vec![(kind << 3) | 0b010];
    out.extend(leb128(payload.len()));
    out.extend_from_slice(payload);
    out
}

/// The smallest level whose picture size limits hold `width × height`
/// (Annex A), or 31, the level of unconstrained parameters.
pub(crate) fn level(width: u32, height: u32) -> u8 {
    /// `seq_level_idx`, `MaxPicSize`, `MaxHSize` and `MaxVSize`.
    const LEVELS: [(u8, u64, u32, u32); 7] = [
        (0, 147_456, 2048, 1152),
        (1, 278_784, 2816, 1584),
        (4, 665_856, 4352, 2448),
        (5, 1_065_024, 5504, 3096),
        (8, 2_359_296, 6144, 3456),
        (12, 8_912_896, 8192, 4352),
        (16, 35_651_584, 16384, 8704),
    ];
    let pixels = u64::from(width) * u64::from(height);
    LEVELS
        .iter()
        .find(|&&(_, size, w, h)| pixels <= size && width <= w && height <= h)
        .map_or(31, |&(level, ..)| level)
}

/// The sequence-level parameters of a picture.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sequence {
    /// `seq_profile`: 0 for 4:2:0 and monochrome, 1 for 4:4:4.
    pub(crate) profile: u8,
    /// `seq_level_idx[0]`.
    pub(crate) level: u8,
    /// The picture size.
    pub(crate) size: (u32, u32),
    /// Whether the picture has only luma.
    pub(crate) monochrome: bool,
    /// The colour signalling; `None` leaves it unspecified.
    pub(crate) color: Option<ColorDescription>,
    /// Whether samples use the full range of 0 to 255.
    pub(crate) full_range: bool,
}

/// The number of bits of `value`, at least 1.
fn bit_length(value: u32) -> u32 {
    value.bit_width().max(1)
}

impl Sequence {
    /// The sequence header OBU's payload, with its trailing bits.
    pub(crate) fn header(&self) -> Vec<u8> {
        let mut w = BitWriter::default();
        w.bits(u32::from(self.profile), 3);
        w.bit(true); // still_picture
        w.bit(true); // reduced_still_picture_header
        w.bits(u32::from(self.level), 5);
        let (width, height) = self.size;
        let width_bits = bit_length(width - 1);
        let height_bits = bit_length(height - 1);
        w.bits(width_bits - 1, 4);
        w.bits(height_bits - 1, 4);
        w.bits(width - 1, width_bits);
        w.bits(height - 1, height_bits);
        // use_128x128_superblock, enable_filter_intra, enable_intra_edge_filter,
        // enable_superres, enable_cdef, enable_restoration.
        w.bits(0, 6);
        self.color_config(&mut w);
        w.bit(false); // film_grain_params_present
        w.trailing_bits();
        w.into_bytes()
    }

    /// `color_config()` at 8 bits.
    fn color_config(&self, w: &mut BitWriter) {
        w.bit(false); // high_bitdepth
        if self.profile != 1 {
            w.bit(self.monochrome);
        }
        w.bit(self.color.is_some());
        if let Some(color) = self.color {
            w.bits(u32::from(color.primaries), 8);
            w.bits(u32::from(color.transfer), 8);
            w.bits(u32::from(color.matrix), 8);
        }
        if self.monochrome {
            w.bit(self.full_range);
            return;
        }
        if self.color.is_some_and(ColorDescription::is_srgb_identity) {
            // Full range and 4:4:4 are implied.
        } else {
            w.bit(self.full_range);
            if self.profile == 0 {
                w.bits(0, 2); // chroma_sample_position: unknown
            }
        }
        w.bit(false); // separate_uv_delta_q
    }
}

/// How a frame splits into tiles: the fewest uniform tiles the limits of
/// tile width and area allow (`tile_info` with `uniform_tile_spacing_flag`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TileLayout {
    /// `TileColsLog2`.
    cols_log2: u32,
    /// `TileRowsLog2`.
    rows_log2: u32,
    /// `maxLog2TileCols`.
    max_cols_log2: u32,
    /// `maxLog2TileRows`.
    max_rows_log2: u32,
    /// `MiColStarts`, ending with `MiCols`.
    pub(crate) col_starts: Vec<usize>,
    /// `MiRowStarts`, ending with `MiRows`.
    pub(crate) row_starts: Vec<usize>,
}

/// `tile_log2`: the least `k` with `block << k >= target`.
fn tile_log2(block: usize, target: usize) -> u32 {
    (0..usize::BITS)
        .find(|&k| block << k >= target)
        .unwrap_or(0)
}

/// The starts of `count` units cut into `2^log2` uniform tiles of whole
/// superblocks, in 4×4 units, ending with `units`.
fn starts(sb_count: usize, log2: u32, units: usize) -> Vec<usize> {
    let tile = (sb_count + (1 << log2) - 1) >> log2;
    let mut starts: Vec<usize> = (0..sb_count).step_by(tile).map(|sb| sb << 4).collect();
    starts.push(units);
    starts
}

impl TileLayout {
    /// The layout of a frame of `mi_cols × mi_rows` 4×4 units.
    pub(crate) fn new(mi_cols: usize, mi_rows: usize) -> Self {
        let sb_cols = (mi_cols + 15) >> 4;
        let sb_rows = (mi_rows + 15) >> 4;
        let min_cols_log2 = tile_log2(MAX_TILE_WIDTH_SB, sb_cols);
        let max_cols_log2 = tile_log2(1, sb_cols.min(MAX_TILES_ACROSS));
        let max_rows_log2 = tile_log2(1, sb_rows.min(MAX_TILES_ACROSS));
        let min_tiles_log2 = min_cols_log2.max(tile_log2(MAX_TILE_AREA_SB, sb_rows * sb_cols));
        let cols_log2 = min_cols_log2;
        let rows_log2 = min_tiles_log2.saturating_sub(cols_log2);
        Self {
            cols_log2,
            rows_log2,
            max_cols_log2,
            max_rows_log2,
            col_starts: starts(sb_cols, cols_log2, mi_cols),
            row_starts: starts(sb_rows, rows_log2, mi_rows),
        }
    }

    /// The number of tiles.
    pub(crate) fn count(&self) -> usize {
        (self.col_starts.len() - 1) * (self.row_starts.len() - 1)
    }

    /// `tile_info()`.
    fn write(&self, w: &mut BitWriter) {
        w.bit(true); // uniform_tile_spacing_flag
        if self.cols_log2 < self.max_cols_log2 {
            w.bit(false); // increment_tile_cols_log2
        }
        if self.rows_log2 < self.max_rows_log2 {
            w.bit(false); // increment_tile_rows_log2
        }
        if self.cols_log2 > 0 || self.rows_log2 > 0 {
            w.bits(0, self.rows_log2 + self.cols_log2); // context_update_tile_id
            w.bits(TILE_SIZE_BYTES as u32 - 1, 2);
        }
    }
}

/// `read_delta_q()`: whether a delta is coded, then the delta as `su(1+6)`.
fn delta_q(w: &mut BitWriter, delta: i32) {
    w.bit(delta != 0);
    if delta != 0 {
        w.bits((delta as u32) & 0x7F, 7);
    }
}

/// The frame header of the key frame of a still picture with base
/// quantizer index `qindex`, with chroma planes the chroma deltas
/// `DeltaQUDc` and `DeltaQUAc`, and the `delta_q_res` of superblock
/// deltas when there are any (not in lossless frames); byte-aligned,
/// followed by the tile group header.
pub(crate) fn frame_header(
    qindex: u8,
    chroma_delta: Option<(i32, i32)>,
    delta_q_res: Option<u32>,
    tiles: &TileLayout,
) -> Vec<u8> {
    let mut w = BitWriter::default();
    w.bit(false); // disable_cdf_update
    w.bit(true); // allow_screen_content_tools: palettes
    w.bit(false); // force_integer_mv
    w.bit(false); // render_and_frame_size_different
    w.bit(false); // allow_intrabc
    tiles.write(&mut w);
    w.bits(u32::from(qindex), 8); // base_q_idx
    delta_q(&mut w, 0); // DeltaQYDc
    if let Some((dc, ac)) = chroma_delta {
        delta_q(&mut w, dc);
        delta_q(&mut w, ac);
    }
    w.bit(false); // using_qmatrix
    w.bit(false); // segmentation_enabled
    if qindex > 0 {
        w.bit(delta_q_res.is_some()); // delta_q_present
        if let Some(res) = delta_q_res {
            w.bits(res, 2);
            w.bit(false); // delta_lf_present
        }
        w.bits(0, 12); // loop_filter_level[0] and [1]
        w.bits(0, 3); // loop_filter_sharpness
        w.bit(false); // loop_filter_delta_enabled
        w.bit(false); // tx_mode_select: TX_MODE_LARGEST
    }
    w.bit(true); // reduced_tx_set
    w.byte_align();
    if tiles.count() > 1 {
        w.bit(false); // tile_start_and_end_present_flag
        w.byte_align();
    }
    w.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_picture_size() {
        assert_eq!(level(426, 240), 0);
        assert_eq!(level(640, 480), 4);
        assert_eq!(level(2049, 10), 1);
        assert_eq!(level(1920, 1080), 8);
        assert_eq!(level(4096, 2176), 12);
        assert_eq!(level(8192, 4352), 16);
        assert_eq!(level(16384, 100), 16);
        assert_eq!(level(20000, 10), 31);
    }

    #[test]
    fn a_monochrome_header_as_libaom_writes_it() {
        // The alpha plane libavif writes for a 13×10 image.
        let sequence = Sequence {
            profile: 0,
            level: 0,
            size: (13, 10),
            monochrome: true,
            color: None,
            full_range: true,
        };
        let header = sequence.header();
        // libaom enables the filter intra and intra edge filter tools,
        // which this encoder leaves out: bits 27 and 28.
        assert_eq!(header, [0x18, 0x0c, 0xf2, 0x40, 0x54]);
        assert_eq!(obu(OBU_SEQUENCE_HEADER, &header)[..2], [0x0a, 0x05]);
    }

    #[test]
    fn tiles_split_wide_frames() {
        let small = TileLayout::new(160, 120);
        assert_eq!(small.count(), 1);
        assert_eq!(
            (small.col_starts.as_slice(), small.row_starts.as_slice()),
            (&[0, 160][..], &[0, 120][..])
        );
        // 8192 samples across need two tiles of at most 4096.
        let wide = TileLayout::new(2048, 64);
        assert_eq!(wide.col_starts, [0, 1024, 2048]);
        assert_eq!(wide.count(), 2);
        // More than 4096 × 2304 samples need more tiles too.
        let large = TileLayout::new(1024, 1024);
        assert_eq!(large.count(), 2);
        assert_eq!(large.row_starts, [0, 512, 1024]);
    }

    #[test]
    fn frame_headers_byte_align() {
        let tiles = TileLayout::new(4, 4);
        let lossless = frame_header(0, Some((0, 0)), None, &tiles);
        // 5 flags, uniform tiles, qindex 0, 3 delta flags, qmatrix,
        // segmentation, reduced_tx_set: 20 bits.
        assert_eq!(lossless.len(), 3);
        // The flags 01000 (screen content tools on), uniform tiles, the
        // index 0110_0100, no deltas, no quantizer matrix, segmentation or
        // superblock deltas, the loop filter off, the largest transforms
        // and the reduced transform set.
        let lossy = frame_header(100, None, None, &tiles);
        assert_eq!(lossy, [0x45, 0x90, 0, 0, 0x10]);
        // Superblock deltas: bit 17 set, then the resolution 01 and no
        // loop filter deltas.
        let deltas = frame_header(100, None, Some(1), &tiles);
        assert_eq!(deltas[2], 0b0101_0000);
        // Twice a present chroma delta of -16: 1, then 111_0000.
        let finer = frame_header(100, Some((-16, -16)), None, &tiles);
        assert_eq!(finer[..4], [0x45, 0x91, 0xE1, 0xE0]);
    }
}
