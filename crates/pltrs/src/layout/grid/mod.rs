//! Layout grids: the rows and columns of a figure or grid specification,
//! each cell with editable margins around its inner box (matplotlib's
//! `_layoutgrid.LayoutGrid`).
//!
//! As in matplotlib, the grids of a figure share one constraint solver
//! (the [`kiwi`](super::kiwi) port of kiwisolver): the outer edges of a
//! grid equal its parent's (inner) edges, neighbouring cells touch, inner
//! sizes follow the width and height ratios, and the margins are edit
//! variables taking their suggested values. Every grid states its
//! variables and constraints, and every margin edit reaches the solver,
//! in matplotlib's order, so that the solved edges are matplotlib's to
//! the last bit. When the margins leave no room, inner sizes come out
//! negative and the layout counts as collapsed, as in matplotlib.

mod constraints;
mod solved;

pub use solved::{GridOwner, MarginValues, SolvedGrid};

use std::ops::Range;

use pltrs_core::Bbox;

use super::kiwi::{Solver, SolverError, Variable, strength};

/// The margins of a cell side of a layout grid: the inner one holds axes
/// decorations, the outer one (`cb`) padding and colorbars.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Margin {
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
    /// Every margin, in the order matplotlib resets them.
    pub const ALL: [Self; 8] = [
        Self::Left,
        Self::Right,
        Self::Bottom,
        Self::Top,
        Self::LeftCb,
        Self::RightCb,
        Self::BottomCb,
        Self::TopCb,
    ];

    /// The margins of columns, in the order matplotlib makes them.
    const HORIZONTAL: [Self; 4] = [Self::Left, Self::Right, Self::LeftCb, Self::RightCb];

    /// The margins of rows, in the order matplotlib makes them.
    const VERTICAL: [Self; 4] = [Self::Bottom, Self::Top, Self::BottomCb, Self::TopCb];

    /// Whether the margin belongs to columns (rather than rows).
    pub fn horizontal(self) -> bool {
        Self::HORIZONTAL.contains(&self)
    }

    /// matplotlib's key for the margin (`"left"`, `"leftcb"`, ...).
    pub fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Top => "top",
            Self::LeftCb => "leftcb",
            Self::RightCb => "rightcb",
            Self::BottomCb => "bottomcb",
            Self::TopCb => "topcb",
        }
    }
}

/// A value per margin and column or row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PerMargin<T>([Vec<T>; 8]);

impl<T: Copy> PerMargin<T> {
    /// The value of margin `kind` of column or row `cell`.
    pub(crate) fn get(&self, kind: Margin, cell: usize) -> T {
        self.0[kind as usize][cell]
    }

    /// The values of margin `kind` of `cells`.
    pub(crate) fn span(&self, kind: Margin, cells: Range<usize>) -> &[T] {
        &self.0[kind as usize][cells]
    }
}

/// The suggested value of every margin of every column or row
/// (matplotlib's `margin_vals`).
pub(crate) type Margins = PerMargin<f64>;

impl Margins {
    fn zeros(nrows: usize, ncols: usize) -> Self {
        Self(Margin::ALL.map(|m| vec![0.0; if m.horizontal() { ncols } else { nrows }]))
    }

    fn set(&mut self, kind: Margin, cell: usize, value: f64) {
        self.0[kind as usize][cell] = value;
    }
}

/// Margins of one cell block for [`GridEdit::edit_outer_margin_mins`].
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

/// What a layout grid sits in (matplotlib's `parent`, `parent_pos` and
/// `parent_inner`).
#[derive(Clone, Debug, PartialEq)]
pub enum GridParent {
    /// A rectangle `[left, bottom, width, height]` in figure fractions:
    /// the figure's grid.
    Rect([f64; 4]),
    /// A block of cells of another grid.
    Cells {
        /// The other grid's index among the grids of the figure.
        grid: usize,
        /// The rows of the block, from the top.
        rows: Range<usize>,
        /// The columns of the block.
        cols: Range<usize>,
        /// Whether the grid sits inside the margins of the block.
        inner: bool,
    },
}

