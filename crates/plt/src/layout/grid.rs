//! Layout grids: the rows and columns of a figure or grid specification,
//! each cell with editable margins around its inner box (matplotlib's
//! `_layoutgrid.LayoutGrid`).
//!
//! matplotlib states the layout as linear constraints for kiwisolver: the
//! outer edges of a grid equal its parent's (inner) edges, neighbouring
//! cells touch, inner sizes follow the width and height ratios, and the
//! margins take their suggested values. Those equations determine every
//! edge, so they are solved directly, grid by grid from the root down.
//! When the margins leave no room, inner sizes come out negative and the
//! layout counts as collapsed, as in matplotlib.

use std::ops::Range;

use plt_core::Bbox;

/// The margins of a cell side: the inner one holds axes decorations, the
/// outer one (`cb`) padding and colorbars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Margin {
    /// Decorations on the left.
    Left,
    /// Decorations on the right.
    Right,
    /// Decorations below.
    Bottom,
    /// Decorations above.
    Top,
    /// Padding and colorbars on the left.
    LeftCb,
    /// Padding and colorbars on the right.
    RightCb,
    /// Padding and colorbars below.
    BottomCb,
    /// Padding and colorbars above.
    TopCb,
}

impl Margin {
    /// Every margin.
    pub(crate) const ALL: [Self; 8] = [
        Self::Left,
        Self::Right,
        Self::Bottom,
        Self::Top,
        Self::LeftCb,
        Self::RightCb,
        Self::BottomCb,
        Self::TopCb,
    ];

    /// Whether the margin belongs to columns (rather than rows).
    fn horizontal(self) -> bool {
        matches!(
            self,
            Self::Left | Self::Right | Self::LeftCb | Self::RightCb
        )
    }
}

/// The value of every margin of every column or row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Margins([Vec<f64>; 8]);

impl Margins {
    fn zeros(nrows: usize, ncols: usize) -> Self {
        Self(Margin::ALL.map(|m| vec![0.0; if m.horizontal() { ncols } else { nrows }]))
    }

    /// The margin `kind` of column or row `cell`.
    pub(crate) fn get(&self, kind: Margin, cell: usize) -> f64 {
        self.0[kind as usize][cell]
    }

    fn set(&mut self, kind: Margin, cell: usize, value: f64) {
        self.0[kind as usize][cell] = value;
    }

    /// The margins `kind` of `cells`.
    pub(crate) fn span(&self, kind: Margin, cells: Range<usize>) -> &[f64] {
        &self.0[kind as usize][cells]
    }

    /// The four margins of column `col` (left, right and their outer
    /// parts) added together.
    fn column_total(&self, col: usize) -> f64 {
        [Margin::Left, Margin::LeftCb, Margin::Right, Margin::RightCb]
            .iter()
            .map(|&m| self.get(m, col))
            .sum()
    }

    /// The four margins of row `row` added together.
    fn row_total(&self, row: usize) -> f64 {
        [Margin::Top, Margin::TopCb, Margin::Bottom, Margin::BottomCb]
            .iter()
            .map(|&m| self.get(m, row))
            .sum()
    }
}

/// Margins of one cell block for [`LayoutGrid::edit_outer_margin_mins`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct CellMargins {
    pub(crate) left: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) top: f64,
    pub(crate) leftcb: f64,
    pub(crate) rightcb: f64,
    pub(crate) bottomcb: f64,
    pub(crate) topcb: f64,
}

/// What a layout grid sits in.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Parent {
    /// A rectangle `[left, bottom, width, height]` in figure fractions.
    Rect([f64; 4]),
    /// A block of cells of another grid (by index), inside its margins
    /// when `inner`.
    Cells {
        grid: usize,
        rows: Range<usize>,
        cols: Range<usize>,
        inner: bool,
    },
}

/// The solved edges of a grid's cells, in figure fractions.
#[derive(Clone, Debug, PartialEq)]
struct Edges {
    lefts: Vec<f64>,
    rights: Vec<f64>,
    bottoms: Vec<f64>,
    tops: Vec<f64>,
    margins: Margins,
}

/// A grid of cells with margins (matplotlib's `LayoutGrid`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayoutGrid {
    nrows: usize,
    ncols: usize,
    width_ratios: Vec<f64>,
    height_ratios: Vec<f64>,
    parent: Parent,
    /// The suggested margins (matplotlib's `margin_vals`).
    margins: Margins,
    /// The last solution; zero until the first solve, as kiwisolver's
    /// variables are.
    solved: Edges,
}

impl LayoutGrid {
    /// A grid of `nrows × ncols` cells in `parent`, sized by the ratios.
    pub(crate) fn new(
        parent: Parent,
        nrows: usize,
        ncols: usize,
        width_ratios: Vec<f64>,
        height_ratios: Vec<f64>,
    ) -> Self {
        Self {
            nrows,
            ncols,
            width_ratios,
            height_ratios,
            parent,
            margins: Margins::zeros(nrows, ncols),
            solved: Edges {
                lefts: vec![0.0; ncols],
                rights: vec![0.0; ncols],
                bottoms: vec![0.0; nrows],
                tops: vec![0.0; nrows],
                margins: Margins::zeros(nrows, ncols),
            },
        }
    }

