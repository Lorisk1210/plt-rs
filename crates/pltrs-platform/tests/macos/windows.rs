//! Windows: their scale factor on the screen, titles, sizes, placement,
//! closing, the Dock icon and full screen.

use std::time::{Duration, Instant};

use pltrs_platform::{
    CursorIcon, Event, EventLoop, Icon, IconImage, IconLayout, LogicalSize, PhysicalSize,
    WindowEvent, WindowId, WindowOptions,
};

use crate::appkit::{self, NsSize};
use crate::{appkit_loop, common, settle};

/// Opens a window with `options` and waits for its first redraw.
fn open_with(event_loop: &mut EventLoop, options: &WindowOptions) -> WindowId {
    let window = event_loop
        .context()
        .create_window(options)
        .expect("the window opens");
    common::pump_until(event_loop, window, |e| *e == WindowEvent::RedrawRequested);
    window
}

/// Windows open visible, at their screen's scale factor (2 on Retina
/// displays): their sizes are the view's points times that factor.
pub(crate) fn open_at_the_screens_scale() {
    let mut event_loop = appkit_loop();
    let options = WindowOptions::new("scale").with_inner_size(LogicalSize::new(320.0, 200.0));
    let window = open_with(&mut event_loop, &options);
    assert!(appkit::is_visible(&mut event_loop, window));
    let scale = appkit::backing_scale(&mut event_loop, window);
    assert_eq!(
        scale,
        appkit::main_screen_scale(),
        "the window is on the main screen"
    );
    assert_eq!(event_loop.context().scale_factor(window), Ok(scale));
    assert_eq!(
        appkit::view_size(&mut event_loop, window),
        NsSize {
            width: 320.0,
            height: 200.0
        }
    );
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(LogicalSize::new(320.0, 200.0).to_physical(scale))
    );

    // Physical sizes are pixels of the main screen.
    let options = WindowOptions::new("pixels").with_inner_size(PhysicalSize::new(300, 200));
    let pixels = open_with(&mut event_loop, &options);
    assert_eq!(
        event_loop.context().inner_size(pixels),
        Ok(PhysicalSize::new(300, 200))
    );
    assert_eq!(
        appkit::view_size(&mut event_loop, pixels),
        NsSize {
            width: 300.0 / scale,
            height: 200.0 / scale
        }
    );
}

/// Titles show as given, and as changed, in any script.
pub(crate) fn titles_show_as_set() {
    let mut event_loop = appkit_loop();
    let window = open_with(&mut event_loop, &WindowOptions::new("Figure 1"));
    assert_eq!(appkit::title(&mut event_loop, window), "Figure 1");
    for title in ["Ünïcödé ✓ — 图 1", "", "x".repeat(300).as_str()] {
        event_loop
            .context()
            .set_title(window, title)
            .expect("the title changes");
        assert_eq!(appkit::title(&mut event_loop, window), title);
    }
}

/// Windows take the minimum size asked for.
pub(crate) fn sizes_and_minimum_sizes() {
    let mut event_loop = appkit_loop();
    let options = WindowOptions::new("minimum")
        .with_inner_size(LogicalSize::new(400.0, 300.0))
        .with_min_inner_size(LogicalSize::new(120.0, 80.0));
    let window = open_with(&mut event_loop, &options);
    assert_eq!(
        appkit::content_min_size(&mut event_loop, window),
        NsSize {
            width: 120.0,
            height: 80.0
        }
    );
    // Logical sizes are points, physical ones pixels.
    let scale = appkit::backing_scale(&mut event_loop, window);
    event_loop
        .context()
        .set_inner_size(window, LogicalSize::new(250.0, 150.0))
        .expect("the window resizes");
    common::pump_until(&mut event_loop, window, |e| {
        matches!(e, WindowEvent::Resized(_))
    });
    assert_eq!(
        event_loop.context().inner_size(window),
        Ok(LogicalSize::new(250.0, 150.0).to_physical(scale))
    );
}

