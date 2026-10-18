//! Handles: the user-facing references to artists inside a figure.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use plt_core::Bbox;

use super::{
    AggFilter, Artist, ArtistBase, ArtistId, ArtistStore, ClipPathSpec, LayoutContext, Picker,
};
use crate::figure::FigureState;
use crate::patheffects::PathEffect;
use crate::transforms::Transform;

/// An untyped reference to one artist of a figure.
///
/// Typed handles (`Line2D`, `Axes`, ...) wrap a `RawHandle` and implement
/// [`ArtistHandle`]. Using a handle after its artist was removed from the
/// figure panics: like a closed file, it no longer refers to anything.
#[derive(Clone)]
pub struct RawHandle {
    figure: Rc<RefCell<FigureState>>,
    id: ArtistId,
}

impl fmt::Debug for RawHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RawHandle")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RawHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.figure, &other.figure) && self.id == other.id
    }
}

impl Eq for RawHandle {}

impl RawHandle {
    pub(crate) fn new(figure: Rc<RefCell<FigureState>>, id: ArtistId) -> Self {
        Self { figure, id }
    }

    /// The artist's id within its figure.
    pub fn id(&self) -> ArtistId {
        self.id
    }

    pub(crate) fn figure_state(&self) -> &Rc<RefCell<FigureState>> {
        &self.figure
    }

    /// Whether the artist still exists.
    pub fn is_alive(&self) -> bool {
        self.figure.borrow().store.contains(self.id)
    }

    /// Reads the artist as concrete type `T`.
    pub(crate) fn with<T: Artist, R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .store
            .get_as::<T>(self.id)
            .unwrap_or_else(|| self.gone()))
    }

    /// Mutates the artist as concrete type `T` and marks the figure stale.
    pub(crate) fn with_mut<T: Artist, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        state.stale = true;
        let artist = state
            .store
            .get_as_mut::<T>(self.id)
            .unwrap_or_else(|| self.gone());
        f(artist)
    }

    /// Mutates the artist as concrete type `T` together with the rest of
    /// the figure's artists (see [`ArtistStore::with_detached`]) and
    /// marks the figure stale.
    pub(crate) fn with_detached<T: Artist, R>(
        &self,
        f: impl FnOnce(&mut T, &mut ArtistStore) -> R,
    ) -> R {
        let mut state = self.figure.borrow_mut();
        state.stale = true;
        state
            .store
            .with_detached(self.id, f)
            .unwrap_or_else(|| self.gone())
    }

    /// Reads the common artist properties.
    pub(crate) fn with_base<R>(&self, f: impl FnOnce(&ArtistBase) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .store
            .get(self.id)
            .unwrap_or_else(|| self.gone())
            .base())
    }

    /// Mutates the common artist properties.
    pub(crate) fn with_base_mut<R>(&self, f: impl FnOnce(&mut ArtistBase) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        state.stale = true;
        f(state
            .store
            .get_mut(self.id)
            .unwrap_or_else(|| self.gone())
            .base_mut())
    }

    fn gone(&self) -> ! {
        panic!(
            "artist {} was removed from its figure or has a different type",
            self.id
        )
    }
}

/// The untyped handle has the properties every artist shares, which is
/// what predicates over a figure's artists look at
/// ([`Figure::findobj`](crate::figure::Figure::findobj)).
impl ArtistHandle for RawHandle {
    fn raw(&self) -> &RawHandle {
        self
    }
}

/// Properties and operations shared by every artist handle
/// (matplotlib's `Artist` API).
///
/// Setters return `&Self` so calls chain:
/// `line.set_alpha(Some(0.5)).set_zorder(3.0);`
pub trait ArtistHandle {
    /// The underlying untyped handle.
    fn raw(&self) -> &RawHandle;

    /// The artist's id within its figure.
    fn id(&self) -> ArtistId {
        self.raw().id()
    }

