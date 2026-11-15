//! Removed artists standing in their figure while they are measured or
//! drawn by their own `draw`.
//!
//! matplotlib's `remove` and `clear` only take an artist out of the lists
//! of its figure and axes: the artist keeps its transforms, and an axes
//! its position, limits, tick helpers and children, so that it measures
//! itself (`get_window_extent`, `get_tightbbox`, its ticks and their
//! labels) as it did while it was drawn. pltrs keeps removed artists
//! apart from the figure's store ([`DetachedArtists`]), where the code
//! laying out and measuring a figure's artists does not look them up. So
//! a measure of a removed artist has its whole removed tree (the artist
//! removed itself with all it owns) *stand in*: the tree moves into the
//! store for the length of the measure, listed by no artist of the
//! figure, which therefore neither draws it nor hands it to a layout
//! engine, while the pass before a measure ([`FigureState::prepare`])
//! autoscales, places and ticks its axes as any other's, and every lookup
//! by id finds its artists. The tree goes back apart when the measure
//! ends ([`StandingIn`]). matplotlib's `artist.draw(renderer)` draws a
//! removed artist with all it owns too, and an Axes made for a figure
//! but never added (pltrs makes it removed): its tree stands in for the
//! length of that draw.
//!
//! [`DetachedArtists`]: crate::artist::DetachedArtists

use std::rc::Rc;

use super::{FigureCell, FigureState};
use crate::artist::{ArtistId, RawHandle};

impl FigureState {
    /// The artist removed itself whose tree holds the removed artist
    /// `id`: `id`, or the removed artist owning it. `None` for an artist
    /// in the figure.
    fn removed_root(&self, id: ArtistId) -> Option<ArtistId> {
        let mut root = id;
        // An owner is older than what it owns in every figure a program
        // makes; the bound keeps the walk finite in any other.
        for _ in 0..=self.detached.ids().count() {
            match self.detached.get(root)?.base().parent {
                Some(parent) => root = parent,
                None => return Some(root),
            }
        }
        None
    }

    /// Moves the removed tree holding `id` into the store, listed by no
    /// artist; returns its root, for [`stand_out`](Self::stand_out).
    /// `None` when `id` is in the figure (or in a tree standing in
    /// already).
    fn stand_in(&mut self, id: ArtistId) -> Option<ArtistId> {
        let root = self.removed_root(id)?;
        for (id, artist) in self.detached.take_tree(root) {
            self.store.put_back(id, artist);
        }
        self.detached.stand_in(root);
        Some(root)
    }

    /// Moves the tree of `root`, standing in, back apart, with what was
    /// added to it meanwhile; a tree added to the figure meanwhile stays.
    fn stand_out(&mut self, root: ArtistId) {
        if !self.detached.stand_out(root) {
            return;
        }
        let unowned = |artist: &dyn crate::artist::Artist| artist.base().parent.is_none();
        if self.store.get(root).is_some_and(unowned) {
            for (id, artist) in self.store.take_tree(root) {
                self.detached.insert(id, artist);
            }
        }
    }

    /// Whether `id` is the root of a removed tree standing in the store.
    pub(super) fn stands_in(&self, id: ArtistId) -> bool {
        self.detached.stands_in(id)
            && (self.store.get(id)).is_some_and(|artist| artist.base().parent.is_none())
    }
}

/// A removed tree standing in its figure (see the module): it goes back
/// apart when this is dropped.
#[must_use = "the tree stands in only while this lives"]
pub(crate) struct StandingIn {
    /// The figure and the root of the tree, when one was moved.
    tree: Option<(Rc<FigureCell>, ArtistId)>,
}

impl StandingIn {
    /// Whether a removed tree moved in (rather than the artist being in
    /// its figure, or its figure busy).
    pub(crate) fn moved(&self) -> bool {
        self.tree.is_some()
    }
}

impl Drop for StandingIn {
    fn drop(&mut self) {
        if let Some((cell, root)) = self.tree.take()
            && let Some(mut state) = cell.try_rearrange()
        {
            state.stand_out(root);
        }
    }
}

