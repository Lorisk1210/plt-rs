//! Coefficient coding (the `coeffs` syntax of section 5.11.39, with the
//! CDF selection of section 8.3.2): the end of block, the levels in
//! reverse scan order, then the signs and Golomb-coded remainders.

use std::ops::Range;

use super::cdf::Cdfs;
use super::context::{Contexts, Edge};
use super::entropy::SymbolSink;
use super::syntax::{Mode, Square};
use super::transform::TxType;

/// `NUM_BASE_LEVELS + COEFF_BASE_RANGE`: the largest level coded with
/// CDFs; larger ones add a Golomb code.
const MAX_CDF_LEVEL: i32 = 14;

/// `Coeff_Base_Ctx_Offset` of the square transforms, by row and column
/// (both capped at 4).
const BASE_CONTEXT_OFFSET: [[usize; 5]; 5] = [
    [0, 1, 6, 6, 21],
    [1, 6, 6, 21, 21],
    [6, 6, 21, 21, 21],
    [6, 21, 21, 21, 21],
    [21, 21, 21, 21, 21],
];

/// `Sig_Ref_Diff_Offset` of two-dimensional transforms: the neighbours,
/// right and below, whose levels set the context of a base level.
const BASE_NEIGHBOURS: [(usize, usize); 5] = [(0, 1), (1, 0), (1, 1), (0, 2), (2, 0)];

/// `Mag_Ref_Offset_With_Tx_Class` of two-dimensional transforms.
const RANGE_NEIGHBOURS: [(usize, usize); 3] = [(0, 1), (1, 0), (1, 1)];

/// The zigzag scans of square transforms (`Default_Scan_4x4` to
/// `Default_Scan_32x32`), which 64×64 transforms share with 32×32.
#[derive(Debug)]
pub(crate) struct Scans {
    scans: [Vec<u16>; 4],
}

impl Scans {
    /// The scans of 4×4 to 32×32 coefficients.
    pub(crate) fn new() -> Self {
        Self {
            scans: [4, 8, 16, 32].map(zigzag),
        }
    }

    /// The scan of a transform's coded coefficients: positions row by
    /// row, `Min(32, side)` to a row.
    pub(crate) fn get(&self, tx: Square) -> &[u16] {
        &self.scans[tx.tx_index().min(3)]
    }
}

/// The zigzag order of a `side × side` block: the anti-diagonals in turn,
/// alternately down-left and up-right.
fn zigzag(side: usize) -> Vec<u16> {
    let mut order = Vec::with_capacity(side * side);
    for diagonal in 0..2 * side - 1 {
        let rows = diagonal.saturating_sub(side - 1)..=diagonal.min(side - 1);
        let cells: Vec<usize> = rows.map(|row| row * side + diagonal - row).collect();
        if diagonal % 2 == 0 {
            order.extend(cells.iter().rev().map(|&pos| pos as u16));
        } else {
            order.extend(cells.iter().map(|&pos| pos as u16));
        }
    }
    order
}

/// A transform block's coefficients, ready to be coded.
#[derive(Clone, Debug)]
pub(crate) struct CodedTx {
    /// The plane: 0 for luma.
    pub(crate) plane: usize,
    /// The column in the plane, in 4×4 units.
    pub(crate) x4: usize,
    /// The row in the plane, in 4×4 units.
    pub(crate) y4: usize,
    /// The transform size.
    pub(crate) tx: Square,
    /// The transform type.
    pub(crate) tx_type: TxType,
    /// The levels, row by row, `Min(32, side)` to a row.
    pub(crate) levels: Vec<i32>,
    /// One past the last nonzero level in scan order; 0 for none.
    pub(crate) eob: usize,
}

impl CodedTx {
    /// A transform block's levels, its end of block found from `scan`.
    pub(crate) fn new(
        plane: usize,
        (x4, y4): (usize, usize),
        (tx, tx_type): (Square, TxType),
        levels: Vec<i32>,
        scan: &[u16],
    ) -> Self {
        let eob = scan
            .iter()
            .rposition(|&pos| levels[usize::from(pos)] != 0)
            .map_or(0, |last| last + 1);
        Self {
            plane,
            x4,
            y4,
            tx,
            tx_type,
            levels,
            eob,
        }
    }
}

