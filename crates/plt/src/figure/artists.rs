//! Adding ready-made artists to a figure (`Figure.add_artist`), and
//! removed artists back.

use std::rc::Rc;

use super::{Figure, attach};
use crate::artist::{Artist, ArtistHandle, AttachError, Custom, RawHandle};
use crate::axes::AxesState;
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

    /// Adds `artist`, removed from this figure with
    /// [`ArtistHandle::remove`], back to it with the artists it owns,
    /// drawn above the artists of equal z-order (matplotlib's
    /// `Figure.add_artist` of a removed artist). A removed axes comes back
    /// as the current axes (matplotlib's `add_axes` of an existing axes).
    ///
    /// ```
    /// use plt::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.remove();
    /// assert!(fig.axes().is_empty());
    /// fig.attach(&ax)?;
    /// assert_eq!(fig.axes(), [ax.clone()]);
    /// assert_eq!(fig.gca(), ax);
    /// # Ok::<(), plt::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError`] when the artist belongs to another figure, was not
    /// removed (or only with an artist owning it), or owns this figure.
    pub fn attach(&self, artist: &impl ArtistHandle) -> Result<&Self, AttachError> {
        attach(&self.state, self.id(), artist.raw())?;
        let mut state = self.state.borrow_mut();
        if state.store.get_as::<AxesState>(artist.id()).is_some() {
            state.current_axes = Some(artist.id());
        }
        Ok(self)
    }
}
