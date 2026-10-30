//! Grid layouts for subplots (matplotlib.gridspec).
//!
//! A [`GridSpec`] divides a figure region into rows and columns; a
//! [`SubplotSpec`] selects a rectangular block of cells and computes the
//! axes position exactly as matplotlib's `get_grid_positions` does.

use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

use pltrs_core::Bbox;

use crate::artist::ArtistHandle;
use crate::figure::Figure;

mod display;
mod params;

pub use params::{GridSpecError, SubplotOverrides, SubplotParams};

#[derive(Debug, PartialEq)]
struct GridData {
    nrows: usize,
    ncols: usize,
    width_ratios: Vec<f64>,
    height_ratios: Vec<f64>,
    /// The grid's own margins and spacing; tight layout of the grid
    /// changes them ([`GridSpec::tight_layout`]).
    overrides: Cell<SubplotOverrides>,
    /// For nested grids: the cell block this grid subdivides.
    parent: Option<SubplotSpec>,
    /// Whether constrained layout places the axes on the grid.
    in_layout: Cell<bool>,
}

pltrs_core::archive_struct!(GridData {
    nrows,
    ncols,
    width_ratios,
    height_ratios,
    overrides,
    parent,
    in_layout
});

/// A grid of `nrows × ncols` cells (cheap to clone; clones are the same grid).
#[derive(Clone, Debug, PartialEq)]
pub struct GridSpec {
    data: Rc<GridData>,
}

pltrs_core::archive_struct!(GridSpec { data });

impl GridSpec {
    /// A grid with equal cells.
    pub fn new(nrows: usize, ncols: usize) -> Result<Self, GridSpecError> {
        Self::with_ratios(nrows, ncols, None, None, SubplotOverrides::default())
    }

    /// A grid with relative column widths / row heights and layout overrides.
    pub fn with_ratios(
        nrows: usize,
        ncols: usize,
        width_ratios: Option<Vec<f64>>,
        height_ratios: Option<Vec<f64>>,
        overrides: SubplotOverrides,
    ) -> Result<Self, GridSpecError> {
        Self::build(nrows, ncols, width_ratios, height_ratios, overrides, None)
    }

    fn build(
        nrows: usize,
        ncols: usize,
        width_ratios: Option<Vec<f64>>,
        height_ratios: Option<Vec<f64>>,
        overrides: SubplotOverrides,
        parent: Option<SubplotSpec>,
    ) -> Result<Self, GridSpecError> {
        if nrows == 0 || ncols == 0 {
            return Err(GridSpecError::EmptyGrid);
        }
        let ratios = |given: Option<Vec<f64>>, n: usize| match given {
            Some(r) if r.len() != n => Err(GridSpecError::RatioCount {
                expected: n,
                actual: r.len(),
            }),
            Some(r) => Ok(r),
            None => Ok(vec![1.0; n]),
        };
        Ok(Self {
            data: Rc::new(GridData {
                nrows,
                ncols,
                width_ratios: ratios(width_ratios, ncols)?,
                height_ratios: ratios(height_ratios, nrows)?,
                overrides: Cell::new(overrides),
                parent,
                in_layout: Cell::new(true),
            }),
        })
    }

    /// Sets whether constrained layout places the axes on the grid, as it
    /// does by default; matplotlib leaves out the grids made without a
    /// figure (`GridSpec(nrows, ncols)` with no `figure`).
    pub fn set_in_layout(&self, in_layout: bool) {
        self.data.in_layout.set(in_layout);
    }

    /// Whether constrained layout places the axes on the grid: unless it
    /// or, for a nested grid, the grid it subdivides was taken out
    /// ([`GridSpec::set_in_layout`]).
    ///
    /// ```
    /// use pltrs::gridspec::GridSpec;
    /// let outer = GridSpec::new(1, 2)?;
    /// let inner = GridSpec::nested(&outer.cell(0, 0)?, 2, 2, None, None, (None, None))?;
    /// assert!(inner.in_layout());
    /// outer.set_in_layout(false);
    /// assert!(!inner.in_layout());
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn in_layout(&self) -> bool {
        self.data.in_layout.get()
            && self
                .data
                .parent
                .as_ref()
                .is_none_or(|cell| cell.gridspec().in_layout())
    }

    /// `(nrows, ncols)`.
    pub fn shape(&self) -> (usize, usize) {
        (self.data.nrows, self.data.ncols)
    }

