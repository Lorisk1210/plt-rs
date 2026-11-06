//! What the tests ask of `AppKit` and Core Graphics directly, behind
//! safe functions: input posted to the application's event queue as the
//! window server would deliver it; what windows hold (titles, scale
//! factors, frames, minimum sizes, the Dock icon); the pixels views
//! draw; the cursor shown; and the user's pasteboard, put back after the
//! tests borrowed it.
//!
//! The macOS backend's window ids are the addresses of the windows'
//! views. A view is only messaged once its window was found open in the
//! event loop, which keeps the view alive meanwhile.
//!
//! * [`events`]: the input posted;
//! * [`pasteboard`]: the pasteboard;
//! * here: the windows, what views draw and the cursor.

use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;

use pltrs_platform::{EventLoop, OwnedFrame, WindowId};

#[allow(
    dead_code,
    reason = "the tests need only part of the backend's runtime bindings"
)]
#[path = "../../src/macos/sys/runtime.rs"]
mod runtime;

mod events;
mod pasteboard;

pub(crate) use events::{
    Action, KeyAction, Keystroke, Mouse, Scroll, post_crossing, post_key, post_mouse, post_scroll,
};
pub(crate) use pasteboard::{SavedPasteboard, pasteboard_data};
use runtime::{NO, Object, YES, autoreleasepool, class, send};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// `kCGColorSpaceSRGB`.
    static kCGColorSpaceSRGB: *const c_void;
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGColorSpaceCreateWithName(name: *const c_void) -> *mut c_void;
    fn CGColorSpaceRelease(space: *mut c_void);
    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: usize,
        height: usize,
        bits_per_component: usize,
        bytes_per_row: usize,
        space: *mut c_void,
        bitmap_info: u32,
    ) -> *mut c_void;
    fn CGBitmapContextGetData(context: *mut c_void) -> *mut c_void;
    fn CGContextScaleCTM(context: *mut c_void, x: f64, y: f64);
    fn CGContextRelease(context: *mut c_void);
}

/// `kCGImageAlphaPremultipliedLast | kCGBitmapByteOrder32Big`: pixels
/// stored red, green, blue, alpha.
const RGBA: u32 = 1 | (4 << 12);

/// `NSPoint`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// `NSSize`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsSize {
    pub(crate) width: f64,
    pub(crate) height: f64,
}

/// `NSRect`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsRect {
    pub(crate) origin: NsPoint,
    pub(crate) size: NsSize,
}

/// Whether windows can be opened: the window server has displays.
pub(crate) fn has_window_server() -> bool {
    let mut count = 0;
    // SAFETY: counts the active displays without listing any.
    let error = unsafe { CGGetActiveDisplayList(0, ptr::null_mut(), &raw mut count) };
    error == 0 && count > 0
}

/// The class named `name`, which `AppKit` has.
fn class_named(name: &CStr) -> *mut Object {
    class(name)
        .unwrap_or_else(|| panic!("AppKit has {name:?}"))
        .as_ptr()
}

/// The shared application (`NSApp`).
fn app() -> *mut Object {
    // SAFETY: a class method returning the immortal application.
    unsafe { send(class_named(c"NSApplication"), c"sharedApplication", ()) }
}

/// The view of `window`, which is open in `event_loop`.
fn view(event_loop: &mut EventLoop, window: WindowId) -> *mut Object {
    assert!(
        event_loop.context().windows().contains(&window),
        "{window:?} is open"
    );
    window.to_raw() as usize as *mut Object
}

/// Runs `f` within an autorelease pool on the view of `window`, which is
/// open in `event_loop`, and on its `NSWindow`. Whatever `AppKit`
/// autoreleases goes into the pool: without one, it would never be
/// released (no `-[NSApplication run]` drains the main thread's).
fn with_window<T>(
    event_loop: &mut EventLoop,
    window: WindowId,
    f: impl FnOnce(*mut Object, *mut Object) -> T,
) -> T {
    let view = view(event_loop, window);
    autoreleasepool(|| {
        // SAFETY: the live view's window, which retains the view.
        let ns_window: *mut Object = unsafe { send(view, c"window", ()) };
        assert!(!ns_window.is_null(), "the view is in a window");
        f(view, ns_window)
    })
}

