//! Least-significant-bit-first input for VP8L bitstreams.

/// Reads bits least significant first. Reading past the end yields zero
/// bits and marks the reader as overrun, which the decoder reports as
/// truncated data.
#[derive(Debug)]
pub(super) struct BitReader<'a> {
    data: &'a [u8],
    /// The next byte to load.
    pos: usize,
    /// The loaded bits, the next one lowest.
    buffer: u64,
    /// How many bits are loaded.
    count: u32,
    /// Whether more bits were consumed than the data holds.
    overrun: bool,
}

impl<'a> BitReader<'a> {
    /// A reader at the first bit of `data`.
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            buffer: 0,
            count: 0,
            overrun: false,
        }
    }

    /// Loads whole bytes until 57 or more bits are buffered, or the data
    /// ends.
    fn refill(&mut self) {
        while self.count <= 56 {
            let Some(&byte) = self.data.get(self.pos) else {
                break;
            };
            self.buffer |= u64::from(byte) << self.count;
            self.pos += 1;
            self.count += 8;
        }
    }

    /// The next `count` (at most 32) bits, not consumed; missing bits past
    /// the end read as zeros.
    pub(super) fn peek(&mut self, count: u32) -> u32 {
        debug_assert!(count <= 32);
        if self.count < count {
            self.refill();
        }
        (self.buffer & ((1u64 << count) - 1)) as u32
    }

    /// Consumes `count` (at most 32) bits.
    pub(super) fn consume(&mut self, count: u32) {
        if count > self.count {
            self.refill();
        }
        if count > self.count {
            self.overrun = true;
            self.buffer = 0;
            self.count = 0;
        } else {
            self.buffer >>= count;
            self.count -= count;
        }
    }

    /// Reads a `count`-bit (at most 32) unsigned value.
    pub(super) fn read(&mut self, count: u32) -> u32 {
        let value = self.peek(count);
        self.consume(count);
        value
    }

    /// Reads a `count`-bit value as an index.
    pub(super) fn read_usize(&mut self, count: u32) -> usize {
        self.read(count) as usize
    }

    /// Reads one bit as a flag.
    pub(super) fn flag(&mut self) -> bool {
        self.read(1) == 1
    }

    /// Whether more bits were consumed than the data holds.
    pub(super) fn is_overrun(&self) -> bool {
        self.overrun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_come_least_significant_first() {
        let mut reader = BitReader::new(&[0b1010_1101, 0xFF]);
        assert_eq!(reader.read(1), 1);
        assert_eq!(reader.read(3), 0b110);
        assert_eq!(reader.peek(8), 0b1111_1010);
        assert_eq!(reader.read(12), 0xFFA);
        assert!(!reader.is_overrun());
    }

    #[test]
    fn reading_past_the_end_reads_zeros_and_is_noticed() {
        let mut reader = BitReader::new(&[0xFF]);
        assert_eq!(reader.read(4), 0xF);
        assert_eq!(reader.peek(8), 0xF);
        assert!(!reader.is_overrun());
        assert_eq!(reader.read(8), 0xF);
        assert!(reader.is_overrun());
        assert_eq!(reader.read(3), 0);
    }
}
