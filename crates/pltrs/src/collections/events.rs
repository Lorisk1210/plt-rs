//! Event collections: one line across a band per event position
//! (matplotlib's `EventCollection`), and the [`EventCollection`] handle
//! that moves, adds and turns the events after the collection was made.

use std::ops::Deref;

use pltrs_core::{Path, Point, Rgba};

use super::{Collection, CollectionArtist, CollectionColorError};
use crate::artist::{ArtistHandle, RawHandle, TypedHandle};
use crate::orientation::Orientation;

/// How the lines of an event collection are laid out (matplotlib's
/// `EventCollection` state besides its segments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EventLines {
    /// [`Orientation::Horizontal`]: events along x, each line vertical.
    pub(crate) orientation: Orientation,
    /// Where the lines are centred across the events' direction.
    pub(crate) lineoffset: f64,
    /// The length of every line.
    pub(crate) linelength: f64,
}

pltrs_core::archive_struct!(EventLines {
    orientation,
    lineoffset,
    linelength
});

impl EventLines {
    /// The ends of a line, from `lineoffset + linelength / 2` to
    /// `lineoffset - linelength / 2`.
    fn ends(self) -> (f64, f64) {
        (
            self.lineoffset + self.linelength / 2.0,
            self.lineoffset - self.linelength / 2.0,
        )
    }

    /// One line per position, in increasing order of the positions
    /// (matplotlib's `set_positions`).
    pub(crate) fn paths(self, positions: &[f64]) -> Vec<Path> {
        let mut sorted = positions.to_vec();
        sorted.sort_by(f64::total_cmp);
        let (start, end) = self.ends();
        sorted
            .into_iter()
            .map(|at| {
                let (a, b) = match self.orientation {
                    Orientation::Horizontal => (Point::new(at, start), Point::new(at, end)),
                    Orientation::Vertical => (Point::new(start, at), Point::new(end, at)),
                };
                Path::polyline(vec![a, b])
            })
            .collect()
    }

    /// The position of the event `path` marks.
    fn position(self, path: &Path) -> Option<f64> {
        let first = path.vertices().first()?;
        Some(match self.orientation {
            Orientation::Horizontal => first.x,
            Orientation::Vertical => first.y,
        })
    }

    /// `path` with its first two points moved across the events to this
    /// layout's ends, its position kept (matplotlib's `set_linelength`
    /// and `set_lineoffset`, which edit the segments in place).
    fn respan(self, path: &Path) -> Path {
        let (start, end) = self.ends();
        let mut vertices = path.vertices().to_vec();
        for (vertex, across) in vertices.iter_mut().zip([start, end]) {
            match self.orientation {
                Orientation::Horizontal => vertex.y = across,
                Orientation::Vertical => vertex.x = across,
            }
        }
        Path::polyline(vertices)
    }
}

/// `path` mirrored in the diagonal: x and y swapped at every vertex
/// (matplotlib's `np.fliplr` of a segment).
fn transposed(path: &Path) -> Path {
    Path::polyline(
        path.vertices()
            .iter()
            .map(|p| Point::new(p.y, p.x))
            .collect(),
    )
}

/// A handle to an event collection: what
/// [`CollectionSpec::events`](super::CollectionSpec::events) and
/// [`Axes::eventplot`](crate::axes::Axes::eventplot) make (matplotlib's
/// `EventCollection`). It dereferences to the [`Collection`] it is.
///
/// ```
/// use pltrs::collections::{CollectionSpec, EventCollection};
/// use pltrs::orientation::Orientation;
/// use pltrs::prelude::*;
///
/// let ax = Figure::new().subplot();
/// let spec = CollectionSpec::events(&[3.0, 1.0], Orientation::Horizontal, 1.0, 0.5);
/// let events: EventCollection = ax.add_collection(spec).raw().downcast().expect("events");
/// assert_eq!(events.positions(), [1.0, 3.0]);
/// events.add_positions(&[2.0]).switch_orientation();
/// assert_eq!((events.positions(), events.orientation()), (vec![1.0, 2.0, 3.0], Orientation::Vertical));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventCollection {
    collection: Collection,
}

impl Deref for EventCollection {
    type Target = Collection;

    fn deref(&self) -> &Collection {
        &self.collection
    }
}

impl ArtistHandle for EventCollection {
    fn raw(&self) -> &RawHandle {
        self.collection.raw()
    }
}

impl TypedHandle for EventCollection {
    fn from_raw_handle(raw: &RawHandle) -> Option<Self> {
        let is_events =
            raw.is::<CollectionArtist>() && raw.with::<CollectionArtist, _>(|c| c.events.is_some());
        is_events.then(|| Self::from_raw(raw.clone()))
    }
}

impl EventCollection {
    /// The handle to the event collection behind `raw`.
    pub(crate) fn from_raw(raw: RawHandle) -> Self {
        Self {
            collection: Collection::from_raw(raw),
        }
    }

    /// Reads the layout and the lines.
    fn read<R>(&self, f: impl FnOnce(EventLines, &[Path]) -> R) -> R {
        self.raw().with::<CollectionArtist, _>(|c| {
            f(
                c.events.expect("an event collection keeps its layout"),
                &c.paths,
            )
        })
    }

    /// Changes the layout and the lines.
    fn edit(&self, f: impl FnOnce(&mut EventLines, &mut Vec<Path>)) -> &Self {
        self.raw().with_mut::<CollectionArtist, _>(|c| {
            let events = c
                .events
                .as_mut()
                .expect("an event collection keeps its layout");
            f(events, &mut c.paths);
        });
        self
    }

    /// The event positions, in the order of the lines (matplotlib's
    /// `get_positions`).
    pub fn positions(&self) -> Vec<f64> {
        self.read(|events, paths| paths.iter().filter_map(|p| events.position(p)).collect())
    }