    /// The relative column widths.
    pub fn width_ratios(&self) -> &[f64] {
        &self.data.width_ratios
    }

    /// The relative row heights.
    pub fn height_ratios(&self) -> &[f64] {
        &self.data.height_ratios
    }

    /// The `(wspace, hspace)` the grid sets itself, if any.
    pub fn spacing(&self) -> (Option<f64>, Option<f64>) {
        let o = self.overrides();
        (o.wspace, o.hspace)
    }

    /// The margins and spacing the grid sets itself (matplotlib's `left`,
    /// `bottom`, `right`, `top`, `wspace` and `hspace` of a grid).
    pub fn overrides(&self) -> SubplotOverrides {
        self.data.overrides.get()
    }

    /// Replaces the margins and spacing given in `changes`, keeping the
    /// others, and moves the axes of `figure` placed on the grid to match
    /// (matplotlib's `GridSpec.update`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::gridspec::{GridSpec, SubplotOverrides};
    /// let fig = Figure::new();
    /// fig.set_layout_engine(None)?;
    /// let grid = GridSpec::new(2, 1)?;
    /// let ax = fig.add_subplot_spec(&grid.cell(0, 0)?)?;
    /// let lower_top = SubplotOverrides { top: Some(0.5), ..SubplotOverrides::default() };
    /// grid.update(&fig, lower_top)?;
    /// assert_eq!(grid.overrides().top, Some(0.5));
    /// let (_, y, _, height) = ax.position().bounds();
    /// assert!((y + height - 0.5).abs() < 1e-12);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the new margins leave no room for the grid, which is then left
    /// unchanged.
    pub fn update(
        &self,
        figure: &Figure,
        changes: SubplotOverrides,
    ) -> Result<&Self, GridSpecError> {
        let previous = self.overrides();
        self.merge_overrides(&changes);
        let params = figure.subplot_params();
        if let Err(err) = self.subplot_params(&params) {
            self.data.overrides.set(previous);
            return Err(err);
        }
        figure
            .raw()
            .figure_state()
            .borrow_mut()
            .set_subplot_params(params)?;
        Ok(self)
    }

    /// Replaces the margins and spacing the grid sets itself, all of them,
    /// without moving any axes (matplotlib's `left`, ..., `hspace`
    /// attributes of a grid, set one by one; [`update`](Self::update)
    /// moves the axes of a figure as well).
    ///
    /// ```
    /// use pltrs::gridspec::{GridSpec, SubplotOverrides};
    /// let grid = GridSpec::new(2, 1)?;
    /// grid.set_overrides(SubplotOverrides { left: Some(0.1), ..SubplotOverrides::default() });
    /// assert_eq!(grid.overrides().left, Some(0.1));
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn set_overrides(&self, overrides: SubplotOverrides) {
        self.data.overrides.set(overrides);
    }

    /// Replaces the margins and spacing given in `changes`, keeping the
    /// others, without moving any axes.
    pub(crate) fn merge_overrides(&self, changes: &SubplotOverrides) {
        let o = self.overrides();
        self.data.overrides.set(SubplotOverrides {
            left: changes.left.or(o.left),
            bottom: changes.bottom.or(o.bottom),
            right: changes.right.or(o.right),
            top: changes.top.or(o.top),
            wspace: changes.wspace.or(o.wspace),
            hspace: changes.hspace.or(o.hspace),
        });
    }

    /// Whether the grid sets non-zero margins or spacings of its own
    /// (matplotlib's `locally_modified_subplot_params`, which tests them
    /// for truth).
    pub fn locally_modified(&self) -> bool {
        let o = self.overrides();
        [o.left, o.bottom, o.right, o.top, o.wspace, o.hspace]
            .into_iter()
            .any(|value| value.is_some_and(|v| v != 0.0))
    }

    /// The cell block a nested grid subdivides (`None` for a figure-level
    /// grid).
    pub fn parent(&self) -> Option<&SubplotSpec> {
        self.data.parent.as_ref()
    }

