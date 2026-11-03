//! An artist's place in its figure: the artists it owns
//! (matplotlib's `Artist.findobj`) and the axes that owns it
//! (`artist.axes`).

use std::rc::Rc;

use super::{Artist, ArtistId, ArtistLookup, RawHandle};
use crate::axes::{Axes, AxesState};
use crate::mplot3d::Axes3DState;

impl RawHandle {
    /// This artist and the artists it owns for which `matches` holds,
    /// each after the artists it owns.
    pub(crate) fn find(&self, matches: &dyn Fn(&RawHandle) -> bool) -> Vec<RawHandle> {
        let mut found = Vec::new();
        self.find_under(self.id(), matches, &mut found);
        found
    }

    /// Adds the artists under and including `id` that `matches` accepts.
    fn find_under(
        &self,
        id: ArtistId,
        matches: &dyn Fn(&RawHandle) -> bool,
        found: &mut Vec<RawHandle>,
    ) {
        let children = self
            .figure_state()
            .borrow()
            .artist(id)
            .map(Artist::children)
            .unwrap_or_default();
        for child in children {
            self.find_under(child, matches, found);
        }
        let handle = RawHandle::new(Rc::clone(self.figure_state()), id);
        if !handle.is_projection_plane() && matches(&handle) {
            found.push(handle);
        }
    }

    /// Whether the artist is the hidden 2-D plane of a 3-D axes, which
    /// matplotlib does not have: its artists are the 3-D axes' own.
    fn is_projection_plane(&self) -> bool {
        let state = self.figure_state().borrow();
        state.artist_as::<AxesState>(self.id()).is_some_and(|axes| {
            axes.base()
                .parent
                .is_some_and(|parent| state.artist_as::<Axes3DState>(parent).is_some())
        })
    }

    /// The nearest 2-D axes among the artist and its owners; `None` when
    /// it is on the figure, in a 3-D axes (whose 2-D plane is not an axes
    /// of its own) or was removed (matplotlib's `remove` sets `axes` to
    /// `None`; the artists it owns keep theirs).
    pub(crate) fn owning_axes(&self) -> Option<Axes> {
        self.nearest_axes(false)
    }

    /// The axes the artist is drawn on: its [owning
    /// axes](Self::owning_axes), or the 2-D plane of the 3-D axes it is in
    /// (on which the contour sets of 3-D axes are drawn).
    pub(crate) fn drawing_axes(&self) -> Option<Axes> {
        self.nearest_axes(true)
    }

    /// The nearest 2-D axes among the artist and its owners, the plane of
    /// a 3-D axes only `through_plane`.
    fn nearest_axes(&self, through_plane: bool) -> Option<Axes> {
        let state = self.figure_state().borrow();
        if state.is_removed(self.id()) {
            return None;
        }
        let is_3d = |id: ArtistId| state.artist_as::<Axes3DState>(id).is_some();
        let mut id = self.id();
        loop {
            if is_3d(id) {
                return None;
            }
            let parent = state.artist(id)?.base().parent;
            if state.artist_as::<AxesState>(id).is_some() {
                let plane = parent.is_some_and(is_3d);
                let axes = RawHandle::new(Rc::clone(self.figure_state()), id);
                return (through_plane || !plane).then(|| Axes::from_raw(axes));
            }
            id = parent?;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::{ArtistHandle, RawHandle};
    use crate::figure::Figure;
    use crate::lines::Line2D;

    #[test]
    fn findobj_walks_the_artist_tree() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot(vec![0.0, 1.0], vec![1.0, 0.0]).unwrap();
        line.set_label("data");
        let everything = fig.findobj(|_| true);
        assert_eq!(everything.last(), Some(fig.raw()), "the figure comes last");
        assert!(everything.contains(ax.raw()) && everything.contains(line.raw()));
        let labelled = fig.findobj(|artist| artist.label() == "data");
        assert_eq!(labelled, [line.raw().clone()]);
    }

    #[test]
    fn findobj_of_an_axes_stays_inside_it() {
        let fig = Figure::new();
        let left = fig.add_subplot(1, 2, 1).unwrap();
        let right = fig.add_subplot(1, 2, 2).unwrap();
        let line = left.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        right.plot([0.0, 1.0], [0.0, 1.0]).unwrap();
        let lines: Vec<Line2D> = left
            .findobj(|_| true)
            .iter()
            .filter_map(RawHandle::downcast)
            .collect();
        assert_eq!(lines, [line]);
        assert_eq!(left.findobj(|_| true).last(), Some(left.raw()));
    }

    #[test]
    fn artists_know_their_axes_and_figure() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).unwrap();
        let note = fig.text(0.5, 0.5, "figure text");
        assert_eq!(line.axes(), Some(ax.clone()));
        assert_eq!(ax.axes(), Some(ax.clone()), "an axes is its own axes");
        assert_eq!(note.axes(), None);
        assert_eq!(line.figure(), fig);
        assert_eq!(note.figure(), fig);
        let title = ax.set_title("title");
        assert_eq!(title.axes(), Some(ax));
    }

    #[test]
    fn artists_of_3d_axes_have_no_2d_axes() {
        let fig = Figure::new();
        let ax = fig.subplot_3d();
        let line = ax.plot([0.0, 1.0], [0.0, 1.0], [0.0, 1.0]).unwrap();
        assert_eq!(line.axes(), None);
        assert_eq!(ax.axes(), None);
        assert_eq!(line.figure(), fig);
    }
}
