//! The callbacks connected to a figure (matplotlib's `CallbackRegistry`
//! behind `mpl_connect`), and blocking them for a while
//! (`CallbackRegistry.blocked`).

use std::cell::RefCell;
use std::fmt;
use std::rc::{Rc, Weak};

use super::{Event, EventKind};
use crate::figure::Figure;

/// Identifies a connected callback, for [`Figure::disconnect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConnectionId(u64);

pltrs_core::archive_struct!(ConnectionId(v0));

/// A callback, shared so that it can run while the registry is borrowed
/// for something else.
type Callback = Rc<RefCell<dyn FnMut(&Event)>>;

/// What one block of the callbacks covers: the callbacks of one kind of
/// event, or all of them.
#[derive(Debug)]
struct BlockScope(Option<EventKind>);

/// The callbacks of a figure blocked until the guard is dropped, from
/// [`Figure::block_callbacks`] (matplotlib's `callbacks.blocked()`
/// context).
#[must_use = "the callbacks are blocked only while the guard lives"]
#[derive(Debug)]
pub struct CallbacksBlocked {
    /// The block, alive as long as the guard: the registry holds it weakly,
    /// so dropping the guard ends it without touching the figure.
    _scope: Rc<BlockScope>,
}

/// The callbacks of one figure, by event kind, in connection order.
#[derive(Default)]
pub(crate) struct CallbackRegistry {
    next: u64,
    callbacks: Vec<(ConnectionId, EventKind, Callback)>,
    /// The blocks in force, and those whose guards are gone.
    blocks: Vec<Weak<BlockScope>>,
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

    /// The callbacks to run for an event of `kind`: none while blocked.
    fn listeners(&self, kind: EventKind) -> Vec<Callback> {
        if self.is_blocked(kind) {
            return Vec::new();
        }
        self.callbacks
            .iter()
            .filter(|(_, k, _)| *k == kind)
            .map(|(_, _, callback)| Rc::clone(callback))
            .collect()
    }

    /// Blocks the callbacks of `signal`, or all of them, for as long as the
    /// returned scope lives.
    fn block(&mut self, signal: Option<EventKind>) -> Rc<BlockScope> {
        let scope = Rc::new(BlockScope(signal));
        self.blocks.retain(|block| block.strong_count() > 0);
        self.blocks.push(Rc::downgrade(&scope));
        scope
    }

