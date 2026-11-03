//! Coding one block: predicting, transforming, quantizing and
//! reconstructing each of its transform blocks, then writing its mode
//! info and coefficients (`decode_block`, `intra_frame_mode_info` and
//! `residual`, sections 5.11.5 to 5.11.34).

use super::coeffs::{self, BlockInfo, CodedTx};
use super::context::{ContextSpan, Decoded, Neighbour};
use super::delta_q;
use super::entropy::SymbolSink;
use super::palette::{Colors, Palette};
use super::predict::{Position, predict};
use super::syntax::{Mode, Prediction, Square};
use super::tile::TileCoder;
use super::transform::{TxType, forward_2d, inverse_2d, walsh_hadamard_4x4};

/// The choices of a block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BlockChoice {
    /// The luma prediction.
    pub(crate) y: Prediction,
    /// The chroma prediction.
    pub(crate) uv: Prediction,
    /// The transform type of the luma transform blocks that code one.
    pub(crate) tx_type: TxType,
    /// The luma palette, which replaces DC prediction.
    pub(crate) y_palette: Option<Palette>,
    /// The chroma palette, which replaces DC prediction.
    pub(crate) uv_palette: Option<Palette>,
}

/// How the samples of a plane of a block are predicted.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Predictor<'a> {
    /// From the decoded neighbours.
    Intra(Prediction),
    /// From a palette, its colour list 0 (Y or U) or 1 (V).
    Palette(&'a Palette, usize),
}

impl Predictor<'_> {
    /// The mode coded: DC for palettes.
    fn mode(self) -> Mode {
        match self {
            Self::Intra(prediction) => prediction.mode,
            Self::Palette(..) => Mode::Dc,
        }
    }
}

/// The transform blocks of one plane of a block, coded.
#[derive(Debug)]
pub(crate) struct PlaneCoding {
    /// The transform blocks in coding order.
    pub(crate) txs: Vec<CodedTx>,
    /// The squared error of the reconstruction over the visible samples.
    pub(crate) distortion: u64,
}

/// The reconstructed samples and the state around a block, to roll back
/// to.
#[derive(Debug)]
pub(crate) struct Snapshot {
    /// The samples of each plane's square, with its position and side.
    planes: Vec<(usize, usize, usize, Vec<u8>)>,
    /// The contexts along the block's edges.
    contexts: ContextSpan,
    /// The decoded units of the superblock.
    decoded: Decoded,
}

