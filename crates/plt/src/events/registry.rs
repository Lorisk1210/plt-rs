//! The callbacks connected to a figure (matplotlib's `CallbackRegistry`
//! behind `mpl_connect`).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::{Event, EventKind};
use crate::figure::Figure;

/// Identifies a connected callback, for [`Figure::disconnect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(u64);

/// A callback, shared so that it can run while the registry is borrowed
/// for something else.
type Callback = Rc<RefCell<dyn FnMut(&Event)>>;

/// The callbacks of one figure, by event kind, in connection order.
#[derive(Default)]
pub(crate) struct CallbackRegistry {
    next: u64,
    callbacks: Vec<(ConnectionId, EventKind, Callback)>,
}

impl fmt::Debug for CallbackRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CallbackRegistry")
            .field("connected", &self.callbacks.len())
            .finish_non_exhaustive()
    }
}

impl CallbackRegistry {
    fn connect(&mut self, kind: EventKind, callback: Callback) -> ConnectionId {
        let id = ConnectionId(self.next);
        self.next += 1;
        self.callbacks.push((id, kind, callback));
        id
    }

    /// Removes callback `id` and returns it, for the caller to drop once
    /// the figure is no longer borrowed (dropping a callback may drop
    /// widgets that disconnect their own callbacks).
    fn take(&mut self, id: ConnectionId) -> Option<Callback> {
        let index = self.callbacks.iter().position(|(cid, _, _)| *cid == id)?;
        Some(self.callbacks.remove(index).2)
    }

    fn listeners(&self, kind: EventKind) -> Vec<Callback> {
        self.callbacks
            .iter()
            .filter(|(_, k, _)| *k == kind)
            .map(|(_, _, callback)| Rc::clone(callback))
            .collect()
    }
}

impl Figure {
    /// Calls `callback` with every event of `kind` (matplotlib's
    /// `mpl_connect`); returns an id for [`Figure::disconnect`].
    pub fn connect(&self, kind: EventKind, callback: impl FnMut(&Event) + 'static) -> ConnectionId {
        self.state_mut()
            .callbacks
            .connect(kind, Rc::new(RefCell::new(callback)))
    }

    /// Disconnects a callback (matplotlib's `mpl_disconnect`); returns
    /// whether it was connected.
    pub fn disconnect(&self, id: ConnectionId) -> bool {
        let removed = self.state_mut().callbacks.take(id);
        removed.is_some()
    }

    /// Delivers `event` to the callbacks connected for its kind, in
    /// connection order (matplotlib's `callbacks.process`); presses and
    /// scrolls first pick the artists under the pointer
    /// ([`Figure::pick`]). Callbacks may change the figure and connect or
    /// disconnect callbacks; a callback that is already running (an event
    /// processed from inside itself) is skipped.
    pub fn process_event(&self, event: &Event) {
        if let Event::ButtonPress(mouse) | Event::Scroll(mouse) = event {
            self.pick(mouse);
        }
        let listeners = self.state().callbacks.listeners(event.kind());
        for callback in listeners {
            if let Ok(mut callback) = callback.try_borrow_mut() {
                callback(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn drawing_the_figure_sends_a_draw_event() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        fig.subplot();
        let draws = Rc::new(Cell::new(0));
        let seen = Rc::clone(&draws);
        let redraw = fig.clone();
        fig.connect(EventKind::Draw, move |event| {
            assert_eq!(event, &Event::Draw);
            seen.set(seen.get() + 1);
            // A callback that draws again does not recurse into itself.
            let mut canvas = redraw.raster_canvas().expect("a canvas");
            redraw.draw(&mut canvas).expect("drawable");
        });
        fig.to_png().expect("saved");
        assert_eq!(draws.get(), 1);
    }

    #[test]
    fn callbacks_run_by_kind_until_disconnected() {
        let fig = Figure::new();
        let count = Rc::new(Cell::new(0));
        let seen = Rc::clone(&count);
        let id = fig.connect(EventKind::Close, move |_| seen.set(seen.get() + 1));
        fig.process_event(&Event::Close);
        fig.process_event(&Event::FigureLeave);
        assert_eq!(count.get(), 1);
        assert!(fig.disconnect(id));
        assert!(!fig.disconnect(id));
        fig.process_event(&Event::Close);
        assert_eq!(count.get(), 1);
    }

    #[test]
    fn callbacks_may_use_the_figure() {
        let fig = Figure::new();
        let inner = fig.clone();
        fig.connect(EventKind::Close, move |event| {
            inner.subplot();
            inner.process_event(event);
        });
        fig.process_event(&Event::Close);
        assert_eq!(fig.axes().len(), 1, "the re-entrant call is skipped");
    }
}
