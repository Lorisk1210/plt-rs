//! Input posted to the application's event queue, as the window server
//! delivers it: mouse buttons and drags, the pointer crossing a view,
//! scrolls and keys.

use std::ffi::c_void;
use std::ptr;

use pltrs_platform::{EventLoop, WindowId};

use super::runtime::{NO, Object, YES, autoreleasepool, send};
use super::{NsPoint, app, class_named, ns_string, view, with_window};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSetType(event: *mut c_void, kind: u32);
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: *mut c_void, field: u32, value: f64);
}

/// `NSEventTypeMouseMoved`: the event scrolls are made from.
const MOUSE_MOVED: u64 = 5;
/// `NSEventTypeMouseEntered`.
const MOUSE_ENTERED: u64 = 8;
/// `NSEventTypeMouseExited`.
const MOUSE_EXITED: u64 = 9;
/// `kCGEventScrollWheel`.
const CG_SCROLL_WHEEL: u32 = 22;
/// `kCGMouseEventButtonNumber`.
const BUTTON_NUMBER: u32 = 3;
/// `kCGScrollWheelEventDeltaAxis1` (vertical lines) and `...Axis2`.
const LINE_DELTA: [u32; 2] = [11, 12];
/// `kCGScrollWheelEventFixedPtDeltaAxis1` and `...Axis2`.
const FIXED_POINT_DELTA: [u32; 2] = [93, 94];
/// `kCGScrollWheelEventPointDeltaAxis1` (vertical points) and `...Axis2`.
const POINT_DELTA: [u32; 2] = [96, 97];
/// `kCGScrollWheelEventIsContinuous`: a touchpad's precise scroll.
const IS_CONTINUOUS: u32 = 88;

/// The seconds since the system started, as events are stamped.
fn uptime() -> f64 {
    autoreleasepool(|| {
        // SAFETY: the process's information, and a getter of it.
        unsafe {
            let info: *mut Object = send(class_named(c"NSProcessInfo"), c"processInfo", ());
            send(info, c"systemUptime", ())
        }
    })
}

/// Puts `event` at the end of the application's event queue.
///
/// # Safety
///
/// `event` is a live `NSEvent`.
unsafe fn post(event: *mut Object) {
    assert!(!event.is_null(), "AppKit made the event");
    // SAFETY: the application retains the event it queues.
    unsafe { send::<_, ()>(app(), c"postEvent:atStart:", (event, NO)) };
}

/// An autoreleased mouse event of `kind` over `view` at `at` (points
/// from its top-left corner), as `AppKit` makes them for the window.
///
/// # Safety
///
/// `view` is a live view in a window, and a pool is in place.
unsafe fn mouse_event(
    view: *mut Object,
    kind: u64,
    at: (f64, f64),
    flags: u64,
    clicks: i64,
) -> *mut Object {
    // SAFETY: getters of the live view and its window, a point
    // conversion, and a class method returning an autoreleased event.
    unsafe {
        let window: *mut Object = send(view, c"window", ());
        let number: i64 = send(window, c"windowNumber", ());
        let point = NsPoint { x: at.0, y: at.1 };
        let location: NsPoint = send(
            view,
            c"convertPoint:toView:",
            (point, ptr::null_mut::<Object>()),
        );
        send(
            class_named(c"NSEvent"),
            c"mouseEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:clickCount:pressure:",
            (
                kind,
                location,
                flags,
                uptime(),
                number,
                ptr::null_mut::<Object>(),
                0_i64,
                clicks,
                1.0_f32,
            ),
        )
    }
}

/// The autoreleased `NSEvent` of the Core Graphics event `event`.
///
/// # Safety
///
/// `event` is a live `CGEventRef`, and a pool is in place.
unsafe fn from_cg_event(event: *mut c_void) -> *mut Object {
    // SAFETY: a class method copying the event.
    unsafe { send(class_named(c"NSEvent"), c"eventWithCGEvent:", (event,)) }
}

/// The autoreleased Core Graphics copy of `event`, which can be changed
/// in ways `NSEvent`'s constructors do not offer.
///
/// # Safety
///
/// `event` is a live `NSEvent`, and a pool is in place.
unsafe fn cg_copy(event: *mut Object) -> *mut c_void {
    // SAFETY: a getter returning an autoreleased copy.
    let copy: *mut c_void = unsafe { send(event, c"CGEvent", ()) };
    assert!(!copy.is_null(), "the event has a Core Graphics copy");
    copy
}

