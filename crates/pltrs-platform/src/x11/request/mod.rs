//! Encoders for the X11 requests this crate sends.
//!
//! Each function returns a [`Request`] (no reply) or a [`Query`] (answered
//! by a reply), so the connection always knows whether to expect one.
//! Window management requests live in [`window`], those of the RENDER
//! extension in [`render`].

pub(crate) mod render;
mod selection;
#[cfg(test)]
pub(crate) mod testing;
mod window;

pub(crate) use selection::{convert_selection, selection_notify, set_selection_owner};
pub(crate) use window::{
    NewWindow, WindowAttribute, WindowClass, change_window_attributes, create_colormap,
    create_window, destroy_window, event_mask, map_window, raise_window, resize_window,
};

use super::wire::{WireWrite, pad4, put_request_header, u32s_to_bytes};

const INTERN_ATOM: u8 = 16;
const CHANGE_PROPERTY: u8 = 18;
const GET_PROPERTY: u8 = 20;
const SEND_EVENT: u8 = 25;
const GET_INPUT_FOCUS: u8 = 43;
const OPEN_FONT: u8 = 45;
const CREATE_PIXMAP: u8 = 53;
const FREE_PIXMAP: u8 = 54;
const CREATE_GC: u8 = 55;
const FREE_GC: u8 = 60;
const PUT_IMAGE: u8 = 72;
const GET_IMAGE: u8 = 73;
const CREATE_GLYPH_CURSOR: u8 = 94;
const QUERY_EXTENSION: u8 = 98;
const GET_KEYBOARD_MAPPING: u8 = 101;
const GET_MODIFIER_MAPPING: u8 = 119;

/// The `ZPixmap` image format: whole pixels, row by row.
const Z_PIXMAP: u8 = 2;
/// `ChangeProperty` mode that replaces the old value.
const PROP_MODE_REPLACE: u8 = 0;
/// Event code of `ClientMessage`.
const CLIENT_MESSAGE: u8 = 33;

/// The body of a request with its opcode and the header's detail byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Encoded {
    opcode: u8,
    detail: u8,
    body: Vec<u8>,
}

impl Encoded {
    /// Bytes on the wire: header (in BIG-REQUESTS form when the length
    /// does not fit 16 bits), body and padding.
    pub(crate) fn wire_len(&self) -> usize {
        let len = 4 + self.body.len() + pad4(self.body.len());
        if len / 4 > usize::from(u16::MAX) {
            len + 4
        } else {
            len
        }
    }

    /// Appends the header only.
    pub(crate) fn write_header(&self, out: &mut Vec<u8>) {
        put_request_header(out, self.opcode, self.detail, self.body.len());
    }

    /// The body, without padding.
    pub(crate) fn body(&self) -> &[u8] {
        &self.body
    }

    /// Number of padding bytes after the body.
    pub(crate) fn padding(&self) -> usize {
        pad4(self.body.len())
    }

    /// Appends the complete request: header, body and padding.
    pub(crate) fn write_to(&self, out: &mut Vec<u8>) {
        self.write_header(out);
        out.extend_from_slice(&self.body);
        out.put_zeros(self.padding());
    }
}

/// A request the server does not answer (unless it fails).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request(Encoded);

/// A request the server answers with a reply (or an error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Query(Encoded);

impl Request {
    /// The encoded request.
    pub(crate) fn encoded(&self) -> &Encoded {
        &self.0
    }
}

impl Query {
    /// The encoded request.
    pub(crate) fn encoded(&self) -> &Encoded {
        &self.0
    }
}

fn build(opcode: u8, detail: u8, write: impl FnOnce(&mut Vec<u8>)) -> Encoded {
    let mut body = Vec::new();
    write(&mut body);
    Encoded {
        opcode,
        detail,
        body,
    }
}

fn request(opcode: u8, detail: u8, write: impl FnOnce(&mut Vec<u8>)) -> Request {
    Request(build(opcode, detail, write))
}

fn query(opcode: u8, detail: u8, write: impl FnOnce(&mut Vec<u8>)) -> Query {
    Query(build(opcode, detail, write))
}

