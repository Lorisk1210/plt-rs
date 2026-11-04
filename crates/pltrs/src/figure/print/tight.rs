//! The tight box of a figure (matplotlib's `Figure.get_tightbbox`) and the
//! padding a tight save puts around it.

use pltrs_core::Bbox;

use super::PadInches;
use crate::artist::{ArtistHandle, ArtistId, LayoutContext, tight_extent_of};
use crate::axes::{AxesState, clipped_extent};
use crate::figure::frame::FigureFrame;
use crate::figure::{Figure, FigureState};
use crate::layout::LayoutEngine;
use crate::transforms::TransformContext;

impl FigureState {
    /// The tight box of frame `frame`, the root figure or a subfigure, in
    /// display pixels (matplotlib's `FigureBase.get_tightbbox`, before the
    /// root's is turned into inches): the union of its artists that are
    /// visible and take part in layout, of its subfigures that do (each
    /// its own tight box, or its box when nothing in it gives one) and of
    /// its visible axes; `None` when none of them gives a box. The figure
    /// is laid out first.
    fn frame_tight_bbox(&mut self, frame: ArtistId) -> Option<Bbox> {
        self.prepare();
        self.frame_tight_extent(frame)
    }

    /// [`frame_tight_bbox`](Self::frame_tight_bbox) in the figure laid out.
    fn frame_tight_extent(&mut self, frame: ArtistId) -> Option<Bbox> {
        let children = self.store.get_as::<FigureFrame>(frame)?.children.clone();
        let mut boxes = Vec::new();
        let mut axes = Vec::new();
        for child in children {
            if self.store.get_as::<FigureFrame>(child).is_some() {
                boxes.extend(self.subfigure_tight_extent(child));
            } else if self.store.get_as::<AxesState>(child).is_some() {
                axes.push(child);
            } else if crate::mplot3d::is_axes3d(&self.store, child) {
                boxes.extend(crate::mplot3d::axes3d_tight_bbox(self, child, false));
            } else {
                boxes.extend(self.artist_extent(child));
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

    /// The tight box of the subfigure `frame` in its parent's: its own,
    /// else its box, `None` while it is hidden or out of layout.
    fn subfigure_tight_extent(&mut self, frame: ArtistId) -> Option<Bbox> {
        let base = self.store.get(frame)?.base();
        if !(base.visible && base.in_layout) {
            return None;
        }
        self.frame_tight_extent(frame)
            .or_else(|| self.frame_bbox(frame))
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
        clipped_extent(layout, base, tight_extent_of(artist, &layout)?)
    }
}

impl Figure {
    /// The area everything visible is drawn in, in inches from the
    /// figure's bottom-left corner (matplotlib's `get_tightbbox`); the
    /// whole figure when nothing is drawn.
    pub fn tight_bbox(&self) -> Bbox {
        self.root_handle().measure_painters();
        self.state_cell().work_on(|state| {
            let dpi = state.dpi();
            let root = state.root;
            state.frame_tight_bbox(root).map_or_else(
                || {
                    let (w, h) = state.size_inches;
                    Bbox::from_bounds(0.0, 0.0, w, h)
                },
                |b| Bbox::from_extents(b.x0() / dpi, b.y0() / dpi, b.x1() / dpi, b.y1() / dpi),
            )
        })
    }

    /// The tight box of this figure or subfigure in display pixels:
    /// everything visible in it that takes part in layout, its subfigures
    /// each counting with its own tight box, or its box when nothing in
    /// it gives one; `None` when nothing does (matplotlib's
    /// `FigureBase.get_tightbbox`, in pixels for a subfigure, which
    /// [`tight_bbox`](Self::tight_bbox) turns into inches for the root).
    ///
    /// ```
    /// use pltrs::figure::{Figure, SubfigureOptions};
    /// let fig = Figure::with_size(6.4, 4.8, 100.0);
    /// let subfigs = fig.subfigures(1, 2, SubfigureOptions::default())?;
    /// subfigs[(0, 0)].subplot();
    /// let left = subfigs[(0, 0)].tight_extent().expect("an axes");
    /// assert!(left.x1() < 320.0);
    /// assert_eq!(subfigs[(0, 1)].tight_extent(), None, "nothing in it");
    /// let whole = fig.tight_extent().expect("both subfigures");
    /// assert_eq!(whole.x1(), 640.0, "the empty one counts with its box");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn tight_extent(&self) -> Option<Bbox> {
        self.root_handle().measure_painters();
        let frame = self.raw().id();
        self.state_cell()
            .work_on(|state| state.frame_tight_bbox(frame))
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
    use crate::artist::ClipPathSpec;

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
