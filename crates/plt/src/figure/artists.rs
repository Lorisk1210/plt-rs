//! Adding ready-made artists to a figure (`Figure.add_artist`) and
//! finding the artists in it (`Artist.findobj`).

use std::cell::RefCell;
use std::rc::Rc;

use super::{Figure, FigureState};
use crate::artist::{Artist, ArtistHandle, ArtistId, Custom, RawHandle};
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::transforms::Transform;

impl Figure {
    /// Adds a patch to the figure, unclipped and in figure fractions unless
    /// its shape chose another coordinate system (matplotlib's
    /// `Figure.add_artist`). Figure artists are drawn above the axes of
    /// equal z-order added before them.
    pub fn add_artist<S: PatchShape>(&self, shape: S) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        let base = patch.base_mut();
        if base.transform.is_none() {
            base.transform = Some(Transform::Figure(self.id()));
        }
        let id = self.state.borrow_mut().adopt(self.id(), Box::new(patch));
        Patch::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// Adds an artist of your own to the figure (matplotlib's
    /// `Figure.add_artist` of an `Artist` subclass; see [`Custom`]):
    /// unclipped and in figure fractions unless its base has a transform.
    pub fn add_custom_artist<A: Artist>(&self, mut artist: A) -> Custom<A> {
        let base = artist.base_mut();
        if base.transform.is_none() {
            base.transform = Some(Transform::Figure(self.id()));
        }
        let id = self.state.borrow_mut().adopt(self.id(), Box::new(artist));
        Custom::from_raw(RawHandle::new(Rc::clone(&self.state), id))
    }

    /// The artists of the figure, the figure itself included, for which
    /// `matches` holds; each comes after the artists it owns (matplotlib's
    /// `Artist.findobj(match)`).
    pub fn findobj(&self, matches: impl Fn(&RawHandle) -> bool) -> Vec<RawHandle> {
        let mut found = Vec::new();
        find_in(&self.state, self.id(), &matches, &mut found);
        found
    }
}

/// Adds the artists under and including `id` that `matches` accepts.
fn find_in(
    state: &Rc<RefCell<FigureState>>,
    id: ArtistId,
    matches: &impl Fn(&RawHandle) -> bool,
    found: &mut Vec<RawHandle>,
) {
    let children = state
        .borrow()
        .store
        .get(id)
        .map(Artist::children)
        .unwrap_or_default();
    for child in children {
        find_in(state, child, matches, found);
    }
    let handle = RawHandle::new(Rc::clone(state), id);
    if matches(&handle) {
        found.push(handle);
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

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
}
