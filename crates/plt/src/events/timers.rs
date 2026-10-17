//! Timers that call back while a figure is shown (matplotlib's
//! `canvas.new_timer`), the basis of animations.

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use crate::figure::Figure;

/// Identifies a timer of a figure, for [`Figure::remove_timer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerKey(u64);

/// A timer callback; returning `false` stops the timer.
type TimerCallback = Rc<RefCell<dyn FnMut() -> bool>>;

/// The timers of one figure.
#[derive(Default)]
pub(crate) struct TimerRegistry {
    next: u64,
    timers: Vec<(TimerKey, Duration, TimerCallback)>,
}

impl fmt::Debug for TimerRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TimerRegistry")
            .field("timers", &self.timers.len())
            .finish_non_exhaustive()
    }
}

impl Figure {
    /// Calls `callback` every `interval` while the figure is shown in a
    /// window (matplotlib's `canvas.new_timer`); the timer stops when the
    /// callback returns `false` or is removed. The window redraws the
    /// figure after a tick that changed it.
    pub fn add_timer(
        &self,
        interval: Duration,
        callback: impl FnMut() -> bool + 'static,
    ) -> TimerKey {
        let mut state = self.state_mut();
        let registry = &mut state.timers;
        let key = TimerKey(registry.next);
        registry.next += 1;
        registry
            .timers
            .push((key, interval, Rc::new(RefCell::new(callback))));
        key
    }

    /// Stops and forgets a timer; returns whether it was running.
    pub fn remove_timer(&self, key: TimerKey) -> bool {
        let mut state = self.state_mut();
        let timers = &mut state.timers.timers;
        let before = timers.len();
        timers.retain(|(k, _, _)| *k != key);
        timers.len() != before
    }

    /// The running timers and their intervals.
    pub fn timers(&self) -> Vec<(TimerKey, Duration)> {
        self.state()
            .timers
            .timers
            .iter()
            .map(|(key, interval, _)| (*key, *interval))
            .collect()
    }

    /// Runs one tick of timer `key`, as a window does when the interval
    /// elapsed; returns whether the timer keeps running.
    pub fn fire_timer(&self, key: TimerKey) -> bool {
        let callback = self
            .state()
            .timers
            .timers
            .iter()
            .find(|(k, _, _)| *k == key)
            .map(|(_, _, callback)| Rc::clone(callback));
        let Some(callback) = callback else {
            return false;
        };
        let keep = callback
            .try_borrow_mut()
            .map_or(true, |mut callback| callback());
        if !keep {
            self.remove_timer(key);
        }
        keep && self.timers().iter().any(|(k, _)| *k == key)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn timers_tick_until_they_stop() {
        let fig = Figure::new();
        let ticks = Rc::new(Cell::new(0));
        let count = Rc::clone(&ticks);
        let key = fig.add_timer(Duration::from_millis(10), move || {
            count.set(count.get() + 1);
            count.get() < 3
        });
        assert_eq!(fig.timers(), [(key, Duration::from_millis(10))]);
        assert!(fig.fire_timer(key));
        assert!(fig.fire_timer(key));
        assert!(!fig.fire_timer(key), "the third tick stops the timer");
        assert_eq!(fig.timers(), []);
        assert!(!fig.fire_timer(key));
        assert_eq!(ticks.get(), 3);
    }
}
