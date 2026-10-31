//! The constraints of a layout grid, stated as matplotlib's
//! `LayoutGrid.add_constraints` states them: in its order, with its
//! strengths and its expressions.

use super::super::kiwi::{Constraint, Expression, Relation, SolverError, Variable, strength};
use super::{GridParent, LayoutGrid, LayoutGrids, Margin};

/// The edges of one cell, outer and margins, along one direction: left
/// and right of a column, or bottom and top of a row, with the margins
/// on those sides.
struct Span {
    low: Variable,
    high: Variable,
    low_margins: [Variable; 2],
    high_margins: [Variable; 2],
}

impl Span {
    /// Column `cell` of `grid`.
    fn column(grid: &LayoutGrid, cell: usize) -> Self {
        let m = |kind| grid.margins.get(kind, cell);
        Self {
            low: grid.lefts[cell],
            high: grid.rights[cell],
            low_margins: [m(Margin::Left), m(Margin::LeftCb)],
            high_margins: [m(Margin::Right), m(Margin::RightCb)],
        }
    }

    /// Row `cell` of `grid` (its top is the high edge).
    fn row(grid: &LayoutGrid, cell: usize) -> Self {
        let m = |kind| grid.margins.get(kind, cell);
        Self {
            low: grid.bottoms[cell],
            high: grid.tops[cell],
            low_margins: [m(Margin::Bottom), m(Margin::BottomCb)],
            high_margins: [m(Margin::Top), m(Margin::TopCb)],
        }
    }

    /// `high - margins - lefts - margins`: matplotlib's inner width
    /// (height) expression.
    fn inner(&self) -> Expression {
        let [high_inner, high_outer] = self.high_margins;
        let [low_inner, low_outer] = self.low_margins;
        (self.high - high_inner) - high_outer - self.low - low_inner - low_outer
    }

    /// The constraints of `hard_constraints`: the cell and its inner box
    /// have no negative size.
    fn hard_constraints(&self) -> [Constraint; 2] {
        let [high_inner, high_outer] = self.high_margins;
        let [low_inner, low_outer] = self.low_margins;
        [
            Constraint::new(self.high, Relation::Ge, self.low, strength::REQUIRED),
            Constraint::new(
                (self.high - high_inner) - high_outer,
                Relation::Ge,
                (self.low - low_inner) - low_outer,
                strength::REQUIRED,
            ),
        ]
    }
}

impl LayoutGrids {
    /// matplotlib's `add_constraints` for the grid `index`: its hard
    /// constraints, those tying it to its parent, then those of its cells.
    pub(super) fn add_constraints(&mut self, index: usize) -> Result<(), SolverError> {
        let grid = &self.grids[index];
        let (ncols, nrows) = (grid.lefts.len(), grid.bottoms.len());
        let columns: Vec<Span> = (0..ncols).map(|c| Span::column(grid, c)).collect();
        let rows: Vec<Span> = (0..nrows).map(|r| Span::row(grid, r)).collect();
        let mut constraints: Vec<Constraint> = columns
            .iter()
            .chain(&rows)
            .flat_map(Span::hard_constraints)
            .collect();
        constraints.extend(self.parent_constraints(grid, &grid.parent));
        constraints.extend(cell_constraints(
            &columns,
            &grid.width_ratios,
            |prev, next| (prev.high, next.low),
        ));
        constraints.extend(cell_constraints(
            &rows,
            &grid.height_ratios,
            |prev, next| (prev.low, next.high),
        ));
        constraints
            .iter()
            .try_for_each(|constraint| self.solver.add_constraint(constraint))
    }

    /// matplotlib's `parent_constraints`: the outer edges of `grid` are
    /// those of the parent's rectangle, or of the parent's cells (inside
    /// their margins when the grid sits inside them).
    fn parent_constraints(&self, grid: &LayoutGrid, parent: &GridParent) -> [Constraint; 4] {
        let (left, right) = (grid.lefts[0], grid.rights[grid.rights.len() - 1]);
        let (top, bottom) = (grid.tops[0], grid.bottoms[grid.bottoms.len() - 1]);
        let eq = |lhs, rhs| Constraint::new(lhs, Relation::Eq, rhs, strength::REQUIRED);
        let eq_expr =
            |lhs, rhs: Expression| Constraint::new(lhs, Relation::Eq, rhs, strength::REQUIRED);
        match parent {
            GridParent::Rect([x, y, width, height]) => [
                Constraint::new(left, Relation::Eq, *x, strength::REQUIRED),
                Constraint::new(right, Relation::Eq, x + width, strength::REQUIRED),
                Constraint::new(top, Relation::Eq, y + height, strength::REQUIRED),
                Constraint::new(bottom, Relation::Eq, *y, strength::REQUIRED),
            ],
            GridParent::Cells {
                grid: outer,
                rows,
                cols,
                inner,
            } => {
                let outer = &self.grids[*outer];
                let first = Span::column(outer, cols.start);
                let last = Span::column(outer, cols.end - 1);
                let top_row = Span::row(outer, rows.start);
                let bottom_row = Span::row(outer, rows.end - 1);
                if *inner {
                    let [l_inner, l_outer] = first.low_margins;
                    let [r_inner, r_outer] = last.high_margins;
                    let [t_inner, t_outer] = top_row.high_margins;
                    let [b_inner, b_outer] = bottom_row.low_margins;
                    [
                        eq_expr(left, (first.low + l_inner) + l_outer),
                        eq_expr(right, (last.high - r_inner) - r_outer),
                        eq_expr(top, (top_row.high - t_inner) - t_outer),
                        eq_expr(bottom, (bottom_row.low + b_inner) + b_outer),
                    ]
                } else {
                    [
                        eq(left, first.low),
                        eq(right, last.high),
                        eq(top, top_row.high),
                        eq(bottom, bottom_row.low),
                    ]
                }
            }
        }
    }
}

/// matplotlib's `grid_constraints` along one direction: the inner size of
/// every cell after the first is the first's scaled by the ratios, and
/// each cell touches the one before it at the edges `touching` gives.
fn cell_constraints(
    cells: &[Span],
    ratios: &[f64],
    touching: impl Fn(&Span, &Span) -> (Variable, Variable),
) -> Vec<Constraint> {
    let unit = cells[0].inner() / ratios[0];
    cells
        .windows(2)
        .zip(&ratios[1..])
        .flat_map(|(pair, &ratio)| {
            let (prev, next) = touching(&pair[0], &pair[1]);
            [
                Constraint::new(
                    pair[1].inner(),
                    Relation::Eq,
                    unit.clone() * ratio,
                    strength::STRONG,
                ),
                Constraint::new(prev, Relation::Eq, next, strength::STRONG),
            ]
        })
        .collect()
}
