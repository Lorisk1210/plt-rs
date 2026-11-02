//! Foundation: strings, dates, data, geometry and the process.

use std::ffi::{CStr, c_char, c_void};

use super::runtime::{Id, Object, YES, send};
use crate::macos::convert::PointRect;

/// `NSUTF8StringEncoding`.
const UTF8: u64 = 4;

/// `NSPoint` (`CGPoint`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsPoint {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// `NSSize` (`CGSize`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsSize {
    pub(crate) width: f64,
    pub(crate) height: f64,
}

/// `NSRect` (`CGRect`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct NsRect {
    pub(crate) origin: NsPoint,
    pub(crate) size: NsSize,
}

impl NsRect {
    /// The rectangle `rect`.
    pub(crate) const fn new(rect: PointRect) -> Self {
        Self {
            origin: NsPoint {
                x: rect.x,
                y: rect.y,
            },
            size: NsSize {
                width: rect.width,
                height: rect.height,
            },
        }
    }

    /// This rectangle as a [`PointRect`].
    pub(crate) const fn to_point_rect(self) -> PointRect {
        PointRect {
            x: self.origin.x,
            y: self.origin.y,
            width: self.size.width,
            height: self.size.height,
        }
    }
}

/// An `NSString` holding `text`.
pub(crate) fn string(text: &str) -> Option<Id> {
    // SAFETY: a new string copying `text`'s UTF-8 bytes (nil if they could
    // not be decoded, which UTF-8 from a `str` always can).
    unsafe {
        let class = super::runtime::class(c"NSString")?;
        let string: *mut Object = send(class.as_ptr(), c"alloc", ());
        let bytes = text.as_ptr().cast::<c_void>();
        Id::owned(send(
            string,
            c"initWithBytes:length:encoding:",
            (bytes, text.len(), UTF8),
        ))
    }
}

/// The text of the `NSString` `string`.
///
/// # Safety
///
/// `string` is a live `NSString`, and an autorelease pool is in place.
pub(crate) unsafe fn read_string(string: *mut Object) -> String {
    // SAFETY: the caller's guarantees; the UTF-8 buffer lives as long as
    // the pool, and is copied at once.
    unsafe {
        let utf8: *const c_char = send(string, c"UTF8String", ());
        if utf8.is_null() {
            return String::new();
        }
        CStr::from_ptr(utf8).to_string_lossy().into_owned()
    }
}

/// An `NSDate` `seconds` from now: `None` for the distant past (do not
/// wait), infinity for the distant future.
///
/// # Safety
///
/// An autorelease pool is in place: the date is autoreleased.
pub(crate) unsafe fn date(seconds: Option<f64>) -> *mut Object {
    // SAFETY: class methods returning autoreleased dates.
    unsafe {
        let Some(class) = super::runtime::class(c"NSDate") else {
            return std::ptr::null_mut();
        };
        match seconds {
            None => send(class.as_ptr(), c"distantPast", ()),
            Some(seconds) if seconds.is_infinite() => send(class.as_ptr(), c"distantFuture", ()),
            Some(seconds) => send(class.as_ptr(), c"dateWithTimeIntervalSinceNow:", (seconds,)),
        }
    }
}

/// An `NSData` holding a copy of `bytes`.
///
/// # Safety
///
/// An autorelease pool is in place: the data is autoreleased.
pub(crate) unsafe fn data(bytes: &[u8]) -> *mut Object {
    // SAFETY: a class method copying `bytes`.
    unsafe {
        let Some(class) = super::runtime::class(c"NSData") else {
            return std::ptr::null_mut();
        };
        let pointer = bytes.as_ptr().cast::<c_void>();
        send(
            class.as_ptr(),
            c"dataWithBytes:length:",
            (pointer, bytes.len()),
        )
    }
}

/// Whether this is the process's main thread, the only one `AppKit` may
/// be used from.
pub(crate) fn is_main_thread() -> bool {
    // SAFETY: a class method without arguments.
    super::runtime::class(c"NSThread")
        .is_some_and(|class| unsafe { send::<_, i8>(class.as_ptr(), c"isMainThread", ()) } == YES)
}

/// The name of the process (the executable's), as the menu bar shows it.
pub(crate) fn process_name() -> String {
    super::runtime::autoreleasepool(|| {
        // SAFETY: class and instance methods returning autoreleased
        // objects, read within the pool.
        unsafe {
            let Some(class) = super::runtime::class(c"NSProcessInfo") else {
                return String::new();
            };
            let info: *mut Object = send(class.as_ptr(), c"processInfo", ());
            let name: *mut Object = send(info, c"processName", ());
            if name.is_null() {
                String::new()
            } else {
                read_string(name)
            }
        }
    })
}
