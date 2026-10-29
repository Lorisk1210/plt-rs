//! Lookup tables for fast Huffman decoding.
//!
//! A table is indexed by the next `primary_bits` input bits. Codes no longer
//! than that fill every entry whose low bits match them; longer codes share a
//! primary entry that points to a sub-table indexed by the following bits.
//! Each entry records the decoded meaning and the code's full length, so one
//! or two array reads decode any symbol.

use super::error::{HuffmanTable, InflateError};
use super::tables::{
    CODE_LENGTH_SYMBOLS, DIST_BASE, DIST_EXTRA, END_OF_BLOCK, FIXED_LITLEN_SYMBOLS, LENGTH_BASE,
    LENGTH_EXTRA, MAX_CODE_BITS,
};

/// What a code decodes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Symbol {
    /// A literal byte, or a code-length symbol (0–18) in the code-length
    /// alphabet.
    Literal(u8),
    /// The end of the current block.
    EndOfBlock,
    /// A match length or distance: `base` plus `extra` more input bits.
    Base { base: u16, extra: u8 },
    /// The code continues in the sub-table at `offset`, indexed by the next
    /// `bits` input bits.
    Subtable { offset: u16, bits: u8 },
    /// No valid code: a reserved symbol or an unused part of the code space.
    Invalid,
}

/// One table slot.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Entry {
    pub(crate) symbol: Symbol,
    /// Number of input bits the code occupies; 0 for slots in unused code
    /// space.
    pub(crate) length: u8,
}

const INVALID: Entry = Entry {
    symbol: Symbol::Invalid,
    length: 0,
};

/// A decoding table for one alphabet, rebuilt in place for every block.
#[derive(Debug)]
pub(crate) struct DecodeTable {
    table: HuffmanTable,
    primary_bits: u32,
    entries: Vec<Entry>,
}

impl DecodeTable {
    /// Creates an empty (all-invalid) table for `table`'s alphabet.
    pub(crate) fn new(table: HuffmanTable) -> Self {
        let primary_bits = match table {
            HuffmanTable::LiteralLength => 10,
            HuffmanTable::Distance => 8,
            HuffmanTable::CodeLength => 7,
        };
        Self {
            table,
            primary_bits,
            entries: vec![INVALID; 1 << primary_bits],
        }
    }

    /// Rebuilds the table from per-symbol code lengths (0 = unused, at most
    /// 15), validating that they form a usable prefix code.
    pub(crate) fn build(&mut self, lengths: &[u8]) -> Result<(), InflateError> {
        debug_assert!(lengths.len() <= FIXED_LITLEN_SYMBOLS);
        let mut counts = [0u16; MAX_CODE_BITS + 1];
        for &length in lengths {
            counts[usize::from(length)] += 1;
        }
        counts[0] = 0;
        self.validate(&counts)?;

        let mut next_code = [0u32; MAX_CODE_BITS + 1];
        let mut code = 0;
        for (length, next) in next_code.iter_mut().enumerate().skip(1) {
            code = (code + u32::from(counts[length - 1])) << 1;
            *next = code;
        }
        let mut reversed = [0u32; FIXED_LITLEN_SYMBOLS];
        for (slot, &length) in reversed.iter_mut().zip(lengths) {
            if length > 0 {
                let length = usize::from(length);
                *slot = next_code[length].reverse_bits() >> (32 - length);
                next_code[length] += 1;
            }
        }

        self.entries.clear();
        self.entries.resize(1 << self.primary_bits, INVALID);
        let subtables = self.allocate_subtables(lengths, &reversed);
        for (symbol, (&length, &code)) in lengths.iter().zip(&reversed).enumerate() {
            if length > 0 {
                let entry = Entry {
                    symbol: self.symbol(symbol),
                    length,
                };
                self.place(entry, code as usize, &subtables);
            }
        }
        Ok(())
    }

    /// Looks up the code at the bottom of `bits`.
    pub(crate) fn lookup(&self, bits: u64) -> Entry {
        let primary_mask = (1 << self.primary_bits) - 1;
        let entry = self.entries[(bits & primary_mask) as usize];
        match entry.symbol {
            Symbol::Subtable { offset, bits: sub } => {
                let sub_index = (bits >> self.primary_bits) & ((1 << sub) - 1);
                self.entries[usize::from(offset) + sub_index as usize]
            }
            _ => entry,
        }
    }

    /// Checks the Kraft inequality the way zlib does: over-subscription is
    /// always an error, an incomplete code only acceptable for a single
    /// one-bit literal/length or distance code. An empty code is accepted;
    /// any attempt to decode with it fails.
    fn validate(&self, counts: &[u16; MAX_CODE_BITS + 1]) -> Result<(), InflateError> {
        let mut unused: i32 = 1;
        for &count in &counts[1..] {
            unused = 2 * unused - i32::from(count);
            if unused < 0 {
                return Err(InflateError::OversubscribedCode(self.table));
            }
        }
        let longest = counts.iter().rposition(|&count| count > 0).unwrap_or(0);
        let incomplete = unused > 0 && longest > 0;
        if incomplete && (longest != 1 || self.table == HuffmanTable::CodeLength) {
            return Err(InflateError::IncompleteCode(self.table));
        }
        Ok(())
    }

