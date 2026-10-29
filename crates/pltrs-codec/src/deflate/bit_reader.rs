//! LSB-first bit input for the DEFLATE decoder.

use super::error::InflateError;

/// Reads bits least-significant first from a byte slice through a 64-bit
/// buffer.
///
/// Bits of `buffer` above `available` are either zero or copies of input bits
/// that have not been counted yet. Peeking past `available` is therefore
/// harmless: lookups see either real upcoming bits or zero padding at the end
/// of the input, and [`consume`](Self::consume) / [`take`](Self::take) refuse
/// to go past the real data.
#[derive(Debug)]
pub(crate) struct BitReader<'a> {
    input: &'a [u8],
    /// Index of the first input byte not yet (fully) loaded into `buffer`.
    position: usize,
    buffer: u64,
    /// Number of valid bits at the bottom of `buffer`.
    available: u32,
}

impl<'a> BitReader<'a> {
    pub(crate) fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
            buffer: 0,
            available: 0,
        }
    }

    /// Tops the buffer up to at least 56 bits, or to whatever input is left.
    pub(crate) fn refill(&mut self) {
        let word = self
            .input
            .get(self.position..)
            .and_then(<[u8]>::first_chunk::<8>);
        if let Some(word) = word {
            // Branch-free refill: load eight bytes, keep as many whole bytes as
            // fit. The partial byte that sticks out above `available` is real
            // input and will be OR-ed in again, identically, next time.
            self.buffer |= u64::from_le_bytes(*word) << self.available;
            self.position += ((63 - self.available) / 8) as usize;
            self.available |= 0b11_1000;
        } else {
            while self.available < 56 {
                let Some(&byte) = self.input.get(self.position) else {
                    break;
                };
                self.buffer |= u64::from(byte) << self.available;
                self.position += 1;
                self.available += 8;
            }
        }
    }

    /// The buffered bits, for table lookups.
    pub(crate) fn peek(&self) -> u64 {
        self.buffer
    }

    /// Whether the whole input has been loaded and fewer than `bits` bits
    /// remain, i.e. a lookup needing that many bits saw padding past the end.
    pub(crate) fn is_short_of(&self, bits: u32) -> bool {
        self.available < bits && self.position >= self.input.len()
    }

    /// Discards `count` bits that a lookup has already interpreted.
    pub(crate) fn consume(&mut self, count: u32) -> Result<(), InflateError> {
        if count > self.available {
            return Err(InflateError::UnexpectedEof);
        }
        self.buffer >>= count;
        self.available -= count;
        Ok(())
    }

    /// Removes and returns the next `count` (at most 32) buffered bits.
    pub(crate) fn take(&mut self, count: u32) -> Result<u32, InflateError> {
        if count > self.available {
            return Err(InflateError::UnexpectedEof);
        }
        let value = (self.buffer & ((1 << count) - 1)) as u32;
        self.buffer >>= count;
        self.available -= count;
        Ok(value)
    }

    /// Reads `count` (at most 32) bits, refilling first if needed.
    pub(crate) fn read(&mut self, count: u32) -> Result<u32, InflateError> {
        if self.available < count {
            self.refill();
        }
        self.take(count)
    }

    /// Skips to the next byte boundary and hands buffered whole bytes back to
    /// the input so that byte-aligned data can be read with
    /// [`read_bytes`](Self::read_bytes).
    pub(crate) fn align_to_byte(&mut self) {
        let whole_bytes = (self.available / 8) as usize;
        self.position -= whole_bytes;
        self.buffer = 0;
        self.available = 0;
    }

    /// Reads `count` raw bytes; only valid right after
    /// [`align_to_byte`](Self::align_to_byte).
    pub(crate) fn read_bytes(&mut self, count: usize) -> Result<&'a [u8], InflateError> {
        debug_assert_eq!(self.available, 0, "read_bytes requires byte alignment");
        let bytes = self
            .input
            .get(self.position..)
            .and_then(|rest| rest.get(..count))
            .ok_or(InflateError::UnexpectedEof)?;
        self.position += count;
        Ok(bytes)
    }

    /// Number of input bytes consumed so far; a partially consumed byte
    /// counts as consumed.
    pub(crate) fn consumed(&self) -> usize {
        self.position - (self.available / 8) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bits_lsb_first() {
        let mut reader = BitReader::new(&[0b1010_1100, 0xFF]);
        assert_eq!(reader.read(2), Ok(0b00));
        assert_eq!(reader.read(3), Ok(0b011));
        assert_eq!(reader.read(5), Ok(0b11_101));
        assert_eq!(reader.consumed(), 2);
        assert_eq!(reader.read(6), Ok(0b11_1111));
        assert_eq!(reader.read(1), Err(InflateError::UnexpectedEof));
    }

    #[test]
    fn long_inputs_refill_in_words() {
        let input: Vec<u8> = (0..40).collect();
        let mut reader = BitReader::new(&input);
        for expected in 0..40 {
            assert_eq!(reader.read(8), Ok(expected));
        }
        assert_eq!(reader.consumed(), 40);
    }

    #[test]
    fn alignment_returns_buffered_bytes() {
        let input: Vec<u8> = (1..=20).collect();
        let mut reader = BitReader::new(&input);
        assert_eq!(reader.read(3), Ok(1));
        reader.align_to_byte();
        assert_eq!(reader.consumed(), 1);
        assert_eq!(reader.read_bytes(3), Ok(&input[1..4]));
        assert_eq!(reader.read(8), Ok(5));
        reader.align_to_byte();
        assert_eq!(reader.read_bytes(15), Ok(&input[5..]));
        assert_eq!(reader.read_bytes(1), Err(InflateError::UnexpectedEof));
    }
}
