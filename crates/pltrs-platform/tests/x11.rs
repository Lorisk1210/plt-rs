//! The public API against a real X server. Runs when `$DISPLAY` is set
//! and passes trivially otherwise; a private Xvfb works well:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! DISPLAY=:117 cargo test -p pltrs-platform --test x11
//! ```
//!
//! X11 is only spoken on Unix: on Windows, where `$DISPLAY` can be set too
//! (for an X server such as `VcXsrv`, or under Wine), there is nothing to
//! test.

#![cfg(unix)]

mod common;

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use pltrs_platform::{BackendKind, Error, EventLoop};

/// Windows of concurrent tests would overlap and spoil read-backs.
static SERVER: Mutex<()> = Mutex::new(());

/// An X11 event loop with exclusive use of the server, or `None` when
/// `$DISPLAY` is unset.
fn x11() -> Option<(MutexGuard<'static, ()>, EventLoop)> {
    std::env::var_os("DISPLAY").filter(|name| !name.is_empty())?;
    let serial = SERVER.lock().unwrap_or_else(PoisonError::into_inner);
    // A server resets when its last client disconnects and drops
    // connection attempts until it is done: retry for a while.
    for _ in 0..100 {
        match EventLoop::with_backend(BackendKind::X11) {
            Ok(event_loop) => {
                assert_eq!(event_loop.backend_kind(), BackendKind::X11);
                return Some((serial, event_loop));
            }
            Err(Error::ConnectionClosed | Error::Io { .. }) => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("cannot connect to $DISPLAY: {error}"),
        }
    }
    panic!("the X server keeps dropping connections");
}

#[test]
fn presented_frames_read_back_exactly() {
    if let Some((_serial, mut event_loop)) = x11() {
        common::presented_frames_read_back_exactly(&mut event_loop);
    }
}

#[test]
fn resizing_reports_the_new_size() {
    if let Some((_serial, mut event_loop)) = x11() {
        let window = common::resizing_reports_the_new_size(&mut event_loop);
        assert_eq!(event_loop.context().scale_factor(window), Ok(1.0));
    }
}

#[test]
fn timers_and_wake_ups_drive_a_blocking_run() {
    if let Some((_serial, mut event_loop)) = x11() {
        common::timers_and_wake_ups_drive_a_blocking_run(&mut event_loop);
    }
}