    /// Replaces the events, drawn in increasing order (matplotlib's
    /// `set_positions`).
    pub fn set_positions(&self, positions: &[f64]) -> &Self {
        self.edit(|events, paths| *paths = events.paths(positions))
    }

    /// Adds events (matplotlib's `add_positions`, `append_positions` and
    /// `extend_positions`).
    pub fn add_positions(&self, positions: &[f64]) -> &Self {
        let mut all = self.positions();
        all.extend_from_slice(positions);
        self.set_positions(&all)
    }

    /// The direction the events run along (matplotlib's
    /// `get_orientation`).
    pub fn orientation(&self) -> Orientation {
        self.read(|events, _| events.orientation)
    }

    /// Whether the events run along x (matplotlib's `is_horizontal`).
    pub fn is_horizontal(&self) -> bool {
        self.orientation() == Orientation::Horizontal
    }

    /// Mirrors the lines in the diagonal, so the events run along the
    /// other axis (matplotlib's `switch_orientation`).
    pub fn switch_orientation(&self) -> &Self {
        self.edit(|events, paths| {
            *paths = paths.iter().map(transposed).collect();
            events.orientation = match events.orientation {
                Orientation::Horizontal => Orientation::Vertical,
                Orientation::Vertical => Orientation::Horizontal,
            };
        })
    }

    /// Makes the events run along `orientation` (matplotlib's
    /// `set_orientation`).
    pub fn set_orientation(&self, orientation: Orientation) -> &Self {
        if orientation == self.orientation() {
            return self;
        }
        self.switch_orientation()
    }

    /// The length of the lines (matplotlib's `get_linelength`).
    pub fn linelength(&self) -> f64 {
        self.read(|events, _| events.linelength)
    }

    /// Sets the length of the lines, keeping their centres (matplotlib's
    /// `set_linelength`).
    pub fn set_linelength(&self, linelength: f64) -> &Self {
        self.edit(|events, paths| {
            events.linelength = linelength;
            *paths = paths.iter().map(|p| events.respan(p)).collect();
        })
    }

    /// Where the lines are centred across the events (matplotlib's
    /// `get_lineoffset`).
    pub fn lineoffset(&self) -> f64 {
        self.read(|events, _| events.lineoffset)
    }

    /// Moves the centres of the lines across the events (matplotlib's
    /// `set_lineoffset`).
    pub fn set_lineoffset(&self, lineoffset: f64) -> &Self {
        self.edit(|events, paths| {
            events.lineoffset = lineoffset;
            *paths = paths.iter().map(|p| events.respan(p)).collect();
        })
    }

    /// The colour of the first line (matplotlib's `get_color`), `None`
    /// when the lines have no colour.
    ///
    /// # Errors
    ///
    /// When the colours cannot be resolved.
    pub fn color(&self) -> Result<Option<Rgba>, CollectionColorError> {
        Ok(self.colors()?.edges.first().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::{CollectionProps, CollectionSpec};
    use crate::figure::Figure;

    /// matplotlib's `generate_EventCollection_plot`.
    fn fibonacci() -> EventCollection {
        let positions = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0];
        let spec = CollectionSpec::events(&positions, Orientation::Horizontal, 1.0, 0.5)
            .with_props(CollectionProps::default().color(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        Figure::new()
            .subplot()
            .add_collection(spec)
            .raw()
            .downcast()
            .expect("an event collection")
    }

    #[test]
    fn layout_reads_back_as_made() {
        let events = fibonacci();
        assert_eq!(
            events.positions(),
            [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0]
        );
        assert!(events.is_horizontal());
        assert_eq!((events.linelength(), events.lineoffset()), (0.5, 1.0));
        assert_eq!(
            events.paths()[7].vertices(),
            [Point::new(21.0, 1.25), Point::new(21.0, 0.75)]
        );
        assert_eq!(events.color().unwrap(), Some(Rgba::new(1.0, 0.0, 0.0, 1.0)));
    }

    #[test]
    fn positions_are_added_in_order() {
        let events = fibonacci();
        events.switch_orientation().add_positions(&[34.0, -1.0]);
        events.switch_orientation();
        assert_eq!(events.positions()[..2], [-1.0, 0.0]);
        assert_eq!(events.positions().last(), Some(&34.0));
        assert_eq!(
            events.paths()[0].vertices(),
            [Point::new(-1.0, 1.25), Point::new(-1.0, 0.75)]
        );
    }

    #[test]
    fn orientation_switches_mirror_the_lines() {
        let events = fibonacci();
        events.set_orientation(Orientation::Vertical);
        assert!(!events.is_horizontal());
        assert_eq!(
            events.paths()[1].vertices(),
            [Point::new(1.25, 1.0), Point::new(0.75, 1.0)]
        );
        events.set_orientation(Orientation::Vertical);
        assert_eq!(events.orientation(), Orientation::Vertical, "unchanged");
        events.switch_orientation();
        assert_eq!(events.positions()[1], 1.0);
    }

    #[test]
    fn lengths_and_offsets_respan_the_lines() {
        let events = fibonacci();
        events.set_linelength(15.0).set_lineoffset(-5.0);
        assert_eq!((events.linelength(), events.lineoffset()), (15.0, -5.0));
        assert_eq!(
            events.paths()[2].vertices(),
            [Point::new(2.0, 2.5), Point::new(2.0, -12.5)]
        );
    }

    #[test]
    fn other_collections_are_not_events() {
        let ax = Figure::new().subplot();
        let lines = ax.add_collection(CollectionSpec::lines(vec![vec![Point::ORIGIN]]));
        assert_eq!(lines.raw().downcast::<EventCollection>(), None);
    }
}