/// `InternAtom`, creating the atom if needed.
pub(crate) fn intern_atom(name: &str) -> Query {
    query(INTERN_ATOM, 0, |b| put_string(b, name.as_bytes()))
}

/// `ChangeProperty` replacing a property with 8-bit data.
pub(crate) fn set_property_bytes(window: u32, property: u32, kind: u32, data: &[u8]) -> Request {
    change_property(window, property, kind, 8, data.len(), data)
}

/// `ChangeProperty` replacing a property with 32-bit data.
pub(crate) fn set_property_u32(window: u32, property: u32, kind: u32, data: &[u32]) -> Request {
    change_property(window, property, kind, 32, data.len(), &u32s_to_bytes(data))
}

fn change_property(
    window: u32,
    property: u32,
    kind: u32,
    format: u8,
    units: usize,
    data: &[u8],
) -> Request {
    request(CHANGE_PROPERTY, PROP_MODE_REPLACE, |b| {
        b.put_u32(window);
        b.put_u32(property);
        b.put_u32(kind);
        b.put_u8(format);
        b.put_zeros(3);
        b.put_u32(u32::try_from(units).unwrap_or(u32::MAX));
        b.put_padded(data);
    })
}

/// `GetProperty` reading up to `max_words` 32-bit units of any type.
pub(crate) fn get_property(window: u32, property: u32, max_words: u32) -> Query {
    property_query(false, window, property, max_words)
}

/// `GetProperty` reading up to `max_words` 32-bit units of any type and
/// deleting the property once all of it is read: how the requestor of a
/// selection takes the value the owner stored (ICCCM section 2.4).
pub(crate) fn take_property(window: u32, property: u32, max_words: u32) -> Query {
    property_query(true, window, property, max_words)
}

fn property_query(delete: bool, window: u32, property: u32, max_words: u32) -> Query {
    query(GET_PROPERTY, u8::from(delete), |b| {
        b.put_u32(window);
        b.put_u32(property);
        b.put_u32(0);
        b.put_u32(0);
        b.put_u32(max_words);
    })
}

/// `SendEvent` of a 32-byte event.
pub(crate) fn send_event(destination: u32, event_mask: u32, event: &[u8; 32]) -> Request {
    request(SEND_EVENT, 0, |b| {
        b.put_u32(destination);
        b.put_u32(event_mask);
        b.extend_from_slice(event);
    })
}

/// A `ClientMessage` event with 32-bit data, for [`send_event`].
pub(crate) fn client_message(window: u32, message_type: u32, data: [u32; 5]) -> [u8; 32] {
    let mut event = Vec::with_capacity(32);
    event.put_u8(CLIENT_MESSAGE);
    event.put_u8(32);
    event.put_u16(0);
    event.put_u32(window);
    event.put_u32(message_type);
    for value in data {
        event.put_u32(value);
    }
    event.try_into().expect("a ClientMessage is 32 bytes")
}

/// `GetInputFocus`, used as a cheap round trip.
pub(crate) fn get_input_focus() -> Query {
    query(GET_INPUT_FOCUS, 0, |_| {})
}

/// `OpenFont`.
pub(crate) fn open_font(font: u32, name: &str) -> Request {
    request(OPEN_FONT, 0, |b| {
        b.put_u32(font);
        put_string(b, name.as_bytes());
    })
}

/// `CreatePixmap` of `depth`, on the screen of `drawable`.
pub(crate) fn create_pixmap(
    pixmap: u32,
    drawable: u32,
    depth: u8,
    (width, height): (u16, u16),
) -> Request {
    request(CREATE_PIXMAP, depth, |b| {
        b.put_u32(pixmap);
        b.put_u32(drawable);
        b.put_u16(width);
        b.put_u16(height);
    })
}

/// `FreePixmap`.
pub(crate) fn free_pixmap(pixmap: u32) -> Request {
    request(FREE_PIXMAP, 0, |b| b.put_u32(pixmap))
}

