//! The timers of served figures (`Figure::add_timer`, the basis of
//! animations), run on the server's thread like matplotlib's
//! `TimerAsyncio`.

use std::time::{Duration, Instant};

use crate::events::TimerKey;
use crate::figure::Figure;

/// The shortest interval a timer runs at, so that a zero interval does
/// not spin.
const MIN_INTERVAL: Duration = Duration::from_millis(1);

/// The running timers of one figure and when each fires next.
#[derive(Debug, Default)]
pub(crate) struct TimerSchedule {
    timers: Vec<(TimerKey, Duration, Instant)>,
}

impl TimerSchedule {
    /// Starts the figure's new timers (first tick one interval from `now`)
    /// and forgets the removed ones.
    pub(crate) fn sync(&mut self, figure: &Figure, now: Instant) {
        let wanted = figure.timers();
        self.timers
            .retain(|(key, _, _)| wanted.iter().any(|(k, _)| k == key));
        for (key, interval) in wanted {
            if !self.timers.iter().any(|(k, _, _)| *k == key) {
                let interval = interval.max(MIN_INTERVAL);
                self.timers.push((key, interval, now + interval));
            }
        }
    }

    /// Fires the timers due at `now`; one that fell behind skips the
    /// missed ticks.
    pub(crate) fn fire_due(&mut self, figure: &Figure, now: Instant) {
        let due: Vec<TimerKey> = self
            .timers
            .iter_mut()
            .filter(|(_, _, next)| *next <= now)
            .map(|(key, interval, next)| {
                *next += *interval;
                if *next <= now {
                    *next = now + *interval;
                }
                *key
            })
            .collect();
        for key in due {
            figure.fire_timer(key);
        }
        self.sync(figure, now);
    }

    /// When the next timer fires.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.timers.iter().map(|(_, _, next)| *next).min()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn timers_fire_when_due_until_they_stop() {
        let figure = Figure::new();
        let ticks = Rc::new(Cell::new(0));
        let count = Rc::clone(&ticks);
        figure.add_timer(Duration::from_millis(10), move || {
            count.set(count.get() + 1);
            count.get() < 2
        });
        let start = Instant::now();
        let mut schedule = TimerSchedule::default();
        schedule.sync(&figure, start);
        assert_eq!(
            schedule.next_deadline(),
            Some(start + Duration::from_millis(10))
        );
        schedule.fire_due(&figure, start);
        assert_eq!(ticks.get(), 0, "not due yet");
        schedule.fire_due(&figure, start + Duration::from_millis(35));
        assert_eq!(ticks.get(), 1, "missed ticks are skipped");
        assert_eq!(
            schedule.next_deadline(),
            Some(start + Duration::from_millis(45))
        );
        schedule.fire_due(&figure, start + Duration::from_millis(45));
        assert_eq!(ticks.get(), 2);
        assert_eq!(schedule.next_deadline(), None, "the timer stopped");
    }
}
