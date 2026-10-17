//! Constrained layout (matplotlib's `_constrained_layout.py`): margins
//! around every grid cell as large as the decorations of its axes, room
//! for colorbars, and the cells sharing what is left.
//!
//! The pass runs twice, because decorations change size once the axes
//! moved (tick labels come and go). Each time it measures every grid-placed
//! axes and its colorbars, raises the margins of their cells, makes
//! submerged interior margins match, solves the grids and moves the axes
//! and colorbars into their cells, unless some cell collapsed.

mod colorbars;

use std::ops::Range;

use plt_core::Bbox;

use super::grid::{CellMargins, LayoutGrid, LayoutGrids, Margin, Margins, Parent};
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::{GridSpec, SubplotSpec};
use crate::rc::RcParams;
use crate::spines::SpineSide;
use crate::text::TextArtist;
use colorbars::{Offsets, parent_spans};

/// The parameters of constrained layout (matplotlib's
/// `ConstrainedLayoutEngine`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConstrainedLayout {
    /// Padding left and right of the decorations, in inches.
    pub w_pad: f64,
    /// Padding above and below the decorations, in inches.
    pub h_pad: f64,
    /// Space between columns, as a fraction of the figure width shared
    /// among the gaps (used when larger than the padding).
    pub wspace: f64,
    /// Space between rows, as a fraction of the figure height.
    pub hspace: f64,
    /// The part of the figure laid out, `[left, bottom, width, height]` in
    /// figure fractions.
    pub rect: [f64; 4],
}

impl ConstrainedLayout {
    /// The `figure.constrained_layout.*` rc parameters over the whole
    /// figure.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            w_pad: rc.figure_constrained_layout_w_pad,
            h_pad: rc.figure_constrained_layout_h_pad,
            wspace: rc.figure_constrained_layout_wspace,
            hspace: rc.figure_constrained_layout_hspace,
            rect: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

impl Default for ConstrainedLayout {
    /// [`ConstrainedLayout::from_rc`] with the current rc parameters.
    fn default() -> Self {
        Self::from_rc(&crate::rc::params())
    }
}

/// The paddings in figure fractions and the spacings.
#[derive(Clone, Copy, Debug)]
struct Pads {
    w_pad: f64,
    h_pad: f64,
    wspace: f64,
    hspace: f64,
}

/// An axes of the figure as the layout sees it.
#[derive(Clone, Debug)]
struct Placed {
    id: ArtistId,
    spec: Option<SubplotSpec>,
    in_layout: bool,
}

impl Placed {
    /// The grid cells of an axes taking part in the layout.
    fn cells(&self) -> Option<&SubplotSpec> {
        self.spec.as_ref().filter(|_| self.in_layout)
    }
}

/// matplotlib's `do_constrained_layout` on the figure.
pub(super) fn execute(fig: &mut FigureState, params: &ConstrainedLayout) {
    let (width, height) = fig.size_inches;
    let pads = Pads {
        w_pad: params.w_pad / width,
        h_pad: params.h_pad / height,
        wspace: params.wspace,
        hspace: params.hspace,
    };
    let axes = figure_axes(fig);
    let Some(mut tree) = LayoutTree::new(&axes, params.rect) else {
        return;
    };
    for _ in 0..2 {
        tree.make_layout_margins(fig, &axes, &pads);
        tree.make_margin_suptitles(fig, &pads);
        tree.match_submerged_margins(&axes);
        tree.grids.solve();
        if tree.no_collapsed_axes(&axes) {
            tree.reposition_axes(fig, &axes);
        }
        tree.reset_margins(&axes);
    }
}

/// The axes of the figure in the order they were added.
fn figure_axes(fig: &FigureState) -> Vec<Placed> {
    let Some(frame) = fig.store.get_as::<FigureFrame>(fig.root) else {
        return Vec::new();
    };
    frame
        .children
        .iter()
        .filter_map(|&id| {
            let axes = fig.store.get_as::<AxesState>(id)?;
            Some(Placed {
                id,
                spec: axes.subplot_spec.clone(),
                in_layout: axes.base().in_layout,
            })
        })
        .collect()
}

/// The layout grids of the figure and of every grid specification in use
/// (matplotlib's `layoutgrids` dictionary).
#[derive(Debug)]
struct LayoutTree {
    grids: LayoutGrids,
    /// The grid of each grid specification, matched by identity.
    specs: Vec<(GridSpec, usize)>,
}

/// The index of the figure's own grid.
const FIGURE: usize = 0;

impl LayoutTree {
    /// matplotlib's `make_layoutgrids`; `None` when no axes is on a grid.
    fn new(axes: &[Placed], rect: [f64; 4]) -> Option<Self> {
        let mut tree = Self {
            grids: LayoutGrids::default(),
            specs: Vec::new(),
        };
        tree.grids.push(LayoutGrid::single(Parent::Rect(rect)));
        for spec in axes.iter().filter_map(|ax| ax.spec.as_ref()) {
            tree.add_gridspec(spec.gridspec());
        }
        (!tree.specs.is_empty()).then_some(tree)
    }

