//! The untyped handle every typed artist handle wraps.

use std::fmt;
use std::rc::Rc;

use pltrs_core::Point;

use super::{Artist, ArtistBase, ArtistId, ArtistLookup, ArtistStore, LayoutContext, TypedHandle};
use crate::figure::{FigureCell, FigureState};

/// An untyped reference to one artist of a figure.
///
/// Typed handles (`Line2D`, `Axes`, ...) wrap a `RawHandle` and implement
/// [`ArtistHandle`](super::ArtistHandle). A handle stays usable after its
/// artist is removed from the figure
/// ([`ArtistHandle::remove`](super::ArtistHandle::remove)): the figure
/// keeps removed artists, apart from what it draws, for as long as it
/// lives.
#[derive(Clone)]
pub struct RawHandle {
    figure: Rc<FigureCell>,
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
    pub(crate) fn new(figure: Rc<FigureCell>, id: ArtistId) -> Self {
        Self { figure, id }
    }

    /// The artist's id within its figure.
    pub fn id(&self) -> ArtistId {
        self.id
    }

    pub(crate) fn figure_state(&self) -> &Rc<FigureCell> {
        &self.figure
    }

    /// Whether `other` is an artist of the same figure, which ids of
    /// artists refer to (a loose artist is alone in a holder of its own).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::{Line2D, LineProps};
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// assert!(line.raw().shares_figure(ax.raw()));
    /// let loose = Line2D::loose(vec![0.0, 1.0], vec![0.0, 1.0], LineProps::default());
    /// assert!(!loose.raw().shares_figure(ax.raw()));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn shares_figure(&self, other: &RawHandle) -> bool {
        Rc::ptr_eq(&self.figure, &other.figure)
    }

    /// Whether code still running uses the artist's figure now: it is
    /// being laid out, drawn or changed by a caller further up, and a
    /// change through any handle meanwhile panics, as a `RefCell` borrowed
    /// twice does, unless it runs inside
    /// [`reenter`](crate::figure::reenter). The figure is in use while a
    /// callback it runs (a program's formatter, say) runs.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// assert!(!ax.raw().figure_in_use());
    /// ```
    pub fn figure_in_use(&self) -> bool {
        self.figure.in_use()
    }

    /// Whether the artist is in its figure: `false` once it, or an artist
    /// owning it, was removed
    /// ([`ArtistHandle::remove`](super::ArtistHandle::remove)).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// let line = ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// assert!(line.raw().is_attached());
    /// ax.remove();
    /// assert!(!line.raw().is_attached() && !ax.raw().is_attached());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn is_attached(&self) -> bool {
        self.figure.borrow().is_attached(self.id)
    }

    /// The typed handle of the artist, if it is of type `H` (matplotlib
    /// code tests `isinstance`, as in `fig.findobj(Line2D)`).
    ///
    /// ```
    /// use pltrs::lines::Line2D;
    /// use pltrs::prelude::*;
    /// use pltrs::text::Text;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// ax.plot([0.0, 1.0], [1.0, 0.0])?;
    /// let lines: Vec<Line2D> = fig.findobj(|_| true).iter().filter_map(|a| a.downcast()).collect();
    /// assert_eq!(lines.len(), 1);
    /// assert!(lines[0].raw().downcast::<Text>().is_none());
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn downcast<H: TypedHandle>(&self) -> Option<H> {
        H::from_raw_handle(self)
    }

    /// Whether the artist is a `T`.
    pub(crate) fn is<T: Artist>(&self) -> bool {
        self.figure.borrow().artist_as::<T>(self.id).is_some()
    }

    /// Whether the artist is a patch of any shape (matplotlib's
    /// `isinstance(artist, Patch)`).
    pub(crate) fn is_patch(&self) -> bool {
        self.figure
            .borrow()
            .artist(self.id)
            .is_some_and(|artist| artist.as_patch().is_some())
    }

    /// Reads the artist as concrete type `T`.
    pub(crate) fn with<T: Artist, R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .artist_as::<T>(self.id)
            .unwrap_or_else(|| self.mismatched()))
    }

    /// Mutates the artist as concrete type `T` and marks the figure stale
    /// (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_mut<T: Artist, R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        let artist = state
            .artist_as_mut::<T>(self.id)
            .unwrap_or_else(|| self.mismatched());
        let result = f(artist);
        self.mark_changed(&mut state);
        result
    }

    /// Mutates the artist as concrete type `T` together with the other
    /// artists of the figure (see [`ArtistStore::with_others`]) and marks
    /// the figure stale (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_others<T: Artist, R>(
        &self,
        f: impl FnOnce(&mut T, &mut ArtistStore) -> R,
    ) -> R {
        let mut state = self.figure.borrow_mut();
        let result = state
            .with_others(self.id, f)
            .unwrap_or_else(|| self.mismatched());
        self.mark_changed(&mut state);
        result
    }

    /// Marks the artist stale after a change to it and, unless it is
    /// animated or was removed, the artists containing it and the
    /// figure: figures leave animated artists out of their draws, so a
    /// change to one only shows once the program draws it (matplotlib's
    /// `Artist.stale` reaches the figure neither from an animated artist
    /// nor from a removed one).
    fn mark_changed(&self, state: &mut FigureState) {
        state.mark_stale(self.id);
    }

    /// Whether the artist changed since it was last drawn (see
    /// [`ArtistHandle::stale`](super::ArtistHandle::stale)).
    pub fn is_stale(&self) -> bool {
        self.figure.borrow().is_stale(self.id)
    }

    /// Marks the artist as changed since it was last drawn, or as drawn
    /// (see [`ArtistHandle::set_stale`](super::ArtistHandle::set_stale)).
    pub fn set_stale(&self, stale: bool) {
        let mut state = self.figure.borrow_mut();
        if stale {
            state.mark_stale(self.id);
        } else {
            state.mark_fresh(self.id);
        }
    }

    /// How many times the artist was drawn, in its figure's draws or
    /// alone (an invisible artist draws nothing): what tells the values
    /// matplotlib's artists compute as they are drawn (an annotation's
    /// transform, a spine's path) from those they hold before.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// let label = fig.subplot().text(0.5, 0.5, "drawn");
    /// assert_eq!(label.raw().draw_count(), 0);
    /// fig.canvas().draw()?;
    /// fig.canvas().draw()?;
    /// assert_eq!(label.raw().draw_count(), 2);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn draw_count(&self) -> u64 {
        self.figure.borrow().draw_count(self.id)
    }

    /// Reads the common artist properties.
    pub(crate) fn with_base<R>(&self, f: impl FnOnce(&ArtistBase) -> R) -> R {
        let state = self.figure.borrow();
        f(state
            .artist(self.id)
            .unwrap_or_else(|| self.mismatched())
            .base())
    }

    /// Mutates the common artist properties and marks the figure stale
    /// (see [`RawHandle::mark_changed`]).
    pub(crate) fn with_base_mut<R>(&self, f: impl FnOnce(&mut ArtistBase) -> R) -> R {
        let mut state = self.figure.borrow_mut();
        let result = f(state
            .artist_mut(self.id)
            .unwrap_or_else(|| self.mismatched())
            .base_mut());
        self.mark_changed(&mut state);
        result
    }

    /// Whether display point `at` hits the artist in the figure's current
    /// layout within the tolerance `radius` (the artist's own with
    /// `None`), and which of its items (matplotlib's `contains`); `None`
    /// when it misses or the artist was removed.
    pub(crate) fn hit_test(&self, at: Point, radius: Option<f64>) -> Option<Vec<usize>> {
        let mut state = self.figure.borrow_mut();
        state.prepare();
        let layout = LayoutContext::new(&state);
        layout.store().get(self.id)?.pick_hit(&layout, at, radius)
    }

    /// Reports a handle that does not match its artist: missing from the
    /// figure, or of another type than the handle's. A figure keeps its
    /// artists, removed or not, and handles are only made for artists of
    /// their figure and type, so this never happens.
    fn mismatched(&self) -> ! {
        panic!("artist {} does not match its handle", self.id)
    }
}
