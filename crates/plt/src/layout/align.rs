//! Aligned axis labels (matplotlib's `Figure.align_labels`): the x labels
//! of subplots ending on the same grid row, and the y labels of subplots
//! starting on the same column, are placed beyond the tick labels of the
//! whole group, so that they line up.

use std::ops::Range;

use crate::artist::{ArtistHandle, ArtistId};
use crate::axes::{Axes, AxesState, Grouper};
use crate::figure::Figure;
use crate::ticker::AxisName;

/// The axes whose labels align, per (sub)figure and axis (matplotlib's
/// `_align_label_groups` of each figure).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct LabelGroups {
    frames: Vec<(ArtistId, [Grouper; 2])>,
}

/// The index of an axis among a figure's groupers.
fn slot(name: AxisName) -> usize {
    match name {
        AxisName::X => 0,
        AxisName::Y => 1,
    }
}

impl LabelGroups {
    /// Aligns the `name` labels of `a` and `b`, and of everything aligned
    /// with either, in the figure `frame` (matplotlib's `Grouper.join`).
    fn join(&mut self, frame: ArtistId, name: AxisName, a: ArtistId, b: ArtistId) {
        let index = self
            .frames
            .iter()
            .position(|(f, _)| *f == frame)
            .unwrap_or_else(|| {
                self.frames.push((frame, Default::default()));
                self.frames.len() - 1
            });
        self.frames[index].1[slot(name)].join(a, b);
    }

    /// The axes whose `name` labels align with those of `axes` in its
    /// figure `frame`, `axes` included (matplotlib's
    /// `Grouper.get_siblings`).
    pub(crate) fn siblings(
        &self,
        frame: ArtistId,
        name: AxisName,
        axes: ArtistId,
    ) -> Vec<ArtistId> {
        self.frames.iter().find(|(f, _)| *f == frame).map_or_else(
            || vec![axes],
            |(_, groups)| groups[slot(name)].siblings(axes),
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
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let axs = fig.subplots(1, 2)?;
    /// for ax in axs.as_slice() {
    ///     ax.set_xlabel("x");
    /// }
    /// fig.align_xlabels(None);
    /// # Ok::<(), plt::gridspec::GridSpecError>(())
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

    fn align(&self, name: AxisName, axes: Option<&[Axes]>) -> &Self {
        let ids: Vec<ArtistId> = match axes {
            Some(axes) => axes.iter().map(ArtistHandle::id).collect(),
            None => self.axes().iter().map(ArtistHandle::id).collect(),
        };
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
                    state.layout.label_groups.join(frame, name, *id, *other);
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

    #[test]
    fn groups_belong_to_their_figure() {
        let fig = Figure::new();
        let ids: Vec<ArtistId> = (0..2)
            .map(|_| fig.add_axes([0.0, 0.0, 1.0, 1.0]).id())
            .collect();
        let (root, other) = (fig.raw().id(), ids[1]);
        let mut groups = LabelGroups::default();
        groups.join(root, AxisName::X, ids[0], ids[1]);
        assert_eq!(groups.siblings(root, AxisName::X, ids[0]), ids);
        assert_eq!(groups.siblings(root, AxisName::Y, ids[0]), vec![ids[0]]);
        assert_eq!(groups.siblings(other, AxisName::X, ids[0]), vec![ids[0]]);
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
}
