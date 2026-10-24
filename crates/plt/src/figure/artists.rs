//! Adding ready-made artists to a figure (`Figure.add_artist`).

use std::rc::Rc;

use super::Figure;
use crate::artist::{Artist, ArtistHandle, Custom, RawHandle};
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
}
