//! Text and PNG images on the clipboard.

use std::time::Duration;

use super::ffi::{self, Handle};
use super::{Hwnd, check, last_error};
use crate::error::Error;
use crate::windows::convert::{from_wide, to_wide};

/// How often to try opening the clipboard, which another application may
/// hold open for a moment.
const OPEN_ATTEMPTS: u32 = 5;
/// The pause between attempts to open the clipboard.
const OPEN_RETRY_DELAY: Duration = Duration::from_millis(10);

/// What to put on the clipboard.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Clipboard<'a> {
    /// Text (`CF_UNICODETEXT`).
    Text(&'a str),
    /// A PNG image (the registered `PNG` format, which browsers and
    /// office and image applications read).
    Png(&'a [u8]),
}

/// Puts `content` on the clipboard, owned by `owner`.
pub(crate) fn set_clipboard(owner: Hwnd, content: Clipboard<'_>) -> Result<(), Error> {
    let (format, bytes) = match content {
        Clipboard::Text(text) => {
            let units = to_wide(text);
            let bytes = units.iter().flat_map(|unit| unit.to_ne_bytes()).collect();
            (ffi::CF_UNICODETEXT, bytes)
        }
        Clipboard::Png(png) => (png_format()?, png.to_vec()),
    };
    let memory = GlobalMemory::holding(&bytes)?;
    let _open = OpenClipboard::open(owner)?;
    // SAFETY: a call without arguments, with the clipboard open.
    check(unsafe { ffi::EmptyClipboard() }, "EmptyClipboard")?;
    // SAFETY: `memory` is movable global memory holding the data; once
    // set, the system owns it.
    if unsafe { ffi::SetClipboardData(format, memory.0) } == 0 {
        return Err(last_error("SetClipboardData"));
    }
    std::mem::forget(memory);
    Ok(())
}

/// The text on the clipboard, if it holds text.
pub(crate) fn clipboard_text(owner: Hwnd) -> Result<Option<String>, Error> {
    let _open = OpenClipboard::open(owner)?;
    // SAFETY: a call with a format, with the clipboard open; the memory
    // stays the clipboard's.
    let memory = unsafe { ffi::GetClipboardData(ffi::CF_UNICODETEXT) };
    if memory == 0 {
        return Ok(None);
    }
    // SAFETY: the clipboard's text is global memory of `GlobalSize`
    // bytes, locked while read and unlocked after.
    let text = unsafe {
        let units = ffi::GlobalLock(memory).cast::<u16>();
        if units.is_null() {
            return Ok(None);
        }
        let len = ffi::GlobalSize(memory) / size_of::<u16>();
        let text = from_wide(std::slice::from_raw_parts(units, len));
        ffi::GlobalUnlock(memory);
        text
    };
    Ok(Some(text))
}

/// The clipboard format of PNG images.
fn png_format() -> Result<u32, Error> {
    let name = to_wide("PNG");
    // SAFETY: the name is NUL-terminated and outlives the call.
    match unsafe { ffi::RegisterClipboardFormatW(name.as_ptr()) } {
        0 => Err(last_error("RegisterClipboardFormatW")),
        format => Ok(format),
    }
}

/// The clipboard, open for this thread until dropped.
struct OpenClipboard;

impl OpenClipboard {
    fn open(owner: Hwnd) -> Result<Self, Error> {
        for _ in 1..OPEN_ATTEMPTS {
            // SAFETY: a handle-only call; `Drop` closes the clipboard.
            if unsafe { ffi::OpenClipboard(owner.0) } != 0 {
                return Ok(Self);
            }
            std::thread::sleep(OPEN_RETRY_DELAY);
        }
        // SAFETY: as above.
        check(unsafe { ffi::OpenClipboard(owner.0) }, "OpenClipboard")?;
        Ok(Self)
    }
}

impl Drop for OpenClipboard {
    fn drop(&mut self) {
        // SAFETY: closes the clipboard this thread opened.
        unsafe { ffi::CloseClipboard() };
    }
}

/// Movable global memory (`GlobalAlloc`), freed when dropped.
struct GlobalMemory(Handle);

impl GlobalMemory {
    /// Global memory holding a copy of `bytes`.
    fn holding(bytes: &[u8]) -> Result<Self, Error> {
        // SAFETY: an allocation, freed by `Drop` unless handed over.
        let memory = Self(unsafe { ffi::GlobalAlloc(ffi::GMEM_MOVEABLE, bytes.len().max(1)) });
        if memory.0 == 0 {
            return Err(last_error("GlobalAlloc"));
        }
        // SAFETY: the memory has room for `bytes`, is locked while
        // written and unlocked after.
        unsafe {
            let target = ffi::GlobalLock(memory.0).cast::<u8>();
            if target.is_null() {
                return Err(last_error("GlobalLock"));
            }
            target.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
            ffi::GlobalUnlock(memory.0);
        }
        Ok(memory)
    }
}

impl Drop for GlobalMemory {
    fn drop(&mut self) {
        // SAFETY: frees the memory this value owns.
        unsafe { ffi::GlobalFree(self.0) };
    }
}
