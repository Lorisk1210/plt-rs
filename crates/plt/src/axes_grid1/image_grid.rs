//! Grids of image axes with fixed paddings and optional colorbar axes
//! (matplotlib's `axes_grid1.axes_grid.ImageGrid`, also known as
//! `AxesGrid`).

use std::fmt;

use super::divider::{Divider, DividerAspect, DividerId, DividerRect};
use super::labels::{LabelMode, apply_label_mode};
use super::size::{Size, SizeAspect, SizeSpec};
use crate::artist::ArtistHandle;
use crate::axes::Axes;
use crate::colorbar::{Colorbar, ColorbarAxes, ColorbarError, ColorbarOptions, Mappable};
use crate::figure::{Figure, insert_axes};
use crate::gridspec::SubplotSpec;
use crate::spines::SpineSide;

/// The order the axes of a grid are created and numbered in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridDirection {
    /// Row by row.
    #[default]
    Row,
    /// Column by column.
    Column,
}

/// Which axes of an [`ImageGrid`] get a colorbar axes (matplotlib's
/// `cbar_mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CbarMode {
    /// One for every axes.
    Each,
    /// One for the whole grid.
    Single,
    /// One for each row (colorbars left or right) or column (top or
    /// bottom) on the colorbar side.
    Edge,
}

/// The region an [`ImageGrid`] divides.
#[derive(Clone, Debug, PartialEq)]
pub enum GridRect {
    /// `[left, bottom, width, height]` in figure fractions.
    Rect([f64; 4]),
    /// The cells of a subplot grid (matplotlib's `111`-style codes).
    Subplot(SubplotSpec),
}

/// Options of [`ImageGrid::new`]; the defaults are matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageGridOptions {
    /// Create only the first axes (`None` = every cell).
    pub n_axes: Option<usize>,
    /// The order the axes are created in.
    pub direction: GridDirection,
    /// Horizontal and vertical padding between axes, in inches.
    pub axes_pad: (f64, f64),
    /// Whether all axes share both axes (otherwise columns share x and
    /// rows share y).
    pub share_all: bool,
    /// Whether the axes keep the aspect of their data limits.
    pub aspect: bool,
    /// Which axes keep their tick labels and axis labels.
    pub label_mode: LabelMode,
    /// Which axes get colorbar axes (`None` = none).
    pub cbar_mode: Option<CbarMode>,
    /// The side of the colorbar axes.
    pub cbar_location: SpineSide,
    /// Padding between axes and colorbars, in inches (`None` = the axes
    /// padding across the colorbar).
    pub cbar_pad: Option<f64>,
    /// The thickness of the colorbars.
    pub cbar_size: SizeSpec,
}

impl Default for ImageGridOptions {
    fn default() -> Self {
        Self {
            n_axes: None,
            direction: GridDirection::Row,
            axes_pad: (0.02, 0.02),
            share_all: false,
            aspect: true,
            label_mode: LabelMode::L,
            cbar_mode: None,
            cbar_location: SpineSide::Right,
            cbar_pad: None,
            cbar_size: SizeSpec::Percent(5.0),
        }
    }
}

/// An invalid grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxesGridError {
    /// The grid has no rows or no columns.
    Empty,
    /// More axes were asked for than the grid has cells, or none.
    AxesCount {
        /// The axes asked for.
        n_axes: usize,
        /// The cells of the grid.
        cells: usize,
    },
}

impl fmt::Display for AxesGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("an axes grid needs at least one row and one column"),
            Self::AxesCount { n_axes, cells } => write!(
                f,
                "n_axes must be positive and at most the {cells} cells of the grid, got {n_axes}"
            ),
        }
    }
}

impl std::error::Error for AxesGridError {}

/// An axes made to hold a colorbar of an [`ImageGrid`] (matplotlib's
/// `CbarAxes`).
#[derive(Clone, Debug, PartialEq)]
pub struct CbarAxes {
    figure: Figure,
    ax: Axes,
    location: SpineSide,
}

impl CbarAxes {
    /// The axes.
    pub fn ax(&self) -> &Axes {
        &self.ax
    }

    /// Draws a colorbar of `mappable` into this axes, on the grid's
    /// colorbar side (matplotlib's `CbarAxes.colorbar`).
    ///
    /// # Errors
    ///
    /// As [`Figure::colorbar`].
    pub fn colorbar<'a>(
        &self,
        mappable: impl Into<Mappable<'a>>,
        options: ColorbarOptions,
    ) -> Result<Colorbar, ColorbarError> {
        let options = ColorbarOptions {
            axes: ColorbarAxes::Cax(self.ax.clone()),
            location: Some(self.location),
            ..options
        };
        self.figure.colorbar(mappable, options)
    }
}

