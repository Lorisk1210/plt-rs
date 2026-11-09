//! Undoing a plotting call that fails. matplotlib's plotting methods make
//! their artists, properties and all, before they add any to the axes, so
//! a call that fails (on an invalid property, say) leaves the axes as it
//! was. A binding that sets properties on the artists a plotting method
//! made and added marks the axes first ([`Axes::call_mark`]) and undoes
//! the call when one fails ([`Axes::undo_since`]).

use std::collections::BTreeSet;

use pltrs_core::Bbox;

use super::aspect::AspectState;
use super::axis::AutoscaleState;
use super::state::NextData;
use super::{Axes, AxesState, DataLimits};
use crate::artist::{ArtistHandle, RawHandle};

/// What an axes held before a plotting call: how many children and
/// containers, its data and view limits (and whether the next data
/// replaces the data limits), its autoscaling and its aspect (which
/// `imshow` sets).
#[derive(Clone, Copy, Debug)]
pub struct CallMark {
    children: usize,
    containers: usize,
    data_lim: DataLimits,
    next_data: NextData,
    view_lim: Bbox,
    view_mutated: (bool, bool),
    autoscale: (AutoscaleState, AutoscaleState),
    aspect: AspectState,
}

impl Axes {
    /// A mark of what the axes holds now, to undo the plotting call that
    /// follows if it fails ([`Axes::undo_since`]).
    #[must_use]
    pub fn call_mark(&self) -> CallMark {
        self.raw().with::<AxesState, _>(|axes| CallMark {
            children: axes.children.len(),
            containers: axes.containers.len(),
            data_lim: axes.data_lim,
            next_data: axes.next_data,
            view_lim: axes.view_lim,
            view_mutated: axes.view_mutated,
            autoscale: (axes.xaxis.autoscale, axes.yaxis.autoscale),
            aspect: axes.aspect,
        })
    }

    /// Puts the limits, autoscaling, aspect and containers of the axes
    /// back as they were at `mark`, and returns the artists added since,
    /// in the order they were added, for the caller to remove them: the
    /// axes is then as it was before the call that failed (the axes
    /// sharing its limits keep those it gave them).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 1.0], [0.0, 1.0])?;
    /// let (mark, limits, view) = (ax.call_mark(), ax.data_lim(), ax.xlim());
    /// let line = ax.plot([5.0, 9.0], [5.0, 9.0])?;
    /// ax.set_xlim(4.0, 10.0)?;
    /// let added = ax.undo_since(&mark);
    /// assert_eq!(added, vec![line.raw().clone()]);
    /// assert_eq!((ax.data_lim(), ax.xlim()), (limits, view));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn undo_since(&self, mark: &CallMark) -> Vec<RawHandle> {
        let since: BTreeSet<_> = self.raw().with_mut::<AxesState, _>(|axes| {
            axes.data_lim = mark.data_lim;
            axes.next_data = mark.next_data;
            axes.view_lim = mark.view_lim;
            axes.view_mutated = mark.view_mutated;
            (axes.xaxis.autoscale, axes.yaxis.autoscale) = mark.autoscale;
            axes.aspect = mark.aspect;
            axes.containers.truncate(mark.containers);
            axes.children
                .get(mark.children..)
                .unwrap_or_default()
                .iter()
                .copied()
                .collect()
        });
        if since.is_empty() {
            return Vec::new();
        }
        self.raw().figure_state().borrow_mut().stale = true;
        // Only what was added: not the parts of the axes a call made on
        // the way (its titles and spines).
        self.added_children()
            .into_iter()
            .filter(|child| since.contains(&child.id()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::{ArtistHandle, RawHandle};
    use crate::axes::{AxesState, BarOptions};
    use crate::figure::Figure;

    #[test]
    fn undoing_a_call_restores_whether_the_next_data_replace_the_limits() {
        let ax = Figure::new().subplot();
        let mark = ax.call_mark();
        ax.plot([0.0, 1.0], [0.0, 2.0]).expect("plotted");
        assert!(!ax.ignores_data_limits());
        ax.undo_since(&mark);
        assert!(ax.ignores_data_limits(), "the next data replace them again");
    }

    #[test]
    fn undoing_nothing_changes_nothing() {
        let ax = Figure::new().subplot();
        ax.plot([0.0, 1.0], [0.0, 2.0]).expect("plotted");
        let mark = ax.call_mark();
        assert_eq!(ax.undo_since(&mark), Vec::<RawHandle>::new());
        assert_eq!(ax.added_children().len(), 1);
    }

    #[test]
    fn a_failed_call_leaves_no_container() {
        let ax = Figure::new().subplot();
        let mark = ax.call_mark();
        ax.bar([1.0, 2.0], [3.0, 4.0], BarOptions::default())
            .expect("bars");
        let added = ax.undo_since(&mark);
        assert_eq!(added.len(), 2, "the bars");
        assert!(
            ax.raw()
                .with::<AxesState, _>(|axes| axes.containers.is_empty())
        );
        assert!(ax.data_lim().bbox.is_null());
    }
}