    /// The block of a figure-level grid this nested grid lies in, through
    /// every level of nesting (matplotlib's
    /// `GridSpecFromSubplotSpec.get_topmost_subplotspec`); `None` for a
    /// figure-level grid.
    ///
    /// ```
    /// use pltrs::gridspec::GridSpec;
    /// let cell = GridSpec::new(1, 2)?.cell(0, 1)?;
    /// let inner = GridSpec::nested(&cell, 2, 1, None, None, (None, None))?;
    /// let innermost = GridSpec::nested(&inner.cell(1, 0)?, 1, 3, None, None, (None, None))?;
    /// assert_eq!(innermost.topmost_subplotspec(), Some(cell));
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn topmost_subplotspec(&self) -> Option<SubplotSpec> {
        self.parent().map(SubplotSpec::topmost_subplotspec)
    }

    /// Whether `other` is this very grid rather than an equal copy (grids
    /// are compared by identity where matplotlib uses them as keys).
    pub fn is_same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }

    /// The single cell at `(row, col)`.
    pub fn cell(&self, row: usize, col: usize) -> Result<SubplotSpec, GridSpecError> {
        self.span(row..row + 1, col..col + 1)
    }

    /// The block of cells covering `rows × cols`.
    pub fn span(
        &self,
        rows: Range<usize>,
        cols: Range<usize>,
    ) -> Result<SubplotSpec, GridSpecError> {
        let (nrows, ncols) = self.shape();
        if rows.is_empty() || cols.is_empty() || rows.end > nrows || cols.end > ncols {
            return Err(GridSpecError::OutOfRange);
        }
        Ok(SubplotSpec {
            gridspec: self.clone(),
            rows,
            cols,
        })
    }

    /// The cell with 1-based, row-major `index` (matplotlib's
    /// `add_subplot(nrows, ncols, index)` numbering).
    pub fn numbered(&self, index: usize) -> Result<SubplotSpec, GridSpecError> {
        let (nrows, ncols) = self.shape();
        if index == 0 || index > nrows * ncols {
            return Err(GridSpecError::OutOfRange);
        }
        let i = index - 1;
        self.cell(i / ncols, i % ncols)
    }

    /// The block from cell `first` to cell `last` (1-based, row-major,
    /// inclusive): every row between theirs and every column between
    /// theirs (matplotlib's `add_subplot(nrows, ncols, (first, last))`).
    ///
    /// ```
    /// use pltrs::gridspec::GridSpec;
    /// let spec = GridSpec::new(3, 2)?.numbered_span(4, 5)?;
    /// assert_eq!((spec.rows(), spec.cols()), (1..3, 0..2));
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a cell is outside the grid or `last` comes before `first`.
    pub fn numbered_span(&self, first: usize, last: usize) -> Result<SubplotSpec, GridSpecError> {
        let (nrows, ncols) = self.shape();
        if first == 0 || first > last || last > nrows * ncols {
            return Err(GridSpecError::OutOfRange);
        }
        let (a, b) = (first - 1, last - 1);
        let (col_a, col_b) = (a % ncols, b % ncols);
        self.span(
            a / ncols..b / ncols + 1,
            col_a.min(col_b)..col_a.max(col_b) + 1,
        )
    }

    /// The layout parameters in effect: the parent cell's extent for nested
    /// grids, otherwise the figure's parameters, then this grid's overrides.
    pub fn subplot_params(&self, figure: &SubplotParams) -> Result<SubplotParams, GridSpecError> {
        let o = &self.overrides();
        let base = match &self.data.parent {
            Some(parent) => {
                let (left, bottom, right, top) = parent.position(figure)?.extents();
                SubplotParams {
                    left,
                    bottom,
                    right,
                    top,
                    wspace: o.wspace.unwrap_or(figure.wspace),
                    hspace: o.hspace.unwrap_or(figure.hspace),
                }
            }
            None => *figure,
        };
        match &self.data.parent {
            Some(_) => Ok(base),
            None => base.updated(*o),
        }
    }

    /// Cell edges `(bottoms, tops, lefts, rights)` in figure fractions
    /// (matplotlib's `get_grid_positions`, same floating-point operations).
    pub fn grid_positions(&self, figure: &SubplotParams) -> Result<GridPositions, GridSpecError> {
        let params = self.subplot_params(figure)?;
        let (nrows, ncols) = self.shape();
        let heights = accumulated(
            params.top - params.bottom,
            params.hspace,
            &self.data.height_ratios,
            nrows,
        );
        let widths = accumulated(
            params.right - params.left,
            params.wspace,
            &self.data.width_ratios,
            ncols,
        );
        Ok(GridPositions {
            bottoms: heights.iter().map(|(_, end)| params.top - end).collect(),
            tops: heights
                .iter()
                .map(|(start, _)| params.top - start)
                .collect(),
            lefts: widths
                .iter()
                .map(|(start, _)| params.left + start)
                .collect(),
            rights: widths.iter().map(|(_, end)| params.left + end).collect(),
        })
    }

    /// A grid subdividing `parent` (matplotlib's `subgridspec`).
    pub fn nested(
        parent: &SubplotSpec,
        nrows: usize,
        ncols: usize,
        width_ratios: Option<Vec<f64>>,
        height_ratios: Option<Vec<f64>>,
        spacing: (Option<f64>, Option<f64>),
    ) -> Result<Self, GridSpecError> {
        let overrides = SubplotOverrides {
            wspace: spacing.0,
            hspace: spacing.1,
            ..SubplotOverrides::default()
        };
        Self::build(
            nrows,
            ncols,
            width_ratios,
            height_ratios,
            overrides,
            Some(parent.clone()),
        )
    }
}

