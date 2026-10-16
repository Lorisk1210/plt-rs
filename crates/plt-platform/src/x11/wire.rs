//! Little-endian encoding helpers for the X11 wire format.
//!
//! The connection is opened in little-endian mode (`'l'`), so every
//! multi-byte field is little-endian regardless of the host.

use crate::error::Error;

/// Padding needed to bring `len` to a multiple of four.
pub(crate) const fn pad4(len: usize) -> usize {
    (4 - len % 4) % 4
}

/// Appends little-endian fields to a request buffer.
pub(crate) trait WireWrite {
    /// Appends one byte.
    fn put_u8(&mut self, value: u8);
    /// Appends a little-endian `u16`.
    fn put_u16(&mut self, value: u16);
    /// Appends a little-endian `i16`.
    fn put_i16(&mut self, value: i16);
    /// Appends a little-endian `u32`.
    fn put_u32(&mut self, value: u32);
    /// Appends `count` zero bytes.
    fn put_zeros(&mut self, count: usize);
    /// Appends `bytes` followed by zero padding to a multiple of four.
    fn put_padded(&mut self, bytes: &[u8]);
}

impl WireWrite for Vec<u8> {
    fn put_u8(&mut self, value: u8) {
        self.push(value);
    }

    fn put_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_i16(&mut self, value: i16) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_u32(&mut self, value: u32) {
        self.extend_from_slice(&value.to_le_bytes());
    }

    fn put_zeros(&mut self, count: usize) {
        self.resize(self.len() + count, 0);
    }

    fn put_padded(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
        self.put_zeros(pad4(bytes.len()));
    }
}

/// Writes a request header for a request whose body (everything after
/// the 4-byte header, before padding) is `body_len` bytes long.
///
/// Requests longer than 2¹⁶ words use the BIG-REQUESTS form (zero length
/// field followed by a 32-bit length); the connection only sends those
/// after enabling the extension.
pub(crate) fn put_request_header(buf: &mut Vec<u8>, opcode: u8, detail: u8, body_len: usize) {
    let words = (4 + body_len + pad4(body_len)) / 4;
    buf.put_u8(opcode);
    buf.put_u8(detail);
    match u16::try_from(words) {
        Ok(words) => buf.put_u16(words),
        Err(_) => {
            buf.put_u16(0);
            buf.put_u32(u32::try_from(words + 1).unwrap_or(u32::MAX));
        }
    }
}

/// Converts `u32`s to their little-endian bytes (32-bit property data).
pub(crate) fn u32s_to_bytes(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// A bounds-checked little-endian reader over a server message.
#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Reads `data` from the start.
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Reads `data` starting at byte `pos`.
    pub(crate) const fn at(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    /// Current offset.
    pub(crate) const fn position(&self) -> usize {
        self.pos
    }

    /// Takes the next `len` bytes.
    pub(crate) fn bytes(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(len).ok_or(Error::Malformed("length overflow"))?;
        let bytes = self.data.get(self.pos..end).ok_or(Error::Malformed("truncated message"))?;
        self.pos = end;
        Ok(bytes)
    }

    /// Skips `len` bytes.
    pub(crate) fn skip(&mut self, len: usize) -> Result<(), Error> {
        self.bytes(len).map(drop)
    }

    /// Reads one byte.
    pub(crate) fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.bytes(1)?[0])
    }

    /// Reads a little-endian `u16`.
    pub(crate) fn u16(&mut self) -> Result<u16, Error> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// Reads a little-endian `i16`.
    pub(crate) fn i16(&mut self) -> Result<i16, Error> {
        let b = self.bytes(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    /// Reads a little-endian `u32`.
    pub(crate) fn u32(&mut self) -> Result<u32, Error> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding() {
        assert_eq!(pad4(0), 0);
        assert_eq!(pad4(1), 3);
        assert_eq!(pad4(2), 2);
        assert_eq!(pad4(3), 1);
        assert_eq!(pad4(4), 0);
    }

    #[test]
    fn writes_little_endian_fields() {
        let mut buf = Vec::new();
        buf.put_u8(1);
        buf.put_u16(0x0302);
        buf.put_i16(-2);
        buf.put_u32(0x0807_0605);
        buf.put_padded(b"abcde");
        assert_eq!(
            buf,
            [1, 2, 3, 0xfe, 0xff, 5, 6, 7, 8, b'a', b'b', b'c', b'd', b'e', 0, 0, 0]
        );
    }

    #[test]
    fn request_header_normal_and_big() {
        let mut buf = Vec::new();
        put_request_header(&mut buf, 8, 0, 4);
        assert_eq!(buf, [8, 0, 2, 0]);
        let mut buf = Vec::new();
        put_request_header(&mut buf, 72, 2, 3);
        assert_eq!(buf, [72, 2, 2, 0], "3 body bytes pad to one word");
        let mut buf = Vec::new();
        put_request_header(&mut buf, 72, 2, 0x4_0000);
        // 0x10001 words do not fit in 16 bits: BIG-REQUESTS form, length
        // counts the extra length word too.
        assert_eq!(buf, [72, 2, 0, 0, 0x02, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn reader_is_bounds_checked() {
        let data = [1, 2, 3, 4, 5, 6, 7];
        let mut reader = Reader::new(&data);
        assert_eq!(reader.u8(), Ok(1));
        assert_eq!(reader.u16(), Ok(0x0302));
        assert_eq!(reader.u32(), Ok(0x0706_0504));
        assert_eq!(reader.position(), 7);
        assert_eq!(reader.u8(), Err(Error::Malformed("truncated message")));
        let mut reader = Reader::at(&data, 5);
        assert_eq!(reader.i16(), Ok(0x0706));
        assert!(Reader::new(&data).skip(usize::MAX).is_err());
    }

    #[test]
    fn u32_property_bytes() {
        assert_eq!(u32s_to_bytes(&[1, 0x0102_0304]), [1, 0, 0, 0, 4, 3, 2, 1]);
    }
}
