//! The macOS backend's unsafe code: the Objective-C runtime, `AppKit`,
//! Foundation and Core Graphics, behind safe wrappers.
//!
//! * [`runtime`]: message sends, reference counts, autorelease pools,
//!   class definitions; `AppKit`, which pltrs does not link, is loaded
//!   when its first class is looked up.
//! * [`foundation`]: strings, dates, data, geometry, the process.
//! * [`app`]: the application, its menu bar and event queue, wake-ups.
//! * [`window`]: windows, pltrs's views and cursors.
//! * [`event`]: the input events views receive.
//! * [`graphics`]: images, drawn into views.
//! * [`pasteboard`]: the clipboard.
//! * [`classes`]: pltrs's view and application delegate classes, whose
//!   methods call the safe [`crate::macos::view`].
//!
//! Every message is sent with the argument and return types `AppKit`
//! declares for its selector. Owned objects are [`Id`]s, released when
//! dropped; objects `AppKit` lends (method arguments, autoreleased
//! results) are only used during the call or the autorelease pool that
//! holds them, and borrowed wrappers carry lifetimes that end there.

mod app;
mod classes;
mod event;
mod foundation;
mod graphics;
mod pasteboard;
mod runtime;
mod window;

use std::ffi::CStr;
use std::io;

pub(crate) use app::{App, WakeTarget};
pub(crate) use classes::{app_delegate, view_class};
pub(crate) use event::EventRef;
pub(crate) use foundation::{NsPoint, is_main_thread, process_name};
pub(crate) use graphics::Image;
pub(crate) use pasteboard::{Content, pasteboard_text, set_pasteboard};
pub(crate) use runtime::{Id, autoreleasepool};
pub(crate) use window::{View, ViewRef, Window, main_screen_scale_factor, set_cursor};

use crate::error::Error;
use runtime::Object;

/// The error of a failed `AppKit` call (they report no cause).
fn failed(operation: &'static str) -> Error {
    Error::Io {
        operation,
        kind: io::ErrorKind::Other,
    }
}

/// A new, uninitialized instance of the class `name` (`+alloc`).
///
/// # Safety
///
/// The instance must be initialized (`-init...`) before any other use,
/// and the initializer's result owned.
unsafe fn new_object(name: &CStr) -> Result<*mut Object, Error> {
    let class = runtime::class(name).ok_or_else(|| failed("finding a class"))?;
    // SAFETY: `+alloc` of a class.
    let object: *mut Object = unsafe { runtime::send(class.as_ptr(), c"alloc", ()) };
    if object.is_null() {
        return Err(failed("allocating an object"));
    }
    Ok(object)
}

#[cfg(test)]
mod tests {
    use super::runtime::class;

    #[test]
    fn appkit_classes_are_found_once_it_is_loaded() {
        assert!(class(c"NSString").is_some(), "Foundation is linked");
        assert!(class(c"NSView").is_some(), "AppKit is loaded on first use");
        assert!(class(c"NSWindow").is_some());
        assert!(class(c"PltrsNoSuchClass").is_none());
    }
}