/// What coding a transform block depends on beyond its coefficients.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlockInfo {
    /// The side of the block in the transform's plane, in samples.
    pub(crate) block_samples: usize,
    /// The luma mode, which selects the CDF of the transform type.
    pub(crate) y_mode: Mode,
    /// Whether the transform type is coded: luma blocks of at most
    /// 16×16 samples, not lossless.
    pub(crate) codes_tx_type: bool,
    /// The plane's 4×4 columns and rows in the frame (`maxX4`, `maxY4`).
    pub(crate) limits: (usize, usize),
}

/// The units from `start` along a transform's `units` that lie inside the
/// frame's `limit` (the `x4 + k < maxX4` tests of the contexts).
fn inside(start: usize, units: usize, limit: usize) -> Range<usize> {
    let end = (start + units).min(limit);
    start.min(end)..end
}

/// The context of `all_zero`.
fn all_zero_context(contexts: &Contexts, tx: &CodedTx, info: &BlockInfo) -> usize {
    let p = tx.plane;
    let units = tx.tx.units();
    let columns = inside(tx.x4, units, info.limits.0);
    let rows = inside(tx.y4, units, info.limits.1);
    if p == 0 {
        if info.block_samples == tx.tx.samples() {
            return 0;
        }
        let top = contexts.above.level[0][columns]
            .iter()
            .max()
            .copied()
            .unwrap_or(0);
        let left = contexts.left.level[0][rows]
            .iter()
            .max()
            .copied()
            .unwrap_or(0);
        let (low, high) = (top.min(left), top.max(left));
        match (low, high) {
            (0, 0) => 1,
            (0, _) => 2 + usize::from(high > 3),
            (_, 0..=3) => 4,
            (0..=3, _) => 5,
            _ => 6,
        }
    } else {
        let any = |edge: &Edge, range: Range<usize>| {
            usize::from(
                range
                    .into_iter()
                    .any(|i| edge.level[p][i] | edge.dc[p][i] != 0),
            )
        };
        let larger = info.block_samples > tx.tx.samples();
        7 + any(&contexts.above, columns) + any(&contexts.left, rows) + if larger { 3 } else { 0 }
    }
}

/// The context of `dc_sign`: the balance of the DC signs along the edges.
fn dc_sign_context(contexts: &Contexts, tx: &CodedTx, info: &BlockInfo) -> usize {
    let units = tx.tx.units();
    let above = &contexts.above.dc[tx.plane][inside(tx.x4, units, info.limits.0)];
    let left = &contexts.left.dc[tx.plane][inside(tx.y4, units, info.limits.1)];
    let balance: i32 = above
        .iter()
        .chain(left)
        .map(|&sign| match sign {
            1 => -1,
            2 => 1,
            _ => 0,
        })
        .sum();
    match balance.signum() {
        -1 => 1,
        1 => 2,
        _ => 0,
    }
}

/// The end-of-block class (`eobPt`) of `eob` (at least 1).
fn eob_class(eob: usize) -> usize {
    if eob < 2 {
        eob
    } else {
        (eob - 1).ilog2() as usize + 2
    }
}

/// The coded levels so far, capped at 15, that the contexts of later
/// levels read; `side` coefficients to a row.
struct Magnitudes {
    levels: Vec<u8>,
    side: usize,
}

impl Magnitudes {
    /// The sum of the capped levels at the neighbours inside the block.
    fn sum(&self, pos: usize, neighbours: &[(usize, usize)], cap: u8) -> usize {
        let (row, col) = (pos / self.side, pos % self.side);
        neighbours
            .iter()
            .filter(|&&(dr, dc)| row + dr < self.side && col + dc < self.side)
            .map(|&(dr, dc)| usize::from(self.levels[(row + dr) * self.side + col + dc].min(cap)))
            .sum()
    }

