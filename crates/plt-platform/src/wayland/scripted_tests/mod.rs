//! The whole backend against the scripted compositor ([`super::fake`]):
//! what live compositors without a seat cannot show — the keyboard, the
//! pointer over content and decorations, the clipboard — and protocols
//! headless weston lacks (server-side decorations, fractional scaling,
//! cursor shapes, activation).
//!
//! The tests are grouped into [`windows`] (configures, decorations and
//! scale), [`input`] and [`clipboard`].

mod clipboard;
mod input;
mod windows;

use std::time::{Duration, Instant};

use super::WaylandBackend;
use super::fake::Fake;
use crate::backend::Backend;
use crate::event::{Event, WindowEvent};
use crate::frame::OwnedFrame;
use crate::geometry::{LogicalSize, PhysicalRect};
use crate::window::{WindowId, WindowOptions};

/// Polls until `done` accepts the events so far; returns them.
fn poll_until(
    backend: &mut WaylandBackend,
    what: &str,
    mut done: impl FnMut(&[Event]) -> bool,
) -> Vec<Event> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while !done(&events) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; saw {events:?}"
        );
        backend
            .poll(
                Some(Instant::now() + Duration::from_millis(10)),
                &mut events,
            )
            .expect("polling succeeds");
    }
    events
}

/// The events of `window` among `events`.
fn of(events: &[Event], window: WindowId) -> Vec<WindowEvent> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Window { window: w, event } if *w == window => Some(event.clone()),
            _ => None,
        })
        .collect()
}

/// Polls until `window` gets `expected`; returns `window`'s events.
fn expect(
    backend: &mut WaylandBackend,
    window: WindowId,
    expected: &WindowEvent,
) -> Vec<WindowEvent> {
    let events = poll_until(backend, &format!("{expected:?}"), |events| {
        of(events, window).contains(expected)
    });
    of(&events, window)
}

/// The surface id of a window.
fn surface(window: WindowId) -> u32 {
    u32::try_from(window.to_raw()).expect("surface ids are 32-bit")
}

/// Opens a 320 × 200 window and presents a white frame, so that it maps.
fn open(backend: &mut WaylandBackend) -> WindowId {
    let options = WindowOptions::new("scripted").with_inner_size(LogicalSize::new(320.0, 200.0));
    let window = backend.create_window(&options).expect("the window opens");
    expect(backend, window, &WindowEvent::RedrawRequested);
    present(backend, window);
    window
}

/// Presents a white frame of the window's size.
fn present(backend: &mut WaylandBackend, window: WindowId) {
    let size = backend.inner_size(window).expect("the window is live");
    let frame = OwnedFrame::filled(size, [255; 4]).expect("the size is valid");
    backend
        .present(window, &frame.as_frame(), PhysicalRect::from_size(size))
        .expect("the frame is shown");
}

/// Polls the backend until the fake has seen `count` requests `opcode`
/// of `interface`.
fn wait_requests(
    backend: &mut WaylandBackend,
    fake: &Fake,
    interface: &str,
    opcode: u16,
    count: usize,
) {
    poll_until(backend, &format!("{interface}#{opcode} ×{count}"), |_| {
        fake.state().requests(interface, opcode).len() >= count
    });
}
