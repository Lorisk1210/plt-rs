//! The variable-width LZW compression of GIF image data: codes grow from
//! `min_code_size + 1` to 12 bits, packed least significant bit first; a
//! clear code restarts the table when all 4096 codes are taken.

/// The widest code GIF allows.
const MAX_WIDTH: u8 = 12;
/// The number of codes a 12-bit table holds.
const TABLE_CODES: u16 = 1 << MAX_WIDTH;

/// Compresses palette indices, each below `1 << min_code_size`, into the
/// raw LZW code stream (before it is cut into sub-blocks).
pub(super) fn compress(indices: &[u8], min_code_size: u8) -> Vec<u8> {
    let clear = 1_u16 << min_code_size;
    let end = clear + 1;
    let mut out = BitWriter::default();
    let mut table = Table::new();
    let mut width = min_code_size + 1;
    let mut next = end + 1;
    out.write(clear, width);
    let mut rest = indices.iter();
    let Some(&first) = rest.next() else {
        out.write(end, width);
        return out.finish();
    };
    let mut prefix = u16::from(first);
    for &index in rest {
        let key = u32::from(prefix) << 8 | u32::from(index);
        if let Some(code) = table.get(key) {
            prefix = code;
            continue;
        }
        out.write(prefix, width);
        if next == TABLE_CODES {
            out.write(clear, width);
            table.clear();
            width = min_code_size + 1;
            next = end + 1;
        } else {
            table.insert(key, next);
            if next == 1 << width && width < MAX_WIDTH {
                width += 1;
            }
            next += 1;
        }
        prefix = u16::from(index);
    }
    out.write(prefix, width);
    out.write(end, width);
    out.finish()
}

/// Codes packed least significant bit first.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    pending: u32,
    bits: u8,
}

impl BitWriter {
    fn write(&mut self, code: u16, width: u8) {
        self.pending |= u32::from(code) << self.bits;
        self.bits += width;
        while self.bits >= 8 {
            self.bytes.push(self.pending.to_le_bytes()[0]);
            self.pending >>= 8;
            self.bits -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.bytes.push(self.pending.to_le_bytes()[0]);
        }
        self.bytes
    }
}

/// The string table: `(prefix code, next index)` keys to codes, in an
/// open-addressed hash table twice the size of the code space.
struct Table {
    keys: Vec<u32>,
    codes: Vec<u16>,
}

impl Table {
    /// Slots in the table (a power of two).
    const SLOTS: usize = 2 * TABLE_CODES as usize;
    /// Marks an empty slot; real keys use at most 20 bits.
    const EMPTY: u32 = u32::MAX;

    fn new() -> Self {
        Self {
            keys: vec![Self::EMPTY; Self::SLOTS],
            codes: vec![0; Self::SLOTS],
        }
    }

    fn clear(&mut self) {
        self.keys.fill(Self::EMPTY);
    }

    /// The slot holding `key`, or the empty slot where it belongs.
    fn slot(&self, key: u32) -> usize {
        let hash = key.wrapping_mul(0x9E37_79B9) >> 19;
        let mut slot = hash as usize;
        while self.keys[slot] != key && self.keys[slot] != Self::EMPTY {
            slot = (slot + 1) % Self::SLOTS;
        }
        slot
    }

    fn get(&self, key: u32) -> Option<u16> {
        let slot = self.slot(key);
        (self.keys[slot] == key).then(|| self.codes[slot])
    }

    fn insert(&mut self, key: u32, code: u16) {
        let slot = self.slot(key);
        self.keys[slot] = key;
        self.codes[slot] = code;
    }
}

/// Decompresses a code stream (the inverse of [`compress`]), or `None`
/// when it is malformed. Only the tests read GIF data back.
#[cfg(test)]
pub(super) fn decompress(data: &[u8], min_code_size: u8) -> Option<Vec<u8>> {
    let clear = 1_usize << min_code_size;
    let end = clear + 1;
    let mut strings: Vec<Vec<u8>> = Vec::new();
    let reset = |strings: &mut Vec<Vec<u8>>| {
        strings.clear();
        strings.extend((0..clear).map(|i| vec![u8::try_from(i).unwrap()]));
        strings.extend([Vec::new(), Vec::new()]);
    };
    reset(&mut strings);
    let mut width = usize::from(min_code_size) + 1;
    let mut out = Vec::new();
    let mut previous: Option<usize> = None;
    let mut bit = 0;
    loop {
        let code = (0..width).try_fold(0, |code, i| {
            let at = bit + i;
            let set = data.get(at / 8)? >> (at % 8) & 1;
            Some(code | usize::from(set) << i)
        })?;
        bit += width;
        if code == clear {
            reset(&mut strings);
            width = usize::from(min_code_size) + 1;
            previous = None;
            continue;
        }
        if code == end {
            return Some(out);
        }
        let string = match (strings.get(code), previous) {
            (Some(string), _) => string.clone(),
            (None, Some(prev)) if code == strings.len() => {
                let mut string = strings[prev].clone();
                string.push(strings[prev][0]);
                string
            }
            _ => return None,
        };
        if let Some(prev) = previous
            && strings.len() < usize::from(TABLE_CODES)
        {
            let mut entry = strings[prev].clone();
            entry.push(string[0]);
            strings.push(entry);
            if strings.len() == 1 << width && width < usize::from(MAX_WIDTH) {
                width += 1;
            }
        }
        out.extend_from_slice(&string);
        previous = Some(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Rng;

    fn round_trip(indices: &[u8], min_code_size: u8) {
        let packed = compress(indices, min_code_size);
        assert_eq!(
            decompress(&packed, min_code_size).as_deref(),
            Some(indices),
            "{} indices at {min_code_size} bits",
            indices.len()
        );
    }

    #[test]
    fn empty_and_tiny_inputs() {
        round_trip(&[], 2);
        round_trip(&[3], 2);
        round_trip(&[0, 0, 0, 0, 0, 0, 0], 2);
        assert_eq!(compress(&[], 2), [0b0010_1100], "clear (4), end (5)");
    }

    #[test]
    fn runs_fill_and_restart_the_table() {
        round_trip(&vec![7; 200_000], 8);
        let mut rng = Rng::new(7);
        for bits in 2..=8 {
            let alphabet = 1 << bits;
            let noise: Vec<u8> = (0..60_000)
                .map(|_| u8::try_from(rng.below(alphabet)).unwrap())
                .collect();
            round_trip(&noise, bits);
        }
    }

    #[test]
    fn repetitive_images_compress_well() {
        let stripes: Vec<u8> = (0..100_000).map(|i| u8::from(i % 640 < 320)).collect();
        assert!(compress(&stripes, 2).len() < stripes.len() / 20);
        round_trip(&stripes, 2);
    }
}
