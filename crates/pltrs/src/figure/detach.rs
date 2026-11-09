//! Removing artists from a figure and adding them back.
//!
//! A removed artist is not dropped: it moves, with the artists it owns,
//! from the figure's store to its [`DetachedArtists`], under the same ids,
//! so that its handles keep working. The figure draws, lays out,
//! searches and autoscales over its store alone; handles find their
//! artist in either place.

use std::rc::Rc;

use super::{FigureCell, FigureFrame, FigureState};
use crate::artist::{
    Artist, ArtistId, ArtistLookup, ArtistStore, AttachError, KeptArtist, RawHandle, downcast_mut,
    downcast_ref,
};
use crate::axes::AxesState;
use crate::legend::LegendArtist;
use crate::mplot3d::Axes3DState;

/// A figure's artists, in the figure or removed from it.
impl ArtistLookup for FigureState {
    fn artist(&self, id: ArtistId) -> Option<&dyn Artist> {
        self.store.get(id).or_else(|| self.detached.get(id))
    }
}

impl FigureState {
    /// The artist with `id`, in the figure or removed from it, shared (see
    /// [`ArtistStore::shared`]).
    pub(crate) fn shared_artist(&self, id: ArtistId) -> Option<Rc<dyn Artist>> {
        self.store.shared(id).or_else(|| self.detached.shared(id))
    }

    /// The artist with `id`, in the figure or removed from it, mutably.
    pub(crate) fn artist_mut(&mut self, id: ArtistId) -> Option<&mut (dyn Artist + 'static)> {
        if self.store.contains(id) {
            self.store.get_mut(id)
        } else {
            self.detached.get_mut(id)
        }
    }

    /// The artist with `id` as concrete type `T`, in the figure or
    /// removed from it, mutably.
    pub(crate) fn artist_as_mut<T: Artist>(&mut self, id: ArtistId) -> Option<&mut T> {
        downcast_mut(self.artist_mut(id)?)
    }

    /// Runs `f` with mutable access to artist `id` (as `T`), in the figure
    /// or removed from it, and to the artists of the figure besides it.
    /// Returns `None` if the artist is not a `T`.
    pub(crate) fn with_others<T: Artist, R>(
        &mut self,
        id: ArtistId,
        f: impl FnOnce(&mut T, &mut ArtistStore) -> R,
    ) -> Option<R> {
        if self.store.contains(id) {
            return self.store.with_others(id, f);
        }
        let mut artist = self.detached.take(id)?;
        let result =
            downcast_mut::<T>(artist.get_mut()).map(|concrete| f(concrete, &mut self.store));
        self.detached.insert(id, artist);
        result
    }

    /// Whether artist `id` is in the figure: neither it nor an artist
    /// owning it was removed.
    pub(crate) fn is_attached(&self, id: ArtistId) -> bool {
        self.store.contains(id)
    }

    /// Whether artist `id` is a part its owner made of itself rather than
    /// one added to it (see [`RawHandle::is_part`]).
    fn is_part(&self, id: ArtistId) -> bool {
        let Some(owner) = self
            .artist(id)
            .and_then(|artist| artist.base().parent)
            .and_then(|parent| self.artist(parent))
        else {
            return false;
        };
        match downcast_ref::<AxesState>(owner) {
            Some(axes) => axes.spines.contains(&id) || axes.titles.contains(&id),
            None => downcast_ref::<LegendArtist>(owner).is_some(),
        }
    }

    /// Whether artist `id` was removed itself, rather than with an artist
    /// owning it: it then has no owner.
    pub(crate) fn is_removed(&self, id: ArtistId) -> bool {
        self.detached
            .get(id)
            .is_some_and(|artist| artist.base().parent.is_none())
    }

    /// Inserts `artist` as a child of the container `parent` (a figure
    /// frame or an axes). An artist made in a removed artist is removed
    /// with it, together with the artists it owns.
    pub(crate) fn adopt(&mut self, parent: ArtistId, artist: impl Into<KeptArtist>) -> ArtistId {
        let mut artist = artist.into();
        artist.get_mut().base_mut().parent = Some(parent);
        let id = self.store.insert_kept(artist);
        if self.detached.contains(parent) {
            for (id, artist) in self.store.take_tree(id) {
                self.detached.insert(id, artist);
            }
        } else {
            self.mark_stale(parent);
            self.stale = true;
            self.layout.pending = true;
        }
        if let Some(parent) = self.artist_mut(parent) {
            push_child(parent, id);
        }
        id
    }

    /// Inserts `artist` as a removed artist: outside the figure, owned by
    /// none, together with the artists of the figure it owns.
    pub(crate) fn insert_removed(&mut self, artist: impl Into<KeptArtist>) -> ArtistId {
        let mut artist = artist.into();
        artist.get_mut().base_mut().parent = None;
        let id = self.store.insert_kept(artist);
        for (id, artist) in self.store.take_tree(id) {
            self.detached.insert(id, artist);
        }
        id
    }