/// A new size is reported, kept and shown. `AppKit` sizes windows in
/// whole points: sizes of half points on Retina screens round to them.
pub(crate) fn resizing_reports_the_new_size() {
    let mut event_loop = appkit_loop();
    let window = common::resizing_reports_the_new_size(&mut event_loop);
    let scale = appkit::backing_scale(&mut event_loop, window);
    assert_eq!(event_loop.context().scale_factor(window), Ok(scale));
    let asked = PhysicalSize::new(401, 301);
    event_loop
        .context()
        .set_inner_size(window, asked)
        .expect("the window resizes");
    common::pump_until(&mut event_loop, window, |e| {
        *e == WindowEvent::RedrawRequested
    });
    let points = appkit::view_size(&mut event_loop, window);
    assert_eq!((points.width.fract(), points.height.fract()), (0.0, 0.0));
    let size = LogicalSize::new(points.width, points.height).to_physical(scale);
    assert_eq!(event_loop.context().inner_size(window), Ok(size));
    let off = |shown: u32, asked: u32| f64::from(shown.abs_diff(asked)) / scale;
    assert!(off(size.width, asked.width) < 1.0, "{size:?}");
    assert!(off(size.height, asked.height) < 1.0, "{size:?}");
}

/// The first window is centred, the next ones cascade from it.
pub(crate) fn new_windows_cascade() {
    let mut event_loop = appkit_loop();
    let options = WindowOptions::new("cascade").with_inner_size(LogicalSize::new(200.0, 120.0));
    let first = open_with(&mut event_loop, &options);
    let second = open_with(&mut event_loop, &options);
    let (a, b) = (
        appkit::frame(&mut event_loop, first),
        appkit::frame(&mut event_loop, second),
    );
    assert_eq!(a.size, b.size);
    assert!(b.origin.x > a.origin.x, "{a:?} then {b:?}");
    assert!(b.origin.y < a.origin.y, "{a:?} then {b:?}");
    assert!(appkit::is_visible(&mut event_loop, second));
}

/// Closes `window` and checks that it is destroyed, and released: the
/// application no longer has it once its closing animation is over.
fn close_and_release(event_loop: &mut EventLoop, window: WindowId) {
    let number = appkit::window_number(event_loop, window);
    assert!(appkit::has_window_numbered(number));
    event_loop
        .context()
        .close_window(window)
        .expect("the window closes");
    let mut destroyed = false;
    event_loop
        .pump_events(Some(Duration::ZERO), |_, event| {
            destroyed |= event
                == Event::Window {
                    window,
                    event: WindowEvent::Destroyed,
                };
        })
        .expect("pumping events succeeds");
    assert!(destroyed);
    let deadline = Instant::now() + Duration::from_secs(2);
    while appkit::has_window_numbered(number) && Instant::now() < deadline {
        settle(event_loop, Duration::from_millis(50));
    }
    assert!(
        !appkit::has_window_numbered(number),
        "the window is released"
    );
    assert!(event_loop.context().set_title(window, "gone").is_err());
}

/// Closing a window takes it off the screen and releases it, whatever
/// was done with it, and its id is unknown afterwards.
pub(crate) fn closing_removes_the_window() {
    let mut event_loop = appkit_loop();
    let window = common::open(&mut event_loop, PhysicalSize::new(64, 64));
    let mut cx = event_loop.context();
    cx.set_inner_size(window, PhysicalSize::new(128, 96))
        .expect("the window resizes");
    cx.set_title(window, "used").expect("the title changes");
    cx.set_cursor(window, CursorIcon::Crosshair)
        .expect("the cursor changes");
    cx.raise_window(window).expect("the window comes up");
    cx.set_fullscreen(window, false)
        .expect("the window stays windowed");
    let frame = common::pattern(PhysicalSize::new(128, 96));
    cx.present(window, &frame.as_frame())
        .expect("the frame is presented");
    common::pump_until(&mut event_loop, window, |e| {
        matches!(e, WindowEvent::Resized(_))
    });
    close_and_release(&mut event_loop, window);
    assert_eq!(event_loop.context().windows(), []);
}

