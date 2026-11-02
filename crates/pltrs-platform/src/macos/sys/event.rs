//! Input events (`NSEvent`), as views receive them.

use std::marker::PhantomData;
use std::ptr::NonNull;

use super::foundation::{NsPoint, read_string};
use super::runtime::{Object, YES, autoreleasepool, send};

/// `NSEventTypeKeyDown`.
const KEY_DOWN: u64 = 10;
/// `NSEventTypeKeyUp`.
const KEY_UP: u64 = 11;
/// `NSEventTypeFlagsChanged`.
const FLAGS_CHANGED: u64 = 12;
/// `NSEventTypeScrollWheel`.
const SCROLL_WHEEL: u64 = 22;

/// An event a view method received, valid during the call.
#[derive(Debug, Clone, Copy)]
pub(crate) struct EventRef<'a> {
    event: *mut Object,
    _life: PhantomData<&'a Object>,
}

impl EventRef<'_> {
    /// The event a method received.
    ///
    /// # Safety
    ///
    /// `event` is a live `NSEvent` for the lifetime chosen.
    pub(super) const unsafe fn argument(event: NonNull<Object>) -> Self {
        Self {
            event: event.as_ptr(),
            _life: PhantomData,
        }
    }

    /// The event type (`NSEventType`).
    pub(crate) fn kind(self) -> u64 {
        // SAFETY: a getter every event has.
        unsafe { send(self.event, c"type", ()) }
    }

    /// The modifier flags.
    pub(crate) fn modifier_flags(self) -> u64 {
        // SAFETY: a getter every event has.
        unsafe { send(self.event, c"modifierFlags", ()) }
    }

    /// Where the event happened, in its window's coordinates.
    pub(super) fn location_in_window(self) -> NsPoint {
        // SAFETY: a getter every event has (meaningful for pointer
        // events).
        unsafe { send(self.event, c"locationInWindow", ()) }
    }

    /// Whether this is a mouse button press or release (`AppKit` raises
    /// on the button getters of other events).
    fn is_button(self) -> bool {
        matches!(self.kind(), 1..=4 | 25 | 26)
    }

    /// The button number of a button press or release (0 left, 1 right,
    /// 2 middle, ...); 0 for other events.
    pub(crate) fn button_number(self) -> i64 {
        if !self.is_button() {
            return 0;
        }
        // SAFETY: a getter of button events, which this is.
        unsafe { send(self.event, c"buttonNumber", ()) }
    }

    /// The click count of a button press or release (2 for a double
    /// click, ...); 0 for other events.
    pub(crate) fn click_count(self) -> i64 {
        if !self.is_button() {
            return 0;
        }
        // SAFETY: a getter of button events, which this is.
        unsafe { send(self.event, c"clickCount", ()) }
    }

    /// The hardware key code of a key press or release, or of the
    /// modifier key of a `flagsChanged:` event; 0 for other events.
    pub(crate) fn key_code(self) -> u16 {
        if !matches!(self.kind(), KEY_DOWN | KEY_UP | FLAGS_CHANGED) {
            return 0;
        }
        // SAFETY: a getter of keyboard events, which this is.
        unsafe { send(self.event, c"keyCode", ()) }
    }

    /// Whether this is a key press or release (`AppKit` raises on the
    /// key-only getters of other events).
    fn is_key(self) -> bool {
        matches!(self.kind(), KEY_DOWN | KEY_UP)
    }

    /// Whether a key event is an auto-repeat; `false` for other events.
    pub(crate) fn is_repeat(self) -> bool {
        // SAFETY: a getter of key events, which this is.
        self.is_key() && unsafe { send::<_, i8>(self.event, c"isARepeat", ()) } == YES
    }

    /// The characters a key event typed; empty for other events.
    pub(crate) fn characters(self) -> String {
        self.string(c"characters")
    }

    /// The characters of a key event with no modifier but Shift applied;
    /// empty for other events.
    pub(crate) fn characters_ignoring_modifiers(self) -> String {
        self.string(c"charactersIgnoringModifiers")
    }

    fn string(self, getter: &std::ffi::CStr) -> String {
        if !self.is_key() {
            return String::new();
        }
        autoreleasepool(|| {
            // SAFETY: a string getter of key events, which this is; the
            // string is read within the pool.
            unsafe {
                let string: *mut Object = send(self.event, getter, ());
                if string.is_null() {
                    String::new()
                } else {
                    read_string(string)
                }
            }
        })
    }

    /// The scroll of a scroll-wheel event, `(x, y, precise)`; zero for
    /// other events.
    pub(crate) fn scroll(self) -> (f64, f64, bool) {
        if self.kind() != SCROLL_WHEEL {
            return (0.0, 0.0, false);
        }
        // SAFETY: getters of scroll-wheel events, which this is.
        unsafe {
            (
                send(self.event, c"scrollingDeltaX", ()),
                send(self.event, c"scrollingDeltaY", ()),
                send::<_, i8>(self.event, c"hasPreciseScrollingDeltas", ()) == YES,
            )
        }
    }
}
