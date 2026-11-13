//! What the system provides besides windows: timers and wake-ups, the
//! clipboard and the cursors.

use std::ffi::CStr;

use pltrs_codec::png::{Encoder, ImageRef};
use pltrs_platform::{CursorIcon, PhysicalSize, WindowEvent};

use crate::appkit::{self, SavedPasteboard};
use crate::{appkit_loop, common, input, open_still};

/// The pasteboard type of plain text.
const TEXT: &str = "public.utf8-plain-text";
/// The pasteboard type of PNG images.
const PNG: &str = "public.png";

/// A blocking run wakes for a timer and for a wake-up from another
/// thread, then ends when its last window is closed.
pub(crate) fn timers_and_wake_ups_drive_a_blocking_run() {
    let mut event_loop = appkit_loop();
    common::timers_and_wake_ups_drive_a_blocking_run(&mut event_loop);
}

/// A loop blocked in `nextEventMatchingMask:` (its wait through the
/// embedder's hook, without dequeuing) wakes for a proxy and for a write
/// to a wake pipe, from other threads.
pub(crate) fn blocked_waits_wake_through_the_hook() {
    let mut event_loop = appkit_loop();
    common::blocked_waits_wake_through_the_hook(&mut event_loop);
}

/// Text put on the clipboard is there for every application, and comes
/// back as it was.
pub(crate) fn text_goes_to_the_clipboard_and_comes_back() {
    let _saved = SavedPasteboard::save();
    let mut event_loop = appkit_loop();
    let window = common::open(&mut event_loop, PhysicalSize::new(64, 64));
    let text = "pltrs ✓ clipboard\nsecond line";
    let mut cx = event_loop.context();
    cx.set_clipboard_text(window, text.to_owned())
        .expect("the text goes to the clipboard");
    assert_eq!(
        appkit::pasteboard_data(TEXT),
        Some(text.as_bytes().to_vec())
    );
    cx.request_clipboard_text(window)
        .expect("the clipboard is read");
    let events = common::pump_until(&mut event_loop, window, |e| {
        matches!(e, WindowEvent::ClipboardText(_))
    });
    assert_eq!(
        events.last(),
        Some(&WindowEvent::ClipboardText(Some(text.to_owned())))
    );
}

/// Images go to the clipboard as PNG, which holds no text then.
pub(crate) fn images_go_to_the_clipboard_as_png() {
    let _saved = SavedPasteboard::save();
    let mut event_loop = appkit_loop();
    let window = common::open(&mut event_loop, PhysicalSize::new(64, 64));
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 128,
    ];
    let image = ImageRef::rgba8(2, 2, &pixels).expect("a 2 × 2 image");
    let png = Encoder::new().encode(image).expect("the image encodes");
    let mut cx = event_loop.context();
    cx.set_clipboard_image(window, png.clone())
        .expect("the image goes to the clipboard");
    assert_eq!(appkit::pasteboard_data(PNG), Some(png));
    cx.request_clipboard_text(window)
        .expect("the clipboard is read");
    let events = common::pump_until(&mut event_loop, window, |e| {
        matches!(e, WindowEvent::ClipboardText(_))
    });
    assert_eq!(events.last(), Some(&WindowEvent::ClipboardText(None)));
}

/// The `NSCursor` each cursor shows: macOS has no busy cursor for
/// applications, so waiting shows the arrow, as in matplotlib.
const CURSORS: [(CursorIcon, &CStr); 8] = [
    (CursorIcon::Default, c"arrowCursor"),
    (CursorIcon::Pointer, c"pointingHandCursor"),
    (CursorIcon::Crosshair, c"crosshairCursor"),
    (CursorIcon::Move, c"openHandCursor"),
    (CursorIcon::Wait, c"arrowCursor"),
    (CursorIcon::Text, c"IBeamCursor"),
    (CursorIcon::ResizeHorizontal, c"resizeLeftRightCursor"),
    (CursorIcon::ResizeVertical, c"resizeUpDownCursor"),
];

/// Each cursor shows over the window at once.
pub(crate) fn cursors_show_over_the_window() {
    let mut event_loop = appkit_loop();
    let window = open_still(&mut event_loop, PhysicalSize::new(200, 200));
    input::enter(&mut event_loop, window, (10.0, 10.0));
    for (icon, name) in CURSORS {
        event_loop
            .context()
            .set_cursor(window, icon)
            .expect("the cursor changes");
        assert!(appkit::cursor_shown_is(name), "{icon:?} shows {name:?}");
    }
    appkit::post_crossing(&mut event_loop, window, false, (10.0, 10.0));
}
