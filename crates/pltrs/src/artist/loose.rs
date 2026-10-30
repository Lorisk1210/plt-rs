//! Artists made outside any figure (matplotlib's `Line2D(...)`,
//! `Rectangle(...)` or `Text(...)` before `ax.add_line(line)`,
//! `ax.add_patch(patch)` or `fig.add_artist(text)`).
//!
//! A loose artist lives in a holder of its own: a figure state that is no
//! figure, which keeps the artist as a removed artist. Its handle works as
//! any handle does, so its properties can be read and set before it is
//! added. An axes or a figure then adopts it
//! ([`Axes::adopt_artist`](crate::axes::Axes::adopt_artist),
//! [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist)): the artist moves into
//! that figure under a new id, and the handle the adoption returns
//! replaces the loose one, whose holder is left empty. A loose artist owns
//! no other artist: the typed constructors (`Line2D::loose`,
//! `Patch::loose`, `Collection::loose`, `Text::loose`, `Table::loose`,
//! `Custom::loose`) only make artists without children.

use std::cell::RefCell;
use std::rc::Rc;

use pltrs_core::{Bbox, Point};

use super::{Artist, ArtistHandle, AttachError, Custom, RawHandle};
use crate::axes::Axes;
use crate::collections::{Collection, CollectionSpec};
use crate::figure::{FigureState, StateRole};
use crate::lines::{Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchProps, PatchShape};
use crate::table::{Table, TableArtist, TableLoc};
use crate::text::{Text, TextArtist};

impl RawHandle {
    /// A handle to `artist`, made outside any figure: it lives in a holder
    /// of its own until an axes or a figure adopts it.
    pub(crate) fn loose(artist: Box<dyn Artist>) -> Self {
        debug_assert!(
            artist.children().is_empty(),
            "a loose artist owns no other artist"
        );
        let mut holder = FigureState::holder();
        let id = holder.insert_removed(artist);
        Self::new(Rc::new(RefCell::new(holder)), id)
    }

    /// Whether the artist was made outside any figure and no axes or
    /// figure adopted it yet (matplotlib's `artist.figure is None` for an
    /// artist never added).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::lines::{Line2D, LineProps};
    /// use pltrs::prelude::*;
    /// let line = Line2D::loose(vec![0.0, 1.0], vec![1.0, 0.0], LineProps::default());
    /// assert!(line.raw().is_loose());
    /// let ax = Figure::new().subplot();
    /// assert!(!ax.raw().is_loose());
    /// ```
    pub fn is_loose(&self) -> bool {
        self.figure_state().borrow().role == StateRole::LooseHolder
    }
}

impl Line2D {
    /// A line through `(x, y)` styled from the rc parameters and `props`,
    /// outside any figure (matplotlib's `Line2D(xdata, ydata, ...)`); add
    /// it with [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) or
    /// [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist). As in matplotlib,
    /// `x` and `y` may differ in length until the line is drawn.
    pub fn loose(x: Vec<f64>, y: Vec<f64>, props: LineProps) -> Self {
        let mut line = LineArtist::new(x, y, crate::rc::params());
        props.apply(&mut line);
        Self::from_raw(RawHandle::loose(Box::new(line)))
    }
}

impl<S: PatchShape> Patch<S> {
    /// A patch of `shape` styled from the rc parameters and `props`,
    /// outside any figure (matplotlib's `Rectangle(...)`, `Circle(...)`,
    /// ...); add it with [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) or
    /// [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist).
    pub fn loose(shape: S, props: PatchProps) -> Self {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        props.apply(&mut patch);
        Self::from_raw(RawHandle::loose(Box::new(patch)))
    }
}

impl Collection {
    /// The collection `spec` describes, outside any figure (matplotlib's
    /// `LineCollection(...)`, `PatchCollection(...)`, ...); add it with
    /// [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) or
    /// [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist).
    pub fn loose(spec: CollectionSpec) -> Self {
        Self::from_raw(RawHandle::loose(Box::new(spec.artist)))
    }
}

impl Text {
    /// The text `text` at `(x, y)`, styled from the rc parameters, outside
    /// any figure (matplotlib's `Text(x, y, text)`); add it with
    /// [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) or
    /// [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist).
    pub fn loose(x: f64, y: f64, text: impl Into<String>) -> Self {
        let artist = TextArtist::new(Point::new(x, y), text, crate::rc::params());
        Self::from_raw(RawHandle::loose(Box::new(artist)))
    }
}

