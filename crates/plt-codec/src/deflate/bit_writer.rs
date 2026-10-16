//! LSB-first bit output for the DEFLATE encoder.

/// Appends bits, least-significant first, to a byte vector.
#[derive(Debug)]
pub(crate) struct BitWriter<'a> {
    out: &'a mut Vec<u8>,
    /// Pending bits not yet written to `out`.
    buffer: u64,
    /// Number of pending bits; always below 32 between calls.
    count: u32,
}

impl<'a> BitWriter<'a> {
    pub(crate) fn new(out: &'a mut Vec<u8>) -> Self {
        Self {
            out,
            buffer: 0,
            count: 0,
        }
    }

    /// Writes the low `count` (at most 32) bits of `bits`; higher bits must
    /// be zero.
    pub(crate) fn write_bits(&mut self, bits: u32, count: u32) {
        debug_assert!(count <= 32 && (count == 32 || bits >> count == 0));
        self.buffer |= u64::from(bits) << self.count;
        self.count += count;
        if self.count >= 32 {
            self.out
                .extend_from_slice(&(self.buffer as u32).to_le_bytes());
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
            self.out.push(self.buffer as u8);
            self.buffer >>= 8;
            self.count = self.count.saturating_sub(8);
        }
    }

    /// Writes raw bytes; only valid right after
    /// [`align_to_byte`](Self::align_to_byte).
    pub(crate) fn write_aligned_bytes(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.count, 0, "write_aligned_bytes requires byte alignment");
        self.out.extend_from_slice(bytes);
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
        assert_eq!(out, [0b1011_0011, 0x01, 0xEF, 0xCD, 0xAB, 0b11]);
    }
}
