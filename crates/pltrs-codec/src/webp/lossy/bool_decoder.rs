//! VP8's boolean entropy decoder (RFC 6386 section 7) with libwebp's
//! arithmetic (`bit_reader_utils`), so that every decoded bit, and the
//! point where the data runs out, are libwebp's.

/// Decodes bits whose probabilities of being zero are known.
#[derive(Debug)]
pub(super) struct BoolDecoder<'a> {
    data: &'a [u8],
    /// The next byte to load.
    pos: usize,
    /// The loaded bits; the current 8-bit window starts at bit `bits`.
    value: u64,
    /// How many loaded bits lie below the window (negative: too few).
    bits: i32,
    /// The current range minus one.
    range: u32,
    /// Whether a byte past the end was needed.
    eof: bool,
}

impl<'a> BoolDecoder<'a> {
    /// A decoder at the start of `data`.
    pub(super) fn new(data: &'a [u8]) -> Self {
        let mut decoder = Self {
            data,
            pos: 0,
            value: 0,
            bits: -8,
            range: 255 - 1,
            eof: false,
        };
        decoder.load();
        decoder
    }

    /// Loads the next byte; past the end, one zero byte and then nothing
    /// (libwebp's `VP8LoadFinalBytes`).
    fn load(&mut self) {
        if let Some(&byte) = self.data.get(self.pos) {
            self.pos += 1;
            self.bits += 8;
            self.value = (self.value << 8) | u64::from(byte);
        } else if !self.eof {
            self.value <<= 8;
            self.bits += 8;
            self.eof = true;
        } else {
            self.bits = 0;
        }
    }

    /// Whether the decoder needed bytes past the end of its data.
    pub(super) fn is_eof(&self) -> bool {
        self.eof
    }

    /// A bit that is zero with probability `probability / 256`.
    pub(super) fn bit(&mut self, probability: u8) -> bool {
        let mut range = self.range;
        if self.bits < 0 {
            self.load();
        }
        let pos = self.bits as u32;
        let split = (range * u32::from(probability)) >> 8;
        let value = (self.value >> pos) as u32;
        let bit = value > split;
        if bit {
            range -= split;
            self.value -= u64::from(split + 1) << pos;
        } else {
            range = split + 1;
        }
        let shift = 7 ^ range.ilog2();
        range <<= shift;
        self.bits -= shift as i32;
        self.range = range - 1;
        bit
    }

    /// `v` or `-v`, by an even bit (libwebp's `VP8GetSigned`).
    pub(super) fn signed(&mut self, v: i32) -> i32 {
        if self.bits < 0 {
            self.load();
        }
        let pos = self.bits as u32;
        let split = self.range >> 1;
        let value = (self.value >> pos) as u32;
        let mask = (split.wrapping_sub(value) as i32) >> 31;
        self.bits -= 1;
        self.range = self.range.wrapping_add(mask as u32) | 1;
        self.value -= u64::from((split + 1) & mask as u32) << pos;
        (v ^ mask) - mask
    }

    /// An unsigned `bits`-bit value of even bits, most significant first.
    pub(super) fn literal(&mut self, bits: u32) -> u32 {
        (0..bits).fold(0, |value, _| (value << 1) | u32::from(self.bit(0x80)))
    }

    /// A flag of one even bit.
    pub(super) fn flag(&mut self) -> bool {
        self.bit(0x80)
    }

    /// A `bits`-bit magnitude followed by a sign bit.
    pub(super) fn signed_literal(&mut self, bits: u32) -> i32 {
        let magnitude = self.literal(bits) as i32;
        if self.flag() { -magnitude } else { magnitude }
    }

    /// A flag, then a signed value if it is set, else zero.
    pub(super) fn optional_signed(&mut self, bits: u32) -> i32 {
        if self.flag() {
            self.signed_literal(bits)
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal boolean encoder (RFC 6386 section 7.3), to make streams.
    struct Encoder {
        out: Vec<u8>,
        range: u32,
        bottom: u32,
        bit_count: i32,
    }

    impl Encoder {
        fn new() -> Self {
            Self {
                out: Vec::new(),
                range: 255,
                bottom: 0,
                bit_count: 24,
            }
        }

        fn add_one_to_output(&mut self) {
            for byte in self.out.iter_mut().rev() {
                if *byte == 255 {
                    *byte = 0;
                } else {
                    *byte += 1;
                    break;
                }
            }
        }

        fn put(&mut self, probability: u8, bit: bool) {
            let split = 1 + (((self.range - 1) * u32::from(probability)) >> 8);
            if bit {
                self.bottom = self.bottom.wrapping_add(split);
                self.range -= split;
            } else {
                self.range = split;
            }
            while self.range < 128 {
                self.range <<= 1;
                if self.bottom & (1 << 31) != 0 {
                    self.add_one_to_output();
                }
                self.bottom <<= 1;
                self.bit_count -= 1;
                if self.bit_count == 0 {
                    self.out.push((self.bottom >> 24) as u8);
                    self.bottom &= (1 << 24) - 1;
                    self.bit_count = 8;
                }
            }
        }

        fn finish(mut self) -> Vec<u8> {
            for _ in 0..32 {
                self.put(128, false);
            }
            self.out
        }
    }

    #[test]
    fn decodes_what_the_reference_encoder_wrote() {
        let mut encoder = Encoder::new();
        let bits: Vec<(u8, bool)> = (0..500u32)
            .map(|i| (((i * 37) % 254 + 1) as u8, (i * 7919) % 3 == 0))
            .collect();
        for &(probability, bit) in &bits {
            encoder.put(probability, bit);
        }
        for value in [5u32, 0, 127] {
            for shift in (0..7).rev() {
                encoder.put(128, (value >> shift) & 1 == 1);
            }
        }
        let data = encoder.finish();
        let mut decoder = BoolDecoder::new(&data);
        for &(probability, bit) in &bits {
            assert_eq!(decoder.bit(probability), bit);
        }
        assert_eq!(decoder.literal(7), 5);
        assert_eq!(decoder.literal(7), 0);
        assert_eq!(decoder.signed(9), -9, "the first bit of 127 is set");
        assert!(!decoder.is_eof());
    }

    #[test]
    fn running_out_of_data_is_noticed() {
        let mut decoder = BoolDecoder::new(&[0x00]);
        assert!(!decoder.is_eof());
        for _ in 0..16 {
            decoder.bit(128);
        }
        assert!(decoder.is_eof());
    }
}
