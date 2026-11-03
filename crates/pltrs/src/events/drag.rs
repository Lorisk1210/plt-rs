//! Dragging artists with the mouse (matplotlib's `DraggableBase`): a press
//! that picks the artist starts a drag, pointer motion moves it and the
//! release ends it.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use pltrs_core::Point;

use super::{ConnectionId, Event, EventKind};
use crate::artist::{ArtistHandle, ArtistId, Picker, RawHandle};
use crate::figure::{Figure, FigureCell};

/// How a draggable artist follows the pointer (the methods matplotlib's
/// `DraggableBase` subclasses override).
pub(crate) trait DragTarget {
    /// Remembers where the artist is as a drag starts (`save_offset`).
    fn save_offset(&mut self, artist: &RawHandle);

    /// Moves the artist `dx, dy` display pixels from where it was when the
    /// drag started (`update_offset`).
    fn update_offset(&mut self, artist: &RawHandle, dx: f64, dy: f64);

    /// Settles the artist where the drag ended (`finalize_offset`).
    fn finalize_offset(&mut self, _artist: &RawHandle) {}
}

/// The callbacks that drag one artist; [`Dragger::disconnect`] ends them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dragger {
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
    /// Makes `artist` draggable as `target` says. An artist without a
    /// picker gets one that ignores the wheel (matplotlib's
    /// `DraggableBase._picker`).
    pub(crate) fn connect(artist: &RawHandle, target: impl DragTarget + 'static) -> Self {
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

    /// Ends dragging (matplotlib's `DraggableBase.disconnect`).
    pub(crate) fn disconnect(&self, figure: &Figure) {
        for id in self.connections {
            figure.disconnect(id);
        }
    }
}

#[cfg(test)]
mod tests;
