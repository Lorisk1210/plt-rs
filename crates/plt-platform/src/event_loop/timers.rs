//! One-shot and repeating timers multiplexed onto the loop's wait deadline.

use std::time::{Duration, Instant};

/// Identifies a timer started with
/// [`Context::set_timeout`](crate::Context::set_timeout) or
/// [`Context::set_interval`](crate::Context::set_interval).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimerId(u64);

#[derive(Debug)]
struct Entry {
    id: TimerId,
    /// `None` when the delay is too large to represent: never fires.
    deadline: Option<Instant>,
    /// Repeat period for intervals, `None` for one-shot timers.
    period: Option<Duration>,
}

/// The set of active timers. Timer counts are small (a handful of
/// animations and matplotlib `Timer`s), so a vector beats a heap here.
#[derive(Debug, Default)]
pub(crate) struct Timers {
    next_id: u64,
    entries: Vec<Entry>,
}

impl Timers {
    /// Starts a timer firing `delay` after `now`, then every `period` if
    /// one is given.
    pub(crate) fn add(&mut self, now: Instant, delay: Duration, period: Option<Duration>) -> TimerId {
        self.next_id += 1;
        let id = TimerId(self.next_id);
        self.entries.push(Entry {
            id,
            deadline: now.checked_add(delay),
            period,
        });
        id
    }

    /// Stops a timer; returns whether it was active.
    pub(crate) fn cancel(&mut self, id: TimerId) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        self.entries.len() != before
    }

    /// The earliest deadline of any active timer.
    pub(crate) fn next_deadline(&self) -> Option<Instant> {
        self.entries.iter().filter_map(|entry| entry.deadline).min()
    }

    /// Ids of the timers due at `now`, earliest first.
    pub(crate) fn due(&self, now: Instant) -> Vec<TimerId> {
        let mut due: Vec<(Instant, TimerId)> = self
            .entries
            .iter()
            .filter_map(|entry| entry.deadline.filter(|&d| d <= now).map(|d| (d, entry.id)))
            .collect();
        due.sort_unstable();
        due.into_iter().map(|(_, id)| id).collect()
    }

    /// Marks timer `id` as fired at `now`: one-shot timers are removed,
    /// intervals rescheduled one period later (skipping missed ticks).
    /// Returns `false` if the timer is no longer active or not due.
    pub(crate) fn fire(&mut self, id: TimerId, now: Instant) -> bool {
        let Some(index) = self.entries.iter().position(|entry| entry.id == id) else {
            return false;
        };
        let entry = &mut self.entries[index];
        match (entry.deadline, entry.period) {
            (Some(deadline), _) if deadline > now => false,
            (None, _) => false,
            (Some(deadline), Some(period)) => {
                let next = deadline.checked_add(period);
                entry.deadline = match next {
                    Some(next) if next > now => Some(next),
                    _ => now.checked_add(period),
                };
                true
            }
            (Some(_), None) => {
                self.entries.swap_remove(index);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn one_shot_fires_once() {
        let t0 = Instant::now();
        let mut timers = Timers::default();
        let id = timers.add(t0, 10 * MS, None);
        assert_eq!(timers.next_deadline(), Some(t0 + 10 * MS));
        assert!(timers.due(t0).is_empty());
        assert!(!timers.fire(id, t0));
        assert_eq!(timers.due(t0 + 10 * MS), vec![id]);
        assert!(timers.fire(id, t0 + 10 * MS));
        assert!(!timers.fire(id, t0 + 20 * MS));
        assert_eq!(timers.next_deadline(), None);
    }

    #[test]
    fn interval_reschedules_and_skips_missed_ticks() {
        let t0 = Instant::now();
        let mut timers = Timers::default();
        let id = timers.add(t0, 10 * MS, Some(10 * MS));
        assert!(timers.fire(id, t0 + 10 * MS));
        assert_eq!(timers.next_deadline(), Some(t0 + 20 * MS));
        // Far behind schedule: the next tick is one period after "now".
        assert!(timers.fire(id, t0 + 95 * MS));
        assert_eq!(timers.next_deadline(), Some(t0 + 105 * MS));
    }

    #[test]
    fn due_orders_by_deadline_and_cancel_works() {
        let t0 = Instant::now();
        let mut timers = Timers::default();
        let late = timers.add(t0, 5 * MS, None);
        let early = timers.add(t0, MS, None);
        assert_eq!(timers.due(t0 + 10 * MS), vec![early, late]);
        assert!(timers.cancel(late));
        assert!(!timers.cancel(late));
        assert_eq!(timers.due(t0 + 10 * MS), vec![early]);
    }

    #[test]
    fn absurd_delays_never_fire() {
        let t0 = Instant::now();
        let mut timers = Timers::default();
        let id = timers.add(t0, Duration::MAX, None);
        assert_eq!(timers.next_deadline(), None);
        assert!(!timers.fire(id, t0));
        assert!(timers.cancel(id));
    }
}
