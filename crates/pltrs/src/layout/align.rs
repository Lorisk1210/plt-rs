//! Aligned axis labels and titles (matplotlib's `Figure.align_labels` and
//! `align_titles`): the x labels of subplots ending on the same grid row,
//! and the y labels of subplots starting on the same column, are placed
//! beyond the tick labels of the whole group; the titles of subplots
//! starting on the same row clear the decorations of the whole row. Either
//! way they line up.

use std::ops::Range;

use crate::artist::{ArtistHandle, ArtistId};
use crate::axes::{Axes, AxesState, Grouper};
use crate::figure::Figure;
use crate::ticker::AxisName;

/// What a figure lines up across its subplots (the keys of matplotlib's
/// `_align_label_groups`: `"x"`, `"y"` and `"title"`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aligned {
    /// The labels of the x or the y axis.
    Labels(AxisName),
    /// The titles.
    Titles,
}

impl Aligned {
    /// The index of the group among a figure's groupers.
    fn slot(self) -> usize {
        match self {
            Self::Labels(AxisName::X) => 0,
            Self::Labels(AxisName::Y) => 1,
            Self::Titles => 2,
        }
    }
}

/// The axes whose labels or titles align, per (sub)figure (matplotlib's
/// `_align_label_groups` of each figure).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LabelGroups {
    frames: Vec<(ArtistId, [Grouper; 3])>,
}

pltrs_core::archive_struct!(LabelGroups { frames });

impl LabelGroups {
    /// Aligns the `aligned` texts of `a` and `b`, and of everything aligned
    /// with either, in the figure `frame` (matplotlib's `Grouper.join`).
    fn join(&mut self, frame: ArtistId, aligned: Aligned, a: ArtistId, b: ArtistId) {
        let index = self
            .frames
            .iter()
            .position(|(f, _)| *f == frame)
            .unwrap_or_else(|| {
                self.frames.push((frame, Default::default()));
                self.frames.len() - 1
            });
        self.frames[index].1[aligned.slot()].join(a, b);
    }

    /// The axes whose `aligned` texts align with those of `axes` in its
    /// figure `frame`, `axes` included (matplotlib's
    /// `Grouper.get_siblings`).
    pub(crate) fn siblings(
        &self,
        frame: ArtistId,
        aligned: Aligned,
        axes: ArtistId,
    ) -> Vec<ArtistId> {
        self.frames.iter().find(|(f, _)| *f == frame).map_or_else(
            || vec![axes],
            |(_, groups)| groups[aligned.slot()].siblings(axes),
        )
    }
}

impl Figure {
    /// Aligns the x labels and the y labels of the subplots (matplotlib's
    /// `align_labels`); see [`Figure::align_xlabels`] and
    /// [`Figure::align_ylabels`]. `None` aligns every axes of the figure.
    pub fn align_labels(&self, axes: Option<&[Axes]>) -> &Self {
        self.align_xlabels(axes).align_ylabels(axes)
    }

