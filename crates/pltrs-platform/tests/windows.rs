//! The public API against Win32, on Windows, or on Linux under Wine with
//! an X server for its windows:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! DISPLAY=:117 crates/pltrs-platform/tools/wine.sh cargo test -p pltrs-platform --test windows
//! ```

#![cfg(windows)]

mod common;

use std::sync::{Mutex, MutexGuard, PoisonError};

use pltrs_platform::{BackendKind, EventLoop, PhysicalSize, WindowEvent};

/// The tests take turns: the clipboard is the desktop's.
static DESKTOP: Mutex<()> = Mutex::new(());

/// A Win32 event loop, the desktop to itself.
fn windows() -> (MutexGuard<'static, ()>, EventLoop) {
    let serial = DESKTOP.lock().unwrap_or_else(PoisonError::into_inner);
    let event_loop = EventLoop::with_backend(BackendKind::Windows).expect("Win32 is there");
    assert_eq!(event_loop.backend_kind(), BackendKind::Windows);
    (serial, event_loop)
}

#[test]
fn presented_frames_read_back_exactly() {
    let (_serial, mut event_loop) = windows();
    common::presented_frames_read_back_exactly(&mut event_loop);
}

#[test]
fn resizing_reports_the_new_size() {
    let (_serial, mut event_loop) = windows();
    common::resizing_reports_the_new_size(&mut event_loop);
}

#[test]
fn timers_and_wake_ups_drive_a_blocking_run() {
    let (_serial, mut event_loop) = windows();
    common::timers_and_wake_ups_drive_a_blocking_run(&mut event_loop);
}

#[test]
fn blocked_waits_wake_through_the_hook() {
    let (_serial, mut event_loop) = windows();
    common::blocked_waits_wake_through_the_hook(&mut event_loop);
}

#[test]
fn text_goes_to_the_clipboard_and_comes_back() {
    let (_serial, mut event_loop) = windows();
    let window = common::open(&mut event_loop, PhysicalSize::new(64, 64));
    let text = "pltrs ✓ clipboard";
    let mut cx = event_loop.context();
    cx.set_clipboard_text(window, text.to_owned()).unwrap();
    cx.request_clipboard_text(window).unwrap();
    let events = common::pump_until(&mut event_loop, window, |e| {
        matches!(e, WindowEvent::ClipboardText(_))
    });
    assert_eq!(
        events.last(),
        Some(&WindowEvent::ClipboardText(Some(text.to_owned())))
    );
}
