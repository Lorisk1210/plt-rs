//! Adding ready-made artists to a figure (`Figure.add_artist`), and
//! removed artists back.

use std::any::Any;
use std::rc::Rc;

use super::frame::FigureFrame;
use super::{Figure, attach};
use crate::artist::{Artist, ArtistHandle, ArtistLookup, AttachError, Custom, RawHandle};
use crate::axes::AxesState;
use crate::image::FigureImageArtist;
use crate::legend::LegendArtist;
use crate::mplot3d::Axes3DState;
use crate::patches::{Patch, PatchArtist, PatchShape};
use crate::text::TextArtist;
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
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.remove();
    /// assert!(fig.axes().is_empty());
    /// fig.attach(&ax)?;
    /// assert_eq!(fig.axes(), [ax.clone()]);
    /// assert_eq!(fig.gca(), ax);
    /// # Ok::<(), pltrs::artist::AttachError>(())
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

    /// The artists added to this figure (or subfigure) itself, in the
    /// order they were added: patches, lines, anchored and annotation
    /// boxes and custom artists (matplotlib's `Figure.artists`, which
    /// `add_artist` fills). The axes, texts, legends, images and
    /// subfigures, which have lists of their own, are not among them.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::LineProps;
    /// use pltrs::patches::Circle;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// fig.subplot();
    /// fig.text(0.5, 0.5, "note");
    /// assert!(fig.artists().is_empty());
    /// let circle = fig.add_artist(Circle::new((0.5, 0.5), 0.1));
    /// let line = fig.add_line(&[0.0, 1.0], &[0.0, 1.0], LineProps::default())?;
    /// assert_eq!(fig.artists(), [circle.raw().clone(), line.raw().clone()]);
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn artists(&self) -> Vec<RawHandle> {
        self.own_artists_where(|artist| {
            let any = artist as &dyn Any;
            !(any.is::<AxesState>()
                || any.is::<Axes3DState>()
                || any.is::<TextArtist>()
                || any.is::<LegendArtist>()
                || any.is::<FigureImageArtist>()
                || any.is::<FigureFrame>())
        })
    }

    /// Handles to the artists placed in this figure's (or subfigure's) own
    /// frame for which `keep` holds, in the order they were added.
    pub(crate) fn own_artists_where(&self, keep: impl Fn(&dyn Artist) -> bool) -> Vec<RawHandle> {
        let children = self
            .root
            .with::<FigureFrame, _>(|frame| frame.children.clone());
        let state = self.state.borrow();
        children
            .into_iter()
            .filter(|&id| state.artist(id).is_some_and(&keep))
            .map(|id| RawHandle::new(Rc::clone(&self.state), id))
            .collect()
    }
}
