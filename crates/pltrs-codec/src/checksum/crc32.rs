//! CRC-32 with the IEEE 802.3 polynomial, as used by PNG, gzip and zip.
//!
//! The implementation uses "slicing-by-8": eight 256-entry tables let the
//! inner loop fold eight input bytes into the register per iteration.

/// The CRC-32 polynomial `0x04C11DB7` in reflected (LSB-first) form.
const POLYNOMIAL: u32 = 0xEDB8_8320;

/// `TABLES[k][b]` is the register contribution of byte `b` followed by `k`
/// zero bytes.
static TABLES: [[u32; 256]; 8] = build_tables();

const fn build_tables() -> [[u32; 256]; 8] {
    let mut tables = [[0; 256]; 8];
    let mut byte = 0;
    while byte < 256 {
        let mut crc = byte as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ POLYNOMIAL
            } else {
                crc >> 1
            };
            bit += 1;
        }
        tables[0][byte] = crc;
        byte += 1;
    }
    let mut k = 1;
    while k < 8 {
        let mut byte = 0;
        while byte < 256 {
            let previous = tables[k - 1][byte];
            tables[k][byte] = (previous >> 8) ^ tables[0][(previous & 0xFF) as usize];
            byte += 1;
        }
        k += 1;
    }
    tables
}

/// Incremental CRC-32 hasher.
///
/// ```
/// use pltrs_codec::checksum::Crc32;
///
/// let mut crc = Crc32::new();
/// crc.update(b"1234");
/// crc.update(b"56789");
/// assert_eq!(crc.finish(), 0xCBF4_3926);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crc32 {
    /// The CRC register, kept in its pre-inverted form.
    register: u32,
}

impl Crc32 {
    /// Creates a hasher for the empty message.
    pub const fn new() -> Self {
        Self { register: u32::MAX }
    }

    /// Feeds `data` into the checksum.
    pub fn update(&mut self, data: &[u8]) {
        let (blocks, tail) = data.as_chunks::<8>();
        let mut crc = self.register;
        for &[b0, b1, b2, b3, b4, b5, b6, b7] in blocks {
            let [c0, c1, c2, c3] = crc.to_le_bytes();
            crc = TABLES[7][usize::from(b0 ^ c0)]
                ^ TABLES[6][usize::from(b1 ^ c1)]
                ^ TABLES[5][usize::from(b2 ^ c2)]
                ^ TABLES[4][usize::from(b3 ^ c3)]
                ^ TABLES[3][usize::from(b4)]
                ^ TABLES[2][usize::from(b5)]
                ^ TABLES[1][usize::from(b6)]
                ^ TABLES[0][usize::from(b7)];
        }
        for &byte in tail {
            crc = (crc >> 8) ^ TABLES[0][usize::from(byte ^ crc as u8)];
        }
        self.register = crc;
    }

    /// Returns the checksum of everything fed so far.
    pub const fn finish(&self) -> u32 {
        !self.register
    }
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the CRC-32 of `data` in one call.
pub fn crc32(data: &[u8]) -> u32 {
    let mut hasher = Crc32::new();
    hasher.update(data);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bit-at-a-time reference implementation.
    fn reference(data: &[u8]) -> u32 {
        let mut crc = u32::MAX;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ POLYNOMIAL
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn known_answers() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn matches_bitwise_reference_for_all_lengths() {
        let data: Vec<u8> = (0..300u32).map(|i| (i * 7 + i / 5) as u8).collect();
        for len in 0..data.len() {
            assert_eq!(crc32(&data[..len]), reference(&data[..len]), "length {len}");
        }
    }

    #[test]
    fn incremental_updates_match_one_shot() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 31 % 251) as u8).collect();
        for split in [0, 1, 7, 8, 9, 500, 999, 1000] {
            let mut hasher = Crc32::default();
            hasher.update(&data[..split]);
            hasher.update(&data[split..]);
            assert_eq!(hasher.finish(), crc32(&data), "split at {split}");
        }
    }
}