    /// Whether a block in force covers events of `kind`.
    fn is_blocked(&self, kind: EventKind) -> bool {
        self.blocks
            .iter()
            .filter_map(Weak::upgrade)
            .any(|scope| scope.0.is_none_or(|signal| signal == kind))
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

    /// Blocks the callbacks connected for `signal`, or every callback with
    /// `None`, until the returned guard is dropped (matplotlib's
    /// `with fig.canvas.callbacks.blocked(signal=...)`). Blocked events
    /// reach no callback, and blocked presses and scrolls pick nothing,
    /// as matplotlib's figure picks from a press callback of its own.
    /// Callbacks connected meanwhile wait like the others and stay
    /// connected once the block ends, where matplotlib forgets them.
    /// Blocks nest: an event is delivered when no guard covers it.
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use pltrs::events::{Event, EventKind};
    /// use pltrs::figure::Figure;
    ///
    /// let fig = Figure::new();
    /// let closes = Rc::new(Cell::new(0));
    /// let seen = Rc::clone(&closes);
    /// fig.connect(EventKind::Close, move |_| seen.set(seen.get() + 1));
    /// let blocked = fig.block_callbacks(Some(EventKind::Close));
    /// fig.process_event(&Event::Close);
    /// drop(blocked);
    /// fig.process_event(&Event::Close);
    /// assert_eq!(closes.get(), 1);
    /// ```
    pub fn block_callbacks(&self, signal: Option<EventKind>) -> CallbacksBlocked {
        CallbacksBlocked {
            _scope: self.state_mut().callbacks.block(signal),
        }
    }

    /// Runs `f` with the callbacks connected for `signal`, or every
    /// callback with `None`, blocked ([`Figure::block_callbacks`]), and
    /// returns what it returns. `f` may use the figure.
    ///
    /// ```
    /// use std::cell::Cell;
    /// use std::rc::Rc;
    /// use pltrs::events::{Event, EventKind};
    /// use pltrs::figure::Figure;
    ///
    /// let fig = Figure::new();
    /// let closes = Rc::new(Cell::new(0));
    /// let seen = Rc::clone(&closes);
    /// fig.connect(EventKind::Close, move |_| seen.set(seen.get() + 1));
    /// let axes = fig.with_callbacks_blocked(None, || {
    ///     fig.process_event(&Event::Close);
    ///     fig.subplot()
    /// });
    /// assert_eq!(closes.get(), 0);
    /// assert_eq!(fig.axes(), [axes]);
    /// ```
    pub fn with_callbacks_blocked<R>(&self, signal: Option<EventKind>, f: impl FnOnce() -> R) -> R {
        let _blocked = self.block_callbacks(signal);
        f()
    }

    /// Delivers `event` to the callbacks connected for its kind, in
    /// connection order (matplotlib's `callbacks.process`), unless they are
    /// blocked ([`Figure::block_callbacks`]); presses and scrolls first
    /// pick the artists under the pointer ([`Figure::pick`]). Callbacks may
    /// change the figure and connect or disconnect callbacks; a callback
    /// that is already running (an event processed from inside itself) is
    /// skipped.
    pub fn process_event(&self, event: &Event) {
        let blocked = self.state().callbacks.is_blocked(event.kind());
        if let Event::ButtonPress(mouse) | Event::Scroll(mouse) = event
            && !blocked
        {
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
    fn blocks_cover_one_kind_or_all_and_nest() {
        let fig = Figure::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        for kind in [EventKind::Close, EventKind::FigureLeave] {
            let seen = Rc::clone(&log);
            fig.connect(kind, move |event| seen.borrow_mut().push(event.kind()));
        }
        let send = || {
            fig.process_event(&Event::Close);
            fig.process_event(&Event::FigureLeave);
        };
        let close = fig.block_callbacks(Some(EventKind::Close));
        send();
        let all = fig.block_callbacks(None);
        send();
        // Connected while blocked: waits, then stays.
        let seen = Rc::clone(&log);
        let late = fig.connect(EventKind::Draw, move |_| {
            seen.borrow_mut().push(EventKind::Draw);
        });
        fig.process_event(&Event::Draw);
        drop(close);
        send();
        drop(all);
        send();
        fig.process_event(&Event::Draw);
        assert_eq!(
            *log.borrow(),
            [
                EventKind::FigureLeave,
                EventKind::Close,
                EventKind::FigureLeave,
                EventKind::Draw
            ]
        );
        assert!(fig.disconnect(late));
    }

    #[test]
    fn blocked_presses_pick_nothing() {
        use crate::artist::{ArtistHandle, Picker};
        use crate::events::EventSource;
        use crate::navigation::PointerButton;

        let fig = Figure::with_size(4.0, 4.0, 100.0);
        let ax = fig
            .add_axes([0.0, 0.0, 1.0, 1.0])
            .expect("a valid rectangle");
        let asked = Rc::new(Cell::new(0));
        let count = Rc::clone(&asked);
        ax.set_picker(Some(Picker::custom(move |_, _| {
            count.set(count.get() + 1);
            Some(Vec::new())
        })));
        let picks = Rc::new(Cell::new(0));
        let seen = Rc::clone(&picks);
        fig.connect(EventKind::Pick, move |_| seen.set(seen.get() + 1));
        let press = || {
            EventSource::new().button_press(&fig, 200.0, 200.0, PointerButton::Left, false);
        };
        fig.with_callbacks_blocked(Some(EventKind::ButtonPress), press);
        assert_eq!((asked.get(), picks.get()), (0, 0), "no pick at all");
        fig.with_callbacks_blocked(Some(EventKind::Pick), press);
        assert_eq!((asked.get(), picks.get()), (1, 0), "picked, not delivered");
        press();
        assert_eq!((asked.get(), picks.get()), (2, 1));
    }

    #[test]
    fn guards_kept_by_their_own_figure_end_quietly() {
        let fig = Figure::new();
        let blocked = fig.block_callbacks(None);
        fig.connect(EventKind::Close, move |_| {
            let _ = &blocked;
        });
        fig.with_callbacks_blocked(None, || fig.process_event(&Event::Close));
        drop(fig);
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