    /// The context of `coeff_base` at `pos`.
    fn base_context(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }
        let (row, col) = (pos / self.side, pos % self.side);
        let ctx = ((self.sum(pos, &BASE_NEIGHBOURS, 3) + 1) >> 1).min(4);
        ctx + BASE_CONTEXT_OFFSET[row.min(4)][col.min(4)]
    }

    /// The context of `coeff_br` at `pos`.
    fn range_context(&self, pos: usize) -> usize {
        let mag = ((self.sum(pos, &RANGE_NEIGHBOURS, 15) + 1) >> 1).min(6);
        let (row, col) = (pos / self.side, pos % self.side);
        if pos == 0 {
            mag
        } else if row < 2 && col < 2 {
            mag + 7
        } else {
            mag + 14
        }
    }
}

/// The context of `coeff_base_eob` for the last coefficient, at scan
/// index `c` of `area` coded coefficients.
fn base_eob_context(c: usize, area: usize) -> usize {
    if c == 0 {
        0
    } else if c <= area / 8 {
        1
    } else if c <= area / 4 {
        2
    } else {
        3
    }
}

/// Writes `value >= 1` as an Exp-Golomb code: `length - 1` zeros, then
/// its `length` bits.
fn golomb<S: SymbolSink>(sink: &mut S, value: u32) {
    let length = value.ilog2() + 1;
    sink.literal(0, length - 1);
    sink.literal(value, length);
}

/// Writes the end of block of a transform block with `eob >= 1`.
fn write_eob<S: SymbolSink>(sink: &mut S, cdfs: &mut Cdfs, tx: &CodedTx) {
    let ptype = usize::from(tx.plane > 0);
    let size = tx.tx.tx_index();
    let class = eob_class(tx.eob);
    let symbol = class - 1;
    match size {
        0 => sink.symbol(&mut cdfs.eob_pt_16[ptype][0], symbol),
        1 => sink.symbol(&mut cdfs.eob_pt_64[ptype][0], symbol),
        2 => sink.symbol(&mut cdfs.eob_pt_256[ptype][0], symbol),
        _ => sink.symbol(&mut cdfs.eob_pt_1024[ptype], symbol),
    }
    if class >= 3 {
        let offset = (tx.eob - (1 << (class - 2)) - 1) as u32;
        let high = class as u32 - 3;
        sink.symbol(
            &mut cdfs.eob_extra[size][ptype][class - 3],
            ((offset >> high) & 1) as usize,
        );
        sink.literal(offset, high);
    }
}

/// Writes the coefficients of a transform block and updates the contexts
/// along its edges.
pub(crate) fn write<S: SymbolSink>(
    sink: &mut S,
    cdfs: &mut Cdfs,
    contexts: &mut Contexts,
    (tx, info): (&CodedTx, &BlockInfo),
    scan: &[u16],
) {
    let ptype = usize::from(tx.plane > 0);
    let size = tx.tx.tx_index();
    let ctx = all_zero_context(contexts, tx, info);
    sink.symbol(&mut cdfs.txb_skip[size][ctx], usize::from(tx.eob == 0));
    let (mut cul_level, mut dc_category) = (0u32, 0u8);
    if tx.eob > 0 {
        if info.codes_tx_type {
            sink.symbol(
                &mut cdfs.tx_type[size][info.y_mode.index()],
                tx.tx_type.symbol(),
            );
        }
        write_eob(sink, cdfs, tx);
        let side = tx.tx.samples().min(32);
        let mut magnitudes = Magnitudes {
            levels: vec![0; side * side],
            side,
        };
        for c in (0..tx.eob).rev() {
            let pos = usize::from(scan[c]);
            let level = tx.levels[pos].unsigned_abs();
            let base = level.min(3) as usize;
            if c + 1 == tx.eob {
                let ctx = base_eob_context(c, side * side);
                sink.symbol(&mut cdfs.coeff_base_eob[size][ptype][ctx], base - 1);
            } else {
                let ctx = magnitudes.base_context(pos);
                sink.symbol(&mut cdfs.coeff_base[size][ptype][ctx], base);
            }
            if level > 2 {
                let ctx = magnitudes.range_context(pos);
                let mut remaining = level - 3;
                for _ in 0..4 {
                    let step = remaining.min(3);
                    sink.symbol(&mut cdfs.coeff_br[size.min(3)][ptype][ctx], step as usize);
                    remaining -= step;
                    if step < 3 {
                        break;
                    }
                }
            }
            magnitudes.levels[pos] = level.min(15) as u8;
        }
        for (c, &pos) in scan[..tx.eob].iter().enumerate() {
            let level = tx.levels[usize::from(pos)];
            if level == 0 {
                continue;
            }
            if c == 0 {
                let ctx = dc_sign_context(contexts, tx, info);
                sink.symbol(&mut cdfs.dc_sign[ptype][ctx], usize::from(level < 0));
                dc_category = if level < 0 { 1 } else { 2 };
            } else {
                sink.literal(u32::from(level < 0), 1);
            }
            if level.abs() > MAX_CDF_LEVEL {
                golomb(sink, level.unsigned_abs() - MAX_CDF_LEVEL as u32);
            }
            cul_level += level.unsigned_abs();
        }
    }
    let cul_level = cul_level.min(63) as u8;
    let units = tx.tx.units();
    let p = tx.plane;
    contexts.above.level[p][tx.x4..tx.x4 + units].fill(cul_level);
    contexts.above.dc[p][tx.x4..tx.x4 + units].fill(dc_category);
    contexts.left.level[p][tx.y4..tx.y4 + units].fill(cul_level);
    contexts.left.dc[p][tx.y4..tx.y4 + units].fill(dc_category);
}

