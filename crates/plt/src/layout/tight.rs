//! Tight layout (matplotlib's `_tight_layout.py`): subplot parameters
//! leaving just enough room around and between grid cells for the
//! decorations of their axes.
//!
//! Tight layout runs once when asked ([`Figure::tight_layout`]), or at
//! every draw as a layout engine ([`LayoutEngine::Tight`](super::LayoutEngine::Tight),
//! `figure.autolayout`). It only changes the figure's subplot parameters,
//! so it handles a single grid of uniform cells: margins are the largest
//! decorations along each edge of the grid, the spacings the largest
//! between neighbouring cells.

use std::fmt;
use std::ops::Range;

use plt_core::Bbox;

use crate::artist::{ArtistHandle, ArtistId};
use crate::axes::AxesState;
use crate::figure::{Figure, FigureState};
use crate::gridspec::{GridSpec, GridSpecError, SubplotParams, SubplotSpec};
use crate::layout::{LayoutEngine, LayoutEngineError};

mod fit;

pub(super) use self::fit::adjusted_params;

/// The parameters of tight layout (matplotlib's `tight_layout` arguments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TightLayout {
    /// Padding between the figure edge and the decorations, as a fraction
    /// of the font size.
    pub pad: f64,
    /// Padding between rows, as a fraction of the font size (`None` =
    /// `pad`).
    pub h_pad: Option<f64>,
    /// Padding between columns, as a fraction of the font size (`None` =
    /// `pad`).
    pub w_pad: Option<f64>,
    /// `[left, bottom, right, top]` in figure fractions that the grid and
    /// its decorations fit into (`None` = the whole figure).
    pub rect: Option<[f64; 4]>,
}

impl Default for TightLayout {
    /// matplotlib's defaults: a padding of 1.08 font sizes everywhere.
    fn default() -> Self {
        Self {
            pad: 1.08,
            h_pad: None,
            w_pad: None,
            rect: None,
        }
    }
}

/// Why tight layout left the figure unchanged (matplotlib warns instead,
/// except for an incompatible layout engine).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TightLayoutError {
    /// The decorations take more than the figure's width.
    TooWide,
    /// The decorations take more than the figure's height.
    TooTall,
    /// The row or column counts of the grids are not multiples of one
    /// another.
    IncompatibleGrids,
    /// The resulting subplot parameters are invalid.
    Grid(GridSpecError),
    /// The figure refuses tight layout as its engine (a colorbar was made
    /// under constrained layout).
    Engine(LayoutEngineError),
}