impl Table {
    /// A table without cells for `axes`, placed at `loc` or stretched into
    /// `bbox` (in its axes fractions), outside any figure (matplotlib's
    /// `Table(ax, loc, bbox)`); add it to `axes` with
    /// [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) and
    /// [`AddAs::Artist`](crate::axes::AddAs::Artist) (matplotlib's
    /// `add_table`). Without an axes it only holds cells made outside any
    /// table until they are put in one ([`Table::put_cell`]).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::axes::AddAs;
    /// use pltrs::prelude::*;
    /// use pltrs::table::{Table, TableLoc};
    /// let ax = Figure::new().subplot();
    /// let table = Table::loose(Some(&ax), TableLoc::Center, None);
    /// table.add_cell(0, 0, 0.5, 0.1).set_text("total");
    /// let added = ax.adopt_artist(table.raw(), AddAs::Artist)?;
    /// assert_eq!(ax.tables()[0].raw(), &added);
    /// # Ok::<(), pltrs::artist::AttachError>(())
    /// ```
    pub fn loose(axes: Option<&Axes>, loc: TableLoc, bbox: Option<Bbox>) -> Self {
        let table = TableArtist::empty(axes.map(Axes::id), loc, bbox, crate::rc::params());
        Self::from_raw(RawHandle::loose(Box::new(table)))
    }
}

impl<A: Artist> Custom<A> {
    /// An artist of your own outside any figure (an instance of a
    /// matplotlib `Artist` subclass before it is added); add it with
    /// [`Axes::adopt_artist`](crate::axes::Axes::adopt_artist) or
    /// [`Figure::adopt_artist`](crate::figure::Figure::adopt_artist). It must own no
    /// other artist.
    pub fn loose(artist: A) -> Self {
        Self::from_raw(RawHandle::loose(Box::new(artist)))
    }
}

/// `artist` as an artist of `figure`, removed from it until it is added:
/// a loose artist moves from its holder into `figure` under a new id (the
/// handle returned), an artist of `figure` stays as it is.
///
/// # Errors
///
/// [`AttachError::OtherFigure`] when the artist belongs to another figure.
pub(crate) fn take_in(
    figure: &Rc<RefCell<FigureState>>,
    artist: &RawHandle,
) -> Result<RawHandle, AttachError> {
    if Rc::ptr_eq(figure, artist.figure_state()) {
        return Ok(artist.clone());
    }
    if !artist.is_loose() {
        return Err(AttachError::OtherFigure);
    }
    let moved = artist
        .figure_state()
        .borrow_mut()
        .detached
        .take(artist.id())
        .ok_or(AttachError::OtherFigure)?;
    let id = figure.borrow_mut().insert_removed(moved);
    Ok(RawHandle::new(Rc::clone(figure), id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;
    use crate::lines::{Line2D, LineProps};

    #[test]
    fn a_loose_artist_is_kept_apart_from_any_figure() {
        let line = Line2D::loose(vec![0.0, 1.0], vec![1.0, 2.0], LineProps::default());
        let holder = line.raw().figure_state().borrow();
        assert!(holder.role == StateRole::LooseHolder && holder.is_removed(line.id()));
        assert!(!line.raw().is_attached());
    }

    #[test]
    fn taking_in_moves_a_loose_artist_once() {
        let fig = Figure::new();
        let line = Line2D::loose(vec![0.0, 1.0], vec![1.0, 2.0], LineProps::default());
        line.set_linewidth(4.0);
        let figure = fig.raw().figure_state();
        let moved = take_in(figure, line.raw()).unwrap();
        assert!(!moved.is_loose() && fig.state().is_removed(moved.id()));
        assert_eq!(Line2D::from_raw(moved.clone()).linewidth(), 4.0);
        assert!(
            line.raw()
                .figure_state()
                .borrow()
                .detached
                .get(line.id())
                .is_none(),
            "the holder is left empty"
        );
        assert_eq!(take_in(figure, &moved), Ok(moved.clone()));
        let other = Figure::new();
        assert_eq!(
            take_in(other.raw().figure_state(), &moved),
            Err(AttachError::OtherFigure)
        );
    }
}