    /// Shows or hides the artist.
    fn set_visible(&self, visible: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.visible = visible);
        self
    }

    /// Whether the artist is drawn.
    fn visible(&self) -> bool {
        self.raw().with_base(|b| b.visible)
    }

    /// Makes mouse presses and scrolls on the artist fire pick events
    /// (matplotlib's `set_picker`); `None` makes it unpickable again.
    fn set_picker(&self, picker: Option<Picker>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.picker = picker);
        self
    }

    /// How the artist is picked, if it is pickable.
    fn picker(&self) -> Option<Picker> {
        self.raw().with_base(|b| b.picker.clone())
    }

    /// Whether the artist fires pick events (matplotlib's `pickable`).
    fn pickable(&self) -> bool {
        self.raw().with_base(|b| b.picker.is_some())
    }

    /// Sets whether a status bar shows the data under the pointer when it
    /// is over the artist (matplotlib's `set_mouseover`; on by default for
    /// images).
    fn set_mouseover(&self, mouseover: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.mouseover = mouseover);
        self
    }

    /// Whether a status bar shows the data under the pointer over the
    /// artist.
    fn mouseover(&self) -> bool {
        self.raw().with_base(|b| b.mouseover)
    }

    /// Sets (or clears) the image filter for what the artist draws
    /// (matplotlib's `set_agg_filter`; see [`AggFilter`]).
    fn set_agg_filter(&self, filter: Option<AggFilter>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.agg_filter = filter);
        self
    }

    /// The image filter, if any.
    fn agg_filter(&self) -> Option<AggFilter> {
        self.raw().with_base(|b| b.agg_filter.clone())
    }

    /// Sets (or clears) the opacity override.
    fn set_alpha(&self, alpha: Option<f64>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.alpha = alpha);
        self
    }

    /// The opacity override.
    fn alpha(&self) -> Option<f64> {
        self.raw().with_base(|b| b.alpha)
    }

    /// Sets the drawing order (higher is drawn later, on top).
    fn set_zorder(&self, zorder: f64) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.zorder = zorder);
        self
    }

    /// The drawing order.
    fn zorder(&self) -> f64 {
        self.raw().with_base(|b| b.zorder)
    }

    /// Sets the legend label (labels starting with `_` are hidden).
    fn set_label(&self, label: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let label = label.into();
        self.raw().with_base_mut(|b| b.label = label);
        self
    }

    /// The legend label.
    fn label(&self) -> String {
        self.raw().with_base(|b| b.label.clone())
    }

    /// Data values that autoscaling margins must not cross, per axis
    /// (matplotlib's `sticky_edges`).
    fn sticky_edges(&self) -> (Vec<f64>, Vec<f64>) {
        self.raw().with_base(|b| b.sticky_edges.clone())
    }

    /// Replaces the sticky edges.
    fn set_sticky_edges(&self, x: Vec<f64>, y: Vec<f64>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.sticky_edges = (x, y));
        self
    }

    /// Places the artist in another coordinate system.
    fn set_transform(&self, transform: Transform) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.transform = Some(transform));
        self
    }

    /// Enables or disables clipping.
    fn set_clip_on(&self, clip_on: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.clip_on = clip_on);
        self
    }

    /// Clips the artist to a region besides its clip box (matplotlib's
    /// `set_clip_path`); `None` removes it.
    fn set_clip_path(&self, clip: Option<ClipPathSpec>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.clip_path = clip);
        self
    }

    /// Sets the effects applied when the artist's paths are drawn
    /// (matplotlib's `set_path_effects`).
    fn set_path_effects(&self, effects: Vec<PathEffect>) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.path_effects = effects);
        self
    }

    /// Requests rasterization of this artist in vector output.
    fn set_rasterized(&self, rasterized: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.rasterized = rasterized);
        self
    }

    /// Marks the artist as animated (skipped by normal draws).
    fn set_animated(&self, animated: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.animated = animated);
        self
    }

    /// Includes or excludes the artist from layout computations.
    fn set_in_layout(&self, in_layout: bool) -> &Self
    where
        Self: Sized,
    {
        self.raw().with_base_mut(|b| b.in_layout = in_layout);
        self
    }

    /// Sets the group id used in vector output.
    fn set_gid(&self, gid: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let gid = gid.into();
        self.raw().with_base_mut(|b| b.gid = Some(gid));
        self
    }

    /// The group id used in vector output, if set.
    fn gid(&self) -> Option<String> {
        self.raw().with_base(|b| b.gid.clone())
    }

    /// Sets the hyperlink used in vector output.
    fn set_url(&self, url: impl Into<String>) -> &Self
    where
        Self: Sized,
    {
        let url = url.into();
        self.raw().with_base_mut(|b| b.url = Some(url));
        self
    }

    /// The artist's extent in display coordinates, if it occupies space.
    fn window_extent(&self) -> Option<Bbox> {
        let state = self.raw().figure_state().borrow();
        let ctx = LayoutContext::new(&state);
        state.store.get(self.id())?.window_extent(&ctx)
    }

    /// Removes the artist (and everything it owns) from the figure.
    fn remove(&self) {
        self.raw()
            .figure_state()
            .borrow_mut()
            .remove_artist(self.id());
    }
}
