//! The CDFs of a tile: copies of the defaults (`init_non_coeff_cdfs` and
//! `init_coeff_cdfs`) that adapt as the tile is coded. Only the syntax
//! elements the encoder writes are kept.

use super::entropy::SymbolSink;
use super::tables::coeff_cdfs::{
    COEFF_BASE, COEFF_BASE_EOB, COEFF_BR, DC_SIGN, EOB_EXTRA, EOB_PT_16, EOB_PT_64, EOB_PT_256,
    EOB_PT_1024, TXB_SKIP,
};
use super::tables::mode_cdfs::{
    ANGLE_DELTA, DELTA_Q, INTRA_FRAME_Y_MODE, INTRA_TX_TYPE_SET2, PARTITION_W8, PARTITION_W16,
    PARTITION_W32, PARTITION_W64, SKIP, UV_MODE_CFL_ALLOWED, UV_MODE_CFL_NOT_ALLOWED,
};

/// The CDFs of the colour indices of palettes, by palette size and
/// colour context.
#[derive(Clone, Debug)]
pub(crate) struct ColorCdfs {
    two: [[u16; 2]; 5],
    three: [[u16; 3]; 5],
    four: [[u16; 4]; 5],
    five: [[u16; 5]; 5],
    six: [[u16; 6]; 5],
    seven: [[u16; 7]; 5],
    eight: [[u16; 8]; 5],
}

impl ColorCdfs {
    /// Codes the rank `symbol` of a colour index of a palette of `size`
    /// colours, in context `ctx`.
    pub(crate) fn code<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        size: usize,
        ctx: usize,
        symbol: usize,
    ) {
        match size {
            2 => sink.symbol(&mut self.two[ctx], symbol),
            3 => sink.symbol(&mut self.three[ctx], symbol),
            4 => sink.symbol(&mut self.four[ctx], symbol),
            5 => sink.symbol(&mut self.five[ctx], symbol),
            6 => sink.symbol(&mut self.six[ctx], symbol),
            7 => sink.symbol(&mut self.seven[ctx], symbol),
            _ => sink.symbol(&mut self.eight[ctx], symbol),
        }
    }
}

/// The palette CDFs of a tile.
#[derive(Clone, Debug)]
pub(crate) struct PaletteCdfs {
    /// `has_palette_y`, by block size and neighbour context.
    pub(crate) y_mode: [[[u16; 2]; 3]; 7],
    /// `has_palette_uv`, by whether the block has a luma palette.
    pub(crate) uv_mode: [[u16; 2]; 2],
    /// `palette_size_y_minus_2`, by block size.
    pub(crate) y_size: [[u16; 7]; 7],
    /// `palette_size_uv_minus_2`, by block size.
    pub(crate) uv_size: [[u16; 7]; 7],
    /// `palette_color_idx_y`.
    pub(crate) y_color: ColorCdfs,
    /// `palette_color_idx_uv`.
    pub(crate) uv_color: ColorCdfs,
}

impl PaletteCdfs {
    /// The defaults.
    fn new() -> Self {
        use super::tables::palette_cdfs as p;
        Self {
            y_mode: p::Y_MODE,
            uv_mode: p::UV_MODE,
            y_size: p::Y_SIZE,
            uv_size: p::UV_SIZE,
            y_color: ColorCdfs {
                two: p::Y_COLOR_2,
                three: p::Y_COLOR_3,
                four: p::Y_COLOR_4,
                five: p::Y_COLOR_5,
                six: p::Y_COLOR_6,
                seven: p::Y_COLOR_7,
                eight: p::Y_COLOR_8,
            },
            uv_color: ColorCdfs {
                two: p::UV_COLOR_2,
                three: p::UV_COLOR_3,
                four: p::UV_COLOR_4,
                five: p::UV_COLOR_5,
                six: p::UV_COLOR_6,
                seven: p::UV_COLOR_7,
                eight: p::UV_COLOR_8,
            },
        }
    }
}

