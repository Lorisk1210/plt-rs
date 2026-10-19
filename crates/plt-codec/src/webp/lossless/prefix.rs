//! The prefix codes of VP8L bitstreams: reading their description (a
//! "simple" list of one or two symbols, or run-length coded code lengths)
//! and decoding symbols through a two-level lookup table.

use super::bit_reader::BitReader;
use crate::webp::DecodeError;

/// The bits the root table resolves at once.
const ROOT_BITS: u32 = 8;
/// The longest code.
const MAX_LENGTH: u32 = 15;
/// The alphabet of code lengths: 0–15, then the three repeat codes.
const CODE_LENGTH_CODES: usize = 19;
/// The order in which the code-length code's lengths are stored.
const CODE_LENGTH_ORDER: [usize; CODE_LENGTH_CODES] = [
    17, 18, 0, 1, 2, 3, 4, 5, 16, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];
/// The length repeated by code 16 before any non-zero length was read.
const DEFAULT_LENGTH: u8 = 8;

/// One lookup table entry: in the root table, a symbol and its code length
/// (at most [`ROOT_BITS`]) or a link to a second-level table (`length`
/// above [`ROOT_BITS`], `value` the table's offset); in a second-level
/// table, a symbol and the length of its code past the root bits.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Entry {
    length: u8,
    value: u16,
}

/// A decoding table for one prefix code.
#[derive(Clone, Debug)]
pub(super) enum PrefixCode {
    /// A code of one symbol, which takes no bits.
    Single(u16),
    /// The root table, followed by the second-level tables.
    Table(Vec<Entry>),
}

impl PrefixCode {
    /// The canonical code with these code lengths (zero for unused
    /// symbols), which must be complete unless it has a single symbol.
    pub(super) fn from_lengths(lengths: &[u8]) -> Result<Self, DecodeError> {
        let mut counts = [0usize; MAX_LENGTH as usize + 1];
        for &length in lengths {
            counts[usize::from(length)] += 1;
        }
        let mut used = lengths
            .iter()
            .enumerate()
            .filter(|&(_, &length)| length > 0);
        match (used.next(), used.next()) {
            (None, _) => return Err(DecodeError::InvalidPrefixCode),
            (Some((symbol, _)), None) => return Ok(Self::Single(symbol as u16)),
            _ => {}
        }
        let mut open: i64 = 1;
        for &count in &counts[1..] {
            open = 2 * open - count as i64;
            if open < 0 {
                return Err(DecodeError::InvalidPrefixCode);
            }
        }
        if open != 0 {
            return Err(DecodeError::InvalidPrefixCode);
        }
        Ok(Self::Table(build_table(lengths, &counts)))
    }

    /// The symbol whose code comes next.
    pub(super) fn read_symbol(&self, reader: &mut BitReader<'_>) -> u16 {
        let table = match self {
            Self::Single(symbol) => return *symbol,
            Self::Table(table) => table,
        };
        let bits = reader.peek(MAX_LENGTH);
        let root = table[(bits & ((1 << ROOT_BITS) - 1)) as usize];
        if u32::from(root.length) <= ROOT_BITS {
            reader.consume(u32::from(root.length));
            return root.value;
        }
        let sub_bits = u32::from(root.length) - ROOT_BITS;
        let index =
            usize::from(root.value) + ((bits >> ROOT_BITS) & ((1 << sub_bits) - 1)) as usize;
        let entry = table[index];
        reader.consume(ROOT_BITS + u32::from(entry.length));
        entry.value
    }
}

