//! Requests of the RENDER extension that make ARGB cursors: the version
//! handshake, the picture formats, a picture of a pixmap and a cursor of
//! a picture. Each goes to the extension's major opcode, with the
//! request's minor opcode in the header's detail byte.

use super::{Query, Request, query, request};
use crate::x11::wire::WireWrite;

const QUERY_VERSION: u8 = 0;
const QUERY_PICT_FORMATS: u8 = 1;
const CREATE_PICTURE: u8 = 4;
const FREE_PICTURE: u8 = 7;
const CREATE_CURSOR: u8 = 27;

/// The RENDER version this crate speaks: 0.5 brought `CreateCursor`.
pub(crate) const VERSION: (u32, u32) = (0, 11);

/// `QueryVersion`, offering [`VERSION`]; servers expect it first.
pub(crate) fn query_version(major_opcode: u8) -> Query {
    query(major_opcode, QUERY_VERSION, |b| {
        b.put_u32(VERSION.0);
        b.put_u32(VERSION.1);
    })
}

/// `QueryPictFormats`.
pub(crate) fn query_pict_formats(major_opcode: u8) -> Query {
    query(major_opcode, QUERY_PICT_FORMATS, |_| {})
}

/// `CreatePicture` of `drawable` in `format`, with default attributes.
pub(crate) fn create_picture(
    major_opcode: u8,
    picture: u32,
    drawable: u32,
    format: u32,
) -> Request {
    request(major_opcode, CREATE_PICTURE, |b| {
        b.put_u32(picture);
        b.put_u32(drawable);
        b.put_u32(format);
        b.put_u32(0);
    })
}

/// `FreePicture`.
pub(crate) fn free_picture(major_opcode: u8, picture: u32) -> Request {
    request(major_opcode, FREE_PICTURE, |b| b.put_u32(picture))
}

/// `CreateCursor` showing `picture` with its hot spot at `(x, y)`.
pub(crate) fn create_cursor(
    major_opcode: u8,
    cursor: u32,
    picture: u32,
    (x, y): (u16, u16),
) -> Request {
    request(major_opcode, CREATE_CURSOR, |b| {
        b.put_u32(cursor);
        b.put_u32(picture);
        b.put_u16(x);
        b.put_u16(y);
    })
}

#[cfg(test)]
mod tests {
    use super::super::wire_bytes;
    use super::*;

    #[test]
    fn requests_carry_the_minor_opcode() {
        let version = wire_bytes(query_version(139).encoded());
        assert_eq!(version, [139, 0, 3, 0, 0, 0, 0, 0, 11, 0, 0, 0]);
        assert_eq!(
            wire_bytes(query_pict_formats(139).encoded()),
            [139, 1, 1, 0]
        );
        let picture = wire_bytes(create_picture(139, 7, 8, 9).encoded());
        assert_eq!(&picture[..4], [139, 4, 5, 0]);
        assert_eq!(
            &picture[4..],
            [7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            wire_bytes(free_picture(139, 7).encoded()),
            [139, 7, 2, 0, 7, 0, 0, 0]
        );
        let cursor = wire_bytes(create_cursor(139, 5, 7, (2, 3)).encoded());
        assert_eq!(cursor, [139, 27, 4, 0, 5, 0, 0, 0, 7, 0, 0, 0, 2, 0, 3, 0]);
    }
}