impl TileCoder<'_> {
    /// The square of plane `plane` that the luma square at `(r, c)`
    /// covers: its position in samples and its size.
    pub(crate) fn plane_square(
        &self,
        plane: usize,
        (r, c): (usize, usize),
        size: Square,
    ) -> (usize, usize, Square) {
        let (ss_x, ss_y) = self.plane_subsampling(plane);
        ((c * 4) >> ss_x, (r * 4) >> ss_y, size.subsampled(ss_x))
    }

    /// The subsampling of `plane`.
    pub(crate) fn plane_subsampling(&self, plane: usize) -> (u32, u32) {
        if plane == 0 { (0, 0) } else { self.frame.ss }
    }

    /// The columns and rows of the square at `(r, c)` in `plane` that lie
    /// within the frame's 4×4 grid (`onscreenWidth`, `onscreenHeight`).
    pub(crate) fn onscreen(
        &self,
        plane: usize,
        (r, c): (usize, usize),
        size: Square,
    ) -> (usize, usize) {
        let (ss_x, ss_y) = self.plane_subsampling(plane);
        let side = size.samples();
        let width = side.min((self.frame.mi.0 - c) * 4);
        let height = side.min((self.frame.mi.1 - r) * 4);
        (width >> ss_x, height >> ss_y)
    }

    /// The state of the square at `(r, c)`, to roll back to.
    pub(crate) fn save(&self, (r, c): (usize, usize), size: Square) -> Snapshot {
        let planes = (0..self.frame.recon.len())
            .map(|plane| {
                let (x, y, square) = self.plane_square(plane, (r, c), size);
                let side = square.samples();
                (x, y, side, self.frame.recon[plane].square(x, y, side))
            })
            .collect();
        Snapshot {
            planes,
            contexts: self.contexts.save(r, c, size.units()),
            decoded: self.decoded.clone(),
        }
    }

    /// Rolls the square of a snapshot back.
    pub(crate) fn restore(&mut self, snapshot: &Snapshot) {
        for (plane, (x, y, side, samples)) in snapshot.planes.iter().enumerate() {
            self.frame.recon[plane].put_square(*x, *y, *side, samples);
        }
        self.contexts.restore(&snapshot.contexts);
        self.decoded.clone_from(&snapshot.decoded);
    }

    /// The transform type of a transform block of `tx` in `plane`, for a
    /// block predicted in `mode` that chose `luma_type`.
    fn tx_type(&self, plane: usize, tx: Square, mode: Mode, luma_type: TxType) -> TxType {
        if self.quantizer.lossless() || tx.log2() > 4 {
            TxType::DctDct
        } else if plane == 0 {
            luma_type
        } else {
            mode.chroma_tx_type()
        }
    }

    /// What coding the transform blocks of `plane` in a block of `size`
    /// depends on.
    pub(crate) fn block_info(&self, plane: usize, size: Square, y_mode: Mode) -> BlockInfo {
        let (ss_x, ss_y) = self.plane_subsampling(plane);
        let tx = self.transform_size(plane, size);
        BlockInfo {
            block_samples: size.subsampled(ss_x).samples(),
            y_mode,
            codes_tx_type: plane == 0 && !self.quantizer.lossless() && tx.log2() <= 4,
            limits: (self.frame.mi.0 >> ss_x, self.frame.mi.1 >> ss_y),
        }
    }

    /// The transform size of `plane` in a block of `size`: 4×4 when
    /// lossless (`TX_MODE_LARGEST` otherwise).
    fn transform_size(&self, plane: usize, size: Square) -> Square {
        if self.quantizer.lossless() {
            Square::S4
        } else {
            size.subsampled(self.plane_subsampling(plane).0)
                .transform(plane > 0)
        }
    }

    /// Where the transform block of `tx` at `(x, y)` in `plane` of the
    /// block at `(r, c)` sits, and which of its edges are decoded; `x0`
    /// and `y0` are the block's own position in the plane.
    pub(crate) fn position(
        &self,
        plane: usize,
        (r, c): (usize, usize),
        (x0, y0): (usize, usize),
        (x, y): (usize, usize),
        tx: Square,
    ) -> Position {
        let (ss_x, ss_y) = self.plane_subsampling(plane);
        let (end_x, end_y) = ((self.frame.mi.0 * 4) >> ss_x, (self.frame.mi.1 * 4) >> ss_y);
        let (ux, uy) = (self.decoded.unit(plane, x), self.decoded.unit(plane, y));
        let step = tx.units() as i32;
        Position {
            x,
            y,
            have_left: self.left_available(c) || x > x0,
            have_above: self.above_available(r) || y > y0,
            have_above_right: self.decoded.get(plane, uy - 1, ux + step),
            have_below_left: self.decoded.get(plane, uy + step, ux - 1),
            max_x: end_x - 1,
            max_y: end_y - 1,
        }
    }

    /// Codes the residual of a transform block of `tx` at `(x, y)` of
    /// `plane` over `predicted`; returns its levels and leaves the
    /// reconstruction in the frame.
    fn code_residual(
        &mut self,
        plane: usize,
        (x, y): (usize, usize),
        (tx, tx_type): (Square, TxType),
        predicted: &[u8],
    ) -> CodedTx {
        let quantizer = self.quantizer;
        let step = tx.samples();
        let source = self.frame.source[plane].square(x, y, step);
        let residual: Vec<i32> = source
            .iter()
            .zip(predicted)
            .map(|(&s, &p)| i32::from(s) - i32::from(p))
            .collect();
        let levels = if quantizer.lossless() {
            walsh_hadamard_4x4(&residual)
                .iter()
                .map(|c| c / 4)
                .collect()
        } else {
            let coefficients = forward_2d(&self.frame.bases, &residual, tx.log2(), tx_type);
            quantizer.quantize(plane, &coefficients, tx)
        };
        let scan = self.frame.scans.get(tx);
        let coded = CodedTx::new(plane, (x / 4, y / 4), (tx, tx_type), levels, scan);
        let reconstruction: Vec<u8> = if coded.eob == 0 {
            predicted.to_vec()
        } else {
            let dequant = quantizer.dequantize(plane, &coded.levels, tx);
            let residual = inverse_2d(&dequant, tx.log2(), tx_type, quantizer.lossless());
            predicted
                .iter()
                .zip(&residual)
                .map(|(&p, &r)| (i32::from(p) + r).clamp(0, 255) as u8)
                .collect()
        };
        self.frame.recon[plane].put_square(x, y, step, &reconstruction);
        coded
    }

    /// Predicts, transforms, quantizes and reconstructs the transform
    /// blocks of `plane` in the block at `(r, c)`, in raster order.
    pub(crate) fn compute_plane(
        &mut self,
        plane: usize,
        (r, c): (usize, usize),
        size: Square,
        (predictor, luma_type): (Predictor<'_>, TxType),
    ) -> PlaneCoding {
        let (ss_x, ss_y) = self.plane_subsampling(plane);
        let (bx, by, block) = self.plane_square(plane, (r, c), size);
        let tx = self.transform_size(plane, size);
        let tx_type = self.tx_type(plane, tx, predictor.mode(), luma_type);
        let (end_x, end_y) = ((self.frame.mi.0 * 4) >> ss_x, (self.frame.mi.1 * 4) >> ss_y);
        let step = tx.samples();
        let mut txs = Vec::new();
        for y in (by..by + block.samples()).step_by(step) {
            for x in (bx..bx + block.samples()).step_by(step) {
                if x >= end_x || y >= end_y {
                    continue;
                }
                let predicted = match predictor {
                    Predictor::Intra(prediction) => {
                        let position = self.position(plane, (r, c), (bx, by), (x, y), tx);
                        predict(&self.frame.recon[plane], position, tx, prediction)
                    }
                    Predictor::Palette(palette, list) => {
                        palette.predict(list, (x - bx, y - by), step)
                    }
                };
                txs.push(self.code_residual(plane, (x, y), (tx, tx_type), &predicted));
                let (ux, uy) = (self.decoded.unit(plane, x), self.decoded.unit(plane, y));
                self.decoded.mark(plane, (uy, ux), tx.units());
            }
        }
        let distortion = self.frame.source[plane].squared_error(
            &self.frame.recon[plane],
            bx,
            by,
            block.samples(),
        );
        PlaneCoding { txs, distortion }
    }

    /// Writes the coefficients of a plane's transform blocks.
    pub(crate) fn write_plane<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        coding: &PlaneCoding,
        info: &BlockInfo,
    ) {
        for tx in &coding.txs {
            let scan = self.frame.scans.get(tx.tx);
            coeffs::write(sink, &mut self.cdfs, self.contexts, (tx, info), scan);
        }
    }

    /// Codes the block at `(r, c)` with `choice`: reconstructs it, writes
    /// it to `sink` and updates the contexts. Returns the squared error
    /// of its reconstruction.
    pub(crate) fn code_block<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        (r, c): (usize, usize),
        size: Square,
        choice: &BlockChoice,
    ) -> u64 {
        let planes = self.frame.recon.len();
        let codings: Vec<PlaneCoding> = (0..planes)
            .map(|plane| {
                let predictor = match (plane, &choice.y_palette, &choice.uv_palette) {
                    (0, Some(palette), _) => Predictor::Palette(palette, 0),
                    (0, None, _) => Predictor::Intra(choice.y),
                    (_, _, Some(palette)) => Predictor::Palette(palette, plane - 1),
                    (_, _, None) => Predictor::Intra(choice.uv),
                };
                self.compute_plane(plane, (r, c), size, (predictor, choice.tx_type))
            })
            .collect();
        let skip = codings
            .iter()
            .flat_map(|coding| &coding.txs)
            .all(|tx| tx.eob == 0);
        let above_skip = self.above_available(r) && self.contexts.above.blocks[c].skip;
        let left_skip = self.left_available(c) && self.contexts.left.blocks[r].skip;
        let skip_ctx = usize::from(above_skip) + usize::from(left_skip);
        sink.symbol(&mut self.cdfs.skip[skip_ctx], usize::from(skip));
        // The superblock's quantizer delta comes with its first block,
        // unless that block is the whole superblock and skips.
        if let Some((next, reduced)) = self.pending_delta.take()
            && !(size == Square::S64 && skip)
        {
            delta_q::write(sink, &mut self.cdfs.delta_q, reduced);
            self.current_qindex = next;
        }
        self.write_y_mode(sink, (r, c), choice.y);
        if planes > 1 {
            self.write_uv_mode(sink, size, choice.y.mode, choice.uv);
        }
        if choice.y.mode == Mode::Dc {
            self.write_luma_palette(sink, ((r, c), size), choice.y_palette.as_ref());
        }
        if planes > 1 && choice.uv.mode == Mode::Dc {
            let has_luma = choice.y_palette.is_some();
            self.write_chroma_palette(sink, ((r, c), size), (has_luma, choice.uv_palette.as_ref()));
        }
        if let Some(palette) = &choice.y_palette {
            self.write_palette_map(sink, true, palette);
        }
        if let Some(palette) = &choice.uv_palette {
            self.write_palette_map(sink, false, palette);
        }
        let units = size.units();
        if skip {
            self.contexts.above.clear_levels(&(c..c + units));
            self.contexts.left.clear_levels(&(r..r + units));
        } else {
            for (plane, coding) in codings.iter().enumerate() {
                let info = self.block_info(plane, size, choice.y.mode);
                self.write_plane(sink, coding, &info);
            }
        }
        let colors = |palette: &Option<Palette>| {
            palette
                .as_ref()
                .map_or_else(Colors::default, |p| Colors::new(&p.colors[0]))
        };
        let neighbour = Neighbour {
            mode: choice.y.mode,
            skip,
            size_log2: size.units_log2() as u8,
            palettes: [colors(&choice.y_palette), colors(&choice.uv_palette)],
        };
        self.contexts.above.set_block(c..c + units, neighbour);
        self.contexts.left.set_block(r..r + units, neighbour);
        codings.iter().map(|coding| coding.distortion).sum()
    }
}
