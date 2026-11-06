//! Dragging artists with the mouse (matplotlib's `DraggableBase`): a press
//! that picks the artist starts a drag, pointer motion moves it and the
//! release ends it. Legends, annotations and offset boxes are made
//! draggable by their `set_draggable`; [`Dragger`] makes any artist
//! draggable as a [`DragTarget`] of the program's says.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use pltrs_core::Point;

use super::{ConnectionId, Event, EventKind};
use crate::artist::{ArtistHandle, ArtistId, Picker, RawHandle};
use crate::figure::{Figure, FigureCell};

/// How a draggable artist follows the pointer (the methods matplotlib's
/// `DraggableBase` subclasses override): [`Dragger::connect`] calls them
/// as the user drags the artist.
pub trait DragTarget {
    /// Remembers where the artist is as a drag starts (`save_offset`).
    fn save_offset(&mut self, artist: &RawHandle);

    /// Moves the artist `dx, dy` display pixels from where it was when the
    /// drag started (`update_offset`).
    fn update_offset(&mut self, artist: &RawHandle, dx: f64, dy: f64);

    /// Settles the artist where the drag ended (`finalize_offset`).
    fn finalize_offset(&mut self, _artist: &RawHandle) {}
}

/// The callbacks that drag one artist (matplotlib's `DraggableBase`):
/// [`Dragger::connect`] makes the artist draggable and
/// [`Dragger::disconnect`] ends it.
///
/// ```
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use pltrs::artist::{RawHandle, TypedHandle};
/// use pltrs::events::{DragTarget, Dragger, EventSource};
/// use pltrs::navigation::PointerButton;
/// use pltrs::prelude::*;
/// use pltrs::text::Text;
/// use pltrs_core::Point;
///
/// /// Moves a text in figure coordinates as the pointer moves.
/// struct MoveText {
///     start: Point,
///     size: (f64, f64),
/// }
///
/// impl DragTarget for MoveText {
///     fn save_offset(&mut self, artist: &RawHandle) {
///         if let Some(text) = Text::from_raw_handle(artist) {
///             self.start = text.position();
///         }
///     }
///
///     fn update_offset(&mut self, artist: &RawHandle, dx: f64, dy: f64) {
///         if let Some(text) = Text::from_raw_handle(artist) {
///             let (width, height) = self.size;
///             text.set_position(Point::new(self.start.x + dx / width, self.start.y + dy / height));
///         }
///     }
/// }
///
/// let fig = Figure::with_size(4.0, 3.0, 100.0);
/// let text = fig.text(0.5, 0.5, "drag me");
/// let target = MoveText { start: Point::new(0.0, 0.0), size: (400.0, 300.0) };
/// let dragger = Dragger::connect(&text, target);
/// let mut mouse = EventSource::new();
/// mouse.button_press(&fig, 205.0, 153.0, PointerButton::Left, false);
/// mouse.motion(&fig, 245.0, 183.0);
/// mouse.button_release(&fig, 245.0, 183.0, PointerButton::Left);
/// assert_eq!(text.position(), Point::new(0.6, 0.6));
/// dragger.disconnect(&fig);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dragger {
    connections: [ConnectionId; 3],
}

pltrs_core::archive_struct!(Dragger { connections });

/// A drag in progress, or waiting for one.
struct Drag {
    target: Box<dyn DragTarget>,
    /// The figure, held weakly so that the callbacks do not keep it alive.
    figure: Weak<FigureCell>,
    artist: ArtistId,
    /// Where the pointer was when the drag started.
    start: Option<Point>,
}

impl Drag {
    /// The dragged artist, while it and its figure exist.
    fn artist(&self) -> Option<RawHandle> {
        let handle = RawHandle::new(self.figure.upgrade()?, self.artist);
        handle.is_attached().then_some(handle)
    }
}

impl Dragger {
    /// Makes `artist` draggable as `target` says: a mouse press that picks
    /// it starts a drag, and the artist follows the pointer until the
    /// button goes up. An artist without a picker gets one that ignores
    /// the wheel (matplotlib's `DraggableBase._picker`).
    pub fn connect(artist: &impl ArtistHandle, target: impl DragTarget + 'static) -> Self {
        let artist = artist.raw();
        if !artist.pickable() {
            artist.set_picker(Some(Picker::Press));
        }
        let drag = Rc::new(RefCell::new(Drag {
            target: Box::new(target),
            figure: Rc::downgrade(artist.figure_state()),
            artist: artist.id(),
            start: None,
        }));
        let figure = Figure::root_of(artist.figure_state());
        let on_pick = Rc::clone(&drag);
        let on_motion = Rc::clone(&drag);
        let connections = [
            figure.connect(EventKind::Pick, move |event| {
                let mut drag = on_pick.borrow_mut();
                if let (Event::Pick(pick), Some(artist)) = (event, drag.artist())
                    && pick.axis.is_none()
                    && pick.artist == artist
                {
                    let mouse = &pick.mouse.location;
                    drag.start = Some(Point::new(mouse.x, mouse.y));
                    drag.target.save_offset(&artist);
                }
            }),
            figure.connect(EventKind::Motion, move |event| {
                let mut drag = on_motion.borrow_mut();
                if let (Event::Motion(mouse), Some(start), Some(artist)) =
                    (event, drag.start, drag.artist())
                {
                    let (dx, dy) = (mouse.location.x - start.x, mouse.location.y - start.y);
                    drag.target.update_offset(&artist, dx, dy);
                }
            }),
            figure.connect(EventKind::ButtonRelease, move |_| {
                let mut drag = drag.borrow_mut();
                if let (Some(_), Some(artist)) = (drag.start.take(), drag.artist()) {
                    drag.target.finalize_offset(&artist);
                }
            }),
        ];
        Self { connections }
    }

    /// Ends dragging (matplotlib's `DraggableBase.disconnect`): removes the
    /// callbacks from `figure`, the figure of the artist.
    pub fn disconnect(&self, figure: &Figure) {
        for id in self.connections {
            figure.disconnect(id);
        }
    }
}

#[cfg(test)]
mod tests;