#[cfg(test)]
mod tests {
    use super::super::entropy::RateEstimate;
    use super::*;

    #[test]
    fn zigzag_scans_match_the_specification() {
        assert_eq!(
            zigzag(4),
            [0, 1, 4, 8, 5, 2, 3, 6, 9, 12, 13, 10, 7, 11, 14, 15]
        );
        let scan_8 = zigzag(8);
        assert_eq!(&scan_8[..12], [0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25]);
        assert_eq!(&scan_8[58..], [61, 54, 47, 55, 62, 63]);
        let scan_32 = zigzag(32);
        assert_eq!(&scan_32[..6], [0, 1, 32, 64, 33, 2]);
        let mut sorted = scan_32.clone();
        sorted.sort_unstable();
        assert!(sorted.iter().enumerate().all(|(i, &p)| usize::from(p) == i));
    }

    #[test]
    fn eob_classes() {
        let classes: Vec<usize> = [1, 2, 3, 4, 5, 8, 9, 16, 17, 1024].map(eob_class).to_vec();
        assert_eq!(classes, [1, 2, 3, 3, 4, 4, 5, 5, 6, 11]);
    }

    #[test]
    fn neighbour_magnitudes_set_the_contexts() {
        let mut magnitudes = Magnitudes {
            levels: vec![0; 16],
            side: 4,
        };
        assert_eq!(magnitudes.base_context(0), 0);
        assert_eq!(magnitudes.base_context(1), 1);
        assert_eq!(magnitudes.base_context(15), 21);
        magnitudes.levels[2] = 9;
        magnitudes.levels[5] = 1;
        // Neighbours of position 1: 2 (capped at 3), 5, 6, 3, 9.
        assert_eq!(magnitudes.base_context(1), 2 + 1);
        assert_eq!(magnitudes.range_context(1), 5 + 7);
        // Position 0 reads 1, 4 and 5: (1 + 1) >> 1.
        assert_eq!(magnitudes.range_context(0), 1);
    }

    #[test]
    fn contexts_follow_the_coded_levels() {
        let scans = Scans::new();
        let mut cdfs = Cdfs::new(100);
        let mut contexts = Contexts::new((16, 16), (1, 1));
        let mut levels = vec![0; 16];
        levels[0] = -5;
        levels[1] = 40;
        let tx = CodedTx::new(
            0,
            (4, 4),
            (Square::S4, TxType::DctDct),
            levels,
            scans.get(Square::S4),
        );
        assert_eq!(tx.eob, 2);
        let info = BlockInfo {
            block_samples: 4,
            y_mode: Mode::Dc,
            codes_tx_type: true,
            limits: (16, 16),
        };
        let mut estimate = RateEstimate::default();
        write(
            &mut estimate,
            &mut cdfs,
            &mut contexts,
            (&tx, &info),
            scans.get(Square::S4),
        );
        assert!(estimate.bits > 10.0);
        assert_eq!(contexts.above.level[0][4], 45);
        assert_eq!(contexts.left.dc[0][4], 1);
        assert_eq!(contexts.above.level[0][5], 0);
    }
}