/// `CreateGC` with default values.
pub(crate) fn create_gc(gc: u32, drawable: u32) -> Request {
    request(CREATE_GC, 0, |b| {
        b.put_u32(gc);
        b.put_u32(drawable);
        b.put_u32(0);
    })
}

/// `FreeGC`.
pub(crate) fn free_gc(gc: u32) -> Request {
    request(FREE_GC, 0, |b| b.put_u32(gc))
}

/// Where a [`put_image`] goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImageTarget {
    /// The window or pixmap.
    pub(crate) drawable: u32,
    /// The graphics context.
    pub(crate) gc: u32,
    /// Depth of the image (that of the drawable).
    pub(crate) depth: u8,
}

/// `PutImage` of a `ZPixmap` image at `(x, y)`; `write_data` appends the
/// pixel rows.
pub(crate) fn put_image(
    target: ImageTarget,
    (x, y, width, height): (i16, i16, u16, u16),
    data_len: usize,
    write_data: impl FnOnce(&mut Vec<u8>),
) -> Request {
    let mut body = Vec::with_capacity(20 + data_len);
    body.put_u32(target.drawable);
    body.put_u32(target.gc);
    body.put_u16(width);
    body.put_u16(height);
    body.put_i16(x);
    body.put_i16(y);
    body.put_u8(0);
    body.put_u8(target.depth);
    body.put_u16(0);
    write_data(&mut body);
    Request(Encoded {
        opcode: PUT_IMAGE,
        detail: Z_PIXMAP,
        body,
    })
}

/// `GetImage` of all planes of a rectangle, in `ZPixmap` format.
pub(crate) fn get_image(drawable: u32, width: u16, height: u16) -> Query {
    query(GET_IMAGE, Z_PIXMAP, |b| {
        b.put_u32(drawable);
        b.put_i16(0);
        b.put_i16(0);
        b.put_u16(width);
        b.put_u16(height);
        b.put_u32(u32::MAX);
    })
}

/// `CreateGlyphCursor` from the cursor font, black on white.
pub(crate) fn create_glyph_cursor(cursor: u32, font: u32, glyph: u16) -> Request {
    request(CREATE_GLYPH_CURSOR, 0, |b| {
        b.put_u32(cursor);
        b.put_u32(font);
        b.put_u32(font);
        b.put_u16(glyph);
        b.put_u16(glyph + 1);
        for channel in [0, 0, 0, u16::MAX, u16::MAX, u16::MAX] {
            b.put_u16(channel);
        }
    })
}

/// `QueryExtension`.
pub(crate) fn query_extension(name: &str) -> Query {
    query(QUERY_EXTENSION, 0, |b| put_string(b, name.as_bytes()))
}

/// `BigReqEnable` of the BIG-REQUESTS extension with major opcode `major`.
pub(crate) fn big_requests_enable(major: u8) -> Query {
    query(major, 0, |_| {})
}

/// `GetKeyboardMapping` for `count` keycodes starting at `first`.
pub(crate) fn get_keyboard_mapping(first: u8, count: u8) -> Query {
    query(GET_KEYBOARD_MAPPING, 0, |b| {
        b.put_u8(first);
        b.put_u8(count);
        b.put_u16(0);
    })
}

/// `GetModifierMapping`.
pub(crate) fn get_modifier_mapping() -> Query {
    query(GET_MODIFIER_MAPPING, 0, |_| {})
}

/// Appends a 16-bit length, two unused bytes and the padded string.
fn put_string(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.put_u16(u16::try_from(bytes.len()).unwrap_or(u16::MAX));
    buf.put_u16(0);
    buf.put_padded(&bytes[..bytes.len().min(usize::from(u16::MAX))]);
}