    /// matplotlib's `make_layoutgrids_gs`: the grid of `gridspec`, nested
    /// in an outer one-cell grid when the specification subdivides a cell.
    fn add_gridspec(&mut self, gridspec: &GridSpec) -> usize {
        if let Some(index) = self.grid_of(gridspec) {
            return index;
        }
        let (nrows, ncols) = gridspec.shape();
        let parent = match gridspec.parent() {
            None => Parent::Cells {
                grid: FIGURE,
                rows: 0..1,
                cols: 0..1,
                inner: true,
            },
            Some(cell) => {
                let outer = self.add_gridspec(cell.gridspec());
                let top = self.grids.push(LayoutGrid::single(Parent::Cells {
                    grid: outer,
                    rows: cell.rows(),
                    cols: cell.cols(),
                    inner: false,
                }));
                Parent::Cells {
                    grid: top,
                    rows: 0..1,
                    cols: 0..1,
                    inner: false,
                }
            }
        };
        let index = self.grids.push(LayoutGrid::new(
            parent,
            nrows,
            ncols,
            gridspec.width_ratios().to_vec(),
            gridspec.height_ratios().to_vec(),
        ));
        self.specs.push((gridspec.clone(), index));
        index
    }

    /// The grid of `gridspec`, if it has one.
    fn grid_of(&self, gridspec: &GridSpec) -> Option<usize> {
        self.specs
            .iter()
            .find(|(known, _)| known.is_same(gridspec))
            .map(|&(_, index)| index)
    }

    /// matplotlib's `make_layout_margins`: each grid-placed axes raises the
    /// margins of its cells to its decorations plus padding, and makes
    /// room for its colorbars.
    fn make_layout_margins(&mut self, fig: &mut FigureState, axes: &[Placed], pads: &Pads) {
        for ax in axes {
            let Some(spec) = ax.cells() else { continue };
            let Some(grid) = self.grid_of(spec.gridspec()) else {
                return;
            };
            let Some((pos, bbox)) = pos_and_bbox(fig, ax.id) else {
                continue;
            };
            let mut margin = margin_from_padding(spec, pads);
            margin.left += pos.x0() - bbox.x0();
            margin.right += bbox.x1() - pos.x1();
            margin.bottom += pos.y0() - bbox.y0();
            margin.top += bbox.y1() - pos.y1();
            let (rows, cols) = (spec.rows(), spec.cols());
            for colorbar in fig.colorbars_of(ax.id) {
                let (Some(pad), Some((cb_rows, cb_cols)), Some((_, cbbbox))) = (
                    self.colorbar_pad(fig, &colorbar),
                    parent_spans(fig, &colorbar),
                    pos_and_bbox(fig, colorbar.cax),
                ) else {
                    continue;
                };
                match colorbar.location {
                    SpineSide::Right if cb_cols.end == cols.end => {
                        margin.rightcb += cbbbox.width() + pad;
                    }
                    SpineSide::Left if cb_cols.start == cols.start => {
                        margin.leftcb += cbbbox.width() + pad;
                    }
                    SpineSide::Top if cb_rows.start == rows.start => {
                        margin.topcb += cbbbox.height() + pad;
                    }
                    SpineSide::Bottom if cb_rows.end == rows.end => {
                        margin.bottomcb += cbbbox.height() + pad;
                    }
                    _ => {}
                }
                // Colorbars wider (or taller) than their parents.
                if matches!(colorbar.location, SpineSide::Top | SpineSide::Bottom) {
                    if cb_cols.start == cols.start && cbbbox.x0() < bbox.x0() {
                        margin.left += bbox.x0() - cbbbox.x0();
                    }
                    if cb_cols.end == cols.end && cbbbox.x1() > bbox.x1() {
                        margin.right += cbbbox.x1() - bbox.x1();
                    }
                } else {
                    if cb_rows.end == rows.end && cbbbox.y0() < bbox.y0() {
                        margin.bottom += bbox.y0() - cbbbox.y0();
                    }
                    if cb_rows.start == rows.start && cbbbox.y1() > bbox.y1() {
                        margin.top += cbbbox.y1() - bbox.y1();
                    }
                }
            }
            self.grids
                .get_mut(grid)
                .edit_outer_margin_mins(&margin, &rows, &cols);
        }
    }

