//! Tests against a real compositor, run when `$WAYLAND_DISPLAY` is set
//! (and passing trivially otherwise), e.g. under a private headless
//! weston (`crates/pltrs-platform/tools/weston.sh`):
//!
//! ```text
//! crates/pltrs-platform/tools/weston.sh install
//! crates/pltrs-platform/tools/weston.sh run &
//! WAYLAND_DISPLAY=wayland-pltrs cargo test -p pltrs-platform wayland
//! ```
//!
//! Headless weston has no seat, so input and the clipboard are left to
//! the tests against a scripted compositor ([`super::fake`]); these cover
//! windows, frames, configures, decorations, scale and the event loop.
//!
//! The harness is here; the tests are grouped into [`windows`] (through
//! the public API) and [`backend`] (the backend's own state).

mod backend;
mod windows;

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::backend::BackendKind;
use crate::event::{Event, WindowEvent};
use crate::event_loop::EventLoop;
use crate::frame::OwnedFrame;
use crate::geometry::PhysicalSize;
use crate::window::{WindowId, WindowOptions};

/// Windows of concurrent tests would take each other's focus and
/// configures; the tests run one at a time.
static COMPOSITOR: Mutex<()> = Mutex::new(());

/// Exclusive use of the compositor named by `$WAYLAND_DISPLAY`, if set.
fn live() -> Option<MutexGuard<'static, ()>> {
    std::env::var_os("WAYLAND_DISPLAY").filter(|name| !name.is_empty())?;
    Some(COMPOSITOR.lock().unwrap_or_else(PoisonError::into_inner))
}

fn wayland_loop() -> EventLoop {
    EventLoop::with_backend(BackendKind::Wayland).expect("connects to $WAYLAND_DISPLAY")
}

/// Pumps the loop until `done` accepts an event; returns every event seen.
fn pump_until(
    event_loop: &mut EventLoop,
    what: &str,
    mut done: impl FnMut(&Event) -> bool,
) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        let mut found = false;
        event_loop
            .pump_events(Some(Duration::from_millis(20)), |_, event| {
                found |= done(&event);
                seen.push(event);
            })
            .expect("pumping events succeeds");
        if found {
            return seen;
        }
    }
    panic!("timed out waiting for {what}; saw {seen:?}");
}

/// Whether `event` is `expected` for `window`.
fn is(event: &Event, window: WindowId, expected: &WindowEvent) -> bool {
    matches!(event, Event::Window { window: w, event } if *w == window && event == expected)
}

/// Opens a window and waits for its first redraw.
fn open_window(event_loop: &mut EventLoop, options: &WindowOptions) -> WindowId {
    let window = event_loop
        .context()
        .create_window(options)
        .expect("window is created");
    pump_until(event_loop, "the first redraw", |e| {
        is(e, window, &WindowEvent::RedrawRequested)
    });
    window
}

/// A frame whose every pixel differs from its neighbours.
fn pattern(size: PhysicalSize, seed: u32) -> OwnedFrame {
    let pixels = (0..size.area() as u32)
        .flat_map(|i| {
            let i = i.wrapping_add(seed);
            [(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255]
        })
        .collect();
    OwnedFrame::new(size.width, size.height, pixels).expect("the size matches")
}