/// The solver variables of a grid of cells with margins (matplotlib's
/// `LayoutGrid`), with what it sits in and its ratios.
#[derive(Clone, Debug, PartialEq)]
struct LayoutGrid {
    parent: GridParent,
    width_ratios: Vec<f64>,
    height_ratios: Vec<f64>,
    lefts: Vec<Variable>,
    rights: Vec<Variable>,
    bottoms: Vec<Variable>,
    tops: Vec<Variable>,
    /// The margin edit variables.
    margins: PerMargin<Variable>,
    /// The suggested margins.
    margin_vals: Margins,
}

impl LayoutGrid {
    /// The variables of a grid of `width_ratios.len()` columns and
    /// `height_ratios.len()` rows in `parent`, with its margins made edit
    /// variables, in matplotlib's order.
    fn new(
        solver: &mut Solver,
        parent: &GridParent,
        width_ratios: &[f64],
        height_ratios: &[f64],
    ) -> Result<Self, SolverError> {
        let (nrows, ncols) = (height_ratios.len(), width_ratios.len());
        let lefts = variables(solver, ncols);
        let rights = variables(solver, ncols);
        let mut margins: [Vec<Variable>; 8] = Default::default();
        for kind in Margin::HORIZONTAL {
            margins[kind as usize] = edit_variables(solver, ncols)?;
        }
        let bottoms = variables(solver, nrows);
        let tops = variables(solver, nrows);
        for kind in Margin::VERTICAL {
            margins[kind as usize] = edit_variables(solver, nrows)?;
        }
        Ok(Self {
            parent: parent.clone(),
            width_ratios: width_ratios.to_vec(),
            height_ratios: height_ratios.to_vec(),
            lefts,
            rights,
            bottoms,
            tops,
            margins: PerMargin(margins),
            margin_vals: Margins::zeros(nrows, ncols),
        })
    }
}

/// `n` new variables.
fn variables(solver: &mut Solver, n: usize) -> Vec<Variable> {
    (0..n).map(|_| solver.new_variable()).collect()
}

/// `n` new edit variables of strong strength.
fn edit_variables(solver: &mut Solver, n: usize) -> Result<Vec<Variable>, SolverError> {
    (0..n)
        .map(|_| {
            let variable = solver.new_variable();
            solver.add_edit_variable(variable, strength::STRONG)?;
            Ok(variable)
        })
        .collect()
}

/// The layout grids of a figure and their solver: the figure's own grid
/// first, every other one after its parent.
#[derive(Debug, Default)]
pub(crate) struct LayoutGrids {
    solver: Solver,
    grids: Vec<LayoutGrid>,
}

impl LayoutGrids {
    /// Adds a grid of a column per width ratio and a row per height ratio
    /// in `parent`, which must already be present, and returns its index
    /// (matplotlib's `LayoutGrid(...)`).
    pub(crate) fn push(
        &mut self,
        parent: &GridParent,
        width_ratios: &[f64],
        height_ratios: &[f64],
    ) -> Result<usize, SolverError> {
        let grid = LayoutGrid::new(&mut self.solver, parent, width_ratios, height_ratios)?;
        self.grids.push(grid);
        let index = self.grids.len() - 1;
        self.get_mut(index).reset_margins()?;
        self.add_constraints(index)?;
        Ok(index)
    }

    /// Adds a single-cell grid in `parent` and returns its index.
    pub(crate) fn push_single(&mut self, parent: &GridParent) -> Result<usize, SolverError> {
        self.push(parent, &[1.0], &[1.0])
    }

    /// The grid at `index`.
    pub(crate) fn get(&self, index: usize) -> GridView<'_> {
        GridView {
            grid: &self.grids[index],
            solver: &self.solver,
        }
    }

    /// The grid at `index`, to edit its margins.
    pub(crate) fn get_mut(&mut self, index: usize) -> GridEdit<'_> {
        GridEdit {
            grid: &mut self.grids[index],
            solver: &mut self.solver,
        }
    }

    /// Takes the solution of every grid (matplotlib's `update_variables`).
    pub(crate) fn solve(&mut self) {
        self.solver.update_variables();
    }
}

/// A grid with its solved edges (matplotlib's `get_*_bbox` methods, which
/// read the values of the last solution: zero until the first).
#[derive(Clone, Copy, Debug)]
pub(crate) struct GridView<'a> {
    grid: &'a LayoutGrid,
    solver: &'a Solver,
}