    /// matplotlib's `make_margin_suptitles`: the figure title and labels
    /// move to the figure edges and the figure's margins make room for
    /// them.
    fn make_margin_suptitles(&mut self, fig: &mut FigureState, pads: &Pads) {
        let Some(texts) = fig
            .store
            .get_as::<FigureFrame>(fig.root)
            .map(|frame| frame.super_texts)
        else {
            return;
        };
        let (width, height) = fig.size_pixels();
        let placements = [
            (texts.title, Margin::Top),
            (texts.xlabel, Margin::Bottom),
            (texts.ylabel, Margin::Left),
        ];
        for (id, margin) in placements {
            let Some(text) = id.and_then(|id| fig.store.get_as_mut::<TextArtist>(id)) else {
                continue;
            };
            if !text.base().in_layout {
                continue;
            }
            match margin {
                Margin::Top => text.position.y = 1.0 - pads.h_pad,
                Margin::Bottom => text.position.y = pads.h_pad,
                _ => text.position.x = pads.w_pad,
            }
            let layout = LayoutContext::new(fig);
            let Some(bbox) = id
                .and_then(|id| layout.store().get(id))
                .and_then(|text| text.window_extent(&layout))
            else {
                continue;
            };
            let size = match margin {
                Margin::Left => bbox.width() / width + 2.0 * pads.w_pad,
                _ => bbox.height() / height + 2.0 * pads.h_pad,
            };
            self.grids.get_mut(FIGURE).edit_margin_min(margin, size, 0);
        }
    }

    /// matplotlib's `match_submerged_margins`: the margins inside axes
    /// spanning several columns (rows) match the largest such margins, so
    /// that offset spans line up.
    fn match_submerged_margins(&mut self, axes: &[Placed]) {
        let spans: Vec<(usize, Range<usize>, Range<usize>)> = axes
            .iter()
            .filter_map(|ax| {
                let spec = ax.cells()?;
                Some((self.grid_of(spec.gridspec())?, spec.rows(), spec.cols()))
            })
            .collect();
        for (grid, rows, cols) in &spans {
            if cols.len() > 1 {
                let all_cols = spans.iter().map(|(g, _, c)| (*g, c.clone()));
                self.match_interior(
                    *grid,
                    cols,
                    all_cols.collect(),
                    [Margin::Left, Margin::LeftCb],
                    [Margin::Right, Margin::RightCb],
                );
            }
            if rows.len() > 1 {
                let all_rows = spans.iter().map(|(g, r, _)| (*g, r.clone()));
                self.match_interior(
                    *grid,
                    rows,
                    all_rows.collect(),
                    [Margin::Top, Margin::TopCb],
                    [Margin::Bottom, Margin::BottomCb],
                );
            }
        }
    }

