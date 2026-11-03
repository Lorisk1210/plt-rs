//! The bit-level writing of OBU headers: `f(n)` fields, most significant
//! bit first, and the `leb128` sizes of OBUs (specification section 4.10).

/// Bits written most significant first into bytes.
#[derive(Debug, Default)]
pub(crate) struct BitWriter {
    bytes: Vec<u8>,
    /// Bits used in the last byte, `0` when it is complete.
    used: u32,
}

impl BitWriter {
    /// Writes the `bits` low bits of `value`, most significant first
    /// (`f(n)`).
    pub(crate) fn bits(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.bit((value >> i) & 1 == 1);
        }
    }

    /// Writes one bit.
    pub(crate) fn bit(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        if bit {
            *self.bytes.last_mut().expect("a byte was pushed") |= 0x80 >> self.used;
        }
        self.used = (self.used + 1) % 8;
    }

    /// Pads with zero bits to a whole byte (`byte_alignment`).
    pub(crate) fn byte_align(&mut self) {
        self.used = 0;
    }

    /// Ends an OBU: a one bit, then zero bits to a whole byte
    /// (`trailing_bits`).
    pub(crate) fn trailing_bits(&mut self) {
        self.bit(true);
        self.byte_align();
    }

    /// The bytes written; a partial last byte is padded with zeros.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// `value` as `leb128()`: seven bits per byte, least significant first,
/// the top bit of each byte but the last set.
pub(crate) fn leb128(mut value: usize) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_written_most_significant_bit_first() {
        let mut writer = BitWriter::default();
        writer.bits(0b101, 3);
        writer.bits(0x1F, 5);
        writer.bit(true);
        writer.trailing_bits();
        writer.bits(0, 0);
        writer.bits(0xA, 4);
        writer.byte_align();
        assert_eq!(writer.into_bytes(), [0b1011_1111, 0b1100_0000, 0b1010_0000]);
    }

    #[test]
    fn sizes_are_little_endian_groups_of_seven_bits() {
        assert_eq!(leb128(0), [0]);
        assert_eq!(leb128(127), [0x7F]);
        assert_eq!(leb128(128), [0x80, 0x01]);
        assert_eq!(leb128(300), [0xAC, 0x02]);
    }
}
