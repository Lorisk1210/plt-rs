//! The public API against a real X server. Runs when `$DISPLAY` is set
//! and passes trivially otherwise; a private Xvfb works well:
//!
//! ```text
//! Xvfb :117 -screen 0 1280x800x24 -nolisten tcp &
//! DISPLAY=:117 cargo test -p pltrs-platform --test x11
//! ```

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use pltrs_platform::{
    BackendKind, Error, Event, EventLoop, Frame, OwnedFrame, PhysicalRect, PhysicalSize,
    WindowEvent, WindowId, WindowOptions,
};

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

/// Pumps until `done` accepts an event; returns the events of `window`.
fn pump_until(
    event_loop: &mut EventLoop,
    window: WindowId,
    mut done: impl FnMut(&WindowEvent) -> bool,
) -> Vec<WindowEvent> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut seen = Vec::new();
    while Instant::now() < deadline {
        let mut found = false;
        let status = event_loop.pump_events(Some(Duration::from_millis(20)), |_, event| {
            if let Event::Window { window: w, event } = event
                && w == window
            {
                found |= done(&event);
                seen.push(event);
            }
        });
        status.expect("pumping events succeeds");
        if found {
            return seen;
        }
    }
    panic!("timed out; saw {seen:?}");
}

fn open(event_loop: &mut EventLoop, size: PhysicalSize) -> WindowId {
    let options = WindowOptions::new("pltrs-platform test").with_inner_size(size);
    let window = event_loop
        .context()
        .create_window(&options)
        .expect("window opens");
    pump_until(event_loop, window, |e| *e == WindowEvent::RedrawRequested);
    window
}

/// A frame whose every pixel differs from its neighbours.
fn pattern(size: PhysicalSize) -> OwnedFrame {
    let pixels = (0..size.height)
        .flat_map(|y| (0..size.width).map(move |x| (x, y)))
        .flat_map(|(x, y)| [x as u8, y as u8, (x ^ y).wrapping_mul(7) as u8, 255])
        .collect();
    OwnedFrame::new(size.width, size.height, pixels).expect("pixels match the size")
}

#[test]
fn presented_frames_read_back_exactly() {
    let Some((_serial, mut event_loop)) = x11() else {
        return;
    };
    // 2.8 MB per frame: more than one request without BIG-REQUESTS.
    let size = PhysicalSize::new(1000, 700);
    let window = open(&mut event_loop, size);
    let mut cx = event_loop.context();
    let frame = pattern(size);
    cx.present(window, &frame.as_frame()).unwrap();
    assert_eq!(cx.read_pixels(window).unwrap(), frame);

    let white = OwnedFrame::filled(size, [255; 4]).unwrap();
    let patch = PhysicalRect::new(100, 50, 30, 20);
    cx.present_region(window, &white.as_frame(), patch).unwrap();
    let shown = cx.read_pixels(window).unwrap();
    assert_eq!(shown.pixel(100, 50), Some([255; 4]));
    assert_eq!(shown.pixel(129, 69), Some([255; 4]));
    assert_eq!(shown.pixel(130, 69), frame.pixel(130, 69));
    assert_eq!(shown.pixel(99, 50), frame.pixel(99, 50));

    // Frames lagging behind a resize are clipped or partial, not errors.
    let large = OwnedFrame::filled(PhysicalSize::new(1200, 900), [0, 0, 255, 255]).unwrap();
    cx.present(window, &large.as_frame()).unwrap();
    let small = OwnedFrame::filled(PhysicalSize::new(10, 10), [0, 255, 0, 255]).unwrap();
    cx.present(window, &small.as_frame()).unwrap();
    let shown = cx.read_pixels(window).unwrap();
    assert_eq!(shown.pixel(9, 9), Some([0, 255, 0, 255]));
    assert_eq!(shown.pixel(10, 10), Some([0, 0, 255, 255]));
    let empty = Frame::new(0, 0, &[]).unwrap();
    cx.present(window, &empty).unwrap();
}

#[test]
fn resizing_reports_the_new_size() {
    let Some((_serial, mut event_loop)) = x11() else {
        return;
    };
    let window = open(&mut event_loop, PhysicalSize::new(200, 100));
    let new_size = PhysicalSize::new(321, 123);
    event_loop
        .context()
        .set_inner_size(window, new_size)
        .unwrap();
    let events = pump_until(&mut event_loop, window, |e| {
        *e == WindowEvent::RedrawRequested
    });
    assert!(
        events.contains(&WindowEvent::Resized(new_size)),
        "{events:?}"
    );
    assert_eq!(event_loop.context().inner_size(window), Ok(new_size));
    assert_eq!(event_loop.context().scale_factor(window), Ok(1.0));
}

#[test]
fn timers_and_wake_ups_drive_a_blocking_run() {
    let Some((_serial, mut event_loop)) = x11() else {
        return;
    };
    let window = open(&mut event_loop, PhysicalSize::new(64, 64));
    let proxy = event_loop.proxy();
    let started = Instant::now();
    let waker = thread::spawn(move || {
        thread::sleep(Duration::from_millis(60));
        proxy.wake_up()
    });
    let timer = event_loop.context().set_timeout(Duration::from_millis(20));
    let mut log = Vec::new();
    event_loop
        .run(|cx, event| match event {
            Event::Timer(id) => {
                assert_eq!(id, timer);
                log.push("timer");
            }
            Event::WakeUp => {
                log.push("wake-up");
                cx.close_window(window).unwrap();
            }
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => log.push("destroyed"),
            _ => {}
        })
        .unwrap();
    assert_eq!(waker.join().unwrap(), Ok(()));
    assert_eq!(log, ["timer", "wake-up", "destroyed"]);
    assert!(started.elapsed() >= Duration::from_millis(60));
    assert_eq!(event_loop.context().windows(), []);
}
