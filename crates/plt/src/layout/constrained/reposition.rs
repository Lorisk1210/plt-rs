//! Moving subfigures, axes and colorbars into their solved cells
//! (matplotlib's `check_no_collapsed_axes`, `reposition_axes` and
//! `get_pos_and_bbox`).

use plt_core::Bbox;

use super::colorbars::Offsets;
use super::tree::{LayoutTree, frame_extent, from_frame, to_frame};
use crate::artist::{Artist, ArtistId};
use crate::axes::AxesState;
use crate::figure::{FigureFrame, FigureState};

impl LayoutTree {
    /// matplotlib's `check_no_collapsed_axes`: whether every cell of every
    /// grid holding an axes kept a positive size.
    pub(super) fn no_collapsed_axes(&self) -> bool {
        self.all_axes()
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

    /// matplotlib's `reposition_axes` for the frame `frame`: each
    /// subfigure moves to the outer box of its grid and is laid out in
    /// turn, then every grid-placed axes moves to the inner box of its
    /// cells and its colorbars follow.
    pub(super) fn reposition_axes(&self, fig: &mut FigureState, frame: usize) {
        let Some(extent) = frame_extent(fig, self.frames[frame].id) else {
            return;
        };
        for &sub in &self.frames[frame].subfigs {
            let bbox = self
                .grids
                .get(self.frames[sub].grid)
                .outer_bbox(&(0..1), &(0..1));
            if let Some(subfig) = fig.store.get_as_mut::<FigureFrame>(self.frames[sub].id) {
                subfig.relative_extent = to_frame(&bbox, &extent);
            }
            self.reposition_axes(fig, sub);
        }
        for ax in &self.frames[frame].axes {
            let Some(spec) = ax.cells() else { continue };
            let Some(grid) = self.grid_of(spec.gridspec()) else {
                return;
            };
            let bbox = self.grids.get(grid).inner_bbox(&spec.rows(), &spec.cols());
            place(fig, ax.id, &bbox);
            let mut offsets = Offsets::default();
            for colorbar in fig.colorbars_of(ax.id).iter().rev() {
                if colorbar.parents.first() == Some(&ax.id) {
                    self.reposition_colorbar(fig, colorbar, &mut offsets);
                }
            }
        }
    }
}

/// The extent of the frame holding `axes`, in figure fractions.
fn axes_frame_extent(fig: &FigureState, axes: ArtistId) -> Option<Bbox> {
    let frame = fig.store.get_as::<AxesState>(axes)?.base().parent?;
    frame_extent(fig, frame)
}

/// matplotlib's `get_pos_and_bbox`: an axes' original position and its
/// extent with decorations, both in figure fractions.
pub(super) fn pos_and_bbox(fig: &mut FigureState, axes: ArtistId) -> Option<(Bbox, Bbox)> {
    let local = fig.store.get_as::<AxesState>(axes)?.position;
    let position = from_frame(&local, &axes_frame_extent(fig, axes)?);
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

/// Moves an axes to `position` in figure fractions (matplotlib's
/// `_set_position` after `trans_fig_to_subfig`).
pub(super) fn place(fig: &mut FigureState, axes: ArtistId, position: &Bbox) {
    let Some(frame) = axes_frame_extent(fig, axes) else {
        return;
    };
    if let Some(state) = fig.store.get_as_mut::<AxesState>(axes) {
        state.position = to_frame(position, &frame);
    }
}
