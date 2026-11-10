//! Colorbars under constrained layout: their padding, the cells of their
//! parents and their placement beyond the parents' decorations
//! (matplotlib's `colorbar_get_pad`, `get_cb_parent_spans` and
//! `reposition_colorbar`).

use std::ops::Range;

use pltrs_core::Bbox;

use super::compress::actual_parent_position;
use super::reposition::{place, pos_and_bbox};
use super::tree::{LayoutTree, subplot_spec};
use crate::artist::Artist;
use crate::axes::AxesState;
use crate::figure::FigureState;
use crate::gridspec::SubplotSpec;
use crate::layout::ColorbarLayout;
use crate::spines::SpineSide;
use crate::transforms::Transform;

/// How far colorbars already push further ones out, per side.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Offsets {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
}

impl LayoutTree {
    /// matplotlib's `reposition_colorbar`: the colorbar goes beyond the
    /// decorations of its parents, past the padding and the colorbars
    /// already placed on that side; in a `compress`ed layout a single
    /// parent's actual box sets its length.
    pub(super) fn reposition_colorbar(
        &self,
        fig: &mut FigureState,
        colorbar: &ColorbarLayout,
        offsets: &mut Offsets,
        compress: bool,
    ) {
        let (Some(grid), Some((rows, cols)), Some(pad), Some((cbpos, cbbbox))) = (
            self.parents_grid(fig, colorbar),
            parent_spans(fig, colorbar),
            self.colorbar_pad(fig, colorbar),
            pos_and_bbox(fig, colorbar.cax),
        ) else {
            return;
        };
        let grid = self.grids.get(grid);
        let parent = grid.bbox_for_cb(&rows, &cols);
        let mut pb = grid.inner_bbox(&rows, &cols);
        if let ([only], true) = (colorbar.parents.as_slice(), compress)
            && let Some(actual) = actual_parent_position(fig, *only)
        {
            pb = match colorbar.location {
                SpineSide::Left | SpineSide::Right => {
                    Bbox::from_extents(pb.x0(), actual.y0(), pb.x1(), actual.y1())
                }
                SpineSide::Top | SpineSide::Bottom => {
                    Bbox::from_extents(actual.x0(), pb.y0(), actual.x1(), pb.y1())
                }
            };
        }
        let (fraction, shrink, anchor) = (colorbar.fraction, colorbar.shrink, colorbar.anchor);
        let placed = match colorbar.location {
            SpineSide::Right => {
                let slot = pb.shrunk(fraction, shrink).anchored(anchor, &pb);
                let margin = cbpos.x0() - cbbbox.x0();
                let dx = parent.x1() - slot.x0() + offsets.right + pad + margin;
                offsets.right += cbbbox.width() + pad;
                slot.translated(dx, 0.0)
            }
            SpineSide::Left => {
                let slot = pb.shrunk(fraction, shrink).anchored(anchor, &pb);
                let margin = cbpos.x0() - cbbbox.x0();
                let dx = parent.x0() - slot.x0() - cbbbox.width() - pad + margin - offsets.left;
                offsets.left += cbbbox.width() + pad;
                slot.translated(dx, 0.0)
            }
            SpineSide::Top => {
                let slot = pb.shrunk(shrink, fraction).anchored(anchor, &pb);
                let margin = cbpos.y0() - cbbbox.y0();
                let dy = parent.y1() - slot.y0() + offsets.top + pad + margin;
                offsets.top += cbbbox.height() + pad;
                slot.translated(0.0, dy)
            }
            SpineSide::Bottom => {
                let slot = pb.shrunk(shrink, fraction).anchored(anchor, &pb);
                let margin = cbpos.y0() - cbbbox.y0();
                let dy = parent.y0() - slot.y0() - cbbbox.height() - pad + margin - offsets.bottom;
                offsets.bottom += cbbbox.height() + pad;
                slot.translated(0.0, dy)
            }
        };
        let box_aspect = match colorbar.location {
            SpineSide::Left | SpineSide::Right => colorbar.aspect,
            SpineSide::Top | SpineSide::Bottom => 1.0 / colorbar.aspect,
        };
        place(fig, colorbar.cax, &placed);
        if let Some(cax) = fig.store.get_as_mut::<AxesState>(colorbar.cax) {
            cax.aspect.fix_box(anchor, box_aspect);
            // matplotlib's `cbax.set_transform(fig.transSubfigure)`: the
            // transform of the colorbar's axes, which draws nothing with it.
            if let Some(frame) = cax.base().parent {
                cax.base_mut().transform = Some(Transform::Figure(frame));
            }
        }
    }

    /// The grid of a colorbar's first parent.
    fn parents_grid(&self, fig: &FigureState, colorbar: &ColorbarLayout) -> Option<usize> {
        let first = subplot_spec(fig, *colorbar.parents.first()?)?;
        self.grid_of(first.gridspec())
    }

    /// matplotlib's `colorbar_get_pad`: the colorbar's pad as a fraction
    /// of its parents' current inner size across it.
    pub(super) fn colorbar_pad(&self, fig: &FigureState, colorbar: &ColorbarLayout) -> Option<f64> {
        let (rows, cols) = parent_spans(fig, colorbar)?;
        let outer = self
            .grids
            .get(self.parents_grid(fig, colorbar)?)
            .inner_bbox(&rows, &cols);
        let size = match colorbar.location {
            SpineSide::Left | SpineSide::Right => outer.width(),
            SpineSide::Top | SpineSide::Bottom => outer.height(),
        };
        Some(colorbar.pad * size)
    }
}

/// matplotlib's `get_cb_parent_spans`: the rows and columns covered by a
/// colorbar's parents (2-D or 3-D axes).
pub(super) fn parent_spans(
    fig: &FigureState,
    colorbar: &ColorbarLayout,
) -> Option<(Range<usize>, Range<usize>)> {
    let specs: Vec<SubplotSpec> = colorbar
        .parents
        .iter()
        .map(|&id| subplot_spec(fig, id))
        .collect::<Option<_>>()?;
    let rows =
        specs.iter().map(|s| s.rows().start).min()?..specs.iter().map(|s| s.rows().end).max()?;
    let cols =
        specs.iter().map(|s| s.cols().start).min()?..specs.iter().map(|s| s.cols().end).max()?;
    Some((rows, cols))
}
