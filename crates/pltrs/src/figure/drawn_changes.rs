//! Changes the program's code asks for while a figure is drawn.
//!
//! Drawing a figure runs the program's code of a host language: a path
//! effect, a norm, a transform. That code may change the figure, as
//! matplotlib's may (a path effect recolouring its artist), but finds it
//! in use, borrowed for reading while its artists are drawn (see
//! [`reenter`](super::reenter)); the host keeps the changes to make them
//! once the figure is free. matplotlib makes them at once, so that what
//! the draw does after sees them: the draw event's callbacks, and whether
//! each artist is left stale. So once a draw has drawn the figure's
//! artists, with the figure free again and before the figure counts as
//! drawn and its draw event is sent, it runs the thread's draw-end handler
//! ([`set_draw_end_handler`]), in which the host makes the changes.
//!
//! An artist matplotlib changes while it draws it is left fresh by the end
//! of its own draw, one it changes after it drew it stale. Every artist
//! drawn takes the next tick of its thread's clock of draws
//! ([`draw_tick`]) as its draw ends; a host notes the tick a change is
//! asked for at, and makes it inside [`replaying_changes_from`] with that
//! tick: the artists it changes whose draw ended after it stay fresh.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

thread_local! {
    /// How many draws of artists ended on this thread.
    static DRAW_TICK: Cell<u64> = const { Cell::new(0) };
    /// The tick a change being made was asked for at, while it is made.
    static REPLAYING_FROM: Cell<Option<u64>> = const { Cell::new(None) };
}

/// What a host runs once a figure's artists are drawn.
type Handler = Rc<dyn Fn()>;

pltrs_core::thread_state! {
    /// The thread's draw-end handler.
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
}

/// Runs `handler` on this thread whenever a draw of a figure has drawn its
/// artists, with the figure free again, before the figure counts as drawn
/// and before its draw event is sent: where a host makes the changes its
/// program's code asked for while the figure was drawn.
///
/// ```
/// use std::cell::Cell;
/// use std::rc::Rc;
/// use pltrs::figure::{Figure, clear_draw_end_handler, set_draw_end_handler};
/// let fig = Figure::new();
/// fig.subplot();
/// let ends = Rc::new(Cell::new(0));
/// let counted = Rc::clone(&ends);
/// set_draw_end_handler(move || counted.set(counted.get() + 1));
/// fig.canvas().draw()?;
/// clear_draw_end_handler();
/// assert_eq!(ends.get(), 1);
/// # Ok::<(), pltrs::figure::CanvasError>(())
/// ```
pub fn set_draw_end_handler(handler: impl Fn() + 'static) {
    HANDLER.set(Some(Rc::new(handler)));
}

/// Removes the thread's draw-end handler.
pub fn clear_draw_end_handler() {
    HANDLER.set(None);
}

/// Runs the thread's draw-end handler, if any (on a copy, so that it may
/// install another or draw a figure).
pub(crate) fn run_draw_end_handler() {
    if let Some(handler) = HANDLER.with(|slot| slot.borrow().clone()) {
        handler();
    }
}

/// The tick of this thread's clock of draws: how many draws of artists
/// ended on it. A change asked for at a tick is made inside
/// [`replaying_changes_from`] with it.
pub fn draw_tick() -> u64 {
    DRAW_TICK.get()
}

/// The next tick, which the draw of an artist ending takes.
pub(crate) fn next_draw_tick() -> u64 {
    let tick = DRAW_TICK.get() + 1;
    DRAW_TICK.set(tick);
    tick
}

/// Runs `make`, which makes a change the program's code asked for at
/// `tick` while a figure was drawn: the artists it changes whose draw
/// ended after that tick stay fresh, as matplotlib's end of their draw
/// leaves them.
pub fn replaying_changes_from<R>(tick: u64, make: impl FnOnce() -> R) -> R {
    /// Restores the tick of the change being made around this one.
    struct Restore(Option<u64>);
    impl Drop for Restore {
        fn drop(&mut self) {
            REPLAYING_FROM.set(self.0);
        }
    }
    let _restore = Restore(REPLAYING_FROM.replace(Some(tick)));
    make()
}

/// Whether a change being made leaves an artist whose last draw ended at
/// tick `drawn` fresh: it is made for the program's code, which asked for
/// it before that draw ended.
pub(crate) fn drawn_after_change(drawn: u64) -> bool {
    REPLAYING_FROM.get().is_some_and(|asked| drawn > asked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle;
    use crate::figure::Figure;

    #[test]
    fn changes_asked_for_before_a_draw_ended_leave_the_artist_fresh() {
        let fig = Figure::new();
        let ax = fig.subplot();
        let line = ax.plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        let before = draw_tick();
        fig.canvas().draw().expect("drawn");
        assert!(draw_tick() > before, "every artist drawn ticks");
        replaying_changes_from(before, || line.set_linewidth(3.0));
        assert!(!line.stale() && !ax.stale(), "drawn after the change");
        replaying_changes_from(draw_tick(), || line.set_linewidth(4.0));
        assert!(line.stale() && ax.stale(), "drawn before the change");
        fig.canvas().draw().expect("drawn");
        line.set_linewidth(5.0);
        assert!(line.stale(), "a change of the program's own");
    }

    #[test]
    fn the_handler_runs_between_drawing_and_the_draw_event() {
        let fig = Figure::new();
        let line = fig.subplot().plot([0.0, 1.0], [1.0, 0.0]).expect("plotted");
        let seen = Rc::new(Cell::new(None));
        let (sink, changed) = (Rc::clone(&seen), line.clone());
        set_draw_end_handler(move || {
            // The figure is free: the handler changes it.
            changed.set_linewidth(3.0);
            sink.set(Some(changed.stale()));
        });
        fig.canvas().draw().expect("drawn");
        clear_draw_end_handler();
        assert_eq!(seen.get(), Some(true), "a change of its own");
        assert!(!fig.is_stale(), "the figure counts as drawn after");
    }
}
