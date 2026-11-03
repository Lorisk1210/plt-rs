//! What the live tests check against a real windowing system, whichever
//! it is: each function takes an event loop of the backend under test.

use std::thread;
use std::time::{Duration, Instant};

use pltrs_platform::{
    Event, EventLoop, Frame, OwnedFrame, PhysicalRect, PhysicalSize, WindowEvent, WindowId,
    WindowOptions,
};

/// Pumps until `done` accepts an event; returns the events of `window`.
pub(crate) fn pump_until(
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

/// Opens a window of `size` and waits for its first redraw.
pub(crate) fn open(event_loop: &mut EventLoop, size: PhysicalSize) -> WindowId {
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

/// Frames and regions of frames presented read back exactly; frames of
/// another size are clipped.
pub(crate) fn presented_frames_read_back_exactly(event_loop: &mut EventLoop) {
    // 2.8 MB per frame: more than one X11 request without BIG-REQUESTS.
    let size = PhysicalSize::new(1000, 700);
    let window = open(event_loop, size);
    let mut cx = event_loop.context();
    let frame = pattern(size);
    cx.present(window, &frame.as_frame())
        .expect("the frame is presented");
    assert_eq!(
        cx.read_pixels(window)
            .expect("the window's pixels read back"),
        frame
    );

    let white = OwnedFrame::filled(size, [255; 4]).expect("a filled frame");
    let patch = PhysicalRect::new(100, 50, 30, 20);
    cx.present_region(window, &white.as_frame(), patch)
        .expect("the region is presented");
    let shown = cx
        .read_pixels(window)
        .expect("the window's pixels read back");
    assert_eq!(shown.pixel(100, 50), Some([255; 4]));
    assert_eq!(shown.pixel(129, 69), Some([255; 4]));
    assert_eq!(shown.pixel(130, 69), frame.pixel(130, 69));
    assert_eq!(shown.pixel(99, 50), frame.pixel(99, 50));

    // Frames lagging behind a resize are clipped or partial, not errors.
    let large =
        OwnedFrame::filled(PhysicalSize::new(1200, 900), [0, 0, 255, 255]).expect("a filled frame");
    cx.present(window, &large.as_frame())
        .expect("a frame of another size is presented");
    let small =
        OwnedFrame::filled(PhysicalSize::new(10, 10), [0, 255, 0, 255]).expect("a filled frame");
    cx.present(window, &small.as_frame())
        .expect("a frame of another size is presented");
    let shown = cx
        .read_pixels(window)
        .expect("the window's pixels read back");
    assert_eq!(shown.pixel(9, 9), Some([0, 255, 0, 255]));
    assert_eq!(shown.pixel(10, 10), Some([0, 0, 255, 255]));
    let empty = Frame::new(0, 0, &[]).expect("an empty frame");
    cx.present(window, &empty)
        .expect("an empty frame is presented");
}

/// A new size is reported and kept; returns the window.
pub(crate) fn resizing_reports_the_new_size(event_loop: &mut EventLoop) -> WindowId {
    let window = open(event_loop, PhysicalSize::new(200, 100));
    let new_size = PhysicalSize::new(321, 123);
    event_loop
        .context()
        .set_inner_size(window, new_size)
        .expect("the window resizes");
    let events = pump_until(event_loop, window, |e| *e == WindowEvent::RedrawRequested);
    assert!(
        events.contains(&WindowEvent::Resized(new_size)),
        "{events:?}"
    );
    assert_eq!(event_loop.context().inner_size(window), Ok(new_size));
    window
}

/// A blocking run wakes for a timer and for a wake-up from another
/// thread, then ends when its last window is closed.
pub(crate) fn timers_and_wake_ups_drive_a_blocking_run(event_loop: &mut EventLoop) {
    let window = open(event_loop, PhysicalSize::new(64, 64));
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
                cx.close_window(window).expect("the window closes");
            }
            Event::Window {
                event: WindowEvent::Destroyed,
                ..
            } => log.push("destroyed"),
            _ => {}
        })
        .expect("the loop runs");
    assert_eq!(waker.join().expect("the waking thread ends"), Ok(()));
    assert_eq!(log, ["timer", "wake-up", "destroyed"]);
    assert!(started.elapsed() >= Duration::from_millis(60));
    assert_eq!(event_loop.context().windows(), []);
}
