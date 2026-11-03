//! Adler-32, the checksum of the zlib format (RFC 1950 §9).

/// The largest prime below 2^16.
const MODULUS: u32 = 65_521;

/// The largest number of bytes that can be summed before the running sums
/// must be reduced to stay within `u32` (zlib's `NMAX`). It is a multiple of
/// [`LANES`], so whole blocks never straddle a reduction.
const MAX_RUN: usize = 5552;

/// The bytes summed side by side, one per lane of the accumulators, which
/// the compiler turns into vector operations.
const LANES: usize = 16;

/// Incremental Adler-32 hasher.
///
/// ```
/// use pltrs_codec::checksum::Adler32;
///
/// let mut adler = Adler32::new();
/// adler.update(b"Wiki");
/// adler.update(b"pedia");
/// assert_eq!(adler.finish(), 0x11E6_0398);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Adler32 {
    /// One plus the sum of all bytes, modulo [`MODULUS`].
    a: u32,
    /// The sum of every intermediate value of `a`, modulo [`MODULUS`].
    b: u32,
}

impl Adler32 {
    /// Creates a hasher for the empty message.
    pub const fn new() -> Self {
        Self { a: 1, b: 0 }
    }

    /// Feeds `data` into the checksum.
    pub fn update(&mut self, data: &[u8]) {
        let (mut a, mut b) = (self.a, self.b);
        for run in data.chunks(MAX_RUN) {
            let (blocks, tail) = run.as_chunks::<LANES>();
            // Lane j sums the j-th byte of every block (`sums`) and, after
            // every block, adds that sum to `steps`: each byte once per block
            // from its own to the last.
            let mut sums = [0u32; LANES];
            let mut steps = [0u32; LANES];
            for block in blocks {
                for ((sum, steps), &byte) in sums.iter_mut().zip(&mut steps).zip(block) {
                    *sum += u32::from(byte);
                    *steps += *sum;
                }
            }
            // Over these n bytes, `b` gains n copies of the old `a` and each
            // byte once per step from its own to the last: LANES per block
            // from its own on, less its lane.
            let n = (blocks.len() * LANES) as u64;
            let wide = |lanes: [u32; LANES]| lanes.map(u64::from);
            let byte_steps = LANES as u64 * wide(steps).iter().sum::<u64>()
                - (0..)
                    .zip(wide(sums))
                    .map(|(lane, sum)| lane * sum)
                    .sum::<u64>();
            let modulus = u64::from(MODULUS);
            b = ((u64::from(b) + n * u64::from(a) + byte_steps) % modulus) as u32;
            a = ((u64::from(a) + wide(sums).iter().sum::<u64>()) % modulus) as u32;
            for &byte in tail {
                a += u32::from(byte);
                b += a;
            }
            a %= MODULUS;
            b %= MODULUS;
        }
        self.a = a;
        self.b = b;
    }

    /// Returns the checksum of everything fed so far.
    pub const fn finish(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

impl Default for Adler32 {
    fn default() -> Self {
        Self::new()
    }
}

/// Computes the Adler-32 of `data` in one call.
pub fn adler32(data: &[u8]) -> u32 {
    let mut hasher = Adler32::new();
    hasher.update(data);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-at-a-time reference with a reduction after every byte.
    fn reference(data: &[u8]) -> u32 {
        let (mut a, mut b) = (1u32, 0u32);
        for &byte in data {
            a = (a + u32::from(byte)) % MODULUS;
            b = (b + a) % MODULUS;
        }
        (b << 16) | a
    }

    #[test]
    fn known_answers() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"a"), 0x0062_0062);
        assert_eq!(adler32(b"abc"), 0x024D_0127);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn matches_reference_for_all_lengths() {
        let data: Vec<u8> = (0..300u32).map(|i| (i * 13 + 7) as u8).collect();
        for len in 0..data.len() {
            assert_eq!(
                adler32(&data[..len]),
                reference(&data[..len]),
                "length {len}"
            );
        }
    }

    #[test]
    fn worst_case_bytes_do_not_overflow() {
        let data = vec![0xFF; 3 * MAX_RUN + 17];
        assert_eq!(adler32(&data), reference(&data));
    }

    #[test]
    fn incremental_updates_match_one_shot() {
        let data: Vec<u8> = (0..20_000u32).map(|i| (i * 31 % 253) as u8).collect();
        for split in [0, 1, 7, 5552, 5553, 19_999, 20_000] {
            let mut hasher = Adler32::default();
            hasher.update(&data[..split]);
            hasher.update(&data[split..]);
            assert_eq!(hasher.finish(), adler32(&data), "split at {split}");
        }
    }
}