/// Cell edges of a grid in figure fractions.
#[derive(Clone, Debug, PartialEq)]
pub struct GridPositions {
    /// Bottom edge of each row.
    pub bottoms: Vec<f64>,
    /// Top edge of each row.
    pub tops: Vec<f64>,
    /// Left edge of each column.
    pub lefts: Vec<f64>,
    /// Right edge of each column.
    pub rights: Vec<f64>,
}

/// `(start, end)` offsets of each cell along one direction, accumulated in
/// matplotlib's order: `cumsum([sep0, cell0, sep1, cell1, ...])`.
fn accumulated(total: f64, space: f64, ratios: &[f64], n: usize) -> Vec<(f64, f64)> {
    let count = n as f64;
    let cell = total / (count + space * (count - 1.0));
    let sep = space * cell;
    let norm = cell * count / ratios.iter().sum::<f64>();
    let mut running = 0.0;
    ratios
        .iter()
        .enumerate()
        .map(|(i, ratio)| {
            running += if i == 0 { 0.0 } else { sep };
            let start = running;
            running += ratio * norm;
            (start, running)
        })
        .collect()
}

/// A block of cells in a [`GridSpec`].
#[derive(Clone, Debug, PartialEq)]
pub struct SubplotSpec {
    gridspec: GridSpec,
    rows: Range<usize>,
    cols: Range<usize>,
}

pltrs_core::archive_struct!(SubplotSpec {
    gridspec,
    rows,
    cols
});

impl SubplotSpec {
    /// The grid this block belongs to.
    pub fn gridspec(&self) -> &GridSpec {
        &self.gridspec
    }

    /// Rows covered.
    pub fn rows(&self) -> Range<usize> {
        self.rows.clone()
    }

    /// Columns covered.
    pub fn cols(&self) -> Range<usize> {
        self.cols.clone()
    }

    /// The block of a figure-level grid this block lies in: itself on a
    /// figure-level grid, else the topmost block of its nested grid
    /// (matplotlib's `SubplotSpec.get_topmost_subplotspec`).
    #[must_use]
    pub fn topmost_subplotspec(&self) -> SubplotSpec {
        self.gridspec
            .topmost_subplotspec()
            .unwrap_or_else(|| self.clone())
    }

    /// Whether the block touches the first row (top of the grid).
    pub fn is_first_row(&self) -> bool {
        self.rows.start == 0
    }

    /// Whether the block touches the last row (bottom of the grid).
    pub fn is_last_row(&self) -> bool {
        self.rows.end == self.gridspec.shape().0
    }

    /// Whether the block touches the first column.
    pub fn is_first_col(&self) -> bool {
        self.cols.start == 0
    }

    /// Whether the block touches the last column.
    pub fn is_last_col(&self) -> bool {
        self.cols.end == self.gridspec.shape().1
    }

