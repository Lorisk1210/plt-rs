//! Huffman coding of quantized blocks (ITU T.81 F.1.2): DC differences and
//! run-length coded AC coefficients, packed most significant bit first
//! with `0xFF` bytes stuffed.

use crate::jpeg::tables::{HuffmanSpec, ZIGZAG};

/// The code and its length for every symbol of a table.
#[derive(Clone, Debug)]
pub(super) struct CodeTable {
    codes: [(u16, u8); 256],
}

impl CodeTable {
    /// The canonical codes of `spec` (Annex C).
    pub(super) fn new(spec: &HuffmanSpec) -> Self {
        let mut codes = [(0, 0); 256];
        let mut code = 0_u16;
        let mut symbols = spec.symbols.iter();
        for (length, &count) in (1_u8..).zip(&spec.counts) {
            for symbol in symbols.by_ref().take(usize::from(count)) {
                codes[usize::from(*symbol)] = (code, length);
                code += 1;
            }
            code <<= 1;
        }
        Self { codes }
    }

    fn code(&self, symbol: u8) -> (u16, u8) {
        self.codes[usize::from(symbol)]
    }
}

/// Entropy-coded data: bits packed most significant first, a zero byte
/// after every `0xFF`.
#[derive(Debug, Default)]
pub(super) struct BitWriter {
    bytes: Vec<u8>,
    pending: u32,
    bits: u32,
}

impl BitWriter {
    /// Appends the low `count` bits of `value`.
    fn write(&mut self, value: u32, count: u8) {
        let count = u32::from(count);
        self.pending = self.pending << count | (value & ((1 << count) - 1));
        self.bits += count;
        while self.bits >= 8 {
            self.bits -= 8;
            let byte = (self.pending >> self.bits).to_le_bytes()[0];
            self.bytes.push(byte);
            if byte == 0xFF {
                self.bytes.push(0);
            }
        }
        self.pending &= (1 << self.bits) - 1;
    }

    /// Pads the last byte with one bits and returns the data.
    pub(super) fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.write(0x7F, u8::try_from(8 - self.bits).expect("under 8 bits"));
        }
        self.bytes
    }
}

/// The magnitude category of a coefficient (its bit length) and the bits
/// that follow its code: the value itself, or its one's complement when
/// negative.
fn category(value: i32) -> (u8, u32) {
    let magnitude = value.unsigned_abs();
    let size = 32 - magnitude.leading_zeros();
    let bits = if value < 0 { !magnitude } else { magnitude };
    let mask = u32::MAX.checked_shr(32 - size).unwrap_or(0);
    (u8::try_from(size).expect("at most 32 bits"), bits & mask)
}

/// Codes one component's blocks, predicting each DC term from the last.
#[derive(Debug)]
pub(super) struct BlockCoder<'a> {
    dc: &'a CodeTable,
    ac: &'a CodeTable,
    last_dc: i32,
}

impl<'a> BlockCoder<'a> {
    /// A coder with the component's tables, its prediction starting at 0.
    pub(super) fn new(dc: &'a CodeTable, ac: &'a CodeTable) -> Self {
        Self { dc, ac, last_dc: 0 }
    }

    /// Writes a block of quantized coefficients in natural order.
    pub(super) fn encode(&mut self, out: &mut BitWriter, block: &[i32; 64]) {
        let (size, bits) = category(block[0] - self.last_dc);
        self.last_dc = block[0];
        let (code, length) = self.dc.code(size);
        out.write(u32::from(code), length);
        out.write(bits, size);
        let mut run = 0_u8;
        for &position in &ZIGZAG[1..] {
            let value = block[position];
            if value == 0 {
                run += 1;
                continue;
            }
            while run > 15 {
                let (code, length) = self.ac.code(0xF0);
                out.write(u32::from(code), length);
                run -= 16;
            }
            let (size, bits) = category(value);
            let (code, length) = self.ac.code(run << 4 | size);
            out.write(u32::from(code), length);
            out.write(bits, size);
            run = 0;
        }
        if run > 0 {
            let (code, length) = self.ac.code(0x00);
            out.write(u32::from(code), length);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg::tables::{AC_LUMINANCE, DC_LUMINANCE};

    #[test]
    fn canonical_codes_follow_annex_k() {
        let dc = CodeTable::new(&DC_LUMINANCE);
        assert_eq!(dc.code(0), (0b00, 2));
        assert_eq!(dc.code(1), (0b010, 3));
        assert_eq!(dc.code(5), (0b110, 3));
        assert_eq!(dc.code(6), (0b1110, 4));
        assert_eq!(dc.code(11), (0b1_1111_1110, 9));
        let ac = CodeTable::new(&AC_LUMINANCE);
        assert_eq!(ac.code(0x00), (0b1010, 4), "end of block");
        assert_eq!(ac.code(0xF0), (0b111_1111_1001, 11), "sixteen zeros");
    }

    #[test]
    fn categories_and_extra_bits() {
        assert_eq!(category(0), (0, 0));
        assert_eq!(category(1), (1, 1));
        assert_eq!(category(-1), (1, 0));
        assert_eq!(category(-3), (2, 0b00));
        assert_eq!(category(-6), (3, 0b001));
        assert_eq!(category(255), (8, 255));
    }

    #[test]
    fn ff_bytes_are_stuffed_and_padding_is_ones() {
        let mut out = BitWriter::default();
        out.write(0xFF, 8);
        out.write(0b1, 1);
        assert_eq!(out.finish(), [0xFF, 0x00, 0xFF, 0x00]);
    }

    #[test]
    fn an_empty_block_is_a_dc_code_and_an_end_of_block() {
        let (dc, ac) = (CodeTable::new(&DC_LUMINANCE), CodeTable::new(&AC_LUMINANCE));
        let mut out = BitWriter::default();
        BlockCoder::new(&dc, &ac).encode(&mut out, &[0; 64]);
        // DC category 0 (00), end of block (1010), then one-bit padding.
        assert_eq!(out.finish(), [0b0010_1011]);
    }
}
