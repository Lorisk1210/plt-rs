//! Live tests of the backend's own state: frame callbacks, buffer reuse,
//! decorations and scale.

use std::time::{Duration, Instant};

use super::super::WaylandBackend;
use super::super::protocol::Interface;
use super::{live, pattern};
use crate::backend::Backend;
use crate::blocking::Blocking;
use crate::decoration::{DecorationPainter, TitleBar};
use crate::event::{Event, WindowEvent};
use crate::geometry::{PhysicalRect, PhysicalSize};
use crate::window::{WindowId, WindowOptions};

/// Polls `backend` until `done` holds (failing after five seconds).
fn poll_until(
    backend: &mut WaylandBackend,
    what: &str,
    mut done: impl FnMut(&WaylandBackend, &[Event]) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut events = Vec::new();
    while !done(backend, &events) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {what}; saw {events:?}"
        );
        backend
            .poll(
                Some(Instant::now() + Duration::from_millis(20)),
                &mut events,
                Blocking::DIRECT,
            )
            .unwrap();
    }
}

fn surface(window: WindowId) -> u32 {
    u32::try_from(window.to_raw()).unwrap()
}

#[test]
fn frames_wait_for_the_frame_callback_and_reuse_buffers() {
    let Some(_live) = live() else { return };
    let mut backend = WaylandBackend::connect().unwrap();
    let size = PhysicalSize::new(160, 120);
    let window = backend
        .create_window(&WindowOptions::new("frames").with_inner_size(size))
        .unwrap();
    poll_until(&mut backend, "the first redraw", |_, events| {
        events.contains(&Event::Window {
            window,
            event: WindowEvent::RedrawRequested,
        })
    });
    let size = backend.inner_size(window).unwrap();
    let full = PhysicalRect::from_size(size);
    for seed in 0..6 {
        let frame = pattern(size, seed);
        backend.present(window, &frame.as_frame(), full).unwrap();
        // Presents while the previous frame is on its way are gathered and
        // shown with the next frame callback.
        backend
            .present(window, &frame.as_frame(), PhysicalRect::new(0, 0, 8, 8))
            .unwrap();
        poll_until(&mut backend, "the frame callback", |backend, _| {
            backend.windows[&surface(window)].frame_pending().is_none()
        });
        assert_eq!(backend.read_pixels(window).unwrap(), frame);
    }
    assert!(
        backend.windows[&surface(window)].buffer_count() <= 3,
        "buffers are reused"
    );
    backend.destroy_window(window).unwrap();
    backend
        .poll(Some(Instant::now()), &mut Vec::new(), Blocking::DIRECT)
        .unwrap();
}

/// A painter that fills the title bar with one colour.
struct Fill([u8; 4]);

impl DecorationPainter for Fill {
    fn paint(&self, _bar: &TitleBar<'_>, pixels: &mut [u8]) {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            *pixel = self.0;
        }
    }
}

#[test]
fn decorations_follow_the_compositor_and_the_painter() {
    let Some(_live) = live() else { return };
    let mut backend = WaylandBackend::connect().unwrap();
    let window = backend
        .create_window(
            &WindowOptions::new("decorated").with_inner_size(PhysicalSize::new(200, 100)),
        )
        .unwrap();
    let server_side = backend.globals.get(Interface::DecorationManager).is_some();
    let state = &backend.windows[&surface(window)];
    if !server_side {
        assert!(
            state.decorations.is_some(),
            "client-side without the protocol"
        );
    }
    let size = backend.inner_size(window).unwrap();
    let frame = pattern(size, 4);
    backend
        .present(window, &frame.as_frame(), PhysicalRect::from_size(size))
        .unwrap();
    backend
        .set_decoration_painter(Box::new(Fill([0, 0, 255, 255])))
        .unwrap();
    poll_until(&mut backend, "the frame callback", |backend, _| {
        backend.windows[&surface(window)].frame_pending().is_none()
    });
    assert_eq!(
        backend.read_pixels(window).unwrap(),
        frame,
        "the content stays"
    );
}

#[test]
fn the_scale_follows_the_outputs() {
    let Some(_live) = live() else { return };
    let mut backend = WaylandBackend::connect().unwrap();
    let window = backend
        .create_window(&WindowOptions::new("scale").with_inner_size(PhysicalSize::new(64, 64)))
        .unwrap();
    let largest = backend.largest_output_scale();
    let scale = backend.scale_factor(window).unwrap();
    if backend.fractional_scaling() {
        assert!(scale > 0.0);
    } else {
        assert!(
            (scale - f64::from(largest)).abs() < f64::EPSILON,
            "{scale} vs {largest}"
        );
    }
    let size = backend.inner_size(window).unwrap();
    assert_eq!(
        size.width % scale as u32,
        0,
        "integer scales divide the buffer"
    );
}
