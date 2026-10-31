//! Adding ready-made artists to a figure (`Figure.add_artist`), removed
//! artists back, and axes made outside the figure until it adds them
//! (matplotlib's `Axes(fig, rect)`).

use std::any::Any;
use std::rc::Rc;

use super::frame::FigureFrame;
use super::{AxesRectError, Figure, attach, axes_bounds, make_axes};
use crate::artist::{Artist, ArtistHandle, ArtistLookup, AttachError, Custom, RawHandle, take_in};
use crate::axes::{Axes, AxesState};
use crate::gridspec::{GridSpecError, SubplotSpec};
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
        if artist.raw().is_loose() {
            return Err(AttachError::OtherFigure);
        }
        self.adopt_artist(artist.raw(), false)?;
        Ok(self)
    }

    /// An axes of this figure (or subfigure) at `[left, bottom, width,
    /// height]` in its fractions, made outside it as a removed axes
    /// (matplotlib's `Axes(fig, rect)`, which leaves adding the axes to
    /// `fig.add_axes(ax)`): [`Figure::attach`] adds it. It does not become
    /// the current axes until then.
    ///
    /// ```
    /// use pltrs::figure::SubfigureOptions;
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let ax = fig.removed_axes([0.1, 0.1, 0.5, 0.5])?;
    /// assert!(fig.axes().is_empty());
    /// fig.attach(&ax)?;
    /// assert_eq!(fig.axes(), [ax]);
    /// let parts = fig.subfigures(2, 1, SubfigureOptions::default())?;
    /// let sub = parts.get(0, 0).expect("a subfigure");
    /// let inner = sub.removed_axes([0.1, 0.1, 0.5, 0.5])?;
    /// assert_eq!(inner.parent_figure().id(), sub.id(), "for the subfigure");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::add_axes`].
    pub fn removed_axes(&self, rect: [f64; 4]) -> Result<Axes, AxesRectError> {
        let position = axes_bounds(rect)?;
        Ok(self.homed(make_axes(&self.state, None, position, None)))
    }

    /// `axes`, made outside this (sub)figure, as an axes for it (see
    /// [`Figure::removed_axes`]).
    fn homed(&self, axes: Axes) -> Axes {
        if let Some(state) = self
            .state
            .borrow_mut()
            .artist_as_mut::<AxesState>(axes.id())
        {
            state.home = Some(self.id());
        }
        axes
    }

    /// An axes of this figure (or subfigure) covering the grid cells of
    /// `spec`, made outside it as a removed axes (matplotlib's `Axes(fig,
    /// subplotspec)`); [`Figure::attach`] adds it.
    ///
    /// # Errors
    ///
    /// As [`Figure::add_subplot_spec`].
    pub fn removed_subplot(&self, spec: &SubplotSpec) -> Result<Axes, GridSpecError> {
        let position = spec.position(&self.subplot_params())?;
        Ok(self.homed(make_axes(&self.state, None, position, Some(spec.clone()))))
    }

    /// Adds a ready-made artist to this figure (or subfigure) and returns
    /// its handle in the figure (matplotlib's `Figure.add_artist(artist,
    /// clip)`): an artist made outside any figure
    /// ([`RawHandle::is_loose`]) moves in under a new id, which the
    /// returned handle refers to, and takes figure fractions
    /// (`transSubfigure`) unless it was given a transform; an artist
    /// removed from this figure comes back under its own id, and a removed
    /// axes as the current axes. With `clip`, an artist without a clip
    /// path is clipped to the figure's frame. It is drawn above the
    /// artists of equal z-order.
    ///
    /// ```
    /// use pltrs::artist::{ArtistHandle, TypedHandle};
    /// use pltrs::prelude::*;
    /// use pltrs::text::Text;
    /// let fig = Figure::new();
    /// let note = Text::loose(0.5, 0.5, "note");
    /// let note = Text::from_raw_handle(&fig.adopt_artist(note.raw(), false)?).unwrap();
    /// assert_eq!(fig.texts(), [note]);
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError`] when the artist belongs to another figure, is still
    /// in this one (or was removed only with an artist owning it), or owns
    /// this figure.
    pub fn adopt_artist(&self, artist: &RawHandle, clip: bool) -> Result<RawHandle, AttachError> {
        let fresh = artist.is_loose();
        let artist = take_in(&self.state, artist)?;
        attach(&self.state, self.id(), &artist)?;
        let frame = self.id();
        let mut state = self.state.borrow_mut();
        if let Some(adopted) = state.artist_mut(artist.id()) {
            let base = adopted.base_mut();
            if fresh {
                base.transform.get_or_insert(Transform::Figure(frame));
            }
            if clip && base.clip_path.is_none() {
                base.clip_box = Some(frame);
            }
        }
        if state.store.get_as::<AxesState>(artist.id()).is_some() {
            state.current_axes = Some(artist.id());
        } else if let Some(axes3d) = state.store.get_as::<Axes3DState>(artist.id()) {
            // A 3-D axes is current through its projection plane.
            state.current_axes = Some(axes3d.plane);
        }
        Ok(artist)
    }

    /// Makes `artist`, made outside any figure, an artist of this figure
    /// that the figure does not draw (matplotlib's `artist.set_figure(fig)`
    /// on an artist no axes or figure added): it moves in under a new id,
    /// which the returned handle refers to, as a removed artist, measured
    /// and drawn in the figure's geometry
    /// ([`RawHandle::measured_extent`], [`RawHandle::draw_onto`]) until
    /// the figure or one of its axes adopts it. An artist of this figure
    /// stays as it is.
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::prelude::*;
    /// use pltrs::text::Text;
    /// let fig = Figure::new();
    /// let note = fig.take_artist(Text::loose(0.0, 0.0, "note").raw())?;
    /// assert!(!note.is_loose() && !note.is_attached());
    /// assert!(note.measured_extent().is_some());
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AttachError::OtherFigure`] when the artist belongs to another
    /// figure.
    pub fn take_artist(&self, artist: &RawHandle) -> Result<RawHandle, AttachError> {
        take_in(&self.state, artist)
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
