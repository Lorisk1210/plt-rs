//! pltrs's Objective-C classes: the view each window shows (and which is
//! the window's delegate), and the application delegate. Their methods
//! hand over to [`crate::macos::view`].

use std::ffi::{CStr, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::NonNull;
use std::sync::OnceLock;

use super::event::EventRef;
use super::failed;
use super::foundation::NsRect;
use super::runtime::{Id, Method, NO, Object, Sel, YES, define_class, sel, send};
use super::window::ViewRef;
use crate::error::Error;
use crate::macos::view::{self, WindowChange};

/// The type encoding of `BOOL`-returning methods without arguments.
#[cfg(target_arch = "aarch64")]
const BOOL_GETTER: &CStr = c"B@:";
/// The type encoding of `BOOL`-returning methods taking an object.
#[cfg(target_arch = "aarch64")]
const BOOL_WITH_OBJECT: &CStr = c"B@:@";
/// The type encoding of `BOOL`-returning methods without arguments.
#[cfg(not(target_arch = "aarch64"))]
const BOOL_GETTER: &CStr = c"c@:";
/// The type encoding of `BOOL`-returning methods taking an object.
#[cfg(not(target_arch = "aarch64"))]
const BOOL_WITH_OBJECT: &CStr = c"c@:@";
/// The type encoding of methods without arguments or result.
const VOID: &CStr = c"v@:";
/// The type encoding of methods taking an object.
const VOID_WITH_OBJECT: &CStr = c"v@:@";
/// The type encoding of `drawRect:`.
const DRAW_RECT: &CStr = c"v@:{CGRect={CGPoint=dd}{CGSize=dd}}";
/// The type encoding of `applicationShouldTerminate:`.
const TERMINATE_REPLY: &CStr = c"Q@:@";

/// The pointer events views take.
const POINTER_EVENTS: [&CStr; 12] = [
    c"mouseDown:",
    c"mouseUp:",
    c"mouseDragged:",
    c"mouseMoved:",
    c"mouseEntered:",
    c"mouseExited:",
    c"rightMouseDown:",
    c"rightMouseUp:",
    c"rightMouseDragged:",
    c"otherMouseDown:",
    c"otherMouseUp:",
    c"otherMouseDragged:",
];

/// The window notifications views take as their window's delegate.
const WINDOW_NOTIFICATIONS: [&CStr; 4] = [
    c"windowDidResize:",
    c"windowDidBecomeKey:",
    c"windowDidResignKey:",
    c"windowDidChangeBackingProperties:",
];

/// A class name unique to this copy of the crate, should a process load
/// two (an executable and a Python extension).
fn class_name(base: &str) -> String {
    static ANCHOR: u8 = 0;
    format!("{base}{:x}", &raw const ANCHOR as usize)
}

/// The address of pltrs's view class, defined on first use.
pub(crate) fn view_class() -> Result<usize, Error> {
    static CLASS: OnceLock<Option<usize>> = OnceLock::new();
    let class =
        CLASS.get_or_init(|| {
            let method = |selector, implementation: *const c_void, types| Method {
                selector,
                implementation,
                types,
            };
            let mut methods = vec![
                method(c"drawRect:", draw_rect as *const c_void, DRAW_RECT),
                method(c"isFlipped", yes as *const c_void, BOOL_GETTER),
                method(c"acceptsFirstResponder", yes as *const c_void, BOOL_GETTER),
                method(c"keyDown:", key_event as *const c_void, VOID_WITH_OBJECT),
                method(c"keyUp:", key_event as *const c_void, VOID_WITH_OBJECT),
                method(
                    c"flagsChanged:",
                    key_event as *const c_void,
                    VOID_WITH_OBJECT,
                ),
                method(
                    c"scrollWheel:",
                    scroll_wheel as *const c_void,
                    VOID_WITH_OBJECT,
                ),
                method(
                    c"resetCursorRects",
                    reset_cursor_rects as *const c_void,
                    VOID,
                ),
                method(
                    c"windowShouldClose:",
                    window_should_close as *const c_void,
                    BOOL_WITH_OBJECT,
                ),
            ];
            methods.extend(POINTER_EVENTS.map(|selector| {
                method(selector, pointer_event as *const c_void, VOID_WITH_OBJECT)
            }));
            methods.extend(WINDOW_NOTIFICATIONS.map(|selector| {
                method(selector, window_changed as *const c_void, VOID_WITH_OBJECT)
            }));
            // SAFETY: each implementation below takes the receiver, the
            // selector and the arguments its type encoding and `AppKit`'s
            // declaration of the selector give, and returns their result.
            unsafe { define_class(&class_name("PltrsView"), c"NSView", &methods) }
        });
    class.ok_or_else(|| failed("defining pltrs's view class"))
}

/// A new application delegate, which turns a request to quit (⌘Q, the
/// Dock's Quit) into close requests for every window.
pub(crate) fn app_delegate() -> Result<Id, Error> {
    static CLASS: OnceLock<Option<usize>> = OnceLock::new();
    let class = CLASS.get_or_init(|| {
        let methods = [
            Method {
                selector: c"applicationShouldTerminate:",
                implementation: should_terminate as *const c_void,
                types: TERMINATE_REPLY,
            },
            Method {
                selector: c"applicationSupportsSecureRestorableState:",
                implementation: yes_with_object as *const c_void,
                types: BOOL_WITH_OBJECT,
            },
        ];
        // SAFETY: as for the view class.
        unsafe { define_class(&class_name("PltrsAppDelegate"), c"NSObject", &methods) }
    });
    let class = class.ok_or_else(|| failed("defining pltrs's application delegate"))?;
    // SAFETY: a new instance of a registered NSObject subclass.
    unsafe {
        let delegate: *mut Object = send(class as *mut Object, c"alloc", ());
        Id::owned(send(delegate, c"init", ())).ok_or_else(|| failed("creating the delegate"))
    }
}

/// Runs `f` on the view that received a method call. A panic is caught:
/// unwinding into Objective-C is undefined behaviour.
fn on_view(this: *mut Object, f: impl FnOnce(ViewRef<'_>)) {
    let Some(this) = NonNull::new(this) else {
        return;
    };
    // SAFETY: the receiver of a method of pltrs's view class, live during
    // the call, which `f` cannot outlive.
    let view = unsafe { ViewRef::receiver(this) };
    // A panic was reported by the panic hook; the event is dropped.
    let _ = catch_unwind(AssertUnwindSafe(|| f(view)));
}

/// Runs `f` on the view that received an event, and the event.
fn on_event(this: *mut Object, event: *mut Object, f: impl FnOnce(ViewRef<'_>, EventRef<'_>)) {
    let Some(event) = NonNull::new(event) else {
        return;
    };
    // SAFETY: the event argument of a view method, live during the call,
    // which `f` cannot outlive.
    let event = unsafe { EventRef::argument(event) };
    on_view(this, |view| f(view, event));
}

extern "C" fn yes(_: *mut Object, _: Sel) -> i8 {
    YES
}

extern "C" fn yes_with_object(_: *mut Object, _: Sel, _: *mut Object) -> i8 {
    YES
}

extern "C" fn draw_rect(this: *mut Object, _: Sel, dirty: NsRect) {
    on_view(this, |view| view::draw(view, dirty.to_point_rect()));
}

extern "C" fn pointer_event(this: *mut Object, _: Sel, event: *mut Object) {
    on_event(this, event, view::pointer);
}

extern "C" fn key_event(this: *mut Object, _: Sel, event: *mut Object) {
    on_event(this, event, view::key);
}

extern "C" fn scroll_wheel(this: *mut Object, _: Sel, event: *mut Object) {
    on_event(this, event, view::scroll);
}

extern "C" fn reset_cursor_rects(this: *mut Object, _: Sel) {
    on_view(this, view::reset_cursor_rects);
}

extern "C" fn window_changed(this: *mut Object, selector: Sel, _notification: *mut Object) {
    let change = if selector == sel(c"windowDidResize:") {
        WindowChange::Resized
    } else if selector == sel(c"windowDidBecomeKey:") {
        WindowChange::Focused(true)
    } else if selector == sel(c"windowDidResignKey:") {
        WindowChange::Focused(false)
    } else {
        WindowChange::Backing
    };
    on_view(this, |view| view::window_changed(view, change));
}

extern "C" fn window_should_close(this: *mut Object, _: Sel, _sender: *mut Object) -> i8 {
    on_view(this, view::close_requested);
    NO
}

extern "C" fn should_terminate(_: *mut Object, _: Sel, _app: *mut Object) -> u64 {
    // A panic was reported by the panic hook; quitting stays cancelled.
    let _ = catch_unwind(view::quit_requested);
    // `NSTerminateCancel`: pltrs closes its windows itself.
    0
}