    /// Removes artist `id`, with the artists it owns, from the figure and
    /// from its owner, and an axes from the axes it shares with
    /// (matplotlib's `Artist.remove`). The artists are kept apart, so their
    /// handles stay usable. An artist that was removed with its owner is
    /// taken from it; removing an artist again, or the root figure, does
    /// nothing.
    pub(crate) fn remove_artist(&mut self, id: ArtistId) {
        if id == self.root || self.is_removed(id) {
            return;
        }
        let parent = self.artist(id).and_then(|artist| artist.base().parent);
        if let Some(owner) = parent.and_then(|parent| self.artist_mut(parent)) {
            owner.remove_child(id);
        }
        if self.is_attached(id) {
            if let Some(parent) = parent {
                self.mark_stale(parent);
            }
            for (id, artist) in self.store.take_tree(id) {
                self.detached.insert(id, artist);
            }
            self.stale = true;
            self.layout.pending = true;
        }
        self.shared.remove(id);
        crate::mplot3d::leave_shares(self, id);
        if let Some(artist) = self.detached.get_mut(id) {
            artist.base_mut().parent = None;
        }
    }

    /// Adds artist `id`, removed from the figure, back as a child of the
    /// container `parent` with the artists it owns: into the figure when
    /// `parent` is in it, else into the removed artist `parent`. The
    /// artist goes last among the children, so it is drawn above those of
    /// equal z-order.
    pub(crate) fn attach_artist(
        &mut self,
        parent: ArtistId,
        id: ArtistId,
    ) -> Result<(), AttachError> {
        if !self.is_removed(id) {
            return Err(if self.detached.contains(id) {
                AttachError::OwnedByRemoved
            } else {
                AttachError::NotRemoved
            });
        }
        if !self.can_hold(parent, id) {
            return Err(AttachError::Misplaced);
        }
        if self.is_attached(parent) {
            for (id, artist) in self.detached.take_tree(id) {
                self.store.put_back(id, artist);
            }
            self.mark_stale(parent);
            self.stale = true;
            self.layout.pending = true;
        }
        if let Some(artist) = self.artist_mut(id) {
            artist.base_mut().parent = Some(parent);
        }
        if let Some(parent) = self.artist_mut(parent) {
            push_child(parent, id);
        }
        Ok(())
    }

    /// Whether the container `parent` can take artist `id`: figure frames
    /// take any artist, axes any but frames and 3-D axes, and no artist
    /// takes one that owns it.
    fn can_hold(&self, parent: ArtistId, id: ArtistId) -> bool {
        let (Some(container), Some(artist)) = (self.artist(parent), self.artist(id)) else {
            return false;
        };
        let takes = if downcast_ref::<FigureFrame>(container).is_some() {
            true
        } else if downcast_ref::<AxesState>(container).is_some() {
            downcast_ref::<FigureFrame>(artist).is_none()
                && downcast_ref::<Axes3DState>(artist).is_none()
        } else {
            false
        };
        takes && !self.owns(id, parent)
    }

    /// Whether `owner` is artist `id` or owns it, directly or not.
    fn owns(&self, owner: ArtistId, id: ArtistId) -> bool {
        let mut current = Some(id);
        while let Some(id) = current {
            if id == owner {
                return true;
            }
            current = self.artist(id).and_then(|artist| artist.base().parent);
        }
        false
    }
}

impl RawHandle {
    /// Whether the artist is a part another artist made of itself, rather
    /// than one added to a figure or an axes: a spine or a title of an
    /// axes, a text, handle or frame of a legend. matplotlib's
    /// `Artist.remove` refuses those, which have no removal method.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let title = ax.set_title("title");
    /// let note = ax.text(0.5, 0.5, "note");
    /// assert!(title.raw().is_part() && !note.raw().is_part());
    /// ```
    pub fn is_part(&self) -> bool {
        self.figure_state().borrow().is_part(self.id())
    }
}

/// Adds the removed artist behind `artist` back to `figure` as a child of
/// its container `parent` (see [`FigureState::attach_artist`]).
pub(crate) fn attach(
    figure: &Rc<FigureCell>,
    parent: ArtistId,
    artist: &RawHandle,
) -> Result<(), AttachError> {
    if !Rc::ptr_eq(figure, artist.figure_state()) {
        return Err(AttachError::OtherFigure);
    }
    figure.borrow_mut().attach_artist(parent, artist.id())
}

