//! Whether a draw ends at its first problem.
//!
//! pltrs draws everything it can draw: an artist that cannot be drawn
//! (an unknown colour, a norm whose `vmin` exceeds its `vmax`) is left out
//! and reported, the rest of the figure is drawn ([`DrawErrors`]).
//! matplotlib's draw instead ends at the first exception an artist
//! raises, with the artists after it undrawn: a binding that raises the
//! first problem as matplotlib would asks for that with
//! [`set_draws_stop_at_first_problem`], which saves it drawing what its
//! program never sees (a figure of costly collections behind a failing
//! image, which matplotlib draws first). The failures of the program's
//! code a draw runs (see [`failures`]) end it too.
//!
//! ```
//! use pltrs::artist::set_draws_stop_at_first_problem;
//! use pltrs::prelude::*;
//!
//! let fig = Figure::new();
//! let ax = fig.subplot();
//! ax.plot([0.0, 1.0], [0.0, 1.0])?.set_color("no such colour");
//! ax.plot([0.0, 1.0], [1.0, 0.0])?.set_color("not one either");
//! assert_eq!(fig.canvas().draw().unwrap_err().to_string().matches("artist").count(), 3);
//! set_draws_stop_at_first_problem(true);
//! assert_eq!(fig.canvas().draw().unwrap_err().to_string().matches("artist").count(), 2);
//! set_draws_stop_at_first_problem(false);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [`DrawErrors`]: crate::figure::DrawErrors
//! [`failures`]: crate::layout::failures

use std::cell::Cell;

pltrs_core::thread_state! {
    /// Whether the draws of the thread's figures end at their first problem.
    static STOPS: Cell<bool> = const { Cell::new(false) };
}

/// Makes the draws of figures on this thread (and the threads sharing its
/// [state](pltrs_core::state)) end at their first problem, leaving the
/// artists after it undrawn, as matplotlib's draw ends at its first
/// exception (`true`); or draw every artist that can be drawn and report
/// every problem (`false`, the default).
pub fn set_draws_stop_at_first_problem(stop: bool) {
    STOPS.set(stop);
}

/// Whether draws end at their first problem (see
/// [`set_draws_stop_at_first_problem`]).
pub(crate) fn draws_stop_at_first_problem() -> bool {
    STOPS.get()
}