/// The lookup tables of a complete code with at least two symbols;
/// `counts` holds how many symbols have each length.
fn build_table(lengths: &[u8], counts: &[usize]) -> Vec<Entry> {
    let mut next_code = [0u32; MAX_LENGTH as usize + 1];
    let mut code = 0;
    for length in 1..=MAX_LENGTH as usize {
        code = (code
            + if length > 1 {
                counts[length - 1] as u32
            } else {
                0
            })
            << 1;
        next_code[length] = code;
    }
    let codes: Vec<(u16, u32, u32)> = lengths
        .iter()
        .enumerate()
        .filter(|&(_, &length)| length > 0)
        .map(|(symbol, &length)| {
            let length = u32::from(length);
            let code = next_code[length as usize];
            next_code[length as usize] += 1;
            (symbol as u16, length, code.reverse_bits() >> (32 - length))
        })
        .collect();
    let root_mask = (1 << ROOT_BITS) - 1;
    let mut sub_bits = vec![0u32; 1 << ROOT_BITS];
    for &(_, length, reversed) in &codes {
        if length > ROOT_BITS {
            let slot = &mut sub_bits[(reversed & root_mask) as usize];
            *slot = (*slot).max(length - ROOT_BITS);
        }
    }
    let mut table = vec![Entry::default(); 1 << ROOT_BITS];
    for (root, &bits) in sub_bits.iter().enumerate() {
        if bits > 0 {
            table[root] = Entry {
                length: (ROOT_BITS + bits) as u8,
                value: table.len() as u16,
            };
            table.resize(table.len() + (1 << bits), Entry::default());
        }
    }
    for (symbol, length, reversed) in codes {
        if length <= ROOT_BITS {
            let entry = Entry {
                length: length as u8,
                value: symbol,
            };
            for index in (reversed as usize..1 << ROOT_BITS).step_by(1 << length) {
                table[index] = entry;
            }
        } else {
            let link = table[(reversed & root_mask) as usize];
            let start = usize::from(link.value);
            let size = 1usize << (u32::from(link.length) - ROOT_BITS);
            let entry = Entry {
                length: (length - ROOT_BITS) as u8,
                value: symbol,
            };
            let step = 1 << (length - ROOT_BITS);
            for index in ((reversed >> ROOT_BITS) as usize..size).step_by(step) {
                table[start + index] = entry;
            }
        }
    }
    table
}

/// Reads the description of a prefix code over `alphabet_size` symbols.
pub(super) fn read_prefix_code(
    reader: &mut BitReader<'_>,
    alphabet_size: usize,
) -> Result<PrefixCode, DecodeError> {
    let mut lengths = vec![0u8; alphabet_size];
    if reader.flag() {
        let two_symbols = reader.flag();
        let first_bits = if reader.flag() { 8 } else { 1 };
        let first = reader.read_usize(first_bits);
        let second = two_symbols.then(|| reader.read_usize(8));
        // Symbols past the alphabet are ignored, as libwebp does.
        for symbol in std::iter::once(first).chain(second) {
            if let Some(length) = lengths.get_mut(symbol) {
                *length = 1;
            }
        }
    } else {
        let stored = reader.read_usize(4) + 4;
        let mut code_length_lengths = [0u8; CODE_LENGTH_CODES];
        for &symbol in &CODE_LENGTH_ORDER[..stored] {
            code_length_lengths[symbol] = reader.read(3) as u8;
        }
        let code_length_code = PrefixCode::from_lengths(&code_length_lengths)?;
        read_code_lengths(reader, &code_length_code, &mut lengths)?;
    }
    if reader.is_overrun() {
        return Err(DecodeError::Truncated);
    }
    PrefixCode::from_lengths(&lengths)
}

