//! The general pasteboard: what it holds, and the user's items, kept
//! aside while the tests borrow it.

use std::ffi::c_void;

use super::runtime::{Id, Object, autoreleasepool, send};
use super::{class_named, ns_string, read_string};

/// The general pasteboard.
///
/// # Safety
///
/// A pool is in place.
unsafe fn general_pasteboard() -> *mut Object {
    // SAFETY: a class method returning the shared pasteboard.
    unsafe { send(class_named(c"NSPasteboard"), c"generalPasteboard", ()) }
}

/// The objects of the `NSArray` `array`.
///
/// # Safety
///
/// `array` is a live `NSArray` (or nil), and a pool is in place; the
/// objects live as long as the array.
unsafe fn objects(array: *mut Object) -> Vec<*mut Object> {
    if array.is_null() {
        return Vec::new();
    }
    // SAFETY: the caller's guarantees; indices below the count.
    unsafe {
        let count: usize = send(array, c"count", ());
        (0..count)
            .map(|index| send(array, c"objectAtIndex:", (index,)))
            .collect()
    }
}

/// The bytes of the `NSData` `data`.
///
/// # Safety
///
/// `data` is a live `NSData`.
unsafe fn bytes(data: *mut Object) -> Vec<u8> {
    // SAFETY: the data's own buffer and length, copied at once.
    unsafe {
        let length: usize = send(data, c"length", ());
        if length == 0 {
            return Vec::new();
        }
        let start: *const u8 = send(data, c"bytes", ());
        std::slice::from_raw_parts(start, length).to_vec()
    }
}

/// The data the general pasteboard holds of the type `kind` (a UTI).
pub(crate) fn pasteboard_data(kind: &str) -> Option<Vec<u8>> {
    autoreleasepool(|| {
        // SAFETY: a pasteboard getter taking a live string and returning
        // autoreleased data (or nil), copied within the pool.
        unsafe {
            let data: *mut Object = send(general_pasteboard(), c"dataForType:", (ns_string(kind),));
            (!data.is_null()).then(|| bytes(data))
        }
    })
}

/// The general pasteboard's items (their types and data), put back when
/// dropped: the tests borrow the user's clipboard.
#[derive(Debug)]
pub(crate) struct SavedPasteboard {
    items: Vec<Vec<(String, Vec<u8>)>>,
}

impl SavedPasteboard {
    /// Keeps what the pasteboard holds.
    pub(crate) fn save() -> Self {
        autoreleasepool(|| {
            // SAFETY: the pasteboard's autoreleased items, their types
            // and data, read within the pool.
            unsafe {
                let items = objects(send(general_pasteboard(), c"pasteboardItems", ()));
                let items = items
                    .into_iter()
                    .map(|item| {
                        objects(send(item, c"types", ()))
                            .into_iter()
                            .filter_map(|kind| {
                                let data: *mut Object = send(item, c"dataForType:", (kind,));
                                (!data.is_null()).then(|| (read_string(kind), bytes(data)))
                            })
                            .collect()
                    })
                    .collect();
                Self { items }
            }
        })
    }
}

impl Drop for SavedPasteboard {
    fn drop(&mut self) {
        autoreleasepool(|| {
            // SAFETY: new items owned here and retained by the array
            // they go into, given autoreleased data and types; the
            // pasteboard copies what it is given.
            unsafe {
                let pasteboard = general_pasteboard();
                send::<_, i64>(pasteboard, c"clearContents", ());
                let array: *mut Object = send(class_named(c"NSMutableArray"), c"array", ());
                for saved in &self.items {
                    let item = Id::owned(send(class_named(c"NSPasteboardItem"), c"new", ()))
                        .expect("a pasteboard item");
                    for (kind, data) in saved {
                        let data: *mut Object = send(
                            class_named(c"NSData"),
                            c"dataWithBytes:length:",
                            (data.as_ptr().cast::<c_void>(), data.len()),
                        );
                        send::<_, i8>(item.as_ptr(), c"setData:forType:", (data, ns_string(kind)));
                    }
                    send::<_, ()>(array, c"addObject:", (item.as_ptr(),));
                }
                send::<_, i8>(pasteboard, c"writeObjects:", (array,));
            }
        });
    }
}
