//! The Windows backend's unsafe code: the Win32 calls it makes, behind
//! safe wrappers.
//!
//! * [`ffi`]: the declarations, and how they are linked.
//! * [`window`]: the window class and its procedure, windows, the message
//!   loop and keyboard and pointer queries.
//! * [`gdi`]: painting, cursors and icons.
//! * [`clipboard`]: text and PNG images on the clipboard.
//! * [`registry`]: the string values of a registry key.
//!
//! Handles are plain values: Win32 checks them on every call and fails
//! cleanly for a stale one, so the wrappers taking them stay sound
//! whatever the handle. Pointers only ever go to memory owned by the
//! wrapper making the call, or to what Win32 documents a message's
//! `lParam` to point to.

mod clipboard;
mod ffi;
mod gdi;
mod registry;
mod window;

use std::io;

pub(crate) use clipboard::{Clipboard, clipboard_text, set_clipboard};
pub(crate) use gdi::{OwnedIcon, Painter, icon_sides, show_cursor};
pub use registry::string_values;
pub(crate) use window::{
    Restore, WindowClass, capture, dispatch_messages, double_click_time, held_modifiers,
    layout_character, next_key_message, post_wake, release_capture, screen_to_client, take_typed,
    track_leave, use_per_monitor_dpi, wait_for_messages,
};

use crate::error::Error;
use crate::window::WindowId;

/// A window handle (`HWND`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Hwnd(ffi::Handle);

impl Hwnd {
    /// The id of the window: its handle's value.
    pub(crate) const fn id(self) -> WindowId {
        WindowId::from_raw(self.0 as u64)
    }

    /// The handle of the window with id `window`.
    pub(crate) const fn from_id(window: WindowId) -> Self {
        Self(window.to_raw() as ffi::Handle)
    }
}

/// The error of the Win32 call `operation`, from `GetLastError`.
fn last_error(operation: &'static str) -> Error {
    Error::io(operation, &io::Error::last_os_error())
}

/// `Ok` if a Win32 call returning `BOOL` succeeded, else its error.
fn check(succeeded: i32, operation: &'static str) -> Result<(), Error> {
    if succeeded == 0 {
        Err(last_error(operation))
    } else {
        Ok(())
    }
}
