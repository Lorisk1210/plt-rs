//! Requests only the tests send, to drive a real server.

use super::{Request, request};
use crate::x11::wire::WireWrite;

/// `SetInputFocus` to `window`, reverting to `PointerRoot`.
pub(crate) fn set_input_focus(window: u32) -> Request {
    request(42, 1, |b| {
        b.put_u32(window);
        b.put_u32(0);
    })
}

/// XTEST `FakeInput`: `kind` is an event code (2 key press, 3 key
/// release, 4 button press, 5 button release, 6 motion).
pub(crate) fn fake_input(major: u8, kind: u8, detail: u8, root: u32, x: i16, y: i16) -> Request {
    request(major, 2, |b| {
        b.put_u8(kind);
        b.put_u8(detail);
        b.put_u16(0);
        b.put_u32(0);
        b.put_u32(root);
        b.put_zeros(8);
        b.put_i16(x);
        b.put_i16(y);
        b.put_zeros(8);
    })
}

/// `DeleteProperty`.
pub(crate) fn delete_property(window: u32, property: u32) -> Request {
    request(19, 0, |b| {
        b.put_u32(window);
        b.put_u32(property);
    })
}

/// `ConvertSelection`: asks the owner of `selection` to put it, converted
/// to `target`, into `property` of `requestor`.
pub(crate) fn convert_selection(
    requestor: u32,
    selection: u32,
    target: u32,
    property: u32,
) -> Request {
    request(24, 0, |b| {
        b.put_u32(requestor);
        b.put_u32(selection);
        b.put_u32(target);
        b.put_u32(property);
        b.put_u32(0);
    })
}