    /// Aligns the x labels of subplots whose labels are on the same side
    /// and that end on the same grid row (bottom labels) or start on it
    /// (top labels), for every later draw (matplotlib's `align_xlabels`).
    /// Axes outside grids are left alone.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let axs = fig.subplots(1, 2)?;
    /// for ax in axs.as_slice() {
    ///     ax.set_xlabel("x");
    /// }
    /// fig.align_xlabels(None);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn align_xlabels(&self, axes: Option<&[Axes]>) -> &Self {
        self.align(AxisName::X, axes)
    }

    /// Aligns the y labels of subplots whose labels are on the same side
    /// and that start on the same grid column (left labels) or end on it
    /// (right labels), for every later draw (matplotlib's
    /// `align_ylabels`). Axes outside grids are left alone.
    pub fn align_ylabels(&self, axes: Option<&[Axes]>) -> &Self {
        self.align(AxisName::Y, axes)
    }

    /// Aligns the titles of subplots that start on the same grid row, for
    /// every later draw, unless their position was set by hand (matplotlib's
    /// `align_titles`). `None` aligns every axes of the figure; axes outside
    /// grids are left alone.
    ///
    /// ```
    /// use pltrs::axes::TickPosition;
    /// use pltrs::prelude::*;
    /// use pltrs::spines::SpineSide;
    /// let fig = Figure::new();
    /// let axs = fig.subplots(1, 2)?;
    /// axs[(0, 0)].set_title("left");
    /// axs[(0, 1)].set_title("right");
    /// // The right title rises above the tick labels on top; the left
    /// // one follows.
    /// axs[(0, 1)].xaxis().set_ticks_position(TickPosition::Side(SpineSide::Top))?;
    /// fig.align_titles(None);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn align_titles(&self, axes: Option<&[Axes]>) -> &Self {
        let ids = self.axes_ids(axes);
        let frame = self.raw().id();
        let mut state = self.raw().figure_state().borrow_mut();
        let rows: Vec<(ArtistId, usize)> = ids
            .into_iter()
            .filter_map(|id| {
                let spec = state.store.get_as::<AxesState>(id)?.subplot_spec.as_ref()?;
                Some((id, spec.rows().start))
            })
            .collect();
        for (id, row) in &rows {
            for (other, other_row) in &rows {
                if row == other_row {
                    state
                        .layout
                        .label_groups
                        .join(frame, Aligned::Titles, *id, *other);
                }
            }
        }
        state.stale = true;
        self
    }

    /// Aligns the `aligned` texts of `a` and `b`, and of every axes
    /// aligned with either, for every later draw, whatever grid they are in
    /// (matplotlib's `_align_label_groups[...].join(a, b)`).
    ///
    /// ```
    /// use pltrs::layout::Aligned;
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::AxisName;
    /// let fig = Figure::new();
    /// let axs = fig.subplots(2, 1)?;
    /// let x = Aligned::Labels(AxisName::X);
    /// fig.join_aligned(x, &axs[(0, 0)], &axs[(1, 0)]);
    /// assert_eq!(fig.aligned_with(x, &axs[(0, 0)]).len(), 2);
    /// # Ok::<(), pltrs::gridspec::GridSpecError>(())
    /// ```
    pub fn join_aligned(&self, aligned: Aligned, a: &Axes, b: &Axes) -> &Self {
        let frame = self.raw().id();
        let mut state = self.raw().figure_state().borrow_mut();
        state
            .layout
            .label_groups
            .join(frame, aligned, a.id(), b.id());
        state.stale = true;
        self
    }

    /// The axes whose `aligned` texts align with those of `axes` in this
    /// figure, `axes` included (matplotlib's
    /// `_align_label_groups[...].get_siblings(axes)`).
    #[must_use]
    pub fn aligned_with(&self, aligned: Aligned, axes: &Axes) -> Vec<Axes> {
        let ids = self
            .raw()
            .figure_state()
            .borrow()
            .layout
            .label_groups
            .siblings(self.raw().id(), aligned, axes.id());
        axes.in_same_figure(ids)
    }

    /// The ids of `axes`, or of every axes of the figure.
    fn axes_ids(&self, axes: Option<&[Axes]>) -> Vec<ArtistId> {
        match axes {
            Some(axes) => axes.iter().map(ArtistHandle::id).collect(),
            None => self.axes().iter().map(ArtistHandle::id).collect(),
        }
    }

    fn align(&self, name: AxisName, axes: Option<&[Axes]>) -> &Self {
        let ids = self.axes_ids(axes);
        let frame = self.raw().id();
        let mut state = self.raw().figure_state().borrow_mut();
        let placed: Vec<(ArtistId, bool, Range<usize>)> = ids
            .into_iter()
            .filter_map(|id| {
                let axes = state.store.get_as::<AxesState>(id)?;
                let spec = axes.subplot_spec.as_ref()?;
                let high = axes.axis(name).label.side.is_high();
                let span = match name {
                    AxisName::X => spec.rows(),
                    AxisName::Y => spec.cols(),
                };
                Some((id, high, span))
            })
            .collect();
        for (id, high, span) in &placed {
            for (other, other_high, other_span) in &placed {
                // Top and left labels align on the first row or column
                // of their span, bottom and right ones on the last.
                let aligned = match (name, high) {
                    (AxisName::X, true) | (AxisName::Y, false) => span.start == other_span.start,
                    (AxisName::X, false) | (AxisName::Y, true) => span.end == other_span.end,
                };
                if high == other_high && aligned {
                    state
                        .layout
                        .label_groups
                        .join(frame, Aligned::Labels(name), *id, *other);
                }
            }
        }
        state.stale = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::LayoutContext;
    use crate::layout::TightLayout;
    use pltrs_core::Bbox;

    #[test]
    fn groups_belong_to_their_figure() {
        let fig = Figure::new();
        let ids: Vec<ArtistId> = (0..2)
            .map(|_| {
                fig.add_axes([0.0, 0.0, 1.0, 1.0])
                    .expect("a valid rectangle")
                    .id()
            })
            .collect();
        let (root, other) = (fig.raw().id(), ids[1]);
        let mut groups = LabelGroups::default();
        let x = Aligned::Labels(AxisName::X);
        groups.join(root, x, ids[0], ids[1]);
        assert_eq!(groups.siblings(root, x, ids[0]), ids);
        assert_eq!(
            groups.siblings(root, Aligned::Labels(AxisName::Y), ids[0]),
            vec![ids[0]]
        );
        assert_eq!(groups.siblings(root, Aligned::Titles, ids[0]), vec![ids[0]]);
        assert_eq!(groups.siblings(other, x, ids[0]), vec![ids[0]]);
    }

    #[test]
    fn labels_align_along_rows_and_columns() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 2).unwrap();
        let ids: Vec<ArtistId> = axs.as_slice().iter().map(ArtistHandle::id).collect();
        fig.align_labels(None);
        let state = fig.raw().figure_state().borrow();
        let layout = LayoutContext::new(&state);
        let sorted = |mut v: Vec<ArtistId>| {
            v.sort_unstable();
            v
        };
        assert_eq!(
            sorted(layout.label_siblings(ids[0], AxisName::X)),
            vec![ids[0], ids[1]]
        );
        assert_eq!(
            sorted(layout.label_siblings(ids[0], AxisName::Y)),
            vec![ids[0], ids[2]]
        );
    }

    /// A figure of two rows whose upper y label aligns with the wide tick
    /// labels of the lower axes, after `measure_first` or not.
    #[allow(
        clippy::approx_constant,
        reason = "a Bode plot's tick values, as python-control prints them"
    )]
    fn aligned_rows_laid_out(measure_first: bool) -> Bbox {
        let fig = Figure::new();
        let axs = fig.subplots(2, 1).unwrap();
        let (upper, lower) = (&axs[(0, 0)], &axs[(1, 0)]);
        fig.align_labels(None);
        upper.plot([0.0, 1.0], [-45.0, -3.0]).unwrap();
        lower.plot([0.0, 1.0], [-4.4, 0.0]).unwrap();
        lower
            .yaxis()
            .set_ticks(vec![0.0, -0.785, -1.571, -2.356, -3.142, -3.927]);
        upper.set_ylabel("Magnitude [dB]");
        if measure_first {
            fig.tight_bbox();
        }
        fig.tight_layout(TightLayout::default()).unwrap();
        upper.position()
    }

    #[test]
    fn a_first_layout_places_labels_beyond_the_siblings_tick_labels() {
        // matplotlib measures the siblings' ticks anew (0.150960 of the
        // figure's width is where its tight layout puts both axes).
        let laid_out = aligned_rows_laid_out(false);
        assert_eq!(laid_out, aligned_rows_laid_out(true));
        assert!((laid_out.x0() - 0.150_960).abs() < 1e-6, "{laid_out:?}");
    }
}
