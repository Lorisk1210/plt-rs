//! The windows' event loop waiting without the GIL.
//!
//! pltrs (Rust)'s windows wait for something to happen (input, the next
//! timer, a wake-up from another thread or a signal) inside a native call,
//! `_native.gui.pump`. Waiting with the GIL held would stop every other
//! Python thread for as long as the windows are idle, now that the wait
//! has no 20 ms limit. So the loop runs its waits through the hook this
//! module makes ([`releasing_gil`]), which releases the GIL around them as
//! a blocking `time.sleep` or socket read does in CPython. That is sound
//! because a wait touches nothing but the window system's connection
//! (`pltrs_platform::WaitHook`'s contract): no Python object and none of
//! the pltrs (Rust) state the GIL guards ([`threads`](super::threads)).

use pltrs_platform::WaitHook;

use super::gil::{gil_held, without_gil};

/// The wait hook of the windows' event loop: runs each wait with the GIL
/// released when the thread holds it (a native call is waiting), as is
/// otherwise.
pub(crate) fn releasing_gil() -> WaitHook {
    Box::new(|wait| {
        if gil_held() {
            // SAFETY: the thread holds the GIL through the thread state of
            // the native call that is waiting (`pump`); the wait touches no
            // Python object and no pltrs (Rust) value, only the window
            // system's connection (the contract of `WaitHook`).
            unsafe { without_gil(wait) }
        } else {
            wait();
        }
    })
}