    /// Raises the interior `first` margins (`[inner, outer]`, on cells
    /// after the first of `span`) and `last` margins (on cells before the
    /// last) of `grid` to the largest interior margins of any spanning
    /// axes.
    fn match_interior(
        &mut self,
        grid: usize,
        span: &Range<usize>,
        others: Vec<(usize, Range<usize>)>,
        first: [Margin; 2],
        last: [Margin; 2],
    ) {
        let after_first = |s: &Range<usize>| s.start + 1..s.end;
        let before_last = |s: &Range<usize>| s.start..s.end - 1;
        let margins = self.grids.get(grid).margins();
        let first_outer = margins.span(first[1], after_first(span)).to_vec();
        let last_outer = margins.span(last[1], before_last(span)).to_vec();
        let mut max_first = max_sum(margins, first, after_first(span));
        let mut max_last = max_sum(margins, last, before_last(span));
        for (other, other_span) in others.into_iter().filter(|(_, s)| s.len() > 1) {
            let margins = self.grids.get(other).margins();
            max_first = max_first.max(max_sum(margins, first, after_first(&other_span)));
            max_last = max_last.max(max_sum(margins, last, before_last(&other_span)));
        }
        let grid = self.grids.get_mut(grid);
        for (cell, outer) in after_first(span).zip(first_outer) {
            grid.edit_margin_min(first[0], max_first - outer, cell);
        }
        for (cell, outer) in before_last(span).zip(last_outer) {
            grid.edit_margin_min(last[0], max_last - outer, cell);
        }
    }

    /// matplotlib's `check_no_collapsed_axes`: whether every cell of every
    /// grid holding an axes kept a positive size.
    fn no_collapsed_axes(&self, axes: &[Placed]) -> bool {
        axes.iter()
            .filter_map(|ax| self.grid_of(ax.spec.as_ref()?.gridspec()))
            .all(|index| {
                let grid = self.grids.get(index);
                let (nrows, ncols) = grid.shape();
                (0..nrows).all(|r| {
                    (0..ncols).all(|c| {
                        let bbox = grid.inner_bbox(&(r..r + 1), &(c..c + 1));
                        bbox.width() > 0.0 && bbox.height() > 0.0
                    })
                })
            })
    }

    /// matplotlib's `reposition_axes`: every grid-placed axes moves to the
    /// inner box of its cells, then its colorbars follow.
    fn reposition_axes(&self, fig: &mut FigureState, axes: &[Placed]) {
        for ax in axes {
            let Some(spec) = ax.cells() else { continue };
            let Some(grid) = self.grid_of(spec.gridspec()) else {
                return;
            };
            let bbox = self.grids.get(grid).inner_bbox(&spec.rows(), &spec.cols());
            place(fig, ax.id, bbox);
            let mut offsets = Offsets::default();
            for colorbar in fig.colorbars_of(ax.id).iter().rev() {
                if colorbar.parents.first() == Some(&ax.id) {
                    self.reposition_colorbar(fig, colorbar, &mut offsets);
                }
            }
        }
    }