    /// Position of the block in figure fractions.
    pub fn position(&self, figure: &SubplotParams) -> Result<Bbox, GridSpecError> {
        let p = self.gridspec.grid_positions(figure)?;
        let rows = self.rows.clone();
        let cols = self.cols.clone();
        let min = |v: &[f64], r: Range<usize>| v[r].iter().copied().fold(f64::INFINITY, f64::min);
        let max =
            |v: &[f64], r: Range<usize>| v[r].iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Ok(Bbox::from_extents(
            min(&p.lefts, cols.clone()),
            min(&p.bottoms, rows.clone()),
            max(&p.rights, cols),
            max(&p.tops, rows),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::RcParams;

    fn defaults() -> SubplotParams {
        SubplotParams::from_rc(&RcParams::matplotlib())
    }

    fn close(a: Bbox, bounds: [f64; 4]) -> bool {
        let (x, y, w, h) = a.bounds();
        [x, y, w, h]
            .iter()
            .zip(bounds)
            .all(|(v, e)| (v - e).abs() < 1e-12)
    }

    #[test]
    fn single_subplot_matches_matplotlib() {
        // spec/goldens/figures/basic/line.json: [0.125, 0.11, 0.775, 0.77]
        let spec = GridSpec::new(1, 1)
            .and_then(|g| g.cell(0, 0))
            .expect("valid");
        assert!(close(
            spec.position(&defaults()).expect("valid"),
            [0.125, 0.11, 0.775, 0.77]
        ));
    }

    #[test]
    fn two_by_two_grid() {
        // matplotlib: fig.subplots(2, 2)[1, 0].get_position().bounds
        let grid = GridSpec::new(2, 2).expect("valid");
        let pos = grid
            .cell(1, 0)
            .and_then(|s| s.position(&defaults()))
            .expect("valid");
        assert!(close(pos, [0.125, 0.11, 0.352_272_727_272_727_24, 0.35]));
        assert_eq!(grid.numbered(3), grid.cell(1, 0));
    }

    #[test]
    fn spans_and_ratios() {
        let grid = GridSpec::with_ratios(
            1,
            3,
            Some(vec![1.0, 2.0, 1.0]),
            None,
            SubplotOverrides::default(),
        )
        .expect("valid");
        let all = grid
            .span(0..1, 0..3)
            .and_then(|s| s.position(&defaults()))
            .expect("valid");
        assert!(close(all, [0.125, 0.11, 0.775, 0.77]));
        let middle = grid
            .cell(0, 1)
            .and_then(|s| s.position(&defaults()))
            .expect("valid");
        assert!(close(
            middle,
            [
                0.341_544_117_647_058_83,
                0.11,
                0.341_911_764_705_882_36,
                0.77
            ]
        ));
    }

    #[test]
    fn nested_grids_use_the_parent_cell() {
        let outer = GridSpec::new(1, 2).expect("valid");
        let left = outer.cell(0, 0).expect("valid");
        let inner = GridSpec::nested(&left, 2, 1, None, None, (None, None)).expect("valid");
        let whole = left.position(&defaults()).expect("valid");
        let top = inner
            .cell(0, 0)
            .and_then(|s| s.position(&defaults()))
            .expect("valid");
        assert!((top.x0() - whole.x0()).abs() < 1e-12 && (top.y1() - whole.y1()).abs() < 1e-12);
        assert_eq!(inner.parent(), Some(&left));
        assert!(outer.parent().is_none());
    }

    #[test]
    fn accessors_and_identity() {
        let overrides = SubplotOverrides {
            hspace: Some(0.3),
            ..SubplotOverrides::default()
        };
        let grid =
            GridSpec::with_ratios(2, 1, None, Some(vec![2.0, 1.0]), overrides).expect("valid");
        assert_eq!(grid.width_ratios(), [1.0]);
        assert_eq!(grid.height_ratios(), [2.0, 1.0]);
        assert_eq!(grid.spacing(), (None, Some(0.3)));
        let twin =
            GridSpec::with_ratios(2, 1, None, Some(vec![2.0, 1.0]), overrides).expect("valid");
        assert_eq!(grid, twin);
        assert!(!grid.is_same(&twin) && grid.is_same(&grid.clone()));
    }

    #[test]
    fn validation() {
        assert_eq!(GridSpec::new(0, 1), Err(GridSpecError::EmptyGrid));
        assert!(GridSpec::new(2, 2).and_then(|g| g.cell(2, 0)).is_err());
        let squashed = SubplotOverrides {
            left: Some(0.9),
            right: Some(0.1),
            ..SubplotOverrides::default()
        };
        assert_eq!(defaults().updated(squashed), Err(GridSpecError::EmptyWidth));
        assert!(
            GridSpec::with_ratios(1, 2, Some(vec![1.0]), None, SubplotOverrides::default())
                .is_err()
        );
    }
}
