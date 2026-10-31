//! The performance counter pltrs reads where matplotlib reads Python's
//! `time.perf_counter`: to time the search for a legend's best location,
//! which warns when it is slow.
//!
//! The counter is the thread's source, which a program installs with
//! [`set_source`] (pltrs (Python) reads `time.perf_counter`, which tests
//! replace to fake slow searches); without one it is [`monotonic`].
//!
//! ```
//! use std::cell::Cell;
//! use std::rc::Rc;
//! use pltrs::clock;
//! let ticks = Rc::new(Cell::new(0.0));
//! let source = Rc::clone(&ticks);
//! clock::set_source(move || {
//!     source.set(source.get() + 1.5);
//!     source.get()
//! });
//! assert_eq!(clock::perf_counter(), 1.5);
//! assert_eq!(clock::perf_counter(), 3.0);
//! clock::clear_source();
//! assert!(clock::perf_counter() >= 0.0);
//! ```

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

/// A function reading the time in seconds.
type Source = Rc<dyn Fn() -> f64>;

thread_local! {
    /// The thread's source; `None` reads [`monotonic`].
    static SOURCE: RefCell<Option<Source>> = const { RefCell::new(None) };
    /// The instant [`monotonic`] counts from.
    static START: Instant = Instant::now();
}

/// Reads the thread's performance counter from `source` from now on.
pub fn set_source(source: impl Fn() -> f64 + 'static) {
    SOURCE.with(|slot| *slot.borrow_mut() = Some(Rc::new(source)));
}

/// Reads the thread's performance counter from [`monotonic`] again.
pub fn clear_source() {
    SOURCE.with(|slot| *slot.borrow_mut() = None);
}

/// The thread's performance counter in seconds; only differences between
/// two readings mean anything (Python's `time.perf_counter`).
pub fn perf_counter() -> f64 {
    // The source runs on a copy, so that one installing another source or
    // reading the counter does not borrow the slot twice.
    match SOURCE.with(|slot| slot.borrow().clone()) {
        Some(source) => source(),
        None => monotonic(),
    }
}

/// The seconds elapsed on a monotonic clock since the thread first read
/// it.
pub fn monotonic() -> f64 {
    START.with(|start| start.elapsed().as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_monotonic_clock_never_goes_back() {
        clear_source();
        let first = perf_counter();
        let second = perf_counter();
        assert!(second >= first && first >= 0.0);
    }
}