/// A grid of axes for images, placed by a divider so that the paddings
/// stay fixed in inches and the axes keep their images' aspect, with
/// optional colorbar axes (matplotlib's `ImageGrid` and `AxesGrid`).
///
/// ```
/// use plt::axes_grid1::{GridRect, ImageGrid, ImageGridOptions};
/// use plt::prelude::*;
/// let fig = Figure::new();
/// let rect = GridRect::Rect([0.1, 0.1, 0.8, 0.8]);
/// let grid = ImageGrid::new(&fig, &rect, (2, 3), &ImageGridOptions::default())?;
/// assert_eq!(grid.axes().len(), 6);
/// # Ok::<(), plt::axes_grid1::AxesGridError>(())
/// ```
#[derive(Clone, Debug)]
pub struct ImageGrid {
    axes: Vec<Axes>,
    cbar_axes: Vec<CbarAxes>,
    /// The index in `cbar_axes` of the colorbar axes of each axes.
    caxes: Vec<usize>,
}

/// matplotlib's `AxesGrid`, the same class as [`ImageGrid`].
pub type AxesGrid = ImageGrid;

impl ImageGrid {
    /// A grid of `nrows_ncols` axes dividing `rect` of `fig`.
    ///
    /// # Errors
    ///
    /// When the grid is empty or `n_axes` does not fit it.
    pub fn new(
        fig: &Figure,
        rect: &GridRect,
        (nrows, ncols): (usize, usize),
        options: &ImageGridOptions,
    ) -> Result<Self, AxesGridError> {
        if nrows == 0 || ncols == 0 {
            return Err(AxesGridError::Empty);
        }
        let cells = nrows * ncols;
        let n_axes = options.n_axes.unwrap_or(cells);
        if n_axes == 0 || n_axes > cells {
            return Err(AxesGridError::AxesCount { n_axes, cells });
        }
        let layout = Layout {
            nrows,
            ncols,
            n_axes,
            direction: options.direction,
        };
        let frame = fig.raw().id();
        let divider_rect = match rect {
            GridRect::Rect(bounds) => DividerRect::Fixed(*bounds),
            GridRect::Subplot(spec) => DividerRect::Subplot(spec.clone()),
        };
        let divider = Divider::new(frame, divider_rect, DividerAspect::Fixed(options.aspect));
        let figure = fig.raw().figure_state();
        let id = figure.borrow_mut().add_divider(divider);
        let position = figure
            .borrow()
            .divider_position(id)
            .unwrap_or(plt_core::Bbox::UNIT);
        let axes = layout.create_axes(fig, position, options.share_all);
        let cbar_axes = (0..n_axes)
            .map(|_| CbarAxes {
                figure: fig.clone(),
                ax: insert_axes(figure, frame, position, None),
                location: options.cbar_location,
            })
            .collect();
        let caxes = (0..n_axes)
            .map(|index| layout.cax_index(index, options))
            .collect();
        let grid = Self {
            axes,
            cbar_axes,
            caxes,
        };
        let rows = layout.rows(&grid.axes);
        grid.init_locators(id, &layout, &rows, options);
        apply_label_mode(&rows, options.label_mode);
        Ok(grid)
    }

    /// The axes in the order of the grid's direction (matplotlib's
    /// `axes_all`).
    pub fn axes(&self) -> &[Axes] {
        &self.axes
    }

    /// The colorbar axes, one per axes; those the colorbar mode does not
    /// use are hidden (matplotlib's `cbar_axes`).
    pub fn cbar_axes(&self) -> &[CbarAxes] {
        &self.cbar_axes
    }