    /// A single-cell grid in `parent`.
    pub(crate) fn single(parent: Parent) -> Self {
        Self::new(parent, 1, 1, vec![1.0], vec![1.0])
    }

    /// `(nrows, ncols)`.
    pub(crate) fn shape(&self) -> (usize, usize) {
        (self.nrows, self.ncols)
    }

    /// The suggested margins.
    pub(crate) fn margins(&self) -> &Margins {
        &self.margins
    }

    /// Suggests `size` for margin `kind` of column or row `cell`
    /// (matplotlib's `edit_margin`).
    pub(crate) fn edit_margin(&mut self, kind: Margin, size: f64, cell: usize) {
        self.margins.set(kind, cell, size);
    }

    /// Raises margin `kind` of `cell` to `size` if it is smaller
    /// (matplotlib's `edit_margin_min`).
    pub(crate) fn edit_margin_min(&mut self, kind: Margin, size: f64, cell: usize) {
        if size > self.margins.get(kind, cell) {
            self.edit_margin(kind, size, cell);
        }
    }

    /// Raises the margins of the block `rows × cols` to `margin`
    /// (matplotlib's `edit_outer_margin_mins`).
    pub(crate) fn edit_outer_margin_mins(
        &mut self,
        margin: &CellMargins,
        rows: &Range<usize>,
        cols: &Range<usize>,
    ) {
        let (first_col, last_col) = (cols.start, cols.end - 1);
        let (first_row, last_row) = (rows.start, rows.end - 1);
        self.edit_margin_min(Margin::Left, margin.left, first_col);
        self.edit_margin_min(Margin::LeftCb, margin.leftcb, first_col);
        self.edit_margin_min(Margin::Right, margin.right, last_col);
        self.edit_margin_min(Margin::RightCb, margin.rightcb, last_col);
        self.edit_margin_min(Margin::Top, margin.top, first_row);
        self.edit_margin_min(Margin::TopCb, margin.topcb, first_row);
        self.edit_margin_min(Margin::Bottom, margin.bottom, last_row);
        self.edit_margin_min(Margin::BottomCb, margin.bottomcb, last_row);
    }

    /// Sets every margin back to zero (matplotlib's `reset_margins`).
    pub(crate) fn reset_margins(&mut self) {
        self.margins = Margins::zeros(self.nrows, self.ncols);
    }

    /// The box enclosing the block, margins included.
    pub(crate) fn outer_bbox(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        let e = &self.solved;
        Bbox::from_extents(
            e.lefts[cols.start],
            e.bottoms[rows.end - 1],
            e.rights[cols.end - 1],
            e.tops[rows.start],
        )
    }

    /// The box inside every margin of the block (matplotlib's
    /// `get_inner_bbox`).
    pub(crate) fn inner_bbox(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        self.inset(rows, cols, true)
    }

    /// The box inside the outer margins of the block: the decorations but
    /// not the colorbars (matplotlib's `get_bbox_for_cb`).
    pub(crate) fn bbox_for_cb(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        self.inset(rows, cols, false)
    }

    fn inset(&self, rows: &Range<usize>, cols: &Range<usize>, inner: bool) -> Bbox {
        let e = &self.solved;
        let (c0, c1) = (cols.start, cols.end - 1);
        let (r0, r1) = (rows.start, rows.end - 1);
        let m = |outer: Margin, inner_margin: Margin, cell: usize| {
            e.margins.get(outer, cell)
                + if inner {
                    e.margins.get(inner_margin, cell)
                } else {
                    0.0
                }
        };
        Bbox::from_extents(
            e.lefts[c0] + m(Margin::LeftCb, Margin::Left, c0),
            e.bottoms[r1] + m(Margin::BottomCb, Margin::Bottom, r1),
            e.rights[c1] - m(Margin::RightCb, Margin::Right, c1),
            e.tops[r0] - m(Margin::TopCb, Margin::Top, r0),
        )
    }

    /// The edges of the parent block this grid fills: `(left, bottom,
    /// right, top)`.
    fn parent_edges(&self, grids: &[Self]) -> (f64, f64, f64, f64) {
        match &self.parent {
            Parent::Rect([left, bottom, width, height]) => {
                (*left, *bottom, left + width, bottom + height)
            }
            Parent::Cells {
                grid,
                rows,
                cols,
                inner,
            } => {
                let parent = &grids[*grid];
                let bbox = if *inner {
                    parent.inner_bbox(rows, cols)
                } else {
                    parent.outer_bbox(rows, cols)
                };
                bbox.extents()
            }
        }
    }