impl<'a> GridView<'a> {
    /// `(nrows, ncols)`.
    pub(crate) fn shape(&self) -> (usize, usize) {
        (self.grid.bottoms.len(), self.grid.lefts.len())
    }

    /// The suggested margins.
    pub(crate) fn margins(&self) -> &'a Margins {
        &self.grid.margin_vals
    }

    /// The solved value of `variable`.
    fn value(&self, variable: Variable) -> f64 {
        self.solver.value(variable)
    }

    /// The solved margin `kind` of `cell`.
    fn margin(&self, kind: Margin, cell: usize) -> f64 {
        self.value(self.grid.margins.get(kind, cell))
    }

    /// The box enclosing the block, margins included (matplotlib's
    /// `get_outer_bbox`).
    pub(crate) fn outer_bbox(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        let g = self.grid;
        Bbox::from_extents(
            self.value(g.lefts[cols.start]),
            self.value(g.bottoms[rows.end - 1]),
            self.value(g.rights[cols.end - 1]),
            self.value(g.tops[rows.start]),
        )
    }

    /// The box inside every margin of the block (matplotlib's
    /// `get_inner_bbox`).
    pub(crate) fn inner_bbox(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        let g = self.grid;
        let (c0, c1, r0, r1) = (cols.start, cols.end - 1, rows.start, rows.end - 1);
        Bbox::from_extents(
            self.value(g.lefts[c0])
                + self.margin(Margin::Left, c0)
                + self.margin(Margin::LeftCb, c0),
            self.value(g.bottoms[r1])
                + self.margin(Margin::Bottom, r1)
                + self.margin(Margin::BottomCb, r1),
            self.value(g.rights[c1])
                - self.margin(Margin::Right, c1)
                - self.margin(Margin::RightCb, c1),
            self.value(g.tops[r0]) - self.margin(Margin::Top, r0) - self.margin(Margin::TopCb, r0),
        )
    }

    /// The box inside the outer margins of the block: the decorations but
    /// not the colorbars (matplotlib's `get_bbox_for_cb`).
    pub(crate) fn bbox_for_cb(&self, rows: &Range<usize>, cols: &Range<usize>) -> Bbox {
        let g = self.grid;
        let (c0, c1, r0, r1) = (cols.start, cols.end - 1, rows.start, rows.end - 1);
        Bbox::from_extents(
            self.value(g.lefts[c0]) + self.margin(Margin::LeftCb, c0),
            self.value(g.bottoms[r1]) + self.margin(Margin::BottomCb, r1),
            self.value(g.rights[c1]) - self.margin(Margin::RightCb, c1),
            self.value(g.tops[r0]) - self.margin(Margin::TopCb, r0),
        )
    }
}

/// A grid whose margins are being edited; each edit suggests the new
/// value to the solver.
#[derive(Debug)]
pub(crate) struct GridEdit<'a> {
    grid: &'a mut LayoutGrid,
    solver: &'a mut Solver,
}

