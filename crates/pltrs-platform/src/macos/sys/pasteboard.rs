//! Text and PNG images on the general pasteboard (the clipboard).

use std::ffi::CStr;

use super::failed;
use super::foundation::{data, read_string, string};
use super::runtime::{Object, YES, autoreleasepool, class, send};
use crate::error::Error;

/// The type of plain text (`NSPasteboardTypeString`).
const TEXT: &str = "public.utf8-plain-text";
/// The type of PNG images (`NSPasteboardTypePNG`).
const PNG: &str = "public.png";

/// What to put on the pasteboard.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Content<'a> {
    /// Text.
    Text(&'a str),
    /// A PNG image.
    Png(&'a [u8]),
}

/// Replaces the pasteboard's contents with `content`.
pub(crate) fn set_pasteboard(content: Content<'_>) -> Result<(), Error> {
    autoreleasepool(|| {
        let error = || failed("writing to the pasteboard");
        let pasteboard = general()?;
        // The string written, kept alive until it is.
        let text_string;
        let (value, kind, setter): (*mut Object, _, &CStr) = match content {
            Content::Text(text) => {
                text_string = string(text).ok_or_else(error)?;
                (text_string.as_ptr(), TEXT, c"setString:forType:")
            }
            // SAFETY: within the pool, which keeps the data alive.
            Content::Png(png) => (unsafe { data(png) }, PNG, c"setData:forType:"),
        };
        let kind = string(kind).ok_or_else(error)?;
        if value.is_null() {
            return Err(error());
        }
        // SAFETY: pasteboard methods taking live objects.
        let written = unsafe {
            send::<_, i64>(pasteboard, c"clearContents", ());
            send::<_, i8>(pasteboard, setter, (value, kind.as_ptr()))
        };
        if written != YES {
            return Err(error());
        }
        Ok(())
    })
}

/// The text on the pasteboard, if it holds text.
pub(crate) fn pasteboard_text() -> Result<Option<String>, Error> {
    autoreleasepool(|| {
        let pasteboard = general()?;
        let kind = string(TEXT).ok_or_else(|| failed("reading the pasteboard"))?;
        // SAFETY: a pasteboard method taking a live type, returning an
        // autoreleased string (or nil), read within the pool.
        unsafe {
            let text: *mut Object = send(pasteboard, c"stringForType:", (kind.as_ptr(),));
            Ok((!text.is_null()).then(|| read_string(text)))
        }
    })
}

/// The general pasteboard.
fn general() -> Result<*mut Object, Error> {
    let error = || failed("opening the pasteboard");
    let class = class(c"NSPasteboard").ok_or_else(error)?;
    // SAFETY: a class method returning the shared pasteboard.
    let pasteboard: *mut Object = unsafe { send(class.as_ptr(), c"generalPasteboard", ()) };
    if pasteboard.is_null() {
        return Err(error());
    }
    Ok(pasteboard)
}
