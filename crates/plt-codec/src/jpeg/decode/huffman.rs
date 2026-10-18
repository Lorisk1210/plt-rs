//! Huffman tables: their DHT definitions (`get_dht`) and the decoding
//! tables derived from them (`jpeg_make_d_derived_tbl`, with libjpeg's
//! validation), read with an 8-bit lookahead (`jpeg_huff_decode`).

use super::bits::BitReader;
use super::error::{DecodeError, TableKind};
use crate::jpeg::marker::DHT;

/// Bits decoded by table lookup; longer codes are decoded bit by bit.
const LOOKAHEAD: u32 = 8;

/// A table as a DHT segment defines it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Definition {
    /// The number of codes of each length 1–16.
    counts: [u8; 16],
    /// The symbols in code order.
    symbols: Vec<u8>,
}

/// The four DC and four AC table slots.
#[derive(Clone, Debug, Default)]
pub(super) struct HuffmanTables {
    dc: [Option<Definition>; 4],
    ac: [Option<Definition>; 4],
}

impl HuffmanTables {
    /// Stores every table of a DHT segment.
    pub(super) fn read(&mut self, payload: &[u8]) -> Result<(), DecodeError> {
        let malformed = || DecodeError::MalformedSegment { marker: DHT };
        let mut rest = payload;
        while rest.len() > 16 {
            let (class_and_slot, tail) = (rest[0], &rest[1..]);
            let counts: [u8; 16] = tail[..16].try_into().expect("16 count bytes");
            let total: usize = counts.iter().map(|&n| usize::from(n)).sum();
            let symbols = tail.get(16..16 + total).ok_or_else(malformed)?;
            if total > 256 {
                return Err(DecodeError::InvalidHuffmanTable);
            }
            let slots = if class_and_slot & 0x10 == 0 {
                &mut self.dc
            } else {
                &mut self.ac
            };
            let slot = slots
                .get_mut(usize::from(class_and_slot & !0x10))
                .ok_or_else(malformed)?;
            *slot = Some(Definition {
                counts,
                symbols: symbols.to_vec(),
            });
            rest = &tail[16 + total..];
        }
        if rest.is_empty() {
            Ok(())
        } else {
            Err(malformed())
        }
    }

    /// The decoding table of DC table `index`.
    pub(super) fn dc(&self, index: u8) -> Result<HuffmanTable, DecodeError> {
        Self::derive(&self.dc, TableKind::DcHuffman, index)
    }

    /// The decoding table of AC table `index`.
    pub(super) fn ac(&self, index: u8) -> Result<HuffmanTable, DecodeError> {
        Self::derive(&self.ac, TableKind::AcHuffman, index)
    }

    fn derive(
        slots: &[Option<Definition>; 4],
        kind: TableKind,
        index: u8,
    ) -> Result<HuffmanTable, DecodeError> {
        let definition = slots
            .get(usize::from(index))
            .and_then(Option::as_ref)
            .ok_or(DecodeError::MissingTable { kind, index })?;
        HuffmanTable::new(definition, kind == TableKind::DcHuffman)
    }
}

/// A table ready for decoding.
#[derive(Clone, Debug)]
pub(super) struct HuffmanTable {
    /// For each 8-bit lookahead: the code length (0 when longer than 8)
    /// and the symbol.
    lookup: [(u8, u8); 1 << LOOKAHEAD],
    /// The largest code of each length (index 1–16), −1 when none.
    max_code: [i32; 18],
    /// For each length, the symbol index of its first code minus that code.
    value_offset: [i32; 18],
    symbols: [u8; 256],
}

impl HuffmanTable {
    /// Derives the decoding table, rejecting counts that overflow the
    /// code space and DC symbols above 15.
    fn new(definition: &Definition, dc: bool) -> Result<Self, DecodeError> {
        let mut symbols = [0; 256];
        symbols[..definition.symbols.len()].copy_from_slice(&definition.symbols);
        if dc && definition.symbols.iter().any(|&s| s > 15) {
            return Err(DecodeError::InvalidHuffmanTable);
        }
        // Canonical codes, length by length (Figure C.2).
        let mut codes = Vec::with_capacity(definition.symbols.len());
        let mut code = 0_u32;
        for (length, &count) in (1_u32..).zip(&definition.counts) {
            for _ in 0..count {
                codes.push((code, length));
                code += 1;
            }
            // The codes must fit in `length` bits without using the
            // all-ones code.
            if count > 0 && code >= 1 << length {
                return Err(DecodeError::InvalidHuffmanTable);
            }
            code <<= 1;
        }
        let mut max_code = [-1; 18];
        let mut value_offset = [0; 18];
        let mut first = 0_usize;
        for (length, &count) in definition.counts.iter().enumerate() {
            if count > 0 {
                let (first_code, _) = codes[first];
                value_offset[length + 1] = first as i32 - first_code as i32;
                first += usize::from(count);
                max_code[length + 1] = codes[first - 1].0 as i32;
            }
        }
        max_code[17] = 0xF_FFFF;
        let mut lookup = [(0, 0); 1 << LOOKAHEAD];
        for (&(code, length), &symbol) in codes.iter().zip(&definition.symbols) {
            if length <= LOOKAHEAD {
                let shift = LOOKAHEAD - length;
                let start = (code << shift) as usize;
                lookup[start..start + (1 << shift)].fill((length as u8, symbol));
            }
        }
        Ok(Self {
            lookup,
            max_code,
            value_offset,
            symbols,
        })
    }