/// The adaptive CDFs of one tile (the specification's `Tile...Cdf`
/// arrays), indexed as the specification indexes them.
#[derive(Clone, Debug)]
pub(crate) struct Cdfs {
    /// `intra_frame_y_mode`, by above and left mode context.
    pub(crate) y_mode: [[[u16; 13]; 5]; 5],
    /// `uv_mode` where chroma from luma is not allowed, by luma mode.
    pub(crate) uv_mode: [[u16; 13]; 13],
    /// `uv_mode` where chroma from luma is allowed, by luma mode.
    pub(crate) uv_mode_cfl: [[u16; 14]; 13],
    /// `angle_delta_y` and `angle_delta_uv`, by directional mode.
    pub(crate) angle_delta: [[u16; 7]; 8],
    /// `partition` of 8×8 blocks, by context.
    pub(crate) partition_8: [[u16; 4]; 4],
    /// `partition` of 16×16, 32×32 and 64×64 blocks, by context.
    pub(crate) partition: [[[u16; 10]; 4]; 3],
    /// `skip`, by context.
    pub(crate) skip: [[u16; 2]; 3],
    /// `delta_q_abs`.
    pub(crate) delta_q: [u16; 4],
    /// Palette mode.
    pub(crate) palette: PaletteCdfs,
    /// `intra_tx_type` of the reduced set, by square size and luma mode.
    pub(crate) tx_type: [[[u16; 5]; 13]; 3],
    /// `all_zero`, by transform size and context.
    pub(crate) txb_skip: [[[u16; 2]; 13]; 5],
    /// `eob_pt_16`, by plane type and transform class.
    pub(crate) eob_pt_16: [[[u16; 5]; 2]; 2],
    /// `eob_pt_64`.
    pub(crate) eob_pt_64: [[[u16; 7]; 2]; 2],
    /// `eob_pt_256`.
    pub(crate) eob_pt_256: [[[u16; 9]; 2]; 2],
    /// `eob_pt_1024`, by plane type.
    pub(crate) eob_pt_1024: [[u16; 11]; 2],
    /// `eob_extra`, by transform size, plane type and end-of-block class.
    pub(crate) eob_extra: [[[[u16; 2]; 9]; 2]; 5],
    /// `dc_sign`, by plane type and context.
    pub(crate) dc_sign: [[[u16; 2]; 3]; 2],
    /// `coeff_base_eob`, by transform size, plane type and context.
    pub(crate) coeff_base_eob: [[[[u16; 3]; 4]; 2]; 5],
    /// `coeff_base`.
    pub(crate) coeff_base: [[[[u16; 4]; 42]; 2]; 5],
    /// `coeff_br`.
    pub(crate) coeff_br: [[[[u16; 4]; 21]; 2]; 5],
}

impl Cdfs {
    /// The default CDFs of a frame with base quantizer index `qindex`,
    /// whose coefficient CDFs come from one of four quantizer contexts.
    pub(crate) fn new(qindex: u8) -> Self {
        let q = match qindex {
            0..=20 => 0,
            21..=60 => 1,
            61..=120 => 2,
            _ => 3,
        };
        Self {
            y_mode: INTRA_FRAME_Y_MODE,
            uv_mode: UV_MODE_CFL_NOT_ALLOWED,
            uv_mode_cfl: UV_MODE_CFL_ALLOWED,
            angle_delta: ANGLE_DELTA,
            partition_8: PARTITION_W8,
            partition: [PARTITION_W16, PARTITION_W32, PARTITION_W64],
            skip: SKIP,
            delta_q: DELTA_Q,
            palette: PaletteCdfs::new(),
            tx_type: INTRA_TX_TYPE_SET2,
            txb_skip: TXB_SKIP[q],
            eob_pt_16: EOB_PT_16[q],
            eob_pt_64: EOB_PT_64[q],
            eob_pt_256: EOB_PT_256[q],
            eob_pt_1024: EOB_PT_1024[q],
            eob_extra: EOB_EXTRA[q],
            dc_sign: DC_SIGN[q],
            coeff_base_eob: COEFF_BASE_EOB[q],
            coeff_base: COEFF_BASE[q],
            coeff_br: COEFF_BR[q],
        }
    }
}
