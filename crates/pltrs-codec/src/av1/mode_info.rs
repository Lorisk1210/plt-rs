//! Writing the mode info of an intra block (`intra_frame_mode_info`): its
//! luma and chroma predictions with their angle deltas, and its palettes
//! (`palette_mode_info` and the colour index maps of `palette_tokens`).

use super::context::Edge;
use super::entropy::SymbolSink;
use super::palette::{self, Colors, Palette, PaletteContext};
use super::syntax::{Mode, Prediction, Square};
use super::tile::TileCoder;

impl TileCoder<'_> {
    /// Writes the angle delta of a directional prediction.
    fn write_angle_delta<S: SymbolSink>(&mut self, sink: &mut S, prediction: Prediction) {
        if prediction.mode.angle().is_some() {
            let cdf = &mut self.cdfs.angle_delta[prediction.mode.index() - 1];
            sink.symbol(cdf, (prediction.delta + Prediction::MAX_DELTA) as usize);
        }
    }

    /// Writes `intra_frame_y_mode` and its angle delta.
    pub(crate) fn write_y_mode<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        (r, c): (usize, usize),
        prediction: Prediction,
    ) {
        let above = if self.above_available(r) {
            self.contexts.above.blocks[c].mode.context()
        } else {
            0
        };
        let left = if self.left_available(c) {
            self.contexts.left.blocks[r].mode.context()
        } else {
            0
        };
        sink.symbol(&mut self.cdfs.y_mode[above][left], prediction.mode.index());
        self.write_angle_delta(sink, prediction);
    }

    /// Writes `uv_mode` and its angle delta, for a block of `size` with
    /// luma mode `y_mode`.
    pub(crate) fn write_uv_mode<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        size: Square,
        y_mode: Mode,
        prediction: Prediction,
    ) {
        let cfl_allowed = if self.quantizer.lossless() {
            size.subsampled(self.frame.ss.0) == Square::S4
        } else {
            size.samples() <= 32
        };
        let symbol = prediction.mode.index();
        if cfl_allowed {
            sink.symbol(&mut self.cdfs.uv_mode_cfl[y_mode.index()], symbol);
        } else {
            sink.symbol(&mut self.cdfs.uv_mode[y_mode.index()], symbol);
        }
        self.write_angle_delta(sink, prediction);
    }

    /// The palettes of `list` (0 luma, 1 U) above (within the superblock
    /// row) and to the left of the block at `(r, c)`, for its cache.
    fn palette_neighbours(&self, (r, c): (usize, usize), list: usize) -> (Colors, Colors) {
        let above = if r % Square::S64.units() == 0 {
            Colors::default()
        } else {
            self.contexts.above.blocks[c].palettes[list]
        };
        let left = if self.left_available(c) {
            self.contexts.left.blocks[r].palettes[list]
        } else {
            Colors::default()
        };
        (above, left)
    }

    /// Writes the luma part of `palette_mode_info` of a block of `size`
    /// at `(r, c)` predicted in DC mode.
    pub(crate) fn write_luma_palette<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        ((r, c), size): ((usize, usize), Square),
        palette: Option<&Palette>,
    ) {
        let (above, left) = self.palette_neighbours((r, c), 0);
        let context = PaletteContext {
            size_ctx: 2 * size.units_log2() as usize - 2,
            neighbours: (above.as_slice(), left.as_slice()),
        };
        let has = |edge: &Edge, i: usize| edge.blocks[i].palettes[0].is_some();
        let ctx = usize::from(self.above_available(r) && has(&self.contexts.above, c))
            + usize::from(self.left_available(c) && has(&self.contexts.left, r));
        palette::write_luma(sink, &mut self.cdfs.palette, (&context, ctx), palette);
    }

    /// Writes the chroma part of `palette_mode_info` of a block of `size`
    /// at `(r, c)` whose chroma is predicted in DC mode.
    pub(crate) fn write_chroma_palette<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        ((r, c), size): ((usize, usize), Square),
        (has_luma, palette): (bool, Option<&Palette>),
    ) {
        let (above, left) = self.palette_neighbours((r, c), 1);
        let context = PaletteContext {
            size_ctx: 2 * size.units_log2() as usize - 2,
            neighbours: (above.as_slice(), left.as_slice()),
        };
        palette::write_chroma(sink, &mut self.cdfs.palette, (&context, has_luma), palette);
    }

    /// Writes the colour index map of a luma or chroma palette.
    pub(crate) fn write_palette_map<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        luma: bool,
        palette: &Palette,
    ) {
        let cdfs = if luma {
            &mut self.cdfs.palette.y_color
        } else {
            &mut self.cdfs.palette.uv_color
        };
        palette::write_map(sink, cdfs, palette);
    }
}