    /// The colorbar axes of the `index`-th axes (matplotlib's `ax.cax`):
    /// its own in the `Each` mode or without colorbars, the one of its row
    /// (colorbars left or right) or column (top or bottom) in the `Edge`
    /// mode, the only one in the `Single` mode; `None` past the last axes.
    ///
    /// ```
    /// use plt::axes_grid1::{CbarMode, GridRect, ImageGrid, ImageGridOptions};
    /// use plt::prelude::*;
    /// use plt::spines::SpineSide;
    /// let fig = Figure::new();
    /// let options = ImageGridOptions {
    ///     cbar_mode: Some(CbarMode::Edge),
    ///     cbar_location: SpineSide::Right,
    ///     ..ImageGridOptions::default()
    /// };
    /// let grid = ImageGrid::new(&fig, &GridRect::Rect([0.1, 0.1, 0.8, 0.8]), (2, 2), &options)?;
    /// // Row by row: the axes of the second row share the second colorbar.
    /// assert_eq!(grid.cax(3), Some(&grid.cbar_axes()[1]));
    /// assert_eq!(grid.cax(1), Some(&grid.cbar_axes()[0]));
    /// assert_eq!(grid.cax(4), None);
    /// # Ok::<(), plt::axes_grid1::AxesGridError>(())
    /// ```
    pub fn cax(&self, index: usize) -> Option<&CbarAxes> {
        self.caxes
            .get(index)
            .and_then(|&cax| self.cbar_axes.get(cax))
    }

    /// matplotlib's `ImageGrid._init_locators`: the divider's columns and
    /// rows (axes, paddings, colorbars) and the cell of every axes, given
    /// the axes of each row.
    fn init_locators(
        &self,
        id: DividerId,
        layout: &Layout,
        rows: &[Vec<Axes>],
        options: &ImageGridOptions,
    ) {
        let plan = super::cbar_layout::plan(self, layout, rows, options);
        let Some(first) = self.axes.first() else {
            return;
        };
        let figure = first.raw().figure_state();
        let mut state = figure.borrow_mut();
        let divider = state.divider_mut(id);
        divider.horizontal = plan.horizontal;
        divider.vertical = plan.vertical;
        let cells: Vec<_> = plan
            .cells
            .iter()
            .map(|(axes, x, y)| (*axes, divider.cell(id, *x, *y)))
            .collect();
        for (axes, cell) in cells {
            state.set_divider_cell(axes, cell);
        }
        drop(state);
        for (cax, visible) in self.cbar_axes.iter().zip(plan.cbar_visible) {
            cax.ax.set_visible(visible);
        }
    }
}

/// The shape of a grid.
#[derive(Clone, Copy, Debug)]
pub(super) struct Layout {
    pub(super) nrows: usize,
    pub(super) ncols: usize,
    pub(super) n_axes: usize,
    pub(super) direction: GridDirection,
}

impl Layout {
    /// The axes of each row, top to bottom, left to right (matplotlib's
    /// `axes_row`).
    fn rows(&self, axes: &[Axes]) -> Vec<Vec<Axes>> {
        let mut placed: Vec<(usize, usize, &Axes)> = axes
            .iter()
            .enumerate()
            .map(|(i, ax)| {
                let (col, row) = self.col_row(i);
                (row, col, ax)
            })
            .collect();
        placed.sort_by_key(|&(row, col, _)| (row, col));
        let mut rows = vec![Vec::new(); self.nrows];
        for (row, _, ax) in placed {
            rows[row].push(ax.clone());
        }
        rows
    }

    /// The column and row of the `n`-th axes (matplotlib's
    /// `_get_col_row`).
    pub(super) fn col_row(&self, n: usize) -> (usize, usize) {
        match self.direction {
            GridDirection::Column => (n / self.nrows, n % self.nrows),
            GridDirection::Row => (n % self.ncols, n / self.ncols),
        }
    }

    /// The index of the colorbar axes of the `n`-th axes (matplotlib's
    /// `ax.cax` assignment at the end of `_init_locators`).
    fn cax_index(&self, n: usize, options: &ImageGridOptions) -> usize {
        match options.cbar_mode {
            Some(CbarMode::Single) => 0,
            Some(CbarMode::Edge) => {
                let (col, row) = self.col_row(n);
                if matches!(options.cbar_location, SpineSide::Left | SpineSide::Right) {
                    row
                } else {
                    col
                }
            }
            Some(CbarMode::Each) | None => n,
        }
    }

    /// Creates the axes at `position`, columns sharing x and rows sharing
    /// y (or all sharing both).
    fn create_axes(&self, fig: &Figure, position: plt_core::Bbox, share_all: bool) -> Vec<Axes> {
        let figure = fig.raw().figure_state();
        let mut grid: Vec<Vec<Option<Axes>>> = vec![vec![None; self.ncols]; self.nrows];
        let mut created = Vec::with_capacity(self.n_axes);
        for i in 0..self.n_axes {
            let (col, row) = self.col_row(i);
            let ax = insert_axes(figure, fig.raw().id(), position, None);
            let (share_x, share_y) = if share_all {
                (grid[0][0].clone(), grid[0][0].clone())
            } else {
                (grid[0][col].clone(), grid[row][0].clone())
            };
            let reason = "a new axes can share with one of its figure";
            if let Some(other) = share_x {
                ax.share_x(&other).expect(reason);
            }
            if let Some(other) = share_y {
                ax.share_y(&other).expect(reason);
            }
            grid[row][col] = Some(ax.clone());
            created.push(ax);
        }
        created
    }
}