/// The complete wire encoding of a request.
#[cfg(test)]
fn wire_bytes(encoded: &Encoded) -> Vec<u8> {
    let mut out = Vec::new();
    encoded.write_to(&mut out);
    assert_eq!(out.len(), encoded.wire_len());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_atom_pads_name() {
        let bytes = wire_bytes(intern_atom("UTF8_STRING").encoded());
        assert_eq!(&bytes[..8], [16, 0, 5, 0, 11, 0, 0, 0]);
        assert_eq!(&bytes[8..19], b"UTF8_STRING");
        assert_eq!(bytes[19], 0);
    }

    #[test]
    fn change_property_counts_units() {
        let bytes = wire_bytes(set_property_u32(7, 8, 6, &[1, 2]).encoded());
        #[rustfmt::skip]
        let expected = [
            18, 0, 8, 0,  7, 0, 0, 0,  8, 0, 0, 0,  6, 0, 0, 0,
            32, 0, 0, 0,  2, 0, 0, 0,  1, 0, 0, 0,  2, 0, 0, 0,
        ];
        assert_eq!(bytes, expected);
        let bytes = wire_bytes(set_property_bytes(7, 39, 31, b"abcde").encoded());
        assert_eq!(bytes[2], 8, "6 fixed words + 2 words of padded data");
        assert_eq!(&bytes[16..20], [8, 0, 0, 0]);
        assert_eq!(&bytes[20..24], [5, 0, 0, 0]);
        assert_eq!(&bytes[24..32], b"abcde\0\0\0");
    }

    #[test]
    fn pixmaps_carry_their_depth() {
        let bytes = wire_bytes(create_pixmap(5, 6, 32, (48, 40)).encoded());
        assert_eq!(bytes, [53, 32, 4, 0, 5, 0, 0, 0, 6, 0, 0, 0, 48, 0, 40, 0]);
        assert_eq!(
            wire_bytes(free_pixmap(5).encoded()),
            [54, 0, 2, 0, 5, 0, 0, 0]
        );
    }

    #[test]
    fn client_message_layout() {
        let event = client_message(0x42, 0x99, [1, 2, 3, 4, 5]);
        assert_eq!(&event[..12], [33, 32, 0, 0, 0x42, 0, 0, 0, 0x99, 0, 0, 0]);
        assert_eq!(&event[12..16], [1, 0, 0, 0]);
        assert_eq!(&event[28..], [5, 0, 0, 0]);
        let bytes = wire_bytes(send_event(0x42, 0, &event).encoded());
        assert_eq!(&bytes[..4], [25, 0, 11, 0]);
        assert_eq!(&bytes[12..], event);
    }

    #[test]
    fn put_image_header_and_big_form() {
        let target = ImageTarget {
            drawable: 5,
            gc: 6,
            depth: 24,
        };
        let request = put_image(target, (1, -2, 2, 1), 8, |b| b.extend_from_slice(&[9; 8]));
        let bytes = wire_bytes(request.encoded());
        #[rustfmt::skip]
        let expected = [
            72, 2, 8, 0,  5, 0, 0, 0,  6, 0, 0, 0,  2, 0, 1, 0,
            1, 0, 0xfe, 0xff,  0, 24, 0, 0,  9, 9, 9, 9,  9, 9, 9, 9,
        ];
        assert_eq!(bytes, expected);
        let big = put_image(target, (0, 0, 512, 128), 512 * 128 * 4, |b| {
            b.resize(b.len() + 512 * 128 * 4, 0);
        });
        let bytes = wire_bytes(big.encoded());
        assert_eq!(&bytes[..4], [72, 2, 0, 0], "BIG-REQUESTS form");
        let words = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(words as usize * 4, bytes.len());
    }

    #[test]
    fn small_requests() {
        assert_eq!(wire_bytes(get_input_focus().encoded()), [43, 0, 1, 0]);
        assert_eq!(
            wire_bytes(get_keyboard_mapping(8, 248).encoded()),
            [101, 0, 2, 0, 8, 248, 0, 0]
        );
        assert_eq!(
            wire_bytes(big_requests_enable(133).encoded()),
            [133, 0, 1, 0]
        );
        let cursor = wire_bytes(create_glyph_cursor(3, 4, 34).encoded());
        assert_eq!(&cursor[..2], [94, 0]);
        assert_eq!(&cursor[16..20], [34, 0, 35, 0]);
        assert_eq!(&cursor[26..32], [0xff; 6]);
    }
}