impl RawHandle {
    /// For each of `ids`, artists of this handle's figure: the removed
    /// artist whose tree holds it (itself, or the removed artist owning
    /// it), `None` for an artist in the figure. Bindings keeping one object
    /// per artist hold those of removed artists only while the program
    /// does, as matplotlib's figure holds no removed artist.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// ax.remove();
    /// let roots = line.raw().removed_roots(&[line.raw().id(), ax.raw().id()]);
    /// assert_eq!(roots, [Some(ax.raw().id()), Some(ax.raw().id())]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    #[must_use]
    pub fn removed_roots(&self, ids: &[ArtistId]) -> Vec<Option<ArtistId>> {
        let state = self.figure_state().borrow();
        ids.iter().map(|&id| state.removed_root(id)).collect()
    }

    /// Has the removed tree the artist is in stand in its figure until
    /// what is returned is dropped (see the module), for a measure of the
    /// artist or a draw of it by its own `draw`. Nothing moves for an artist in the figure, nor while code
    /// further up the call stack uses the figure.
    pub(crate) fn standing_in(&self) -> StandingIn {
        let cell = self.figure_state();
        let root = (cell.try_rearrange()).and_then(|mut state| state.stand_in(self.id()));
        StandingIn {
            tree: root.map(|root| (Rc::clone(cell), root)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::{Figure, FigureFrame, SubfigureOptions};
    use crate::spines::SpineSide;

    #[test]
    fn a_removed_tree_stands_in_only_while_measured() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        ax.remove();
        {
            let _standing = line.raw().standing_in();
            let state = fig.state();
            assert!(state.store.contains(ax.id()) && state.store.contains(line.id()));
            assert!(state.is_removed(ax.id()), "still removed");
            assert!(!state.is_removed(line.id()));
            let root = state.store.get_as::<FigureFrame>(state.root);
            assert!(!root.unwrap().children.contains(&ax.id()), "listed by none");
        }
        let state = fig.state();
        assert!(!state.store.contains(ax.id()) && state.detached.contains(line.id()));
        assert!(state.is_removed(ax.id()));
    }

    #[test]
    fn standing_in_nests_and_leaves_live_artists_alone() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let _live = ax.raw().standing_in();
        assert!(fig.state().store.contains(ax.id()));
        ax.remove();
        let outer = ax.raw().standing_in();
        {
            let _inner = ax.raw().standing_in();
            assert!(fig.state().store.contains(ax.id()));
        }
        assert!(fig.state().store.contains(ax.id()), "the outer one holds");
        drop(outer);
        assert!(!fig.state().store.contains(ax.id()));
    }

    #[test]
    fn an_axes_added_back_while_standing_in_stays() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.remove();
        let standing = ax.raw().standing_in();
        fig.attach(&ax).unwrap();
        drop(standing);
        assert!(fig.state().store.contains(ax.id()));
        assert_eq!(fig.axes(), [ax]);
    }

    #[test]
    fn removed_axes_measure_their_parts_where_they_were() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let ax = fig.subplot();
        let spine = ax.spines().get(SpineSide::Left).clone();
        let (drawn, tight) = (spine.window_extent(), ax.tight_bbox(false));
        let labels = ax.xaxis().tight_bbox();
        assert!(drawn.is_some() && tight.is_some() && labels.is_some());
        ax.remove();
        assert_eq!(spine.raw().measured_extent(), drawn);
        assert_eq!(ax.tight_bbox(false), tight);
        assert_eq!(ax.xaxis().tight_bbox(), labels);
    }

    #[test]
    fn a_removed_subfigure_keeps_its_place() {
        let fig = Figure::with_size(6.4, 4.8, 100.0);
        let parts = fig.subfigures(1, 2, SubfigureOptions::default()).unwrap();
        let left = parts[(0, 0)].clone();
        let inner = left.subplot();
        let (tight, axes) = (left.tight_extent(), inner.window_extent());
        assert!(tight.is_some_and(|tight| tight.x1() < 320.0) && axes.is_some());
        left.remove();
        assert_eq!(left.tight_extent(), tight);
        assert_eq!(inner.raw().measured_extent(), axes);
    }
}
