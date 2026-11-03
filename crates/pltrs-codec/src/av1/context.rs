//! The neighbour state that the CDFs of a block depend on: the levels and
//! DC signs of the transform blocks along its top and left edges, and the
//! modes, skip flags, sizes and palettes of the blocks there. Coding
//! decisions try alternatives and roll back, so spans of this state can
//! be saved and restored.

use std::ops::Range;

use super::palette::Colors;
use super::syntax::Mode;

/// What a block leaves along its edges for the blocks below and to the
/// right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Neighbour {
    /// The luma mode.
    pub(crate) mode: Mode,
    /// Whether the block skips its residual.
    pub(crate) skip: bool,
    /// `Mi_Width_Log2` (above) or `Mi_Height_Log2` (left) of the block.
    pub(crate) size_log2: u8,
    /// The luma and U palettes (`PaletteColors`), empty without one.
    pub(crate) palettes: [Colors; 2],
}

impl Default for Neighbour {
    fn default() -> Self {
        Self {
            mode: Mode::Dc,
            skip: false,
            size_log2: 0,
            palettes: [Colors::default(); 2],
        }
    }
}

/// The state along one edge: the bottom row of the blocks above, by
/// column, or the right column of the blocks to the left, by row.
#[derive(Clone, Debug)]
pub(crate) struct Edge {
    /// `AboveLevelContext` or `LeftLevelContext` of each plane, by 4×4
    /// unit of the plane: the clamped sum of the levels of the transform
    /// block there.
    pub(crate) level: [Vec<u8>; 3],
    /// `AboveDcContext` or `LeftDcContext`: 1 for a negative DC
    /// coefficient, 2 for a positive one, 0 for none.
    pub(crate) dc: [Vec<u8>; 3],
    /// The neighbouring block, by 4×4 luma unit.
    pub(crate) blocks: Vec<Neighbour>,
    /// The subsampling of the chroma planes along the edge.
    ss: u32,
}

/// A saved span of an [`Edge`].
#[derive(Clone, Debug)]
pub(crate) struct EdgeSpan {
    luma: Range<usize>,
    level: [Vec<u8>; 3],
    dc: [Vec<u8>; 3],
    blocks: Vec<Neighbour>,
}

impl Edge {
    /// An edge of `units` 4×4 luma units, its chroma subsampled by `ss`.
    pub(crate) fn new(units: usize, ss: u32) -> Self {
        let chroma = units >> ss;
        Self {
            level: [vec![0; units], vec![0; chroma], vec![0; chroma]],
            dc: [vec![0; units], vec![0; chroma], vec![0; chroma]],
            blocks: vec![Neighbour::default(); units],
            ss,
        }
    }

    /// The units of `plane` that the luma units `luma` cover.
    pub(crate) fn plane_range(&self, plane: usize, luma: &Range<usize>) -> Range<usize> {
        let ss = if plane == 0 { 0 } else { self.ss };
        luma.start >> ss..luma.end >> ss
    }

    /// Zeroes the levels and DC signs over the luma units `luma`
    /// (`clear_above_context`, `clear_left_context`, and the reset of a
    /// skipped block).
    pub(crate) fn clear_levels(&mut self, luma: &Range<usize>) {
        for plane in 0..3 {
            let range = self.plane_range(plane, luma);
            self.level[plane][range.clone()].fill(0);
            self.dc[plane][range].fill(0);
        }
    }

    /// Records a block along `luma`.
    pub(crate) fn set_block(&mut self, luma: Range<usize>, block: Neighbour) {
        self.blocks[luma].fill(block);
    }

    /// A copy of the state over the luma units `luma`.
    pub(crate) fn save(&self, luma: Range<usize>) -> EdgeSpan {
        let planes = |values: &[Vec<u8>; 3]| {
            [0, 1, 2].map(|plane| values[plane][self.plane_range(plane, &luma)].to_vec())
        };
        EdgeSpan {
            level: planes(&self.level),
            dc: planes(&self.dc),
            blocks: self.blocks[luma.clone()].to_vec(),
            luma,
        }
    }

    /// Puts back a saved span.
    pub(crate) fn restore(&mut self, span: &EdgeSpan) {
        for plane in 0..3 {
            let range = self.plane_range(plane, &span.luma);
            self.level[plane][range.clone()].copy_from_slice(&span.level[plane]);
            self.dc[plane][range].copy_from_slice(&span.dc[plane]);
        }
        self.blocks[span.luma.clone()].copy_from_slice(&span.blocks);
    }
}

/// The edges of a frame: above by column, left by row.
#[derive(Clone, Debug)]
pub(crate) struct Contexts {
    /// The state of the blocks above, by column.
    pub(crate) above: Edge,
    /// The state of the blocks to the left, by row.
    pub(crate) left: Edge,
}

/// Saved spans of both edges.
#[derive(Clone, Debug)]
pub(crate) struct ContextSpan {
    above: EdgeSpan,
    left: EdgeSpan,
}

impl Contexts {
    /// The contexts of a padded frame of `columns × rows` 4×4 luma units.
    pub(crate) fn new((columns, rows): (usize, usize), (ss_x, ss_y): (u32, u32)) -> Self {
        Self {
            above: Edge::new(columns, ss_x),
            left: Edge::new(rows, ss_y),
        }
    }

    /// A copy of the state along the edges of the square of `units` 4×4
    /// luma units at row `r`, column `c`.
    pub(crate) fn save(&self, r: usize, c: usize, units: usize) -> ContextSpan {
        ContextSpan {
            above: self.above.save(c..c + units),
            left: self.left.save(r..r + units),
        }
    }

