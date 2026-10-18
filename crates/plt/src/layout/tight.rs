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

use crate::artist::{Artist, ArtistHandle, ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::figure::{Figure, FigureFrame, FigureState};
use crate::gridspec::{GridSpecError, SubplotOverrides, SubplotParams, SubplotSpec};
use crate::text::TextArtist;

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

/// Why tight layout left the figure unchanged (matplotlib warns instead).
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
        }
    }
}

impl std::error::Error for TightLayoutError {}

impl From<GridSpecError> for TightLayoutError {
    fn from(err: GridSpecError) -> Self {
        Self::Grid(err)
    }
}

impl Figure {
    /// Adjusts the subplot parameters once so that the decorations of the
    /// grid-placed axes (tick labels, axis labels, titles) and the figure
    /// title and labels fit (matplotlib's `tight_layout`). Any layout
    /// engine is removed, as matplotlib does.
    ///
    /// Axes outside grids, and grids with margins of their own, are left
    /// out.
    ///
    /// # Errors
    ///
    /// When the decorations cannot fit, or the grids cannot be combined;
    /// the figure is then left unchanged.
    ///
    /// ```
    /// use plt::layout::TightLayout;
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
    /// fig.tight_layout(&TightLayout::default())?;
    /// let (x, _, w, _) = ax.position().bounds();
    /// assert!(x < 0.125 && x + w > 0.9);
    /// # Ok::<(), plt::layout::TightLayoutError>(())
    /// ```
    pub fn tight_layout(&self, params: &TightLayout) -> Result<&Self, TightLayoutError> {
        let axes: Vec<ArtistId> = self.axes().iter().map(ArtistHandle::id).collect();
        let mut state = self.raw().figure_state().borrow_mut();
        state.layout.engine = None;
        state.layout.pending = true;
        state.tight_layout_pass(self.raw().id(), &axes, params)?;
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
        let measured = Measured::new(self, axes, &subplot_params)?;
        if let Some(overrides) = adjusted_params(self, &measured, params)? {
            self.set_subplot_params(frame, subplot_params.updated(&overrides)?)?;
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
struct Measured {
    /// The common grid: the largest row and column counts (`None` without
    /// grid-placed axes).
    shape: Option<(usize, usize)>,
    cells: Vec<Cell>,
}

impl Measured {
    /// matplotlib's `get_tight_layout_figure` up to the margins, over
    /// `axes`.
    fn new(
        fig: &mut FigureState,
        axes: &[ArtistId],
        subplot_params: &SubplotParams,
    ) -> Result<Self, TightLayoutError> {
        let blocks = blocks(fig, axes);
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
/// order the blocks first appear; axes off grids or on grids with margins
/// of their own are left out (matplotlib's `get_subplotspec_list`).
fn blocks(fig: &FigureState, axes: &[ArtistId]) -> Vec<(SubplotSpec, Vec<ArtistId>)> {
    let mut blocks: Vec<(SubplotSpec, Vec<ArtistId>)> = Vec::new();
    for &id in axes {
        let Some(spec) = fig
            .store
            .get_as::<AxesState>(id)
            .and_then(|state| state.subplot_spec.as_ref())
            .map(topmost)
            .filter(|spec| !spec.gridspec().locally_modified())
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

/// The room decorations take beyond the cell edges, in figure fractions
/// (matplotlib's `vspaces` and `hspaces`).
struct Spaces {
    /// Above and below each cell: `rows + 1` row edges × `cols`.
    vertical: Vec<Vec<f64>>,
    /// Left and right of each cell: `rows` × `cols + 1` column edges.
    horizontal: Vec<Vec<f64>>,
}

impl Spaces {
    /// Adds up what the decorations of each cell take beyond its edges.
    fn new((rows, cols): (usize, usize), cells: &[Cell], (width, height): (f64, f64)) -> Self {
        let mut spaces = Self {
            vertical: vec![vec![0.0; cols]; rows + 1],
            horizontal: vec![vec![0.0; cols + 1]; rows],
        };
        for cell in cells {
            let Some(tight) = cell.tight else { continue };
            let tight = Bbox::from_extents(
                tight.x0() / width,
                tight.y0() / height,
                tight.x1() / width,
                tight.y1() / height,
            );
            let ax = cell.position;
            for row in &mut spaces.horizontal[cell.rows.clone()] {
                row[cell.cols.start] += ax.xmin() - tight.xmin();
                row[cell.cols.end] += tight.xmax() - ax.xmax();
            }
            for col in cell.cols.clone() {
                spaces.vertical[cell.rows.start][col] += tight.ymax() - ax.ymax();
                spaces.vertical[cell.rows.end][col] += ax.ymin() - tight.ymin();
            }
        }
        spaces
    }

    /// The largest room along the column edge `edge`.
    fn column_edge(&self, edge: usize) -> f64 {
        largest(self.horizontal.iter().map(|row| row[edge]))
    }

    /// The largest room along the row edge `edge`.
    fn row_edge(&self, edge: usize) -> f64 {
        largest(self.vertical[edge].iter().copied())
    }

    /// The largest room at the interior column edges.
    fn between_columns(&self) -> f64 {
        largest(
            self.horizontal
                .iter()
                .flat_map(|row| row[1..row.len() - 1].iter().copied()),
        )
    }

    /// The largest room at the interior row edges.
    fn between_rows(&self) -> f64 {
        let rows = self.vertical.len() - 1;
        largest(self.vertical[1..rows].iter().flatten().copied())
    }
}

/// The largest value, `-inf` for none.
fn largest(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(f64::NEG_INFINITY, f64::max)
}

/// matplotlib's `get_tight_layout_figure` from the margins on: the
/// subplot parameters (`None` without grid-placed axes), fitted into
/// `params.rect` by a second pass if given.
fn adjusted_params(
    fig: &FigureState,
    measured: &Measured,
    params: &TightLayout,
) -> Result<Option<SubplotOverrides>, TightLayoutError> {
    let Some(shape) = measured.shape else {
        return Ok(None);
    };
    let grid = Grid {
        fig,
        shape,
        spaces: Spaces::new(shape, &measured.cells, fig.size_pixels()),
    };
    let first = grid.subplot_params(params, None)?;
    let Some([left, bottom, right, top]) = params.rect else {
        return Ok(Some(first));
    };
    let set = |value: Option<f64>| value.unwrap_or_default();
    let rect = [
        left + set(first.left),
        bottom + set(first.bottom),
        right - (1.0 - set(first.right)),
        top - (1.0 - set(first.top)),
    ];
    grid.subplot_params(params, Some(rect)).map(Some)
}

/// The common grid being fitted.
struct Grid<'a> {
    fig: &'a FigureState,
    shape: (usize, usize),
    spaces: Spaces,
}

impl Grid<'_> {
    /// matplotlib's `_auto_adjust_subplotpars`: margins as large as the
    /// decorations along the grid's edges (plus the figure's title and
    /// labels) and spacings as large as those between cells, each with
    /// padding. The non-zero margins of `rect` (`[left, bottom, right,
    /// top]`) are kept as they are, as in matplotlib's truthiness tests.
    fn subplot_params(
        &self,
        params: &TightLayout,
        rect: Option<[f64; 4]>,
    ) -> Result<SubplotOverrides, TightLayoutError> {
        let (rows, cols) = self.shape;
        let (fig_width, fig_height) = self.fig.size_inches;
        let font_size_inch = crate::rc::params().font_size / 72.0;
        let pad = params.pad * font_size_inch;
        let w_pad = params.w_pad.map_or(pad, |p| p * font_size_inch) / fig_width;
        let h_pad = params.h_pad.map_or(pad, |p| p * font_size_inch) / fig_height;
        let (pad_w, pad_h) = (pad / fig_width, pad / fig_height);
        let given = |value: f64| (value != 0.0).then_some(value);
        let [left, bottom, right, top] = rect.map_or([None; 4], |[l, b, r, t]| {
            [
                given(l),
                given(b),
                given(r).map(|r| 1.0 - r).and_then(given),
                given(t).map(|t| 1.0 - t).and_then(given),
            ]
        });
        let texts = self.super_text_extents();
        let left = left.unwrap_or_else(|| {
            self.spaces.column_edge(0).max(0.0)
                + pad_w
                + texts.ylabel.map_or(0.0, |w| w.width() + pad_w)
        });
        let right = right.unwrap_or_else(|| self.spaces.column_edge(cols).max(0.0) + pad_w);
        let top = top.unwrap_or_else(|| {
            self.spaces.row_edge(0).max(0.0)
                + pad_h
                + texts.title.map_or(0.0, |t| t.height() + pad_h)
        });
        let bottom = bottom.unwrap_or_else(|| {
            self.spaces.row_edge(rows).max(0.0)
                + pad_h
                + texts.xlabel.map_or(0.0, |x| x.height() + pad_h)
        });
        if left + right >= 1.0 {
            return Err(TightLayoutError::TooWide);
        }
        if bottom + top >= 1.0 {
            return Err(TightLayoutError::TooTall);
        }
        let mut overrides = SubplotOverrides {
            left: Some(left),
            right: Some(1.0 - right),
            bottom: Some(bottom),
            top: Some(1.0 - top),
            ..SubplotOverrides::default()
        };
        if cols > 1 {
            let hspace = self.spaces.between_columns() + w_pad;
            let axes_width = (1.0 - right - left - hspace * (cols - 1) as f64) / cols as f64;
            if axes_width < 0.0 {
                return Err(TightLayoutError::TooWide);
            }
            overrides.wspace = Some(hspace / axes_width);
        }
        if rows > 1 {
            let vspace = self.spaces.between_rows() + h_pad;
            let axes_height = (1.0 - top - bottom - vspace * (rows - 1) as f64) / rows as f64;
            if axes_height < 0.0 {
                return Err(TightLayoutError::TooTall);
            }
            overrides.hspace = Some(vspace / axes_height);
        }
        Ok(overrides)
    }

    /// The extents of the figure's title and labels taking part in the
    /// layout, in figure fractions.
    fn super_text_extents(&self) -> SuperExtents {
        let fig = self.fig;
        let layout = LayoutContext::new(fig);
        let (width, height) = fig.size_pixels();
        let extent = |id: Option<ArtistId>| {
            let text = layout.store().get_as::<TextArtist>(id?)?;
            let bbox = text
                .base()
                .in_layout
                .then(|| text.window_extent(&layout))??;
            Some(Bbox::from_extents(
                bbox.x0() / width,
                bbox.y0() / height,
                bbox.x1() / width,
                bbox.y1() / height,
            ))
        };
        let texts = fig
            .store
            .get_as::<FigureFrame>(fig.root)
            .map(|frame| frame.super_texts)
            .unwrap_or_default();
        SuperExtents {
            title: extent(texts.title),
            xlabel: extent(texts.xlabel),
            ylabel: extent(texts.ylabel),
        }
    }
}

/// The extents of the figure's title and labels, in figure fractions.
struct SuperExtents {
    title: Option<Bbox>,
    xlabel: Option<Bbox>,
    ylabel: Option<Bbox>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gridspec::GridSpec;

    fn cell(rows: Range<usize>, cols: Range<usize>, position: Bbox, tight: Bbox) -> Cell {
        Cell {
            rows,
            cols,
            position,
            tight: Some(tight),
        }
    }

    #[test]
    fn spaces_add_up_per_edge() {
        // Two cells side by side on a 100 × 100 pixel canvas.
        let left = cell(
            0..1,
            0..1,
            Bbox::from_extents(0.1, 0.1, 0.4, 0.9),
            Bbox::from_extents(5.0, 8.0, 42.0, 93.0),
        );
        let right = cell(
            0..1,
            1..2,
            Bbox::from_extents(0.6, 0.1, 0.9, 0.9),
            Bbox::from_extents(55.0, 10.0, 90.0, 90.0),
        );
        let spaces = Spaces::new((1, 2), &[left, right], (100.0, 100.0));
        let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
        assert!(close(spaces.column_edge(0), 0.05));
        // The interior edge holds the left cell's right overhang plus the
        // right cell's left overhang.
        assert!(close(spaces.between_columns(), 0.02 + 0.05));
        assert!(close(spaces.row_edge(0), 0.03));
        assert!(close(spaces.row_edge(1), 0.02));
    }

    #[test]
    fn blocks_group_axes_of_the_same_cells() {
        let fig = Figure::new();
        let grid = GridSpec::new(1, 2).unwrap();
        let a = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        let b = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        let c = fig.add_subplot_spec(&grid.cell(0, 1).unwrap()).unwrap();
        let free = fig.add_axes([0.1, 0.1, 0.2, 0.2]);
        let ids = [a.id(), b.id(), c.id(), free.id()];
        let state = fig.raw().figure_state().borrow();
        let found = blocks(&state, &ids);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].1, vec![a.id(), b.id()]);
    }

    #[test]
    fn decorations_too_wide_leave_the_figure_alone() {
        let fig = Figure::with_size(1.0, 1.0, 100.0);
        let ax = fig.subplot();
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        ax.set_ylabel("a label\nover\nmany\nmany\nlines");
        let params = TightLayout {
            pad: 4.0,
            ..TightLayout::default()
        };
        assert_eq!(fig.tight_layout(&params), Err(TightLayoutError::TooWide));
        assert_eq!(ax.position().bounds().0, 0.125);
    }

    #[test]
    fn empty_cells_keep_their_share_of_the_grid() {
        let fig = Figure::new();
        let grid = GridSpec::new(2, 2).unwrap();
        let ax = fig.add_subplot_spec(&grid.cell(0, 0).unwrap()).unwrap();
        ax.set_xlim(0.0, 1.0).set_ylim(0.0, 1.0);
        fig.tight_layout(&TightLayout::default()).unwrap();
        let (_, y, w, h) = ax.position().bounds();
        // The empty cells to the right and below still take their share.
        assert!(w < 0.5 && h < 0.5 && y > 0.5, "{y} {w} {h}");
        let params = fig.subplot_params();
        assert!(params.wspace > 0.0 && params.hspace > 0.0);
    }
}
