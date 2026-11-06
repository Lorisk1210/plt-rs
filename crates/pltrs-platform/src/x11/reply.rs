//! Parsers for the replies this crate reads.
//!
//! Every reply starts with a 32-byte header (type 1, a detail byte, the
//! sequence number and the length of the data after the header); the
//! connection hands over complete replies, header included.

use super::wire::Reader;
use crate::error::Error;

/// Length of the fixed reply header.
const HEADER_LEN: usize = 32;

/// The atom of an `InternAtom` reply.
pub(crate) fn intern_atom(reply: &[u8]) -> Result<u32, Error> {
    Reader::at(reply, 8).u32()
}

/// The major opcode of a `QueryExtension` reply, `None` if the extension
/// is absent.
pub(crate) fn query_extension(reply: &[u8]) -> Result<Option<u8>, Error> {
    let mut r = Reader::at(reply, 8);
    let present = r.u8()? != 0;
    let major_opcode = r.u8()?;
    Ok(present.then_some(major_opcode))
}

/// The maximum request length (in 4-byte units) of a `BigReqEnable` reply.
pub(crate) fn big_requests_enable(reply: &[u8]) -> Result<u32, Error> {
    Reader::at(reply, 8).u32()
}

/// A property value from a `GetProperty` reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Property<'a> {
    /// The property type (0 if the property does not exist).
    pub(crate) kind: u32,
    /// 8, 16 or 32 (0 if the property does not exist).
    pub(crate) format: u8,
    /// The raw value bytes.
    pub(crate) value: &'a [u8],
}

/// Parses a `GetProperty` reply.
pub(crate) fn property(reply: &[u8]) -> Result<Property<'_>, Error> {
    let format = Reader::at(reply, 1).u8()?;
    let mut r = Reader::at(reply, 8);
    let kind = r.u32()?;
    r.skip(4)?; // bytes after
    let units = usize::try_from(r.u32()?).map_err(|_| Error::Malformed("huge property"))?;
    let len = units
        .checked_mul(usize::from(format / 8))
        .ok_or(Error::Malformed("huge property"))?;
    let value = Reader::at(reply, HEADER_LEN).bytes(len)?;
    Ok(Property {
        kind,
        format,
        value,
    })
}

/// The server's version in a RENDER `QueryVersion` reply.
pub(crate) fn render_version(reply: &[u8]) -> Result<(u32, u32), Error> {
    let mut r = Reader::at(reply, 8);
    Ok((r.u32()?, r.u32()?))
}

/// The `PICTFORMAT` of 32-bit ARGB, 8 bits a channel with alpha highest,
/// among those of a RENDER `QueryPictFormats` reply, if the server has it.
pub(crate) fn argb32_format(reply: &[u8]) -> Result<Option<u32>, Error> {
    /// `PictTypeDirect`: true colour, channels at shifts and masks.
    const DIRECT: u8 = 1;
    /// Shift and mask of red, green, blue and alpha in ARGB32.
    const ARGB32: [(u16, u16); 4] = [(16, 0xff), (8, 0xff), (0, 0xff), (24, 0xff)];
    let count = Reader::at(reply, 8).u32()?;
    let mut r = Reader::at(reply, HEADER_LEN);
    for _ in 0..count {
        let id = r.u32()?;
        let (kind, depth) = (r.u8()?, r.u8()?);
        r.skip(2)?;
        let mut channels = [(0, 0); 4];
        for channel in &mut channels {
            *channel = (r.u16()?, r.u16()?);
        }
        // The colormap of indexed formats.
        r.skip(4)?;
        if kind == DIRECT && depth == 32 && channels == ARGB32 {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// The pixel data of a `GetImage` reply.
pub(crate) fn image_data(reply: &[u8]) -> Result<&[u8], Error> {
    reply
        .get(HEADER_LEN..)
        .ok_or(Error::Malformed("truncated GetImage reply"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(detail: u8, fields: &[u32], data: &[u8]) -> Vec<u8> {
        let mut out = vec![1, detail, 0, 0];
        out.extend(((data.len() / 4) as u32).to_le_bytes());
        for field in fields {
            out.extend(field.to_le_bytes());
        }
        out.resize(HEADER_LEN, 0);
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn simple_replies() {
        assert_eq!(intern_atom(&reply(0, &[0x1a5], &[])), Ok(0x1a5));
        assert_eq!(
            big_requests_enable(&reply(0, &[0x0040_0000], &[])),
            Ok(0x0040_0000)
        );
        assert_eq!(
            query_extension(&reply(0, &[0x0000_8301], &[])),
            Ok(Some(0x83))
        );
        assert_eq!(query_extension(&reply(0, &[0], &[])), Ok(None));
        assert!(intern_atom(&[1, 0, 0]).is_err());
    }

    #[test]
    fn property_values() {
        let text = reply(8, &[31, 0, 5], b"hello\0\0\0");
        let property = property(&text).unwrap();
        assert_eq!(
            (property.kind, property.format, property.value),
            (31, 8, &b"hello"[..])
        );
        let words = reply(32, &[6, 0, 2], &[1, 0, 0, 0, 2, 0, 0, 0]);
        assert_eq!(super::property(&words).unwrap().value.len(), 8);
        let missing = reply(0, &[0, 0, 0], &[]);
        assert_eq!(super::property(&missing).unwrap().kind, 0);
        let lying = reply(32, &[6, 0, 100], &[0; 8]);
        assert!(super::property(&lying).is_err());
    }

    /// A `PICTFORMINFO`: id, type, depth and the channels' shifts and
    /// masks (red, green, blue, alpha).
    fn pict_format(id: u32, kind: u8, depth: u8, channels: [(u16, u16); 4]) -> Vec<u8> {
        let mut out = id.to_le_bytes().to_vec();
        out.extend([kind, depth, 0, 0]);
        for (shift, mask) in channels {
            out.extend(shift.to_le_bytes());
            out.extend(mask.to_le_bytes());
        }
        out.extend([0; 4]);
        out
    }

    #[test]
    fn render_replies() {
        assert_eq!(render_version(&reply(0, &[0, 11], &[])), Ok((0, 11)));
        let rgb24 = pict_format(0x20, 1, 24, [(16, 0xff), (8, 0xff), (0, 0xff), (0, 0)]);
        let a8 = pict_format(0x21, 1, 8, [(0, 0), (0, 0), (0, 0), (0, 0xff)]);
        let argb32 = pict_format(0x22, 1, 32, [(16, 0xff), (8, 0xff), (0, 0xff), (24, 0xff)]);
        let formats = [rgb24.clone(), a8, argb32].concat();
        assert_eq!(argb32_format(&reply(0, &[3], &formats)), Ok(Some(0x22)));
        assert_eq!(argb32_format(&reply(0, &[1], &rgb24)), Ok(None));
        assert!(
            argb32_format(&reply(0, &[1000], &rgb24)).is_err(),
            "more formats than the reply holds"
        );
    }

    #[test]
    fn image_reply_data() {
        let image = reply(24, &[0x21], &[1, 2, 3, 4]);
        assert_eq!(image_data(&image), Ok(&[1, 2, 3, 4][..]));
        assert!(image_data(&[1, 24]).is_err());
    }
}