impl GridEdit<'_> {
    /// Suggests `size` for margin `kind` of column or row `cell`
    /// (matplotlib's `edit_margin`).
    pub(crate) fn edit_margin(
        &mut self,
        kind: Margin,
        size: f64,
        cell: usize,
    ) -> Result<(), SolverError> {
        self.solver
            .suggest_value(self.grid.margins.get(kind, cell), size)?;
        self.grid.margin_vals.set(kind, cell, size);
        Ok(())
    }

    /// Raises margin `kind` of `cell` to `size` if it is smaller
    /// (matplotlib's `edit_margin_min`).
    pub(crate) fn edit_margin_min(
        &mut self,
        kind: Margin,
        size: f64,
        cell: usize,
    ) -> Result<(), SolverError> {
        if size > self.grid.margin_vals.get(kind, cell) {
            self.edit_margin(kind, size, cell)?;
        }
        Ok(())
    }

    /// Raises the margins of the block `rows × cols` to `margin`
    /// (matplotlib's `edit_outer_margin_mins`).
    pub(crate) fn edit_outer_margin_mins(
        &mut self,
        margin: &CellMargins,
        rows: &Range<usize>,
        cols: &Range<usize>,
    ) -> Result<(), SolverError> {
        let (first_col, last_col) = (cols.start, cols.end - 1);
        let (first_row, last_row) = (rows.start, rows.end - 1);
        self.edit_margin_min(Margin::Left, margin.left, first_col)?;
        self.edit_margin_min(Margin::LeftCb, margin.leftcb, first_col)?;
        self.edit_margin_min(Margin::Right, margin.right, last_col)?;
        self.edit_margin_min(Margin::RightCb, margin.rightcb, last_col)?;
        self.edit_margin_min(Margin::Top, margin.top, first_row)?;
        self.edit_margin_min(Margin::TopCb, margin.topcb, first_row)?;
        self.edit_margin_min(Margin::Bottom, margin.bottom, last_row)?;
        self.edit_margin_min(Margin::BottomCb, margin.bottomcb, last_row)
    }

    /// Sets every margin back to zero (matplotlib's `reset_margins`).
    pub(crate) fn reset_margins(&mut self) -> Result<(), SolverError> {
        for kind in Margin::ALL {
            for cell in 0..self.grid.margin_vals.0[kind as usize].len() {
                self.edit_margin(kind, 0.0, cell)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn figure_and_grid(nrows: usize, ncols: usize) -> LayoutGrids {
        let mut grids = LayoutGrids::default();
        grids
            .push_single(&GridParent::Rect([0.0, 0.0, 1.0, 1.0]))
            .unwrap();
        let parent = GridParent::Cells {
            grid: 0,
            rows: 0..1,
            cols: 0..1,
            inner: true,
        };
        grids
            .push(&parent, &vec![1.0; ncols], &vec![1.0; nrows])
            .unwrap();
        grids
    }

    #[test]
    fn equal_cells_share_the_room_left_by_margins() {
        let mut grids = figure_and_grid(1, 2);
        let mut grid = grids.get_mut(1);
        grid.edit_margin_min(Margin::Left, 0.1, 0).unwrap();
        grid.edit_margin_min(Margin::RightCb, 0.05, 1).unwrap();
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
        let mut grid = grids.get_mut(1);
        grid.edit_margin_min(Margin::Top, 0.2, 1).unwrap();
        grid.edit_margin_min(Margin::Top, 0.1, 1).unwrap();
        assert_eq!(grids.get(1).margins().get(Margin::Top, 1), 0.2);
        grids.get_mut(1).reset_margins().unwrap();
        assert_eq!(grids.get(1).margins().get(Margin::Top, 1), 0.0);
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
    fn crowded_cells_collapse_as_in_matplotlib() {
        let mut grids = figure_and_grid(1, 2);
        // Margins wider than the figure leave no room: matplotlib 3.11
        // (kiwisolver 1.5.1) solves this to the bit.
        grids
            .get_mut(1)
            .edit_margin_min(Margin::Left, 1.2, 1)
            .unwrap();
        grids.solve();
        let first = grids.get(1).inner_bbox(&(0..1), &(0..1));
        let second = grids.get(1).inner_bbox(&(0..1), &(1..2));
        assert_eq!(
            [first.x0(), first.x1()].map(f64::to_bits),
            [0xBFC9_9999_9999_9996, 0xBFC9_9999_9999_9997]
        );
        assert_eq!((second.x0(), second.x1()), (1.0, 1.0));
    }

    #[test]
    fn ratios_size_the_inner_boxes() {
        let mut grids = LayoutGrids::default();
        let figure = GridParent::Rect([0.0, 0.0, 1.0, 1.0]);
        grids.push(&figure, &[1.0, 3.0], &[1.0]).unwrap();
        grids
            .get_mut(0)
            .edit_margin_min(Margin::Left, 0.2, 1)
            .unwrap();
        grids.solve();
        let grid = grids.get(0);
        let (left, right) = (
            grid.inner_bbox(&(0..1), &(0..1)),
            grid.inner_bbox(&(0..1), &(1..2)),
        );
        assert!((right.width() - 3.0 * left.width()).abs() < 1e-15);
        assert!((left.width() - 0.2).abs() < 1e-15);
        assert_eq!(
            grid.outer_bbox(&(0..1), &(0..2)).extents(),
            (0.0, 0.0, 1.0, 1.0)
        );
    }
}
