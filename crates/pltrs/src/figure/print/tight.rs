//! The tight box of a figure (matplotlib's `Figure.get_tightbbox`) and the
//! padding a tight save puts around it.

use pltrs_core::Bbox;

use super::PadInches;
use crate::artist::{ArtistId, LayoutContext};
use crate::axes::{AxesState, clipped_extent};
use crate::figure::frame::FigureFrame;
use crate::figure::{Figure, FigureState};
use crate::layout::LayoutEngine;

impl FigureState {
    /// The area everything visible is drawn in, in display pixels
    /// (matplotlib's `Figure.get_tightbbox`): the figure-level artists
    /// that take part in layout and the tight boxes of the visible axes;
    /// `None` when nothing is drawn.
    fn figure_tight_bbox(&mut self) -> Option<Bbox> {
        self.prepare();
        let mut boxes = Vec::new();
        let mut frames = vec![self.root];
        let mut axes = Vec::new();
        while let Some(frame) = frames.pop() {
            let Some(node) = self.store.get_as::<FigureFrame>(frame) else {
                continue;
            };
            for &child in &node.children {
                if self.store.get_as::<FigureFrame>(child).is_some() {
                    frames.push(child);
                } else if self.store.get_as::<AxesState>(child).is_some() {
                    axes.push(child);
                } else if crate::mplot3d::is_axes3d(&self.store, child) {
                    boxes.extend(crate::mplot3d::axes3d_tight_bbox(self, child, false));
                } else {
                    boxes.extend(self.artist_extent(child));
                }
            }
        }
        boxes.extend(
            axes.into_iter()
                .filter_map(|id| self.axes_tight_bbox(id, false)),
        );
        boxes
            .into_iter()
            .filter(|b| {
                b.width().is_finite()
                    && b.height().is_finite()
                    && (b.width() != 0.0 || b.height() != 0.0)
            })
            .reduce(|a, b| a.union(&b))
    }

    /// The extent of a visible figure-level artist that takes part in
    /// layout, reduced by its clipping (matplotlib's
    /// `Artist.get_tightbbox`).
    fn artist_extent(&self, id: ArtistId) -> Option<Bbox> {
        let artist = self.store.get(id)?;
        let base = artist.base();
        if !(base.visible && base.in_layout) {
            return None;
        }
        let layout = LayoutContext::new(self);
        clipped_extent(layout, base, artist.tight_extent(&layout)?)
    }
}

impl Figure {
    /// The area everything visible is drawn in, in inches from the
    /// figure's bottom-left corner (matplotlib's `get_tightbbox`); the
    /// whole figure when nothing is drawn.
    pub fn tight_bbox(&self) -> Bbox {
        let mut state = self.state_mut();
        let dpi = state.dpi();
        state.figure_tight_bbox().map_or_else(
            || {
                let (w, h) = state.size_inches;
                Bbox::from_bounds(0.0, 0.0, w, h)
            },
            |b| Bbox::from_extents(b.x0() / dpi, b.y0() / dpi, b.x1() / dpi, b.y1() / dpi),
        )
    }

    /// The horizontal and vertical padding, in inches, around a tight saved
    /// area: `pad`, or `default` (`savefig.pad_inches`) when unset or when
    /// the layout's padding is asked for without constrained layout.
    pub(super) fn tight_padding(&self, pad: Option<PadInches>, default: f64) -> (f64, f64) {
        match (pad, self.layout_engine()) {
            (Some(PadInches::Inches(pad)), _) => (pad, pad),
            (Some(PadInches::Layout), Some(LayoutEngine::Constrained(layout))) => {
                (layout.w_pad, layout.h_pad)
            }
            (Some(PadInches::Layout) | None, _) => (default, default),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::{ArtistHandle, ClipPathSpec};

    #[test]
    fn layout_padding_follows_constrained_layout_only() {
        // matplotlib's test_bbox_inches_tight_layout_constrained and
        // _notconstrained: `pad_inches="layout"` takes the constrained
        // layout's paddings, else falls back to `savefig.pad_inches`.
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let default = crate::rc::params().savefig_pad_inches;
        assert_eq!(
            fig.tight_padding(Some(PadInches::Layout), default),
            (default, default)
        );
        assert_eq!(fig.tight_padding(Some(0.3.into()), default), (0.3, 0.3));
        let layout = crate::layout::ConstrainedLayout {
            h_pad: 0.5,
            ..crate::layout::ConstrainedLayout::default()
        };
        fig.set_layout_engine(Some(LayoutEngine::Constrained(layout)))
            .expect("a new figure has no colorbars");
        assert_eq!(
            fig.tight_padding(Some(PadInches::Layout), default),
            (layout.w_pad, 0.5)
        );
        assert_eq!(fig.tight_padding(None, default), (default, default));
    }

    #[test]
    fn tight_boxes_leave_out_hidden_annotations() {
        // matplotlib's test_only_on_non_finite_bbox: an annotation whose
        // point cannot be shown reports the unit box as its window extent
        // but takes no space in the tight box.
        let fig = Figure::with_size(8.0, 6.0, 100.0);
        let ax = fig.subplot();
        ax.set_axis_off();
        let before = fig.tight_bbox();
        let note = ax
            .annotate("", [0.0, f64::NAN], crate::text::AnnotateOptions::default())
            .unwrap();
        assert_eq!(note.window_extent(), Some(Bbox::UNIT));
        assert_eq!(fig.tight_bbox(), before);
    }

    #[test]
    fn tight_boxes_take_the_clipping_of_figure_artists() {
        // matplotlib's test_bbox_inches_tight_clipping: a huge patch
        // clipped to a small path adds only the path's extent.
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let patch = fig.add_patch(crate::patches::Rectangle::new((-50.0, -50.0), 100.0, 100.0));
        patch.set_clip_path(Some(ClipPathSpec::Path {
            path: pltrs_core::Path::unit_rectangle(),
            transform: crate::transforms::Transform::Inches,
        }));
        assert_eq!(fig.tight_bbox(), Bbox::from_extents(0.0, 0.0, 1.0, 1.0));
    }
}
