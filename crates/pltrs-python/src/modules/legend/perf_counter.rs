//! The clock pltrs (Rust) times the search for a legend's best location
//! with (`pltrs::clock`): Python's `time.perf_counter`, read through the
//! package's registered `perf_counter` at every reading, as matplotlib's
//! `legend` module reads it, so that a test replacing it fakes a slow
//! search.

use crate::binding::lookup;
use crate::binding::prelude::*;

/// Reads pltrs (Rust)'s performance counter from the package's
/// `perf_counter` from now on.
pub(super) fn register() {
    pltrs::clock::set_source(perf_counter);
}

/// The reading of the registered `perf_counter`; pltrs (Rust)'s own
/// clock before the package registered it. A counter that raises reads
/// as NaN, its exception raised by the native call running.
fn perf_counter() -> f64 {
    let registered = Python::with_gil(|py| {
        lookup(py, "perf_counter").and_then(|counter| counter.extract::<Callback>())
    });
    match registered {
        Ok(counter) => counter.call::<f64>(()).unwrap_or(f64::NAN),
        Err(_) => pltrs::clock::monotonic(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::testing::{eval, extension, str, with_python};

    #[test]
    fn the_counter_is_the_registered_one() {
        with_python(|py| {
            let read = eval(
                py,
                "import itertools\nticks = itertools.count(0, 1.5)",
                "lambda: next(ticks)",
            );
            extension(py)
                .getattr("register")
                .and_then(|register| register.call(&[str(py, "perf_counter"), read], &[]))
                .unwrap();
            register();
            let first = pltrs::clock::perf_counter();
            assert_eq!(pltrs::clock::perf_counter() - first, 1.5);
            pltrs::clock::clear_source();
        });
    }
}