/// A window's icon becomes the application's Dock icon.
pub(crate) fn icons_go_to_the_dock() {
    let mut event_loop = appkit_loop();
    let (width, height) = (40, 24);
    let rgba = (0..width * height)
        .flat_map(|i| [(i * 7) as u8, 128, 255 - (i as u8), 255])
        .collect();
    let icon = Icon::from_rgba(width, height, rgba).expect("a valid icon");
    let options = WindowOptions::new("icon").with_icon(icon);
    open_with(&mut event_loop, &options);
    assert_eq!(
        appkit::app_icon_size(),
        NsSize {
            width: f64::from(width),
            height: f64::from(height)
        }
    );
}

/// A rendered icon reaches the Dock drawn on Apple's grid at every icon
/// size from 16 to 512 points, at 1× and 2×, and is drawn once however
/// many windows show it.
pub(crate) fn rendered_icons_go_to_the_dock_in_every_size() {
    let mut event_loop = appkit_loop();
    let drawn = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&drawn);
    let icon = Icon::from_renderer(move |side, layout| {
        assert_eq!(layout, IconLayout::AppleGrid);
        counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        IconImage::from_rgba(side, side, vec![200; (side * side * 4) as usize]).ok()
    });
    let options = WindowOptions::new("icon").with_icon(icon);
    open_with(&mut event_loop, &options);
    open_with(&mut event_loop, &options);
    assert_eq!(drawn.load(std::sync::atomic::Ordering::Relaxed), 7);
    assert_eq!(
        appkit::app_icon_size(),
        NsSize {
            width: 512.0,
            height: 512.0
        }
    );
    let mut representations: Vec<(u32, i64)> = appkit::app_icon_representations()
        .iter()
        .map(|&(points, pixels)| (points.width as u32, pixels))
        .collect();
    representations.sort_unstable();
    let expected: Vec<(u32, i64)> = [16, 32, 64, 128, 256, 512]
        .into_iter()
        .flat_map(|points| [(points, i64::from(points)), (points, 2 * i64::from(points))])
        .collect();
    assert_eq!(representations, expected);
}

/// Pumps until `window` reports a size `accept`s, for up to 10 seconds
/// (full-screen transitions animate); returns that size.
fn resized_to(
    event_loop: &mut EventLoop,
    window: WindowId,
    accept: impl Fn(PhysicalSize) -> bool,
) -> PhysicalSize {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut sizes = Vec::new();
    while Instant::now() < deadline {
        event_loop
            .pump_events(Some(Duration::from_millis(20)), |_, event| {
                if let Event::Window {
                    window: w,
                    event: WindowEvent::Resized(size),
                } = event
                    && w == window
                {
                    sizes.push(size);
                }
            })
            .expect("pumping events succeeds");
        if let Some(&size) = sizes.iter().rev().find(|&&size| accept(size)) {
            return size;
        }
    }
    panic!("no size accepted; saw {sizes:?}");
}

/// Full screen makes the window fill its screen; leaving it restores the
/// size the window had.
pub(crate) fn full_screen_and_back() {
    let mut event_loop = appkit_loop();
    let options = WindowOptions::new("full screen").with_inner_size(LogicalSize::new(300.0, 200.0));
    let window = open_with(&mut event_loop, &options);
    let windowed = event_loop.context().inner_size(window).expect("a size");
    event_loop
        .context()
        .set_fullscreen(window, true)
        .expect("the window goes full screen");
    let full = resized_to(&mut event_loop, window, |size| {
        size.width > windowed.width * 2 && size.height > windowed.height * 2
    });
    assert_eq!(event_loop.context().inner_size(window), Ok(full));
    // The transition ends after the last resize; asking twice changes
    // nothing.
    settle(&mut event_loop, Duration::from_secs(1));
    event_loop
        .context()
        .set_fullscreen(window, true)
        .expect("the window stays full screen");
    event_loop
        .context()
        .set_fullscreen(window, false)
        .expect("the window leaves full screen");
    resized_to(&mut event_loop, window, |size| size == windowed);
    settle(&mut event_loop, Duration::from_secs(1));
    close_and_release(&mut event_loop, window);
}