impl fmt::Display for TightLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tight layout not applied: ")?;
        match self {
            Self::TooWide => f.write_str("the axes decorations do not fit the figure width"),
            Self::TooTall => f.write_str("the axes decorations do not fit the figure height"),
            Self::IncompatibleGrids => f.write_str(
                "the numbers of rows and columns of the grids must be multiples of one another",
            ),
            Self::Grid(err) => err.fmt(f),
            Self::Engine(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for TightLayoutError {}

impl From<LayoutEngineError> for TightLayoutError {
    fn from(err: LayoutEngineError) -> Self {
        Self::Engine(err)
    }
}

impl From<GridSpecError> for TightLayoutError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

impl Figure {
    /// Adjusts the subplot parameters once so that the decorations of the
    /// grid-placed axes, 3-D ones included (tick labels, axis labels,
    /// titles), and the figure title and labels fit (matplotlib's
    /// `tight_layout`). As in matplotlib, the figure takes tight layout as
    /// its engine for this pass and is left with a
    /// [`LayoutEngine::PlaceHolder`] for it, so later draws keep the
    /// result.
    ///
    /// Axes outside grids, and grids with margins of their own, are left
    /// out.
    ///
    /// # Errors
    ///
    /// When the decorations cannot fit, or the grids cannot be combined;
    /// the subplot parameters are then left unchanged. When a colorbar was
    /// made under an engine placing colorbars differently, such as
    /// constrained layout; the figure is then left with a placeholder for
    /// that engine.
    ///
    /// Under `plt.tight_layout.keep_constrained` (set by plt-rs' modern
    /// looks), a figure laid out by constrained layout keeps it, and this
    /// does nothing.
    ///
    /// ```
    /// use plt::layout::TightLayout;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_xlim(0.0, 1.0)?.set_ylim(0.0, 1.0)?;
    /// fig.tight_layout(TightLayout::default())?;
    /// let (x, _, w, _) = ax.position().bounds();
    /// assert!(x < 0.125 && x + w > 0.9);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tight_layout(&self, params: TightLayout) -> Result<&Self, TightLayoutError> {
        let axes: Vec<ArtistId> = (self.axes().iter().map(ArtistHandle::id))
            .chain(self.axes3d().iter().map(ArtistHandle::id))
            .collect();
        let mut state = self.raw().figure_state().borrow_mut();
        if crate::rc::params().plt_tight_layout_keep_constrained
            && matches!(state.layout.engine, Some(LayoutEngine::Constrained(_)))
        {
            return Ok(self);
        }
        let has_colorbars = crate::colorbar::has_colorbars(&state.store);
        let swapped = state
            .layout
            .swap_engine(Some(LayoutEngine::Tight(params)), has_colorbars);
        // The pass runs once, by hand: the engine left behind places nothing.
        state.layout.remove_engine();
        swapped?;
        state.tight_layout_pass(self.raw().id(), &axes, &params)?;
        Ok(self)
    }
}

impl FigureState {
    /// One tight layout pass over `axes`, changing the subplot parameters
    /// of figure frame `frame` (matplotlib's `TightLayoutEngine.execute`).
    pub(crate) fn tight_layout_pass(
        &mut self,
        frame: ArtistId,
        axes: &[ArtistId],
        params: &TightLayout,
    ) -> Result<(), TightLayoutError> {
        let Some(subplot_params) = self.subplot_params_of(frame) else {
            return Ok(());
        };
        let measured = Measured::new(self, axes, &subplot_params, None)?;
        if let Some(overrides) = adjusted_params(self, &measured, params)? {
            self.set_subplot_params(subplot_params.updated(overrides)?)?;
        }
        Ok(())
    }
}

/// A block of grid cells as the margins see it: its cells scaled to the
/// common grid, its position from the subplot parameters and the extent
/// of its axes with decorations (display pixels; `None` when they are
/// all invisible).
#[derive(Clone, Debug)]
struct Cell {
    rows: Range<usize>,
    cols: Range<usize>,
    position: Bbox,
    tight: Option<Bbox>,
}

/// The blocks of the grid-placed axes on a common grid, measured.
pub(super) struct Measured {
    /// The common grid: the largest row and column counts (`None` without
    /// grid-placed axes).
    shape: Option<(usize, usize)>,
    cells: Vec<Cell>,
}

impl Measured {
    /// matplotlib's `get_tight_layout_figure` up to the margins, over
    /// `axes` (only those on `grid` when given).
    pub(super) fn new(
        fig: &mut FigureState,
        axes: &[ArtistId],
        subplot_params: &SubplotParams,
        grid: Option<&GridSpec>,
    ) -> Result<Self, TightLayoutError> {
        let blocks = blocks(fig, axes, grid);
        let shapes = || blocks.iter().map(|(spec, _)| spec.gridspec().shape());
        let (Some(max_rows), Some(max_cols)) = (
            shapes().map(|(rows, _)| rows).max(),
            shapes().map(|(_, cols)| cols).max(),
        ) else {
            return Ok(Self {
                shape: None,
                cells: Vec::new(),
            });
        };
        let scale = |span: Range<usize>, by: usize| span.start * by..span.end * by;
        let mut cells = Vec::with_capacity(blocks.len());
        for (spec, ids) in blocks {
            let (rows, cols) = spec.gridspec().shape();
            if max_rows % rows != 0 || max_cols % cols != 0 {
                return Err(TightLayoutError::IncompatibleGrids);
            }
            cells.push(Cell {
                rows: scale(spec.rows(), max_rows / rows),
                cols: scale(spec.cols(), max_cols / cols),
                position: spec.position(subplot_params)?,
                tight: ids
                    .into_iter()
                    .filter_map(|id| fig.axes_tight_bbox(id, true))
                    .reduce(|a, b| a.union(&b)),
            });
        }
        Ok(Self {
            shape: Some((max_rows, max_cols)),
            cells,
        })
    }
}

/// The axes on grids grouped by their topmost block of cells, in the
/// order the blocks first appear; axes off grids are left out, and so are
/// those on grids with margins of their own, or, when `grid` is given,
/// those on other grids (matplotlib's `get_subplotspec_list`). Axes a
/// divider places count as on its cells, as matplotlib asks their
/// locators first.
fn blocks(
    fig: &FigureState,
    axes: &[ArtistId],
    grid: Option<&GridSpec>,
) -> Vec<(SubplotSpec, Vec<ArtistId>)> {
    let mut blocks: Vec<(SubplotSpec, Vec<ArtistId>)> = Vec::new();
    let included = |spec: &&SubplotSpec| match grid {
        Some(grid) => spec.gridspec().is_same(grid),
        None => !spec.gridspec().locally_modified(),
    };
    for &id in axes {
        let Some(spec) = fig
            .divider_subplot_spec(id)
            .or_else(|| {
                fig.store
                    .get_as::<AxesState>(id)
                    .and_then(|state| state.subplot_spec.as_ref())
            })
            .or_else(|| crate::mplot3d::axes3d_subplot_spec(&fig.store, id))
            .map(topmost)
            .filter(included)
        else {
            continue;
        };
        match blocks.iter_mut().find(|(known, _)| same_block(known, spec)) {
            Some((_, ids)) => ids.push(id),
            None => blocks.push((spec.clone(), vec![id])),
        }
    }
    blocks
}

/// The block of the figure-level grid that `spec` lies in (matplotlib's
/// `get_topmost_subplotspec`).
fn topmost(spec: &SubplotSpec) -> &SubplotSpec {
    spec.gridspec().parent().map_or(spec, topmost)
}

/// Whether two blocks cover the same cells of the very same grid, as
/// matplotlib compares subplot specifications.
fn same_block(a: &SubplotSpec, b: &SubplotSpec) -> bool {
    a.gridspec().is_same(b.gridspec()) && a.rows() == b.rows() && a.cols() == b.cols()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gridspec::GridSpec;
    use crate::layout::LayoutEngine;

    #[test]
    fn blocks_group_axes_of_the_same_cells() {
        let fig = Figure::new();
        let grid = GridSpec::new(1, 2).unwrap();
        let a = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        let b = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        let c = fig.add_subplot_spec(&grid.cell(0, 1).unwrap()).unwrap();
        let free = fig
            .add_axes([0.1, 0.1, 0.2, 0.2])
            .expect("a valid rectangle");
        let ids = [a.id(), b.id(), c.id(), free.id()];
        let state = fig.raw().figure_state().borrow();
        let found = blocks(&state, &ids, None);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].1, vec![a.id(), b.id()]);
        let other = GridSpec::new(1, 2).unwrap();
        assert_eq!(blocks(&state, &ids, Some(&other)).len(), 0);
        assert_eq!(blocks(&state, &ids, Some(&grid)).len(), 2);
    }

    #[test]
    fn divided_axes_count_in_the_cells_of_their_divider() {
        use crate::axes_grid1::{SizeSpec, make_axes_locatable};
        use crate::spines::SpineSide;
        let fig = Figure::new();
        let ax = fig.subplot();
        let divider = make_axes_locatable(&ax);
        let cax = divider.append_axes(SpineSide::Right, SizeSpec::Percent(5.0), None);
        let ids = [ax.id(), cax.id()];
        let state = fig.raw().figure_state().borrow();
        let found = blocks(&state, &ids, None);
        assert_eq!(found.len(), 1, "one cell");
        assert_eq!(found[0].1, ids.to_vec());
    }

    #[test]
    fn decorations_too_wide_leave_the_figure_alone() {
        let fig = Figure::with_size(1.0, 1.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        ax.set_ylabel("a label\nover\nmany\nmany\nlines");
        let params = TightLayout {
            pad: 4.0,
            ..TightLayout::default()
        };
        assert_eq!(fig.tight_layout(params), Err(TightLayoutError::TooWide));
        assert_eq!(ax.position().bounds().0, 0.125);
    }

    #[test]
    fn empty_cells_keep_their_share_of_the_grid() {
        let fig = Figure::new();
        let grid = GridSpec::new(2, 2).unwrap();
        let ax = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        ax.set_xlim(0.0, 1.0)
            .and_then(|ax| ax.set_ylim(0.0, 1.0))
            .expect("finite limits");
        fig.tight_layout(TightLayout::default()).unwrap();
        let (_, y, w, h) = ax.position().bounds();
        // The empty cells to the right and below still take their share.
        assert!(w < 0.5 && h < 0.5 && y > 0.5, "{y} {w} {h}");
        let params = fig.subplot_params();
        assert!(params.wspace > 0.0 && params.hspace > 0.0);
    }

    #[test]
    fn three_d_axes_are_laid_out_too() {
        use crate::mplot3d::{Axes3DOptions, ProjType};
        // matplotlib's test_tight_layout_text: one 3-D axes moves into
        // the room the default margins leave.
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        fig.tight_layout(TightLayout::default()).unwrap();
        assert!(ax.position().bounds().0 < 0.2, "{:?}", ax.position());
        // test_axes3d_primary_views: the engine lays out a grid of
        // orthographic views, edge-on ones included, at every draw.
        let fig = Figure::new();
        fig.set_layout_engine(Some(LayoutEngine::Tight(TightLayout::default())))
            .unwrap();
        let options = Axes3DOptions {
            proj_type: ProjType::Ortho,
            ..Axes3DOptions::default()
        };
        let views = [(90.0, -90.0), (0.0, -90.0), (0.0, 0.0)];
        let axs: Vec<_> = (1..=3)
            .map(|i| fig.add_subplot_3d(1, 3, i, options.clone()).unwrap())
            .collect();
        for (ax, (elev, azim)) in axs.iter().zip(views) {
            ax.set_zlabel("z").view_init(elev, azim, 0.0);
        }
        fig.layout();
        assert!(
            axs[0].position().bounds().0 < 0.1,
            "{:?}",
            axs[0].position()
        );
    }
}
