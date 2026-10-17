//! The margins of constrained layout: room for the decorations and
//! colorbars of every axes, for the titles, labels and outside legends of
//! every (sub)figure, and matching margins inside spanning axes (matplotlib's
//! `make_layout_margins`, `make_margin_suptitles`,
//! `match_submerged_margins` and `reset_margins`).

use std::ops::Range;

use super::super::grid::{CellMargins, Margin, Margins};
use super::Pads;
use super::colorbars::parent_spans;
use super::reposition::pos_and_bbox;
use super::tree::{LayoutTree, frame_extent};
use crate::artist::{Artist, ArtistId, LayoutContext};
use crate::figure::{FigureFrame, FigureState};
use crate::gridspec::SubplotSpec;
use crate::legend::{OutsideEdge, outside_legends};
use crate::spines::SpineSide;
use crate::text::TextArtist;

impl LayoutTree {
    /// matplotlib's `make_layout_margins` for the frame `frame` and its
    /// subfigures: each subfigure raises the margins of its cells to the
    /// grid spacing, each grid-placed axes to its decorations plus
    /// padding, with room for its colorbars.
    pub(super) fn make_layout_margins(&mut self, fig: &mut FigureState, frame: usize, pads: &Pads) {
        for sub in self.frames[frame].subfigs.clone() {
            self.make_layout_margins(fig, sub, pads);
            let Some(spec) = self.frames[sub].spec.clone() else {
                continue;
            };
            let Some(grid) = self.grid_of(spec.gridspec()) else {
                continue;
            };
            let no_pad = Pads {
                w_pad: 0.0,
                h_pad: 0.0,
                ..*pads
            };
            let margin = margin_from_padding(&spec, &no_pad);
            self.grids
                .get_mut(grid)
                .edit_outer_margin_mins(&margin, &spec.rows(), &spec.cols());
        }
        for ax in self.frames[frame].axes.clone() {
            let Some(spec) = ax.cells() else { continue };
            let Some(grid) = self.grid_of(spec.gridspec()) else {
                return;
            };
            let Some(margin) = self.axes_margin(fig, ax.id, spec, pads) else {
                continue;
            };
            self.grids
                .get_mut(grid)
                .edit_outer_margin_mins(&margin, &spec.rows(), &spec.cols());
        }
        self.make_legend_margins(fig, frame, pads);
    }

    /// Room at the frame's edges for its `"outside ..."` legends.
    fn make_legend_margins(&mut self, fig: &FigureState, frame: usize, pads: &Pads) {
        let (width, height) = fig.size_pixels();
        let (id, grid) = (self.frames[frame].id, self.frames[frame].grid);
        for (edge, bbox) in outside_legends(fig, id) {
            let w = bbox.width() / width + 2.0 * pads.w_pad;
            let h = bbox.height() / height + 2.0 * pads.h_pad;
            let (margin, size) = match edge {
                OutsideEdge::Upper => (Margin::Top, h),
                OutsideEdge::Lower => (Margin::Bottom, h),
                OutsideEdge::Left => (Margin::Left, w),
                OutsideEdge::Right => (Margin::Right, w),
                OutsideEdge::Center => continue,
            };
            self.grids.get_mut(grid).edit_margin_min(margin, size, 0);
        }
    }

    /// The margins an axes on `spec` needs: the padding, its decorations
    /// and room for its colorbars.
    fn axes_margin(
        &self,
        fig: &mut FigureState,
        axes: ArtistId,
        spec: &SubplotSpec,
        pads: &Pads,
    ) -> Option<CellMargins> {
        let (pos, bbox) = pos_and_bbox(fig, axes)?;
        let mut margin = margin_from_padding(spec, pads);
        margin.left += pos.x0() - bbox.x0();
        margin.right += bbox.x1() - pos.x1();
        margin.bottom += pos.y0() - bbox.y0();
        margin.top += bbox.y1() - pos.y1();
        let (rows, cols) = (spec.rows(), spec.cols());
        for colorbar in fig.colorbars_of(axes) {
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
        Some(margin)
    }

    /// matplotlib's `make_margin_suptitles` for the frame `frame` and its
    /// subfigures: the title and labels of each move to its edges and the
    /// frame's margins make room for them.
    pub(super) fn make_margin_suptitles(
        &mut self,
        fig: &mut FigureState,
        frame: usize,
        pads: &Pads,
    ) {
        for sub in self.frames[frame].subfigs.clone() {
            self.make_margin_suptitles(fig, sub, pads);
        }
        let (id, grid) = (self.frames[frame].id, self.frames[frame].grid);
        let (Some(texts), Some(extent)) = (
            fig.store
                .get_as::<FigureFrame>(id)
                .map(|frame| frame.super_texts),
            frame_extent(fig, id),
        ) else {
            return;
        };
        // The paddings in the frame's own fractions.
        let w_pad_local = pads.w_pad / extent.width();
        let h_pad_local = pads.h_pad / extent.height();
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
                Margin::Top => text.position.y = 1.0 - h_pad_local,
                Margin::Bottom => text.position.y = h_pad_local,
                _ => text.position.x = w_pad_local,
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
            self.grids.get_mut(grid).edit_margin_min(margin, size, 0);
        }
    }

    /// matplotlib's `match_submerged_margins`, frame by frame: the
    /// margins inside axes spanning several columns (rows) match the
    /// largest such margins among the frame's axes, so that offset spans
    /// line up.
    pub(super) fn match_submerged_margins(&mut self, frame: usize) {
        for sub in self.frames[frame].subfigs.clone() {
            self.match_submerged_margins(sub);
        }
        let spans: Vec<(usize, Range<usize>, Range<usize>)> = self.frames[frame]
            .axes
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

    /// matplotlib's `reset_margins`: the grids of the axes taking part,
    /// then those of the frames.
    pub(super) fn reset_margins(&mut self) {
        let grids: Vec<usize> = self
            .all_axes()
            .filter(|ax| ax.in_layout)
            .filter_map(|ax| self.grid_of(ax.spec.as_ref()?.gridspec()))
            .chain(self.frames.iter().map(|frame| frame.grid))
            .collect();
        for grid in grids {
            self.grids.get_mut(grid).reset_margins();
        }
    }
}

/// The largest sum of margins `kinds` over `cells`.
fn max_sum(margins: &Margins, kinds: [Margin; 2], cells: Range<usize>) -> f64 {
    let [a, b] = kinds;
    cells
        .map(|cell| margins.get(a, cell) + margins.get(b, cell))
        .fold(f64::NEG_INFINITY, f64::max)
}

/// matplotlib's `get_margin_from_padding`: the outer margins of the
/// cells of `spec`, from the padding or, between cells, half the grid
/// spacing.
pub(super) fn margin_from_padding(spec: &SubplotSpec, pads: &Pads) -> CellMargins {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gridspec::GridSpec;

    #[test]
    fn padding_between_cells_follows_the_spacing() {
        let pads = Pads {
            w_pad: 0.01,
            h_pad: 0.01,
            wspace: 0.02,
            hspace: 0.3,
        };
        let grid = GridSpec::new(2, 2).unwrap();
        let margin = margin_from_padding(&grid.cell(1, 0).unwrap(), &pads);
        // wspace / 2 / ncols = 0.005 is below the pad; hspace / 2 / nrows
        // = 0.075 is above it, between the rows only.
        assert_eq!((margin.leftcb, margin.rightcb), (0.01, 0.01));
        assert_eq!((margin.topcb, margin.bottomcb), (0.075, 0.01));
    }
}