/// The text of `string`, an `NSString`.
///
/// # Safety
///
/// `string` is a live `NSString` (or nil), and a pool is in place.
unsafe fn read_string(string: *mut Object) -> String {
    if string.is_null() {
        return String::new();
    }
    // SAFETY: the caller's guarantees; the UTF-8 copy lives as long as
    // the pool and is copied at once.
    unsafe {
        let utf8: *const c_char = send(string, c"UTF8String", ());
        CStr::from_ptr(utf8).to_string_lossy().into_owned()
    }
}

/// An autoreleased `NSString` holding `text`.
///
/// # Safety
///
/// An autorelease pool is in place.
unsafe fn ns_string(text: &str) -> *mut Object {
    let text = CString::new(text).expect("the test's text has no NUL");
    // SAFETY: a class method copying a NUL-terminated UTF-8 string.
    unsafe {
        send(
            class_named(c"NSString"),
            c"stringWithUTF8String:",
            (text.as_ptr(),),
        )
    }
}

/// The title `window` shows.
pub(crate) fn title(event_loop: &mut EventLoop, window: WindowId) -> String {
    // SAFETY: a getter of the live window, read within the pool.
    with_window(event_loop, window, |_, ns_window| unsafe {
        read_string(send(ns_window, c"title", ()))
    })
}

/// The scale factor of `window`'s backing store.
pub(crate) fn backing_scale(event_loop: &mut EventLoop, window: WindowId) -> f64 {
    // SAFETY: a getter of the live window.
    with_window(event_loop, window, |_, ns_window| unsafe {
        send(ns_window, c"backingScaleFactor", ())
    })
}

/// The scale factor of the main screen.
pub(crate) fn main_screen_scale() -> f64 {
    autoreleasepool(|| {
        // SAFETY: the main screen, which a window server with displays
        // has, and a getter of it.
        unsafe {
            let screen: *mut Object = send(class_named(c"NSScreen"), c"mainScreen", ());
            assert!(!screen.is_null(), "there is a main screen");
            send(screen, c"backingScaleFactor", ())
        }
    })
}

/// The size of `window`'s view, in points.
pub(crate) fn view_size(event_loop: &mut EventLoop, window: WindowId) -> NsSize {
    // SAFETY: a getter of the live view.
    let bounds: NsRect = with_window(event_loop, window, |view, _| unsafe {
        send(view, c"bounds", ())
    });
    bounds.size
}

/// The frame of `window` on the screen, in points.
pub(crate) fn frame(event_loop: &mut EventLoop, window: WindowId) -> NsRect {
    // SAFETY: a getter of the live window.
    with_window(event_loop, window, |_, ns_window| unsafe {
        send(ns_window, c"frame", ())
    })
}

/// The smallest size the user can give `window`'s content, in points.
pub(crate) fn content_min_size(event_loop: &mut EventLoop, window: WindowId) -> NsSize {
    // SAFETY: a getter of the live window.
    with_window(event_loop, window, |_, ns_window| unsafe {
        send(ns_window, c"contentMinSize", ())
    })
}

/// Whether `window` is on the screen.
pub(crate) fn is_visible(event_loop: &mut EventLoop, window: WindowId) -> bool {
    // SAFETY: a getter of the live window.
    with_window(event_loop, window, |_, ns_window| unsafe {
        send::<_, i8>(ns_window, c"isVisible", ()) == YES
    })
}

/// The window server's number of `window`.
pub(crate) fn window_number(event_loop: &mut EventLoop, window: WindowId) -> i64 {
    // SAFETY: a getter of the live window.
    with_window(event_loop, window, |_, ns_window| unsafe {
        send(ns_window, c"windowNumber", ())
    })
}