    /// Solves the edges inside the parent's solved edges (matplotlib's
    /// constraints, see the module documentation).
    fn solve(&mut self, (left, bottom, right, top): (f64, f64, f64, f64)) {
        let margins = self.margins.clone();
        let widths = cell_sizes(
            right - left,
            &(0..self.ncols)
                .map(|c| margins.column_total(c))
                .collect::<Vec<_>>(),
            &self.width_ratios,
        );
        let heights = cell_sizes(
            top - bottom,
            &(0..self.nrows)
                .map(|r| margins.row_total(r))
                .collect::<Vec<_>>(),
            &self.height_ratios,
        );
        let e = &mut self.solved;
        let mut x = left;
        for (c, width) in widths.into_iter().enumerate() {
            e.lefts[c] = x;
            x += width;
            e.rights[c] = x;
        }
        let mut y = top;
        for (r, height) in heights.into_iter().enumerate() {
            e.tops[r] = y;
            y -= height;
            e.bottoms[r] = y;
        }
        e.margins = margins;
    }
}

/// The outer size of each cell: its margins plus an inner size
/// proportional to its ratio, all adding up to `total`.
fn cell_sizes(total: f64, margins: &[f64], ratios: &[f64]) -> Vec<f64> {
    let unit = (total - margins.iter().sum::<f64>()) / ratios.iter().sum::<f64>();
    margins
        .iter()
        .zip(ratios)
        .map(|(margin, ratio)| margin + unit * ratio)
        .collect()
}

/// The layout grids of a figure: the figure's own first, every other one
/// after its parent.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LayoutGrids(Vec<LayoutGrid>);

impl LayoutGrids {
    /// Adds `grid`, whose parent must already be present, and returns its
    /// index.
    pub(crate) fn push(&mut self, grid: LayoutGrid) -> usize {
        self.0.push(grid);
        self.0.len() - 1
    }

    /// The grid at `index`.
    pub(crate) fn get(&self, index: usize) -> &LayoutGrid {
        &self.0[index]
    }

    /// The grid at `index`, mutably.
    pub(crate) fn get_mut(&mut self, index: usize) -> &mut LayoutGrid {
        &mut self.0[index]
    }

    /// Solves every grid from the root down (matplotlib's
    /// `update_variables`).
    pub(crate) fn solve(&mut self) {
        for index in 0..self.0.len() {
            let edges = self.0[index].parent_edges(&self.0);
            self.0[index].solve(edges);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figure_and_grid(nrows: usize, ncols: usize) -> LayoutGrids {
        let mut grids = LayoutGrids::default();
        grids.push(LayoutGrid::single(Parent::Rect([0.0, 0.0, 1.0, 1.0])));
        grids.push(LayoutGrid::new(
            Parent::Cells {
                grid: 0,
                rows: 0..1,
                cols: 0..1,
                inner: true,
            },
            nrows,
            ncols,
            vec![1.0; ncols],
            vec![1.0; nrows],
        ));
        grids
    }

    #[test]
    fn equal_cells_share_the_room_left_by_margins() {
        let mut grids = figure_and_grid(1, 2);
        let grid = grids.get_mut(1);
        grid.edit_margin_min(Margin::Left, 0.1, 0);
        grid.edit_margin_min(Margin::RightCb, 0.05, 1);
        grids.solve();
        let grid = grids.get(1);
        let left = grid.inner_bbox(&(0..1), &(0..1));
        let right = grid.inner_bbox(&(0..1), &(1..2));
        assert!((left.width() - right.width()).abs() < 1e-15);
        assert!((left.x0() - 0.1).abs() < 1e-15);
        assert!((right.x1() - 0.95).abs() < 1e-15);
        assert!((left.width() - 0.425).abs() < 1e-15);
        assert_eq!(grid.bbox_for_cb(&(0..1), &(1..2)).x1(), 0.95);
    }

    #[test]
    fn margins_only_grow_until_reset() {
        let mut grids = figure_and_grid(2, 1);
        let grid = grids.get_mut(1);
        grid.edit_margin_min(Margin::Top, 0.2, 1);
        grid.edit_margin_min(Margin::Top, 0.1, 1);
        assert_eq!(grid.margins().get(Margin::Top, 1), 0.2);
        grid.reset_margins();
        assert_eq!(grid.margins().get(Margin::Top, 1), 0.0);
    }

    #[test]
    fn unsolved_grids_are_zero_and_rows_run_downwards() {
        let mut grids = figure_and_grid(2, 1);
        assert_eq!(
            grids.get(1).inner_bbox(&(0..1), &(0..1)),
            Bbox::from_extents(0.0, 0.0, 0.0, 0.0)
        );
        grids.solve();
        let top = grids.get(1).outer_bbox(&(0..1), &(0..1));
        assert_eq!((top.y0(), top.y1()), (0.5, 1.0));
    }

    #[test]
    fn crowded_cells_collapse() {
        let mut grids = figure_and_grid(1, 2);
        // Margins wider than the figure leave negative inner widths.
        grids.get_mut(1).edit_margin_min(Margin::Left, 1.2, 1);
        grids.solve();
        assert!(grids.get(1).inner_bbox(&(0..1), &(0..1)).width() < 0.0);
        assert!(grids.get(1).inner_bbox(&(0..1), &(1..2)).width() < 0.0);
    }
}