    /// Puts back saved spans.
    pub(crate) fn restore(&mut self, span: &ContextSpan) {
        self.above.restore(&span.above);
        self.left.restore(&span.left);
    }
}

/// Which 4×4 units of the superblock being coded hold decoded samples
/// (`BlockDecoded`), per plane, with a border of one unit above and to
/// the left: rows and columns from -1 to the superblock's side.
#[derive(Clone, Debug)]
pub(crate) struct Decoded {
    /// The superblock's side in 4×4 units of each plane.
    sides: Vec<i32>,
    /// The flags of each plane, `side + 2` to a row.
    flags: Vec<Vec<bool>>,
}

impl Decoded {
    /// The flags of `planes` planes, chroma subsampled by `ss`.
    pub(crate) fn new(planes: usize, ss: u32) -> Self {
        let sides: Vec<i32> = (0..planes)
            .map(|plane| 16 >> if plane == 0 { 0 } else { ss })
            .collect();
        let flags = sides
            .iter()
            .map(|&side| vec![false; ((side + 2) * (side + 2)) as usize])
            .collect();
        Self { sides, flags }
    }

    /// The index of unit `(y, x)` of `plane`.
    fn index(&self, plane: usize, y: i32, x: i32) -> usize {
        ((y + 1) * (self.sides[plane] + 2) + x + 1) as usize
    }

    /// The unit within the superblock of sample column or row `sample`
    /// of `plane`.
    pub(crate) fn unit(&self, plane: usize, sample: usize) -> i32 {
        (sample as i32 >> 2) & (self.sides[plane] - 1)
    }

    /// Whether unit `(y, x)` of `plane` is decoded.
    pub(crate) fn get(&self, plane: usize, y: i32, x: i32) -> bool {
        self.flags[plane][self.index(plane, y, x)]
    }

    /// Readies the flags for the superblock at `(r, c)` of a tile that
    /// ends at row `end.0` and column `end.1` (`clear_block_decoded_flags`):
    /// the row above and the column on the left are decoded inside the
    /// tile, but for the unit below the superblock's bottom left.
    pub(crate) fn clear(&mut self, (r, c): (usize, usize), end: (usize, usize), ss: u32) {
        for plane in 0..self.sides.len() {
            let ss = if plane == 0 { 0 } else { ss };
            let side = self.sides[plane];
            let width = ((end.1 - c) >> ss) as i32;
            let height = ((end.0 - r) >> ss) as i32;
            for y in -1..=side {
                for x in -1..=side {
                    let decoded = (y < 0 && x < width) || (x < 0 && y >= 0 && y < height);
                    let index = self.index(plane, y, x);
                    self.flags[plane][index] = decoded;
                }
            }
            let corner = self.index(plane, side, -1);
            self.flags[plane][corner] = false;
        }
    }

    /// Marks the `units × units` square at unit `(y, x)` of `plane` decoded.
    pub(crate) fn mark(&mut self, plane: usize, (y, x): (i32, i32), units: usize) {
        for row in y..y + units as i32 {
            let start = self.index(plane, row, x);
            self.flags[plane][start..start + units].fill(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoded_units_follow_the_tile() {
        let mut decoded = Decoded::new(3, 1);
        // A superblock 24 units from the tile's right edge and 8 from its
        // bottom.
        decoded.clear((0, 0), (8, 24), 1);
        assert!(decoded.get(0, -1, 16) && decoded.get(0, -1, -1));
        assert!(decoded.get(0, 7, -1) && !decoded.get(0, 8, -1));
        assert!(!decoded.get(0, 0, 0) && !decoded.get(0, 16, -1));
        // Chroma halves both.
        assert!(decoded.get(1, -1, 8) && decoded.get(1, 3, -1) && !decoded.get(1, 4, -1));
        decoded.mark(0, (2, 4), 4);
        assert!(decoded.get(0, 5, 7) && !decoded.get(0, 6, 7) && !decoded.get(0, 5, 8));
        assert_eq!(decoded.unit(0, 70), 1);
        assert_eq!(decoded.unit(1, 36), 1);
    }

    #[test]
    fn spans_round_trip() {
        let mut contexts = Contexts::new((16, 8), (1, 1));
        contexts.above.level[0][4] = 7;
        contexts.above.level[1][2] = 3;
        contexts.left.blocks[2].mode = Mode::Paeth;
        let saved = contexts.save(2, 4, 4);
        contexts.above.level[0][4] = 0;
        contexts.above.level[1][2] = 0;
        contexts.left.blocks[2].mode = Mode::Dc;
        contexts.above.blocks[9].skip = true;
        contexts.restore(&saved);
        assert_eq!(contexts.above.level[0][4], 7);
        assert_eq!(contexts.above.level[1][2], 3);
        assert_eq!(contexts.left.blocks[2].mode, Mode::Paeth);
        // Outside the span, changes stay.
        assert!(contexts.above.blocks[9].skip);
    }

    #[test]
    fn clearing_covers_the_chroma_units() {
        let mut edge = Edge::new(8, 1);
        edge.level = [vec![1; 8], vec![1; 4], vec![1; 4]];
        edge.clear_levels(&(2..6));
        assert_eq!(edge.level[0], [1, 1, 0, 0, 0, 0, 1, 1]);
        assert_eq!(edge.level[2], [1, 0, 0, 1]);
    }
}