/// Whether the application has a window numbered `number`.
pub(crate) fn has_window_numbered(number: i64) -> bool {
    autoreleasepool(|| {
        // SAFETY: a lookup returning one of the application's windows,
        // or nil.
        unsafe {
            let found: *mut Object = send(app(), c"windowWithWindowNumber:", (number,));
            !found.is_null()
        }
    })
}

/// The size of the application's Dock icon, in points.
pub(crate) fn app_icon_size() -> NsSize {
    autoreleasepool(|| {
        // SAFETY: the application's icon (the default one if none was
        // set), and its size.
        unsafe {
            let icon: *mut Object = send(app(), c"applicationIconImage", ());
            assert!(!icon.is_null(), "the application has an icon");
            send(icon, c"size", ())
        }
    })
}

/// The representations of the application's Dock icon: their sizes in
/// points and their widths in pixels, in the order they were added.
pub(crate) fn app_icon_representations() -> Vec<(NsSize, i64)> {
    autoreleasepool(|| {
        // SAFETY: the application's icon, its array of representations
        // and each representation's size and pixel width.
        unsafe {
            let icon: *mut Object = send(app(), c"applicationIconImage", ());
            assert!(!icon.is_null(), "the application has an icon");
            let representations: *mut Object = send(icon, c"representations", ());
            let count: usize = send(representations, c"count", ());
            (0..count)
                .map(|index| {
                    let representation: *mut Object =
                        send(representations, c"objectAtIndex:", (index,));
                    let points: NsSize = send(representation, c"size", ());
                    let pixels: i64 = send(representation, c"pixelsWide", ());
                    (points, pixels)
                })
                .collect()
        }
    })
}

/// What `window`'s view draws, drawn by `AppKit` as into the window but
/// into an sRGB bitmap of the view's pixel size: RGBA pixels, top row
/// first.
pub(crate) fn drawn_pixels(event_loop: &mut EventLoop, window: WindowId) -> OwnedFrame {
    let scale = backing_scale(event_loop, window);
    let view = view(event_loop, window);
    autoreleasepool(|| {
        // SAFETY: a bitmap context owned here, wrapped by an autoreleased
        // graphics context the live view draws into; its pixels are
        // copied before the context is released.
        unsafe {
            let bounds: NsRect = send(view, c"bounds", ());
            let width = (bounds.size.width * scale).round() as usize;
            let height = (bounds.size.height * scale).round() as usize;
            let space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
            let context =
                CGBitmapContextCreate(ptr::null_mut(), width, height, 8, width * 4, space, RGBA);
            CGColorSpaceRelease(space);
            assert!(!context.is_null(), "a {width}×{height} bitmap");
            CGContextScaleCTM(context, scale, scale);
            let graphics: *mut Object = send(
                class_named(c"NSGraphicsContext"),
                c"graphicsContextWithCGContext:flipped:",
                (context, NO),
            );
            send::<_, ()>(
                view,
                c"displayRectIgnoringOpacity:inContext:",
                (bounds, graphics),
            );
            let data = CGBitmapContextGetData(context).cast::<u8>();
            let pixels = std::slice::from_raw_parts(data, width * height * 4).to_vec();
            CGContextRelease(context);
            OwnedFrame::new(width as u32, height as u32, pixels)
                .expect("the pixels fill the bitmap")
        }
    })
}

/// Whether the cursor shown is the shared cursor `NSCursor` gives for
/// the class method `name` (`arrowCursor`, ...).
pub(crate) fn cursor_shown_is(name: &CStr) -> bool {
    autoreleasepool(|| {
        // SAFETY: class methods returning shared cursors.
        unsafe {
            let cursor = class_named(c"NSCursor");
            let shown: *mut Object = send(cursor, c"currentCursor", ());
            let expected: *mut Object = send(cursor, name, ());
            shown == expected
        }
    })
}
