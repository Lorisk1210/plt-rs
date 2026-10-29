//! SHA-1 (RFC 3174), the digest the WebSocket handshake (RFC 6455) uses to
//! derive `Sec-WebSocket-Accept` from the client's key.
//!
//! SHA-1 is broken as a collision-resistant hash: use it only where a
//! protocol requires it, never to protect data.
//!
//! ```
//! use pltrs_codec::sha1::{Sha1, sha1};
//!
//! let digest = sha1(b"abc");
//! assert_eq!(digest[..4], [0xa9, 0x99, 0x3e, 0x36]);
//!
//! let mut hasher = Sha1::new();
//! hasher.update(b"a");
//! hasher.update(b"bc");
//! assert_eq!(hasher.finish(), digest);
//! ```

/// The length of a digest in bytes.
pub const DIGEST_LEN: usize = 20;

/// The initial hash value (RFC 3174 section 6.1).
const INITIAL: [u32; 5] = [
    0x6745_2301,
    0xEFCD_AB89,
    0x98BA_DCFE,
    0x1032_5476,
    0xC3D2_E1F0,
];

/// An incremental SHA-1 hasher: feeding data in pieces gives the same
/// digest as hashing it in one go.
#[derive(Clone, Debug)]
pub struct Sha1 {
    state: [u32; 5],
    /// Bytes of the current, incomplete block.
    block: [u8; 64],
    /// How many bytes of `block` are filled.
    filled: usize,
    /// Total message length in bytes.
    length: u64,
}

impl Default for Sha1 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha1 {
    /// A hasher that has seen no data.
    pub fn new() -> Self {
        Self {
            state: INITIAL,
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }

    /// Adds `data` to the message.
    pub fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        if self.filled > 0 {
            let take = data.len().min(64 - self.filled);
            self.block[self.filled..self.filled + take].copy_from_slice(&data[..take]);
            self.filled += take;
            data = &data[take..];
            if self.filled < 64 {
                return;
            }
            compress(&mut self.state, &self.block);
            self.filled = 0;
        }
        let (blocks, rest) = data.as_chunks::<64>();
        for block in blocks {
            compress(&mut self.state, block);
        }
        self.block[..rest.len()].copy_from_slice(rest);
        self.filled = rest.len();
    }

    /// The digest of everything added (RFC 3174 padding: a one bit, zeros,
    /// and the message length in bits).
    pub fn finish(mut self) -> [u8; DIGEST_LEN] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        debug_assert_eq!(self.filled, 0, "the padding completes a block");
        let mut digest = [0; DIGEST_LEN];
        for (out, word) in digest.as_chunks_mut::<4>().0.iter_mut().zip(self.state) {
            *out = word.to_be_bytes();
        }
        digest
    }
}

/// The SHA-1 digest of `data`.
pub fn sha1(data: &[u8]) -> [u8; DIGEST_LEN] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finish()
}

/// Processes one 512-bit block (RFC 3174 section 6.1, method 1).
fn compress(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0_u32; 80];
    for (word, bytes) in w.iter_mut().zip(block.as_chunks::<4>().0) {
        *word = u32::from_be_bytes(*bytes);
    }
    for t in 16..80 {
        w[t] = (w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (t, &word) in w.iter().enumerate() {
        let (f, k) = match t {
            0..20 => ((b & c) | (!b & d), 0x5A82_7999),
            20..40 => (b ^ c ^ d, 0x6ED9_EBA1),
            40..60 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
            _ => (b ^ c ^ d, 0xCA62_C1D6),
        };
        let temp = a
            .rotate_left(5)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(word)
            .wrapping_add(k);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = temp;
    }
    for (word, value) in state.iter_mut().zip([a, b, c, d, e]) {
        *word = word.wrapping_add(value);
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    fn hex(digest: [u8; DIGEST_LEN]) -> String {
        let mut out = String::new();
        for byte in digest {
            write!(out, "{byte:02x}").unwrap();
        }
        out
    }

    #[test]
    fn rfc_3174_test_vectors() {
        assert_eq!(
            hex(sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(sha1(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1"
        );
        let mut hasher = Sha1::new();
        for _ in 0..1_000_000 / 1000 {
            hasher.update(&[b'a'; 1000]);
        }
        assert_eq!(
            hex(hasher.finish()),
            "34aa973cd4c4daa4f61eeb2bdbad27316534016f"
        );
        let mut hasher = Sha1::new();
        for _ in 0..10 {
            hasher.update(b"0123456701234567012345670123456701234567012345670123456701234567");
        }
        assert_eq!(
            hex(hasher.finish()),
            "dea356a2cddd90c7a7ecedc5ebb563934f460452"
        );
    }

    #[test]
    fn empty_and_block_boundaries() {
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        // Lengths around the 55/56/64-byte padding boundaries, fed whole
        // and byte by byte.
        let data: Vec<u8> = (0..=200_u8).collect();
        for len in [55, 56, 63, 64, 65, 119, 120, 128, 200] {
            let whole = sha1(&data[..len]);
            let mut hasher = Sha1::new();
            for byte in &data[..len] {
                hasher.update(std::slice::from_ref(byte));
            }
            assert_eq!(hasher.finish(), whole, "length {len}");
        }
        assert_eq!(
            hex(sha1(&[b'a'; 64])),
            "0098ba824b5c16427bd7a1122a5a442a25ec644d"
        );
    }
}
