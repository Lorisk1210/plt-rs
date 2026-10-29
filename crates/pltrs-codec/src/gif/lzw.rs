//! The variable-width LZW coding of GIF image data: codes grow from
//! `min_code_size + 1` to 12 bits, packed least significant bit first. The
//! compressor restarts the table with a clear code when all 4096 codes are
//! taken; the decompressor follows Pillow's, accepting deferred clears.

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

/// Why a code stream could not be decompressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LzwError {
    /// The minimum code size is above 12.
    InvalidCodeSize,
    /// A code that is neither a palette index, a code in the table, nor
    /// the next code to be added.
    InvalidCode,
    /// The data ended before the image was complete.
    Truncated,
}

/// How decompression stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Finish {
    /// The sink reported the image complete; any further data is ignored.
    Complete,
    /// The stream's end code came first.
    EndCode,
}

/// Decompresses a code stream read from `bytes`, the way Pillow's GIF
/// decoder does (`GifDecode.c`), handing each palette index to `sink`,
/// which returns `true` once the image is complete.
///
/// Codes grow one bit when the table's next free code reaches all ones;
/// a full table stops growing until a clear code (a deferred clear); the
/// first code after a clear is taken literally and adds no entry.
pub(super) fn decompress(
    bytes: impl IntoIterator<Item = u8>,
    min_code_size: u8,
    mut sink: impl FnMut(u8) -> bool,
) -> Result<Finish, LzwError> {
    if min_code_size > MAX_WIDTH {
        return Err(LzwError::InvalidCodeSize);
    }
    let clear = 1_u16 << min_code_size;
    let end = clear + 1;
    let mut bytes = bytes.into_iter();
    let (mut buffer, mut buffered) = (0_u32, 0_u8);
    // Each table entry: the code of its prefix and its last index.
    let mut links = vec![0_u16; usize::from(TABLE_CODES)];
    let mut suffixes = vec![0_u8; usize::from(TABLE_CODES)];
    let mut string: Vec<u8> = Vec::with_capacity(usize::from(TABLE_CODES));
    let mut width = min_code_size + 1;
    let mut next = end + 1;
    // The previous code and the first index of its string; none right
    // after a clear code.
    let mut previous: Option<(u16, u8)> = None;
    loop {
        while buffered < width {
            let byte = bytes.next().ok_or(LzwError::Truncated)?;
            buffer |= u32::from(byte) << buffered;
            buffered += 8;
        }
        let code = u16::try_from(buffer & ((1 << width) - 1)).expect("at most 13 bits");
        buffer >>= width;
        buffered -= width;
        if code == clear {
            if previous.is_some() {
                width = min_code_size + 1;
                next = end + 1;
                previous = None;
            }
            continue;
        }
        if code == end {
            return Ok(Finish::EndCode);
        }
        string.clear();
        let first = match previous {
            None if code > clear => return Err(LzwError::InvalidCode),
            // Indices are bytes: with code sizes above 8, Pillow keeps the
            // low byte.
            None => code as u8,
            Some((last_code, last_first)) => {
                if code > next {
                    return Err(LzwError::InvalidCode);
                }
                let mut walk = code;
                if code == next {
                    // The string of the previous code, then its first index.
                    string.push(last_first);
                    walk = last_code;
                }
                while walk >= clear {
                    if string.len() >= usize::from(TABLE_CODES) || walk >= TABLE_CODES {
                        return Err(LzwError::InvalidCode);
                    }
                    string.push(suffixes[usize::from(walk)]);
                    walk = links[usize::from(walk)];
                }
                let first = walk as u8;
                if next < TABLE_CODES {
                    suffixes[usize::from(next)] = first;
                    links[usize::from(next)] = last_code;
                    if next == (1 << width) - 1 && width < MAX_WIDTH {
                        width += 1;
                    }
                    next += 1;
                }
                first
            }
        };
        previous = Some((code, first));
        if sink(first) {
            return Ok(Finish::Complete);
        }
        // The rest of the string was collected last index first.
        for &index in string.iter().rev() {
            if sink(index) {
                return Ok(Finish::Complete);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::Rng;

    fn unpack(packed: &[u8], min_code_size: u8) -> Result<(Vec<u8>, Finish), LzwError> {
        let mut indices = Vec::new();
        let finish = decompress(packed.iter().copied(), min_code_size, |index| {
            indices.push(index);
            false
        })?;
        Ok((indices, finish))
    }

    fn round_trip(indices: &[u8], min_code_size: u8) {
        let packed = compress(indices, min_code_size);
        assert_eq!(
            unpack(&packed, min_code_size),
            Ok((indices.to_vec(), Finish::EndCode)),
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

    #[test]
    fn a_complete_image_stops_decoding() {
        let packed = compress(&[1, 2, 3, 1, 2, 3], 2);
        let mut seen = Vec::new();
        let finish = decompress(packed.iter().copied(), 2, |index| {
            seen.push(index);
            seen.len() == 4
        });
        assert_eq!(finish, Ok(Finish::Complete));
        assert_eq!(seen, [1, 2, 3, 1]);
    }

    #[test]
    fn malformed_streams_are_errors() {
        assert_eq!(unpack(&[], 2), Err(LzwError::Truncated));
        assert_eq!(unpack(&[0xFF], 13), Err(LzwError::InvalidCodeSize));
        // Clear (4), index 1, then code 7 beyond the table (next is 6).
        assert_eq!(
            unpack(&[0b1100_1100, 0b0000_0001], 2),
            Err(LzwError::InvalidCode)
        );
        // A first code above the clear code.
        assert_eq!(unpack(&[0b0000_0110], 2), Err(LzwError::InvalidCode));
        let mut rng = Rng::new(3);
        for _ in 0..2000 {
            let len = rng.below(64);
            let noise = rng.bytes(len);
            let bits = u8::try_from(rng.below(13)).unwrap();
            let mut count = 0_usize;
            let _ = decompress(noise.iter().copied(), bits, |_| {
                count += 1;
                count > 100_000
            });
        }
    }
}
