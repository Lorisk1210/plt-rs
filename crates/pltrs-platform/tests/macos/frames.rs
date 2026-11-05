//! Frames: what is presented is kept, and views draw it pixel for pixel
//! on the screen's backing store, at its scale factor.

use pltrs_platform::{
    EventLoop, LogicalSize, OwnedFrame, PhysicalRect, WindowEvent, WindowId, WindowOptions,
};

use crate::{appkit, appkit_loop, common};

/// Frames and regions of frames presented read back exactly; frames of
/// another size are clipped.
pub(crate) fn presented_frames_read_back_exactly() {
    let mut event_loop = appkit_loop();
    common::presented_frames_read_back_exactly(&mut event_loop);
}

/// Asserts that `drawn` shows `frame`, naming the first pixel that
/// differs.
fn assert_shows(drawn: &OwnedFrame, frame: &OwnedFrame) {
    assert_eq!(drawn.size(), frame.size());
    let size = frame.size();
    let mut differing = (0..size.height)
        .flat_map(|y| (0..size.width).map(move |x| (x, y)))
        .filter(|&(x, y)| drawn.pixel(x, y) != frame.pixel(x, y));
    if let Some((x, y)) = differing.next() {
        panic!(
            "{} pixels differ, the first at ({x}, {y}): drawn {:?}, presented {:?}",
            1 + differing.count(),
            drawn.pixel(x, y),
            frame.pixel(x, y)
        );
    }
}

/// Opens a window of `size` points and waits for its first redraw.
fn open(event_loop: &mut EventLoop, size: LogicalSize) -> WindowId {
    let options = WindowOptions::new("frames").with_inner_size(size);
    let window = event_loop
        .context()
        .create_window(&options)
        .expect("the window opens");
    common::pump_until(event_loop, window, |e| *e == WindowEvent::RedrawRequested);
    window
}

/// Views draw what was presented exactly: each pixel of the frame is a
/// pixel of the backing store (no scaling, no interpolation, upright),
/// and regions presented replace just their pixels.
pub(crate) fn views_draw_frames_pixel_for_pixel() {
    let mut event_loop = appkit_loop();
    let window = open(&mut event_loop, LogicalSize::new(300.0, 200.0));
    let size = event_loop.context().inner_size(window).expect("a size");
    let frame = common::pattern(size);
    event_loop
        .context()
        .present(window, &frame.as_frame())
        .expect("the frame is presented");
    assert_shows(&appkit::drawn_pixels(&mut event_loop, window), &frame);

    let white = OwnedFrame::filled(size, [255; 4]).expect("a filled frame");
    // An odd region: on Retina screens, half a point.
    let patch = PhysicalRect::new(51, 33, 17, 9);
    event_loop
        .context()
        .present_region(window, &white.as_frame(), patch)
        .expect("the region is presented");
    let drawn = appkit::drawn_pixels(&mut event_loop, window);
    for (x, y) in [(51, 33), (67, 41)] {
        assert_eq!(
            drawn.pixel(x, y),
            Some([255; 4]),
            "({x}, {y}) is in the region"
        );
    }
    for (x, y) in [(50, 33), (68, 41), (51, 32), (67, 42), (0, 0)] {
        assert_eq!(
            drawn.pixel(x, y),
            frame.pixel(x, y),
            "({x}, {y}) is outside"
        );
    }
}

/// After a resize, the view draws frames of the new size.
pub(crate) fn views_draw_frames_after_a_resize() {
    let mut event_loop = appkit_loop();
    let window = open(&mut event_loop, LogicalSize::new(120.0, 90.0));
    event_loop
        .context()
        .set_inner_size(window, LogicalSize::new(210.0, 140.0))
        .expect("the window resizes");
    let events = common::pump_until(&mut event_loop, window, |e| {
        *e == WindowEvent::RedrawRequested
    });
    let size = event_loop.context().inner_size(window).expect("a size");
    assert!(events.contains(&WindowEvent::Resized(size)), "{events:?}");
    let frame = common::pattern(size);
    event_loop
        .context()
        .present(window, &frame.as_frame())
        .expect("the frame is presented");
    assert_shows(&appkit::drawn_pixels(&mut event_loop, window), &frame);
}
