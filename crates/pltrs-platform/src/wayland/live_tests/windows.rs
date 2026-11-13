//! Live tests through the public API: opening, presenting, resizing,
//! full screen, closing, and the event loop's wake-ups and timers.

use std::time::Duration;

use super::{is, live, open_window, pattern, pump_until, wayland_loop};
use crate::error::Error;
use crate::event::{Event, WindowEvent};
use crate::event_loop::wake_tests;
use crate::geometry::{LogicalSize, PhysicalRect, PhysicalSize};
use crate::window::{CursorIcon, WindowOptions};

#[test]
fn windows_open_at_their_size_and_show_frames() {
    let Some(_live) = live() else { return };
    let mut event_loop = wayland_loop();
    let options = WindowOptions::new("Figure 1 – α")
        .with_app_id("pltrs-live")
        .with_inner_size(LogicalSize::new(320.0, 200.0))
        .with_min_inner_size(LogicalSize::new(100.0, 80.0));
    let window = open_window(&mut event_loop, &options);
    let mut cx = event_loop.context();
    let scale = cx.scale_factor(window).unwrap();
    assert!(scale >= 1.0);
    let size = cx.inner_size(window).unwrap();
    assert_eq!(size, LogicalSize::new(320.0, 200.0).to_physical(scale));
    let frame = pattern(size, 0);
    cx.present(window, &frame.as_frame()).unwrap();
    assert_eq!(cx.read_pixels(window).unwrap(), frame);

    let dot = PhysicalRect::new(10, 20, 30, 5);
    let other = pattern(size, 7);
    cx.present_region(window, &other.as_frame(), dot).unwrap();
    let shown = cx.read_pixels(window).unwrap();
    assert_eq!(shown.pixel(10, 20), other.pixel(10, 20));
    assert_eq!(shown.pixel(9, 20), frame.pixel(9, 20), "outside the region");
    cx.set_title(window, "Renamed").unwrap();
    for cursor in [CursorIcon::Crosshair, CursorIcon::Default] {
        cx.set_cursor(window, cursor).unwrap();
    }
    // A frame larger than the window is clipped, a smaller one leaves the
    // rest alone.
    let large = pattern(PhysicalSize::new(size.width + 50, size.height + 50), 3);
    cx.present(window, &large.as_frame()).unwrap();
    assert_eq!(cx.read_pixels(window).unwrap().size(), size);
}

#[test]
fn resizing_and_full_screen_report_new_sizes() {
    let Some(_live) = live() else { return };
    let mut event_loop = wayland_loop();
    let window = open_window(&mut event_loop, &WindowOptions::new("resize"));
    let scale = event_loop.context().scale_factor(window).unwrap();
    let wanted = LogicalSize::new(400.0, 300.0).to_physical(scale);
    event_loop.context().set_inner_size(window, wanted).unwrap();
    pump_until(&mut event_loop, "the new size", |e| {
        is(e, window, &WindowEvent::Resized(wanted))
    });
    let frame = pattern(wanted, 1);
    event_loop
        .context()
        .present(window, &frame.as_frame())
        .unwrap();

    event_loop.context().set_fullscreen(window, true).unwrap();
    let events = pump_until(
        &mut event_loop,
        "the full-screen size",
        |e| matches!(e, Event::Window { window: w, event: WindowEvent::Resized(size) } if *w == window && *size != wanted),
    );
    let full = events
        .iter()
        .rev()
        .find_map(|e| match e {
            Event::Window {
                event: WindowEvent::Resized(size),
                ..
            } => Some(*size),
            _ => None,
        })
        .unwrap();
    assert!(
        full.width > wanted.width && full.height > wanted.height,
        "{full:?}"
    );
    event_loop
        .context()
        .present(window, &pattern(full, 2).as_frame())
        .unwrap();
    event_loop.context().set_fullscreen(window, false).unwrap();
    pump_until(&mut event_loop, "the size before", |e| {
        is(e, window, &WindowEvent::Resized(wanted))
    });
    assert_eq!(
        event_loop
            .context()
            .set_inner_size(window, PhysicalSize::new(0, 5)),
        Err(Error::InvalidArgument("window size must be non-zero"))
    );
}

#[test]
fn closing_destroys_the_window() {
    let Some(_live) = live() else { return };
    let mut event_loop = wayland_loop();
    let window = open_window(&mut event_loop, &WindowOptions::new("closing"));
    let mut cx = event_loop.context();
    cx.close_window(window).unwrap();
    assert_eq!(
        cx.set_title(window, "gone"),
        Err(Error::UnknownWindow(window))
    );
    assert_eq!(cx.windows(), []);
    let events = pump_until(&mut event_loop, "destruction", |e| {
        is(e, window, &WindowEvent::Destroyed)
    });
    assert_eq!(
        events
            .iter()
            .rev()
            .find(|e| matches!(e, Event::Window { .. })),
        Some(&Event::Window {
            window,
            event: WindowEvent::Destroyed
        })
    );
    let second = open_window(&mut event_loop, &WindowOptions::new("again"));
    assert_eq!(event_loop.context().windows(), [second]);
}

#[test]
fn wake_ups_and_timers_interrupt_the_wait() {
    let Some(_live) = live() else { return };
    let mut event_loop = wayland_loop();
    let proxy = event_loop.proxy();
    let waker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        proxy.wake_up().unwrap();
    });
    pump_until(&mut event_loop, "the wake-up", |e| *e == Event::WakeUp);
    waker.join().unwrap();
    let timer = event_loop.context().set_timeout(Duration::from_millis(30));
    pump_until(&mut event_loop, "the timer", |e| *e == Event::Timer(timer));
    let proxy = event_loop.proxy();
    drop(event_loop);
    assert_eq!(proxy.wake_up(), Err(Error::EventLoopClosed));
}

#[test]
fn a_blocked_wait_is_woken_by_a_proxy_and_by_a_wake_pipe() {
    let Some(_live) = live() else { return };
    let mut event_loop = wayland_loop();
    let proxy = event_loop.proxy();
    wake_tests::assert_woken_while_blocked(&mut event_loop, move || proxy.wake_up().unwrap());
    wake_tests::assert_woken_by_a_wake_pipe(&mut event_loop);
}