/// What a posted mouse event does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    /// A button goes down.
    Press,
    /// A button goes up.
    Release,
    /// The pointer moves with a button held.
    Drag,
}

/// A mouse button event to post.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Mouse {
    /// What happens.
    pub(crate) action: Action,
    /// The button, numbered as `AppKit` does (0 left, 1 right, 2 middle,
    /// 3 back, 4 forward, ...).
    pub(crate) button: i64,
    /// Where, in points from the view's top-left corner.
    pub(crate) at: (f64, f64),
    /// The modifier flags (`NSEventModifierFlags`).
    pub(crate) flags: u64,
    /// The click count: 2 for the second press of a double click.
    pub(crate) clicks: i64,
}

impl Mouse {
    /// `action` of `button` at `at`, without modifiers, a single click.
    pub(crate) const fn new(action: Action, button: i64, at: (f64, f64)) -> Self {
        Self {
            action,
            button,
            at,
            flags: 0,
            clicks: 1,
        }
    }

    /// The same with modifier `flags` held.
    pub(crate) const fn with_flags(self, flags: u64) -> Self {
        Self { flags, ..self }
    }

    /// The same as the `clicks`th click in a row.
    pub(crate) const fn with_clicks(self, clicks: i64) -> Self {
        Self { clicks, ..self }
    }

    /// The `NSEventType` of this event.
    const fn kind(self) -> u64 {
        match (self.action, self.button) {
            (Action::Press, 0) => 1,
            (Action::Release, 0) => 2,
            (Action::Drag, 0) => 6,
            (Action::Press, 1) => 3,
            (Action::Release, 1) => 4,
            (Action::Drag, 1) => 7,
            (Action::Press, _) => 25,
            (Action::Release, _) => 26,
            (Action::Drag, _) => 27,
        }
    }
}

/// Posts `mouse` to `window`. `NSEvent` numbers the buttons of the events
/// it makes 0; the Core Graphics copy takes the button's number.
pub(crate) fn post_mouse(event_loop: &mut EventLoop, window: WindowId, mouse: Mouse) {
    let view = view(event_loop, window);
    autoreleasepool(|| {
        // SAFETY: an event over the live view, copied, renumbered and
        // posted within the pool.
        unsafe {
            let event = mouse_event(view, mouse.kind(), mouse.at, mouse.flags, mouse.clicks);
            let copy = cg_copy(event);
            CGEventSetIntegerValueField(copy, BUTTON_NUMBER, mouse.button);
            post(from_cg_event(copy));
        }
    });
}

/// Posts the pointer entering (`true`) or leaving `window`'s view at
/// `at`, as its tracking area reports it.
pub(crate) fn post_crossing(
    event_loop: &mut EventLoop,
    window: WindowId,
    entered: bool,
    at: (f64, f64),
) {
    let view = view(event_loop, window);
    autoreleasepool(|| {
        // SAFETY: getters of the live view and its window, a point
        // conversion, and an autoreleased event naming the view's
        // tracking area (which `AppKit` numbers by its address), posted
        // within the pool.
        unsafe {
            let window: *mut Object = send(view, c"window", ());
            let number: i64 = send(window, c"windowNumber", ());
            let areas: *mut Object = send(view, c"trackingAreas", ());
            let area: *mut Object = send(areas, c"firstObject", ());
            assert!(!area.is_null(), "the view tracks the pointer");
            let point = NsPoint { x: at.0, y: at.1 };
            let location: NsPoint = send(
                view,
                c"convertPoint:toView:",
                (point, ptr::null_mut::<Object>()),
            );
            let kind = if entered { MOUSE_ENTERED } else { MOUSE_EXITED };
            post(send(
                class_named(c"NSEvent"),
                c"enterExitEventWithType:location:modifierFlags:timestamp:windowNumber:context:eventNumber:trackingNumber:userData:",
                (
                    kind,
                    location,
                    0_u64,
                    uptime(),
                    number,
                    ptr::null_mut::<Object>(),
                    0_i64,
                    area as i64,
                    ptr::null_mut::<c_void>(),
                ),
            ));
        }
    });
}

