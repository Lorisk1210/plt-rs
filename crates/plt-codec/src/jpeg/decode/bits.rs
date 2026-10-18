//! Reading the bits of an entropy-coded segment, most significant first:
//! `0xFF 0x00` stands for an `0xFF` data byte and any other marker ends
//! the data (libjpeg's `jpeg_fill_bit_buffer`). Bits past the end read as
//! zeros, so that codes can be looked ahead, but consuming them is an
//! error: valid data never runs short.

use super::error::DecodeError;
use super::markers::Cursor;
use crate::jpeg::marker::RST0;

/// The bit reader of one scan.
#[derive(Debug)]
pub(super) struct BitReader<'a> {
    data: &'a [u8],
    /// The next byte to fetch.
    pos: usize,
    /// Fetched bits, the oldest highest; `count` of them are valid.
    acc: u64,
    count: u32,
    /// How many of the valid bits are zero padding past the data.
    padding: u32,
    /// Whether fetching stopped at a marker or at the end of the data.
    stopped: bool,
}

impl<'a> BitReader<'a> {
    /// A reader of the entropy-coded data starting at `pos` of `data`.
    pub(super) fn new(data: &'a [u8], pos: usize) -> Self {
        Self {
            data,
            pos,
            acc: 0,
            count: 0,
            padding: 0,
            stopped: false,
        }
    }

    /// The next data byte, or `None` at a marker or the end of the data.
    fn fetch(&mut self) -> Option<u8> {
        let byte = *self.data.get(self.pos)?;
        if byte != 0xFF {
            self.pos += 1;
            return Some(byte);
        }
        let mut next = self.pos + 1;
        while self.data.get(next) == Some(&0xFF) {
            next += 1;
        }
        if self.data.get(next) == Some(&0) {
            self.pos = next + 1;
            Some(0xFF)
        } else {
            None
        }
    }

    /// Tops the buffer up to more than 56 valid bits.
    fn fill(&mut self) {
        while self.count <= 56 {
            let byte = if self.stopped { None } else { self.fetch() };
            if let Some(byte) = byte {
                self.acc = self.acc << 8 | u64::from(byte);
            } else {
                self.stopped = true;
                self.acc <<= 8;
                self.padding += 8;
            }
            self.count += 8;
        }
    }

    /// The next `n` bits (at most 16) without consuming them.
    pub(super) fn peek(&mut self, n: u32) -> u32 {
        if self.count < n {
            self.fill();
        }
        let bits = (self.acc >> (self.count - n)) & ((1 << n) - 1);
        u32::try_from(bits).expect("at most 16 bits")
    }

    /// Consumes `n` bits that have been peeked.
    pub(super) fn consume(&mut self, n: u32) -> Result<(), DecodeError> {
        if n > self.count - self.padding {
            return Err(DecodeError::Truncated);
        }
        self.count -= n;
        Ok(())
    }

    /// The next `n` bits (at most 16) as a number.
    pub(super) fn bits(&mut self, n: u32) -> Result<u32, DecodeError> {
        let value = self.peek(n);
        self.consume(n)?;
        Ok(value)
    }

    /// The next bit.
    pub(super) fn bit(&mut self) -> Result<bool, DecodeError> {
        Ok(self.bits(1)? == 1)
    }

    /// A coefficient of magnitude category `size` (its `size` extra bits,
    /// ones' complement coded when negative; `HUFF_EXTEND`).
    pub(super) fn extend(&mut self, size: u8) -> Result<i32, DecodeError> {
        if size == 0 {
            return Ok(0);
        }
        let size = u32::from(size);
        let value = i32::try_from(self.bits(size)?).expect("at most 16 bits");
        Ok(if value < 1 << (size - 1) {
            value - (1 << size) + 1
        } else {
            value
        })
    }

    /// Skips to the restart marker that must follow (any bytes before it
    /// are discarded, as libjpeg does) and checks its number.
    pub(super) fn restart(&mut self, expected: u8) -> Result<(), DecodeError> {
        let mut cursor = Cursor::at(self.data, self.pos);
        let marker = cursor.next_marker();
        if marker != Ok(RST0 + expected) {
            return Err(DecodeError::MissingRestart { expected });
        }
        *self = Self::new(self.data, cursor.position());
        Ok(())
    }

    /// Where reading markers resumes after the scan: bits still buffered
    /// are padding and are dropped.
    pub(super) fn end(&self) -> usize {
        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stuffed_bytes_and_markers() {
        let data = [0b1010_0000, 0xFF, 0x00, 0x0F, 0xFF, 0xD9];
        let mut reader = BitReader::new(&data, 0);
        assert_eq!(reader.bits(3), Ok(0b101));
        assert_eq!(reader.bits(5), Ok(0));
        assert_eq!(reader.bits(8), Ok(0xFF), "the stuffed zero is dropped");
        assert_eq!(reader.bits(8), Ok(0x0F));
        assert_eq!(reader.peek(8), 0, "the marker reads as zeros");
        assert_eq!(reader.consume(1), Err(DecodeError::Truncated));
        assert_eq!(reader.end(), 4, "positioned at the marker");
    }

    #[test]
    fn extension_follows_huff_extend() {
        let data = [0b0111_1000, 0];
        let mut reader = BitReader::new(&data, 0);
        assert_eq!(reader.extend(3), Ok(-4), "011 is below 100");
        assert_eq!(reader.extend(3), Ok(6), "110");
        assert_eq!(reader.extend(0), Ok(0));
    }

    #[test]
    fn restarts_need_the_right_marker() {
        let data = [0x80, 0xFF, 0xD0, 0x40, 0xFF, 0xD3];
        let mut reader = BitReader::new(&data, 0);
        assert_eq!(reader.bits(1), Ok(1));
        reader.restart(0).unwrap();
        assert_eq!(reader.bits(2), Ok(0b01));
        assert_eq!(
            reader.restart(1),
            Err(DecodeError::MissingRestart { expected: 1 })
        );
    }
}
