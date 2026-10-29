//! TIFF's LZW decompression as libtiff 4.7 does it (`LZWDecode`): codes
//! of 9 to 12 bits, most significant bit first, widening one code early;
//! 256 clears the table and 257 ends the data, which must start with a
//! clear code. Old-style data, recognised as libtiff does by its first
//! two bytes, packs codes least significant bit first and widens them a
//! code later (`LZWDecodeCompat`).
//!
//! Decompression stops once `expected` bytes are out (a longer string is
//! cut short); ending earlier is an error.

/// The clear code.
const CLEAR: u16 = 256;
/// The end-of-information code.
const END: u16 = 257;
/// The first code of a string table entry.
const FIRST: u16 = 258;
/// Table entries libtiff allocates (codes stop at 4095, entries do not).
const ENTRIES: usize = 4095 + 1024;
/// The widest code.
const MAX_BITS: u32 = 12;

/// The string table: for each entry the entry it extends, its last and
/// first bytes and its length (0 for undefined entries).
struct Table {
    prefix: Vec<u16>,
    last: Vec<u8>,
    first: Vec<u8>,
    length: Vec<u16>,
}

impl Table {
    fn new() -> Self {
        let mut table = Self {
            prefix: vec![0; ENTRIES],
            last: vec![0; ENTRIES],
            first: vec![0; ENTRIES],
            length: vec![0; ENTRIES],
        };
        for (code, byte) in (0..256).zip(0..=255) {
            table.last[code] = byte;
            table.first[code] = byte;
            table.length[code] = 1;
        }
        table
    }

    /// Appends the first `room` bytes of entry `code`'s string to `out`.
    fn write(&self, code: usize, room: usize, out: &mut Vec<u8>) {
        let length = usize::from(self.length[code]);
        let start = out.len();
        out.resize(start + length.min(room), 0);
        let mut entry = code;
        for at in (0..length).rev() {
            if at < room {
                out[start + at] = self.last[entry];
            }
            entry = usize::from(self.prefix[entry]);
        }
    }
}

/// Codes read from the data, in either bit order.
struct Codes<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u32,
    buffered: u32,
    msb_first: bool,
}

impl Codes<'_> {
    /// The next code of `bits` bits, or `None` when the data runs out.
    fn next(&mut self, bits: u32) -> Option<u16> {
        while self.buffered < bits {
            let byte = u32::from(*self.data.get(self.pos)?);
            self.pos += 1;
            if self.msb_first {
                self.buffer = self.buffer << 8 | byte;
            } else {
                self.buffer |= byte << self.buffered;
            }
            self.buffered += 8;
        }
        self.buffered -= bits;
        let mask = (1 << bits) - 1;
        let code = if self.msb_first {
            (self.buffer >> self.buffered) & mask
        } else {
            let code = self.buffer & mask;
            self.buffer >>= bits;
            code
        };
        Some(u16::try_from(code).expect("at most 12 bits"))
    }
}

/// Decompresses LZW `data` into exactly `expected` bytes, or `None` when
/// the data is corrupt or too short.
pub(super) fn decompress(data: &[u8], expected: usize) -> Option<Vec<u8>> {
    let old_style = data.len() >= 2 && data[0] == 0 && data[1] & 1 == 1;
    let mut codes = Codes {
        data,
        pos: 0,
        buffer: 0,
        buffered: 0,
        msb_first: !old_style,
    };
    // Old-style codes widen once the next entry passes all ones, new
    // ones a code earlier.
    let early = u16::from(!old_style);
    let mut table = Table::new();
    let mut out = Vec::with_capacity(expected);
    let mut bits = 9;
    // The next free entry; `None` until a clear code, and again once the
    // table is exhausted.
    let mut next: Option<u16> = None;
    let mut previous = 0_u16;
    while out.len() < expected {
        let code = codes.next(bits)?;
        match code {
            END => return None,
            CLEAR => {
                let mut code = CLEAR;
                while code == CLEAR {
                    code = codes.next(9)?;
                }
                if code >= END {
                    return None;
                }
                table.length[usize::from(FIRST)..].fill(0);
                bits = 9;
                next = Some(FIRST);
                out.push(u8::try_from(code).expect("below 256"));
                previous = code;
            }
            _ => {
                let free = next?;
                if code > free || table.length[usize::from(code)] == 0 && code != free {
                    return None;
                }
                let entry = usize::from(free);
                let old = usize::from(previous);
                table.prefix[entry] = previous;
                table.first[entry] = table.first[old];
                table.length[entry] = table.length[old] + 1;
                table.last[entry] = if code == free {
                    table.first[old]
                } else {
                    table.first[usize::from(code)]
                };
                let grown = free + 1;
                next = (usize::from(grown) < ENTRIES).then_some(grown);
                if grown > (1 << bits) - 1 - early && bits < MAX_BITS {
                    bits += 1;
                }
                previous = code;
                table.write(usize::from(code), expected - out.len(), &mut out);
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Packs codes of the given widths, most significant bit first.
    fn pack(codes: &[(u16, u32)]) -> Vec<u8> {
        let (mut out, mut buffer, mut bits) = (Vec::new(), 0_u64, 0);
        for &(code, width) in codes {
            buffer = buffer << width | u64::from(code);
            bits += width;
            while bits >= 8 {
                bits -= 8;
                out.push((buffer >> bits) as u8);
            }
        }
        if bits > 0 {
            out.push((buffer << (8 - bits)) as u8);
        }
        out
    }

    #[test]
    fn strings_build_up() {
        // "ABABABA": A, B, AB (258), ABA (260, defined as it is used).
        let data = pack(&[(256, 9), (65, 9), (66, 9), (258, 9), (260, 9), (257, 9)]);
        assert_eq!(decompress(&data, 7).as_deref(), Some(&b"ABABABA"[..]));
        assert_eq!(
            decompress(&data, 4).as_deref(),
            Some(&b"ABAB"[..]),
            "cut short"
        );
        assert_eq!(decompress(&data, 8), None, "ends early");
    }

    #[test]
    fn corrupt_streams_are_rejected() {
        assert_eq!(
            decompress(&pack(&[(65, 9)]), 1),
            None,
            "no clear code first"
        );
        assert_eq!(decompress(&pack(&[(256, 9), (300, 9)]), 1), None);
        assert_eq!(decompress(&pack(&[(256, 9), (65, 9), (300, 9)]), 3), None);
        assert_eq!(decompress(&[], 1), None);
        let mut rng = crate::test_util::Rng::new(5);
        for _ in 0..2000 {
            let len = rng.below(80);
            let mut noise = rng.bytes(len);
            if let Some(first) = noise.first_mut() {
                *first = 0x80;
            }
            let _ = decompress(&noise, rng.below(5000));
        }
    }
}