/// Lists `child` among the children of `parent`, when it is a container
/// (a figure frame or an axes).
fn push_child(parent: &mut dyn Artist, child: ArtistId) {
    if let Some(frame) = downcast_mut::<FigureFrame>(parent) {
        frame.children.push(child);
    } else if let Some(axes) = downcast_mut::<AxesState>(parent) {
        axes.children.push(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::{Figure, SubfigureOptions};
    use crate::text::TextArtist;

    #[test]
    fn parts_are_told_from_added_artists() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax
            .plot_with(
                [0.0, 1.0],
                [0.0, 1.0],
                crate::lines::LineProps::default().label("line"),
            )
            .unwrap();
        let legend = ax.legend(crate::legend::LegendOptions::default()).unwrap();
        let spine = ax
            .spines()
            .get(crate::spines::SpineSide::Left)
            .raw()
            .clone();
        assert!(spine.is_part() && legend.frame().raw().is_part());
        assert!(legend.texts().iter().all(|text| text.raw().is_part()));
        assert!(!line.raw().is_part() && !legend.raw().is_part() && !ax.raw().is_part());
        line.remove();
        assert!(!line.raw().is_part(), "a removed artist has no owner");
    }

    /// The children of the axes `id`, wherever it is.
    fn children(fig: &Figure, id: ArtistId) -> Vec<ArtistId> {
        fig.state()
            .artist_as::<AxesState>(id)
            .map(|axes| axes.children.clone())
            .unwrap_or_default()
    }

    #[test]
    fn removal_moves_the_whole_tree_apart() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        ax.remove();
        let state = fig.state();
        for id in [ax.id(), line.id()] {
            assert!(!state.store.contains(id) && state.detached.contains(id));
        }
        assert!(state.is_removed(ax.id()) && !state.is_removed(line.id()));
        assert_eq!(
            state.artist(line.id()).unwrap().base().parent,
            Some(ax.id())
        );
        assert_eq!(state.artist(ax.id()).unwrap().base().parent, None);
    }

    #[test]
    fn removing_again_or_removing_the_figure_does_nothing() {
        let fig = Figure::new();
        let line = fig.subplot().plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        line.remove();
        fig.state_mut().stale = false;
        line.remove();
        fig.remove();
        let state = fig.state();
        assert!(!state.stale, "nothing changed");
        assert!(state.store.contains(state.root));
    }

    #[test]
    fn an_artist_removed_with_its_owner_can_be_taken_from_it() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        ax.remove();
        assert!(children(&fig, ax.id()).contains(&line.id()));
        line.remove();
        assert!(!children(&fig, ax.id()).contains(&line.id()));
        assert!(fig.state().is_removed(line.id()));
        assert_eq!(line.axes(), None);
    }

    #[test]
    fn artists_made_in_removed_artists_are_removed() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.remove();
        let text = ax.text(0.5, 0.5, "late");
        let state = fig.state();
        assert!(state.detached.contains(text.id()) && !state.is_removed(text.id()));
        assert!(children(&fig, ax.id()).contains(&text.id()));
        drop(state);
        let removed = fig.state_mut().insert_removed(Box::new(TextArtist::new(
            pltrs_core::Point::ORIGIN,
            "loose",
            crate::rc::params(),
        )));
        assert!(fig.state().is_removed(removed));
    }

    #[test]
    fn attaching_puts_the_tree_back() {
        let fig = Figure::new();
        let (left, right) = (
            fig.add_subplot(1, 2, 1).unwrap(),
            fig.add_subplot(1, 2, 2).unwrap(),
        );
        let line = left.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        let attach = |parent: &dyn ArtistHandle, id| fig.state_mut().attach_artist(parent.id(), id);
        assert_eq!(attach(&right, line.id()), Err(AttachError::NotRemoved));
        left.remove();
        assert_eq!(attach(&right, line.id()), Err(AttachError::OwnedByRemoved));
        assert_eq!(
            attach(&line, left.id()),
            Err(AttachError::Misplaced),
            "a line holds nothing"
        );
        let sub = fig.subfigures(1, 1, SubfigureOptions::default()).unwrap()[(0, 0)].clone();
        sub.remove();
        assert_eq!(attach(&right, sub.id()), Err(AttachError::Misplaced));
        let inner = sub.subplot();
        assert_eq!(
            attach(&inner, sub.id()),
            Err(AttachError::Misplaced),
            "it owns the axes"
        );
        assert_eq!(attach(&fig, left.id()), Ok(()));
        let state = fig.state();
        assert!(state.store.contains(left.id()) && state.store.contains(line.id()));
        assert_eq!(
            state.artist(left.id()).unwrap().base().parent,
            Some(state.root)
        );
        assert_eq!(
            state
                .artist_as::<FigureFrame>(state.root)
                .unwrap()
                .children
                .last(),
            Some(&left.id())
        );
    }
}