    /// matplotlib's `reset_margins`.
    fn reset_margins(&mut self, axes: &[Placed]) {
        for ax in axes.iter().filter(|ax| ax.in_layout) {
            if let Some(grid) = ax.spec.as_ref().and_then(|s| self.grid_of(s.gridspec())) {
                self.grids.get_mut(grid).reset_margins();
            }
        }
        self.grids.get_mut(FIGURE).reset_margins();
    }
}

/// The largest sum of margins `kinds` over `cells`.
fn max_sum(margins: &Margins, kinds: [Margin; 2], cells: Range<usize>) -> f64 {
    let [a, b] = kinds;
    cells
        .map(|cell| margins.get(a, cell) + margins.get(b, cell))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// matplotlib's `get_margin_from_padding`: the outer margins of an axes'
/// cells, from the padding or, between cells, half the grid spacing.
fn margin_from_padding(spec: &SubplotSpec, pads: &Pads) -> CellMargins {
    let gridspec = spec.gridspec();
    let (wspace, hspace) = gridspec.spacing();
    let wspace = wspace.unwrap_or(pads.wspace) / 2.0;
    let hspace = hspace.unwrap_or(pads.hspace) / 2.0;
    let (nrows, ncols) = gridspec.shape();
    let (rows, cols) = (spec.rows(), spec.cols());
    let mut margin = CellMargins {
        leftcb: pads.w_pad,
        rightcb: pads.w_pad,
        bottomcb: pads.h_pad,
        topcb: pads.h_pad,
        ..CellMargins::default()
    };
    let column_space = wspace / ncols as f64;
    if column_space > pads.w_pad {
        if cols.start > 0 {
            margin.leftcb = column_space;
        }
        if cols.end < ncols {
            margin.rightcb = column_space;
        }
    }
    let row_space = hspace / nrows as f64;
    if row_space > pads.h_pad {
        if rows.end < nrows {
            margin.bottomcb = row_space;
        }
        if rows.start > 0 {
            margin.topcb = row_space;
        }
    }
    margin
}

/// matplotlib's `get_pos_and_bbox`: an axes' original position and its
/// extent with decorations, both in figure fractions.
fn pos_and_bbox(fig: &mut FigureState, axes: ArtistId) -> Option<(Bbox, Bbox)> {
    let position = fig.store.get_as::<AxesState>(axes)?.position;
    let (width, height) = fig.size_pixels();
    let bbox = fig.axes_tight_bbox(axes, true).map_or(position, |tight| {
        Bbox::from_extents(
            tight.x0() / width,
            tight.y0() / height,
            tight.x1() / width,
            tight.y1() / height,
        )
    });
    Some((position, bbox))
}

/// Moves an axes to `position` (matplotlib's `_set_position`).
fn place(fig: &mut FigureState, axes: ArtistId, position: Bbox) {
    if let Some(state) = fig.store.get_as_mut::<AxesState>(axes) {
        state.position = position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    fn pads() -> Pads {
        Pads {
            w_pad: 0.01,
            h_pad: 0.01,
            wspace: 0.02,
            hspace: 0.3,
        }
    }

    #[test]
    fn padding_between_cells_follows_the_spacing() {
        let grid = GridSpec::new(2, 2).unwrap();
        let margin = margin_from_padding(&grid.cell(1, 0).unwrap(), &pads());
        // wspace / 2 / ncols = 0.005 is below the pad; hspace / 2 / nrows
        // = 0.075 is above it, between the rows only.
        assert_eq!((margin.leftcb, margin.rightcb), (0.01, 0.01));
        assert_eq!((margin.topcb, margin.bottomcb), (0.075, 0.01));
    }

    #[test]
    fn titles_labels_and_suptitle_get_room() {
        let fig = Figure::new();
        fig.set_layout_engine(Some(super::super::LayoutEngine::Constrained(
            ConstrainedLayout::from_rc(&RcParams::matplotlib()),
        )));
        let axs = fig.subplots(2, 2).unwrap();
        for (i, ax) in axs.as_slice().iter().enumerate() {
            ax.set_xlim(0.0, 10.0).set_ylim(-1.0, 1.0);
            ax.set_title(format!("title {i}"));
            ax.set_xlabel("x label").set_ylabel("y label");
        }
        fig.suptitle("suptitle");
        fig.layout();
        // matplotlib 3.11 under the spec's reference configuration.
        let expected = [
            [0.104_818, 0.572_328, 0.374_852, 0.323_126],
            [0.604_818, 0.572_328, 0.374_852, 0.323_126],
            [0.104_818, 0.098_38, 0.374_852, 0.323_126],
            [0.604_818, 0.098_38, 0.374_852, 0.323_126],
        ];
        for (ax, expected) in axs.as_slice().iter().zip(expected) {
            let (x, y, w, h) = ax.position().bounds();
            for (actual, expected) in [x, y, w, h].into_iter().zip(expected) {
                assert!((actual - expected).abs() < 1e-6, "{actual} vs {expected}");
            }
        }
    }

    #[test]
    fn nested_grids_sit_in_an_outer_cell() {
        let fig = Figure::new();
        let outer = GridSpec::new(1, 2).unwrap();
        let inner =
            GridSpec::nested(&outer.cell(0, 1).unwrap(), 2, 1, None, None, (None, None)).unwrap();
        fig.add_subplot_spec(&inner.cell(0, 0).unwrap()).unwrap();
        fig.add_subplot_spec(&outer.cell(0, 0).unwrap()).unwrap();
        let axes = figure_axes(&fig.raw().figure_state().borrow());
        let mut tree = LayoutTree::new(&axes, [0.0, 0.0, 1.0, 1.0]).unwrap();
        assert_eq!(tree.specs.len(), 2, "the outer and the nested grid");
        tree.grids.solve();
        let nested = tree.grid_of(&inner).unwrap();
        let top = tree.grids.get(nested).outer_bbox(&(0..1), &(0..1));
        assert_eq!(
            (top.x0(), top.x1(), top.y0(), top.y1()),
            (0.5, 1.0, 0.5, 1.0)
        );
        assert!(LayoutTree::new(&[], [0.0, 0.0, 1.0, 1.0]).is_none());
    }
}
