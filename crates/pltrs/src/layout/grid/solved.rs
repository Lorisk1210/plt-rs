//! Layout grids as constrained layout left them: plain values read from
//! the solver once the layout is done (what matplotlib's
//! `ConstrainedLayoutEngine.execute` returns, its `layoutgrids`, for
//! debugging and `_layoutgrid.plot_children`).

use crate::artist::ArtistId;
use crate::gridspec::GridSpec;

use super::{GridParent, GridView, LayoutGrids, Margin, PerMargin};

/// What a layout grid lays out (matplotlib's key for it in `layoutgrids`).
#[derive(Clone, Debug, PartialEq)]
pub enum GridOwner {
    /// The figure (`figlb`) or a subfigure (`panellb`): its one cell,
    /// whose margins hold the (sub)figure's titles and labels.
    Frame(ArtistId),
    /// A grid specification (`gridspec`).
    GridSpec(GridSpec),
    /// The one cell over the cells of its parent grid that the nested
    /// grid specification subdivides (`top`, matplotlib's key `(gs,
    /// "top")`).
    Nested(GridSpec),
}

/// A value of every margin of every column or row of a grid, read with
/// [`get`](Self::get).
#[derive(Clone, Debug, PartialEq)]
pub struct MarginValues(PerMargin<f64>);

impl MarginValues {
    /// The values of margin `kind`, one per column for the horizontal
    /// margins ([`Margin::horizontal`]), one per row (from the top) for
    /// the others.
    pub fn get(&self, kind: Margin) -> &[f64] {
        &self.0.0[kind as usize]
    }
}

/// A layout grid of constrained layout, solved (matplotlib's
/// `_layoutgrid.LayoutGrid` once the layout ran): the edges of its
/// columns and rows and its margins, in figure fractions.
#[derive(Clone, Debug, PartialEq)]
pub struct SolvedGrid {
    /// What the grid lays out.
    pub owner: GridOwner,
    /// What the grid sits in.
    pub parent: GridParent,
    /// The relative widths of the columns.
    pub width_ratios: Vec<f64>,
    /// The relative heights of the rows.
    pub height_ratios: Vec<f64>,
    /// The left edge of each column.
    pub lefts: Vec<f64>,
    /// The right edge of each column.
    pub rights: Vec<f64>,
    /// The bottom edge of each row, from the top row.
    pub bottoms: Vec<f64>,
    /// The top edge of each row, from the top row.
    pub tops: Vec<f64>,
    /// The margins as solved.
    pub margins: MarginValues,
    /// The margins last suggested to the solver (matplotlib's
    /// `margin_vals`): zero, as the layout resets them when it is done.
    pub margin_vals: MarginValues,
}

impl LayoutGrids {
    /// The grid at `index` as solved last, laying out what `owner` is.
    pub(crate) fn solved(&self, index: usize, owner: GridOwner) -> SolvedGrid {
        let view = self.get(index);
        let grid = view.grid;
        let values = |variables: &[super::Variable]| -> Vec<f64> {
            variables.iter().map(|&v| view.value(v)).collect()
        };
        SolvedGrid {
            owner,
            parent: grid.parent.clone(),
            width_ratios: grid.width_ratios.clone(),
            height_ratios: grid.height_ratios.clone(),
            lefts: values(&grid.lefts),
            rights: values(&grid.rights),
            bottoms: values(&grid.bottoms),
            tops: values(&grid.tops),
            margins: MarginValues(solved_margins(&view)),
            margin_vals: MarginValues(grid.margin_vals.clone()),
        }
    }
}

/// The solved value of every margin of `view`.
fn solved_margins(view: &GridView<'_>) -> PerMargin<f64> {
    PerMargin(
        view.grid
            .margins
            .0
            .each_ref()
            .map(|variables| variables.iter().map(|&v| view.value(v)).collect()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solved_grids_read_the_last_solution() {
        let mut grids = LayoutGrids::default();
        let figure = GridParent::Rect([0.0, 0.0, 1.0, 1.0]);
        grids.push(&figure, &[1.0, 3.0], &[1.0]).unwrap();
        let unsolved = grids.solved(0, GridOwner::Frame(ArtistId::from_index(0)));
        assert_eq!(unsolved.rights, [0.0, 0.0], "zero before the solver ran");
        grids
            .get_mut(0)
            .edit_margin_min(Margin::Left, 0.2, 1)
            .unwrap();
        grids.solve();
        grids.get_mut(0).reset_margins().unwrap();
        let solved = grids.solved(0, GridOwner::Frame(ArtistId::from_index(0)));
        assert_eq!(solved.parent, figure);
        assert_eq!(solved.width_ratios, [1.0, 3.0]);
        assert_eq!((solved.lefts[0], solved.rights[1]), (0.0, 1.0));
        assert_eq!((solved.bottoms, solved.tops), (vec![0.0], vec![1.0]));
        assert_eq!(solved.margins.get(Margin::Left), [0.0, 0.2]);
        assert_eq!(solved.margins.get(Margin::Top), [0.0]);
        assert_eq!(
            solved.margin_vals.get(Margin::Left),
            [0.0, 0.0],
            "the suggestions were reset, the solution stays"
        );
    }
}
