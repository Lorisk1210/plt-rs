//! Coding a tile: superblocks in raster order, each partitioned by a
//! rate-distortion search and then coded with the decisions made.

use super::block::BlockChoice;
use super::cdf::Cdfs;
use super::coeffs::Scans;
use super::context::{Contexts, Decoded};
use super::delta_q;
use super::entropy::{SymbolEncoder, SymbolSink};
use super::plane::Plane;
use super::quant::Quantizer;
use super::syntax::Square;
use super::transform::Bases;

/// `PARTITION_NONE`.
const PARTITION_NONE: usize = 0;
/// `PARTITION_SPLIT`.
const PARTITION_SPLIT: usize = 3;

/// The frame being coded: its planes and the settings every tile shares.
#[derive(Debug)]
pub(crate) struct Frame {
    /// The source planes, padded to whole superblocks.
    pub(crate) source: Vec<Plane>,
    /// The reconstructed planes, as a decoder will see them.
    pub(crate) recon: Vec<Plane>,
    /// The chroma subsampling, horizontal and vertical.
    pub(crate) ss: (u32, u32),
    /// `MiCols` and `MiRows`: the frame in 4×4 luma units, rounded up to
    /// 8×8.
    pub(crate) mi: (usize, usize),
    /// The quantizers of the base quantizer index.
    pub(crate) quantizer: Quantizer,
    /// The base-2 logarithm of the step of superblock quantizer deltas
    /// (`delta_q_res`); `None` without deltas.
    pub(crate) delta_q_res: Option<u32>,
    /// The forward transform bases.
    pub(crate) bases: Bases,
    /// The coefficient scans.
    pub(crate) scans: Scans,
}

/// The 4×4 units a tile covers.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TileBounds {
    /// `MiRowStart` and `MiRowEnd`.
    pub(crate) rows: (usize, usize),
    /// `MiColStart` and `MiColEnd`.
    pub(crate) cols: (usize, usize),
}

/// How a square is coded: as one block, as four quarters, or not at all
/// when it lies outside the frame.
#[derive(Clone, Debug)]
pub(crate) enum Partition {
    /// `PARTITION_NONE`.
    Whole(BlockChoice),
    /// `PARTITION_SPLIT`, the quarters in raster order.
    Split(Box<[Partition; 4]>),
    /// Outside the frame.
    Outside,
}

/// The probability of no split in the flag that replaces `partition` at
/// the frame's edges: 32768 minus the probabilities of `splits`.
fn unsplit_probability(cdf: &[u16; 10], splits: [usize; 6]) -> u16 {
    let at = |i: usize| if i == 9 { 32768 } else { u32::from(cdf[i]) };
    let psum: u32 = splits.into_iter().map(|p| at(p) - at(p - 1)).sum();
    (32768 - psum) as u16
}

/// The state of one tile being coded.
#[derive(Debug)]
pub(crate) struct TileCoder<'a> {
    /// The frame.
    pub(crate) frame: &'a mut Frame,
    /// The neighbour state.
    pub(crate) contexts: &'a mut Contexts,
    /// The tile's CDFs.
    pub(crate) cdfs: Cdfs,
    /// The tile's extent.
    pub(crate) bounds: TileBounds,
    /// The decoded units of the superblock being coded.
    pub(crate) decoded: Decoded,
    /// The quantizers of the superblock being coded.
    pub(crate) quantizer: Quantizer,
    /// The quantizer index a decoder has reached (`CurrentQIndex`).
    pub(crate) current_qindex: u8,
    /// The superblock's quantizer index and its reduced delta, until its
    /// first block writes them.
    pub(crate) pending_delta: Option<(u8, i32)>,
}

