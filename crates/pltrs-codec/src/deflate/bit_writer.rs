//! LSB-first bit output for the DEFLATE encoder.

use pltrs_core::alloc::OutOfMemory;

/// Appends bits, least-significant first, to a byte vector, which grows
/// fallibly: once the memory for more output is refused, nothing more is
/// written and [`finish`](BitWriter::finish) reports it.
#[derive(Debug)]
pub(crate) struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    /// Pending bits not yet written to `out`.
    buffer: u64,
    /// Number of pending bits; always below 32 between calls.
    count: u32,
    /// The allocation refused, if one was.
    refused: Option<OutOfMemory>,
}

impl<'a> BitWriter<'a> {
    pub(crate) fn new(out: &'a mut Vec<u8>) -> Self {
        Self {
            out,
            buffer: 0,
            count: 0,
            refused: None,
        }
    }

    /// Whether `out` has room for `bytes` more, growing it as a `Vec` does
    /// (doubling) if it can, else by `bytes` alone.
    fn room(&mut self, bytes: usize) -> bool {
        if self.refused.is_some() {
            return false;
        }
        if self.out.capacity() - self.out.len() >= bytes {
            return true;
        }
        let doubled = bytes.max(self.out.len());
        if self.out.try_reserve(doubled).is_ok() || self.out.try_reserve_exact(bytes).is_ok() {
            return true;
        }
        self.refused = Some(OutOfMemory::of::<u8>(bytes));
        false
    }

    /// The end of writing: an error when memory for the output was refused.
    pub(crate) fn finish(self) -> Result<(), OutOfMemory> {
        self.refused.map_or(Ok(()), Err)
    }

    /// Writes the low `count` (at most 32) bits of `bits`; higher bits must
    /// be zero.
    pub(crate) fn write_bits(&mut self, bits: u32, count: u32) {
        debug_assert!(count <= 32 && (count == 32 || bits >> count == 0));
        self.buffer |= u64::from(bits) << self.count;
        self.count += count;
        if self.count >= 32 {
            if self.room(4) {
                self.out
                    .extend_from_slice(&(self.buffer as u32).to_le_bytes());
            }
            self.buffer >>= 32;
            self.count -= 32;
        }
    }

    /// Number of bits written into the current, partially filled byte.
    pub(crate) fn bit_offset(&self) -> u32 {
        self.count % 8
    }

    /// Pads with zero bits to the next byte boundary and flushes.
    pub(crate) fn align_to_byte(&mut self) {
        while self.count > 0 {
            if self.room(1) {
                self.out.push(self.buffer as u8);
            }
            self.buffer >>= 8;
            self.count = self.count.saturating_sub(8);
        }
    }

    /// Writes raw bytes; only valid right after
    /// [`align_to_byte`](Self::align_to_byte).
    pub(crate) fn write_aligned_bytes(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.count, 0, "write_aligned_bytes requires byte alignment");
        if self.room(bytes.len()) {
            self.out.extend_from_slice(bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_bits_lsb_first() {
        let mut out = Vec::new();
        let mut writer = BitWriter::new(&mut out);
        writer.write_bits(0b1, 1);
        writer.write_bits(0b01, 2);
        writer.write_bits(0b10110, 5);
        writer.write_bits(0xABCD_EF01, 32);
        writer.write_bits(0b11, 2);
        assert_eq!(writer.bit_offset(), 2);
        writer.align_to_byte();
        assert_eq!(writer.finish(), Ok(()));
        assert_eq!(out, [0b1011_0011, 0x01, 0xEF, 0xCD, 0xAB, 0b11]);
    }

    #[test]
    fn refused_growth_is_reported() {
        let mut out = vec![0_u8; 8];
        let mut writer = BitWriter::new(&mut out);
        assert!(!writer.room(usize::MAX), "more than any system has");
        writer.write_aligned_bytes(&[1, 2, 3]);
        assert_eq!(writer.finish(), Err(OutOfMemory::of::<u8>(usize::MAX)));
        assert_eq!(out.len(), 8, "nothing written once refused");
    }
}