/// A scroll to post.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Scroll {
    /// Where, in points from the view's top-left corner.
    pub(crate) at: (f64, f64),
    /// `scrollingDeltaX` and `scrollingDeltaY`, as `AppKit` reports them:
    /// lines of a wheel, or points of a touchpad.
    pub(crate) delta: (i32, i32),
    /// Whether a touchpad scrolls (precise deltas in points).
    pub(crate) precise: bool,
    /// The modifier flags.
    pub(crate) flags: u64,
}

/// Posts `scroll` to `window`. `NSEvent` makes no scroll events: they are
/// made from a mouse event's Core Graphics copy, which keeps the window
/// and the location.
pub(crate) fn post_scroll(event_loop: &mut EventLoop, window: WindowId, scroll: Scroll) {
    let view = view(event_loop, window);
    let (dx, dy) = scroll.delta;
    autoreleasepool(|| {
        // SAFETY: an event over the live view, copied, turned into a
        // scroll and posted within the pool.
        unsafe {
            let copy = cg_copy(mouse_event(view, MOUSE_MOVED, scroll.at, scroll.flags, 0));
            CGEventSetType(copy, CG_SCROLL_WHEEL);
            CGEventSetIntegerValueField(copy, IS_CONTINUOUS, i64::from(scroll.precise));
            for (axis, delta) in [dy, dx].into_iter().enumerate() {
                if scroll.precise {
                    CGEventSetIntegerValueField(copy, POINT_DELTA[axis], i64::from(delta));
                } else {
                    CGEventSetIntegerValueField(copy, LINE_DELTA[axis], i64::from(delta));
                    CGEventSetDoubleValueField(copy, FIXED_POINT_DELTA[axis], f64::from(delta));
                }
            }
            post(from_cg_event(copy));
        }
    });
}

/// What a posted keyboard event does (`NSEventType`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    /// A key goes down.
    Press = 10,
    /// A key goes up.
    Release = 11,
    /// A modifier key goes down or up: the flags say which.
    FlagsChanged = 12,
}

/// A keyboard event to post.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Keystroke<'a> {
    /// What happens.
    pub(crate) action: KeyAction,
    /// The hardware key code (`kVK_*`).
    pub(crate) code: u16,
    /// The characters typed (`characters`).
    pub(crate) typed: &'a str,
    /// The characters with no modifier but Shift
    /// (`charactersIgnoringModifiers`).
    pub(crate) unmodified: &'a str,
    /// The modifier flags after the event.
    pub(crate) flags: u64,
    /// Whether the key repeats.
    pub(crate) repeat: bool,
}

impl<'a> Keystroke<'a> {
    /// `action` of the key `code` typing `typed`, without modifiers.
    pub(crate) const fn new(action: KeyAction, code: u16, typed: &'a str) -> Self {
        Self {
            action,
            code,
            typed,
            unmodified: typed,
            flags: 0,
            repeat: false,
        }
    }

    /// A change of the modifier key `code`, leaving `flags`.
    pub(crate) const fn flags_changed(code: u16, flags: u64) -> Self {
        Self::new(KeyAction::FlagsChanged, code, "").with_flags(flags)
    }

    /// The same with modifier `flags`.
    pub(crate) const fn with_flags(self, flags: u64) -> Self {
        Self { flags, ..self }
    }

    /// The same typing `unmodified` without the modifiers.
    pub(crate) const fn unmodified(self, unmodified: &'a str) -> Self {
        Self { unmodified, ..self }
    }

    /// The same as an auto-repeat.
    pub(crate) const fn repeated(self) -> Self {
        Self {
            repeat: true,
            ..self
        }
    }
}

/// Posts `key` to `window`.
pub(crate) fn post_key(event_loop: &mut EventLoop, window: WindowId, key: Keystroke<'_>) {
    with_window(event_loop, window, |_, ns_window| {
        // SAFETY: the live window's number, autoreleased strings and an
        // autoreleased event, posted within the pool.
        unsafe {
            let number: i64 = send(ns_window, c"windowNumber", ());
            post(send(
                class_named(c"NSEvent"),
                c"keyEventWithType:location:modifierFlags:timestamp:windowNumber:context:characters:charactersIgnoringModifiers:isARepeat:keyCode:",
                (
                    key.action as u64,
                    NsPoint::default(),
                    key.flags,
                    uptime(),
                    number,
                    ptr::null_mut::<Object>(),
                    ns_string(key.typed),
                    ns_string(key.unmodified),
                    if key.repeat { YES } else { NO },
                    key.code,
                ),
            ));
        }
    });
}