    /// Appends one sub-table for every primary prefix shared by codes longer
    /// than `primary_bits`, sized for the longest such code, and returns each
    /// prefix's sub-table `(offset, index bits)` (zero bits where there is
    /// none).
    fn allocate_subtables(&mut self, lengths: &[u8], reversed: &[u32]) -> Vec<(u16, u8)> {
        let primary_size = 1 << self.primary_bits;
        let primary_mask = primary_size - 1;
        let mut subtables = vec![(0u16, 0u8); primary_size];
        for (&length, &code) in lengths.iter().zip(reversed) {
            if u32::from(length) > self.primary_bits {
                let bits = &mut subtables[code as usize & primary_mask].1;
                *bits = (*bits).max(length - self.primary_bits as u8);
            }
        }
        for (prefix, (offset, bits)) in subtables.iter_mut().enumerate() {
            if *bits > 0 {
                *offset = self.entries.len() as u16;
                self.entries[prefix] = Entry {
                    symbol: Symbol::Subtable {
                        offset: *offset,
                        bits: *bits,
                    },
                    length: self.primary_bits as u8,
                };
                self.entries
                    .resize(self.entries.len() + (1 << *bits), INVALID);
            }
        }
        subtables
    }

    /// Writes `entry` into every slot whose index starts with the
    /// bit-reversed `code`.
    fn place(&mut self, entry: Entry, code: usize, subtables: &[(u16, u8)]) {
        let length = usize::from(entry.length);
        let primary_bits = self.primary_bits as usize;
        let (slots, step) = if length <= primary_bits {
            (&mut self.entries[code..1 << primary_bits], 1 << length)
        } else {
            let (offset, bits) = subtables[code & ((1 << primary_bits) - 1)];
            let offset = usize::from(offset);
            let first = offset + (code >> primary_bits);
            (
                &mut self.entries[first..offset + (1 << bits)],
                1 << (length - primary_bits),
            )
        };
        for slot in slots.iter_mut().step_by(step) {
            *slot = entry;
        }
    }

    /// The meaning of `symbol` in this table's alphabet.
    fn symbol(&self, symbol: usize) -> Symbol {
        match self.table {
            HuffmanTable::CodeLength => {
                debug_assert!(symbol < CODE_LENGTH_SYMBOLS);
                Symbol::Literal(symbol as u8)
            }
            HuffmanTable::LiteralLength => match symbol {
                0..=255 => Symbol::Literal(symbol as u8),
                END_OF_BLOCK => Symbol::EndOfBlock,
                _ => match (
                    LENGTH_BASE.get(symbol - 257),
                    LENGTH_EXTRA.get(symbol - 257),
                ) {
                    (Some(&base), Some(&extra)) => Symbol::Base { base, extra },
                    _ => Symbol::Invalid,
                },
            },
            HuffmanTable::Distance => match (DIST_BASE.get(symbol), DIST_EXTRA.get(symbol)) {
                (Some(&base), Some(&extra)) => Symbol::Base { base, extra },
                _ => Symbol::Invalid,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deflate::huffman::HuffmanCode;

    #[test]
    fn long_codes_resolve_through_subtables() {
        // Lengths 1, 2, ..., 15, 15 form a complete code whose longest codes
        // need sub-tables beyond the 10-bit primary table.
        let mut lengths: Vec<u8> = (1..=15).collect();
        lengths.push(15);
        let mut table = DecodeTable::new(HuffmanTable::LiteralLength);
        table.build(&lengths).unwrap();
        let code = HuffmanCode::from_lengths(lengths.clone());
        for (symbol, (&length, &bits)) in lengths.iter().zip(&code.codes).enumerate() {
            // Garbage above the code must not matter.
            let entry = table.lookup(u64::from(bits) | 0xABCD << length);
            assert_eq!(
                entry.symbol,
                Symbol::Literal(symbol as u8),
                "symbol {symbol}"
            );
            assert_eq!(entry.length, length);
        }
    }

    #[test]
    fn symbols_map_to_lengths_distances_and_reserved_codes() {
        let mut table = DecodeTable::new(HuffmanTable::LiteralLength);
        table
            .build(&crate::deflate::tables::fixed_litlen_lengths())
            .unwrap();
        let code =
            HuffmanCode::from_lengths(crate::deflate::tables::fixed_litlen_lengths().to_vec());
        let lookup = |symbol: usize| table.lookup(u64::from(code.codes[symbol])).symbol;
        assert_eq!(lookup(65), Symbol::Literal(65));
        assert_eq!(lookup(256), Symbol::EndOfBlock);
        assert_eq!(lookup(265), Symbol::Base { base: 11, extra: 1 });
        assert_eq!(
            lookup(285),
            Symbol::Base {
                base: 258,
                extra: 0
            }
        );
        assert_eq!(lookup(287), Symbol::Invalid);
    }

    #[test]
    fn validation_follows_zlib() {
        let mut dist = DecodeTable::new(HuffmanTable::Distance);
        assert_eq!(dist.build(&[0, 0]), Ok(()), "no distance codes at all");
        assert_eq!(dist.build(&[1]), Ok(()), "a single one-bit code");
        assert_eq!(dist.lookup(1).symbol, Symbol::Invalid);
        assert_eq!(
            dist.build(&[2, 2, 2]),
            Err(InflateError::IncompleteCode(HuffmanTable::Distance))
        );
        assert_eq!(
            dist.build(&[1, 1, 1]),
            Err(InflateError::OversubscribedCode(HuffmanTable::Distance))
        );
        let mut code_lengths = DecodeTable::new(HuffmanTable::CodeLength);
        assert_eq!(
            code_lengths.build(&[1]),
            Err(InflateError::IncompleteCode(HuffmanTable::CodeLength))
        );
    }
}