    /// Decodes the next symbol.
    pub(super) fn decode(&self, reader: &mut BitReader<'_>) -> Result<u8, DecodeError> {
        let (length, symbol) = self.lookup[reader.peek(LOOKAHEAD) as usize];
        if length > 0 {
            reader.consume(u32::from(length))?;
            return Ok(symbol);
        }
        let mut length = LOOKAHEAD + 1;
        let mut code = i32::try_from(reader.bits(length)?).expect("9 bits");
        while length <= 16 && code > self.max_code[length as usize] {
            code = code << 1 | i32::from(reader.bit()?);
            length += 1;
        }
        if length > 16 {
            return Err(DecodeError::InvalidHuffmanCode);
        }
        let index = code + self.value_offset[length as usize];
        Ok(self.symbols[usize::try_from(index).map_err(|_| DecodeError::InvalidHuffmanCode)?])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jpeg::tables::{AC_LUMINANCE, DC_LUMINANCE};

    fn segment(class_and_slot: u8, spec: &crate::jpeg::tables::HuffmanSpec) -> Vec<u8> {
        let mut payload = vec![class_and_slot];
        payload.extend_from_slice(&spec.counts);
        payload.extend_from_slice(spec.symbols);
        payload
    }

    #[test]
    fn standard_tables_decode_their_codes() {
        let mut tables = HuffmanTables::default();
        let mut payload = segment(0x00, &DC_LUMINANCE);
        payload.extend(segment(0x10, &AC_LUMINANCE));
        tables.read(&payload).unwrap();
        let (dc, ac) = (tables.dc(0).unwrap(), tables.ac(0).unwrap());
        // DC category 5 (110), AC end of block (1010), AC 0xF0 (11 bits:
        // 111 1111 1001), then DC category 11 (9 bits: 1 1111 1110).
        let bits = [0b1101_0101, 0b1111_1110, 0b0111_1111, 0b1100_0000];
        let mut reader = BitReader::new(&bits, 0);
        assert_eq!(dc.decode(&mut reader), Ok(5));
        assert_eq!(ac.decode(&mut reader), Ok(0x00));
        assert_eq!(ac.decode(&mut reader), Ok(0xF0));
        assert_eq!(dc.decode(&mut reader), Ok(11));
    }

    #[test]
    fn invalid_codes_and_tables_are_rejected() {
        let mut tables = HuffmanTables::default();
        tables.read(&segment(0x00, &DC_LUMINANCE)).unwrap();
        let dc = tables.dc(0).unwrap();
        let mut reader = BitReader::new(&[0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00], 0);
        assert_eq!(dc.decode(&mut reader), Err(DecodeError::InvalidHuffmanCode));
        assert!(matches!(
            tables.ac(0),
            Err(DecodeError::MissingTable { .. })
        ));
        assert!(matches!(
            tables.dc(5),
            Err(DecodeError::MissingTable { .. })
        ));

        // Three codes of length 1 overflow the code space.
        let mut overfull = vec![0x01, 3];
        overfull.extend([0; 15]);
        overfull.extend([0, 1, 2]);
        tables.read(&overfull).unwrap();
        assert_eq!(tables.dc(1).err(), Some(DecodeError::InvalidHuffmanTable));
        // DC symbols above 15.
        let mut big_symbol = vec![0x02, 1];
        big_symbol.extend([0; 15]);
        big_symbol.push(16);
        tables.read(&big_symbol).unwrap();
        assert_eq!(tables.dc(2).err(), Some(DecodeError::InvalidHuffmanTable));
        assert!(tables.read(&[0x00, 1, 2]).is_err(), "leftover bytes");
        let mut missing_symbols = vec![0x00, 4];
        missing_symbols.extend([0; 15]);
        assert!(tables.read(&missing_symbols).is_err());
    }
}