/// `Size.AxesX(ax, aspect="axes", ref_ax=reference)` and its `AxesY`
/// counterpart.
pub(super) fn relative_extent(ax: &Axes, reference: &Axes, horizontal: bool) -> Size {
    let aspect = SizeAspect::RelativeTo(reference.id());
    if horizontal {
        Size::AxesX(ax.id(), aspect)
    } else {
        Size::AxesY(ax.id(), aspect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rc::Aspect;

    #[test]
    fn grids_must_hold_their_axes() {
        let fig = Figure::new();
        let rect = GridRect::Rect([0.1, 0.1, 0.8, 0.8]);
        let options = ImageGridOptions {
            n_axes: Some(5),
            ..ImageGridOptions::default()
        };
        let err = ImageGrid::new(&fig, &rect, (2, 2), &options).unwrap_err();
        assert_eq!(
            err,
            AxesGridError::AxesCount {
                n_axes: 5,
                cells: 4
            }
        );
        let empty = ImageGrid::new(&fig, &rect, (0, 2), &ImageGridOptions::default());
        assert_eq!(empty.unwrap_err(), AxesGridError::Empty);
    }

    #[test]
    fn every_axes_names_its_colorbar_axes() {
        let fig = Figure::new();
        let rect = GridRect::Rect([0.1, 0.1, 0.8, 0.8]);
        let caxes = |cbar_mode, direction, cbar_location| {
            let options = ImageGridOptions {
                direction,
                cbar_mode,
                cbar_location,
                ..ImageGridOptions::default()
            };
            let grid = ImageGrid::new(&fig, &rect, (2, 3), &options).unwrap();
            (0..6)
                .map(|i| {
                    let cax = grid.cax(i).expect("a colorbar axes");
                    grid.cbar_axes().iter().position(|c| c == cax)
                })
                .collect::<Option<Vec<usize>>>()
                .unwrap()
        };
        let edge = Some(CbarMode::Edge);
        // Column by column, the colorbars at the bottom follow the columns.
        let columns = caxes(edge, GridDirection::Column, SpineSide::Bottom);
        assert_eq!(columns, [0, 0, 1, 1, 2, 2]);
        let rows = caxes(edge, GridDirection::Column, SpineSide::Left);
        assert_eq!(rows, [0, 1, 0, 1, 0, 1]);
        let single = caxes(Some(CbarMode::Single), GridDirection::Row, SpineSide::Right);
        assert_eq!(single, [0; 6]);
        let own = caxes(None, GridDirection::Row, SpineSide::Right);
        assert_eq!(own, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_single_colorbar_follows_the_grid() {
        let fig = Figure::with_size(5.0, 4.0, 100.0);
        let options = ImageGridOptions {
            axes_pad: (0.1, 0.1),
            cbar_mode: Some(CbarMode::Single),
            cbar_pad: Some(0.1),
            ..ImageGridOptions::default()
        };
        let rect = GridRect::Rect([0.125, 0.11, 0.775, 0.77]);
        let grid = ImageGrid::new(&fig, &rect, (2, 2), &options).unwrap();
        for ax in grid.axes() {
            ax.set_xlim(-0.5, 19.5).set_ylim(19.5, -0.5);
            ax.set_aspect(Aspect::Equal).unwrap();
        }
        let visible: Vec<bool> = grid.cbar_axes().iter().map(|c| c.ax().visible()).collect();
        assert_eq!(visible, [true, false, false, false]);
        // matplotlib 3.11: the axes and the colorbar of
        // `misc/imagegrid_single_cbar`.
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        let (x, y, w, h) = grid.axes()[0].position().bounds();
        assert!(close(x, 0.1796) && close(y, 0.5075) && close(w, 0.298) && close(h, 0.3725));
        let (x, y, w, h) = grid.cbar_axes()[0].ax().position().bounds();
        assert!(close(x, 0.8156) && close(y, 0.11) && close(w, 0.0298) && close(h, 0.77));
    }
}