/// Codes one tile of `frame` and returns its bytes.
pub(crate) fn encode_tile(
    frame: &mut Frame,
    contexts: &mut Contexts,
    bounds: TileBounds,
) -> Vec<u8> {
    let base = frame.quantizer;
    let cdfs = Cdfs::new(base.qindex());
    let decoded = Decoded::new(frame.recon.len(), frame.ss.0);
    let mut tile = TileCoder {
        frame,
        contexts,
        cdfs,
        bounds,
        decoded,
        quantizer: base,
        current_qindex: base.qindex(),
        pending_delta: None,
    };
    let columns = tile.contexts.above.blocks.len();
    tile.contexts.above.clear_levels(&(0..columns));
    let mut encoder = SymbolEncoder::new();
    let superblock = Square::S64.units();
    for r in (bounds.rows.0..bounds.rows.1).step_by(superblock) {
        tile.contexts.left.clear_levels(&(r..r + superblock));
        for c in (bounds.cols.0..bounds.cols.1).step_by(superblock) {
            let delta = tile.frame.delta_q_res.map(|res| {
                let target = delta_q::target(&tile.frame.source[0], (c * 4, r * 4), base.qindex());
                delta_q::adjust(res, tile.current_qindex, target)
            });
            tile.quantizer = delta.map_or(base, |(qindex, _)| base.at(qindex));
            let end = (bounds.rows.1, bounds.cols.1);
            tile.decoded.clear((r, c), end, tile.frame.ss.0);
            let saved = tile.contexts.save(r, c, superblock);
            let (_, tree) = tile.search_partition((r, c), Square::S64);
            tile.contexts.restore(&saved);
            tile.decoded.clear((r, c), end, tile.frame.ss.0);
            tile.pending_delta = delta;
            tile.code_partition(&mut encoder, (r, c), Square::S64, &tree);
        }
    }
    encoder.finish()
}

impl TileCoder<'_> {
    /// Whether the block at row `r` has decoded samples above (`AvailU`).
    pub(crate) fn above_available(&self, r: usize) -> bool {
        r > self.bounds.rows.0
    }

    /// Whether the block at column `c` has decoded samples to the left
    /// (`AvailL`).
    pub(crate) fn left_available(&self, c: usize) -> bool {
        c > self.bounds.cols.0
    }

    /// Whether the square at `(r, c)` reaches past the half of it into
    /// the frame, down and across (`hasRows`, `hasCols`).
    pub(crate) fn has_rows_cols(&self, (r, c): (usize, usize), size: Square) -> (bool, bool) {
        let half = size.units() / 2;
        (r + half < self.frame.mi.1, c + half < self.frame.mi.0)
    }

    /// Writes how the square at `(r, c)` is partitioned: the `partition`
    /// symbol, or the `split_or_horz` and `split_or_vert` flags at the
    /// frame's bottom and right edges (where only a split is coded here).
    pub(crate) fn write_partition<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        (r, c): (usize, usize),
        size: Square,
        split: bool,
    ) {
        let (has_rows, has_cols) = self.has_rows_cols((r, c), size);
        let bsl = size.units_log2();
        let above =
            self.above_available(r) && u32::from(self.contexts.above.blocks[c].size_log2) < bsl;
        let left =
            self.left_available(c) && u32::from(self.contexts.left.blocks[r].size_log2) < bsl;
        let ctx = usize::from(left) * 2 + usize::from(above);
        let symbol = if split {
            PARTITION_SPLIT
        } else {
            PARTITION_NONE
        };
        if size == Square::S8 {
            sink.symbol(&mut self.cdfs.partition_8[ctx], symbol);
            return;
        }
        let cdf = &mut self.cdfs.partition[bsl as usize - 2][ctx];
        match (has_rows, has_cols) {
            (true, true) => sink.symbol(cdf, symbol),
            // split_or_horz: the partitions with a vertical split count
            // as a split.
            (false, true) => sink.bool_with(split, unsplit_probability(cdf, [2, 3, 4, 6, 7, 9])),
            // split_or_vert.
            (true, false) => sink.bool_with(split, unsplit_probability(cdf, [1, 3, 4, 5, 6, 8])),
            (false, false) => {}
        }
    }

    /// Codes the square at `(r, c)` as `tree` decides.
    fn code_partition<S: SymbolSink>(
        &mut self,
        sink: &mut S,
        (r, c): (usize, usize),
        size: Square,
        tree: &Partition,
    ) {
        match tree {
            Partition::Outside => {}
            Partition::Whole(choice) => {
                self.write_partition(sink, (r, c), size, false);
                self.code_block(sink, (r, c), size, choice);
            }
            Partition::Split(quarters) => {
                self.write_partition(sink, (r, c), size, true);
                let half = size.units() / 2;
                for (i, quarter) in quarters.iter().enumerate() {
                    let at = (r + half * (i / 2), c + half * (i % 2));
                    self.code_partition(sink, at, size.half(), quarter);
                }
            }
        }
    }
}