/// Reads run-length coded code lengths into `lengths`; a stored limit may
/// end the list early, leaving the remaining lengths zero.
fn read_code_lengths(
    reader: &mut BitReader<'_>,
    code: &PrefixCode,
    lengths: &mut [u8],
) -> Result<(), DecodeError> {
    let mut remaining = if reader.flag() {
        let bits = 2 + 2 * reader.read(3);
        let limit = 2 + reader.read_usize(bits);
        if limit > lengths.len() {
            return Err(DecodeError::InvalidPrefixCode);
        }
        limit
    } else {
        lengths.len()
    };
    let mut symbol = 0;
    let mut previous = DEFAULT_LENGTH;
    while symbol < lengths.len() && remaining > 0 {
        remaining -= 1;
        let length = code.read_symbol(reader) as u8;
        if length < 16 {
            lengths[symbol] = length;
            symbol += 1;
            if length != 0 {
                previous = length;
            }
            continue;
        }
        let (extra_bits, offset, value) = match length {
            16 => (2, 3, previous),
            17 => (3, 3, 0),
            _ => (7, 11, 0),
        };
        let repeat = reader.read_usize(extra_bits) + offset;
        let run = lengths
            .get_mut(symbol..symbol + repeat)
            .ok_or(DecodeError::InvalidPrefixCode)?;
        run.fill(value);
        symbol += repeat;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes `(value, bits)` pairs least significant bit first.
    fn pack(fields: &[(u32, u32)]) -> Vec<u8> {
        let mut out = Vec::new();
        let (mut buffer, mut count) = (0u64, 0);
        for &(value, bits) in fields {
            buffer |= u64::from(value) << count;
            count += bits;
            while count >= 8 {
                out.push(buffer as u8);
                buffer >>= 8;
                count -= 8;
            }
        }
        if count > 0 {
            out.push(buffer as u8);
        }
        out
    }

    #[test]
    fn canonical_codes_decode_their_symbols() {
        // a: 0, b: 10, c: 110, d: 111 (written bit-reversed).
        let code = PrefixCode::from_lengths(&[1, 2, 3, 3]).unwrap();
        let data = pack(&[(0b0, 1), (0b01, 2), (0b011, 3), (0b111, 3)]);
        let mut reader = BitReader::new(&data);
        let symbols: Vec<u16> = (0..4).map(|_| code.read_symbol(&mut reader)).collect();
        assert_eq!(symbols, [0, 1, 2, 3]);
    }

    #[test]
    fn long_codes_go_through_second_level_tables() {
        // Lengths 1, 2, ..., 14, 15, 15: a complete code reaching 15 bits.
        let mut lengths: Vec<u8> = (1..=15).collect();
        lengths.push(15);
        let code = PrefixCode::from_lengths(&lengths).unwrap();
        // Symbol 15 is fifteen 1-bits; symbol 9 is nine 1-bits and a 0.
        let data = pack(&[(0x7FFF, 15), (0x1FF, 10)]);
        let mut reader = BitReader::new(&data);
        assert_eq!(code.read_symbol(&mut reader), 15);
        assert_eq!(code.read_symbol(&mut reader), 9);
        assert!(!reader.is_overrun());
    }

    #[test]
    fn degenerate_codes_are_rejected_but_single_symbols_take_no_bits() {
        assert_eq!(
            PrefixCode::from_lengths(&[0, 0]).unwrap_err(),
            DecodeError::InvalidPrefixCode
        );
        assert!(PrefixCode::from_lengths(&[1, 2, 0]).is_err(), "incomplete");
        assert!(
            PrefixCode::from_lengths(&[1, 1, 1]).is_err(),
            "over-subscribed"
        );
        let single = PrefixCode::from_lengths(&[0, 0, 7]).unwrap();
        assert!(matches!(single, PrefixCode::Single(2)));
        let mut reader = BitReader::new(&[]);
        assert_eq!(single.read_symbol(&mut reader), 2);
        assert!(!reader.is_overrun());
    }

    #[test]
    fn simple_codes_ignore_symbols_past_the_alphabet() {
        // Simple code, two symbols, 8-bit first symbol: 3 and 200.
        let data = pack(&[(1, 1), (1, 1), (1, 1), (3, 8), (200, 8)]);
        let code = read_prefix_code(&mut BitReader::new(&data), 40).unwrap();
        assert!(matches!(code, PrefixCode::Single(3)));
    }

    #[test]
    fn code_lengths_repeat_and_stop_at_the_stored_limit() {
        // A code-length code where 1 is coded `0` and 16 is coded `1`.
        let mut code_length_lengths = [0; CODE_LENGTH_CODES];
        code_length_lengths[1] = 1;
        code_length_lengths[16] = 1;
        let code = PrefixCode::from_lengths(&code_length_lengths).unwrap();
        let data = pack(&[
            // A limit of 2 + 1 code-length symbols, in 2 + 2·0 bits.
            (1, 1),
            (0, 3),
            (1, 2),
            // 1; 16 repeating it 3 + 3 times; 1.
            (0, 1),
            (1, 1),
            (3, 2),
            (0, 1),
        ]);
        let mut lengths = [0; 10];
        read_code_lengths(&mut BitReader::new(&data), &code, &mut lengths).unwrap();
        assert_eq!(lengths, [1, 1, 1, 1, 1, 1, 1, 1, 0, 0]);
    }
}
