//! Hash maps keyed by triangle edges, pairs of point indices.
//!
//! The standard library's `SipHash` guards maps keyed by strings an
//! adversary chooses; a triangulation's edges are small integers, which
//! multiplicative hashing (the `FxHash` scheme) spreads well enough at a
//! fraction of the cost.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// A map from a directed edge `(start, end)` to `V`.
pub(crate) type EdgeMap<V> = HashMap<(usize, usize), V, BuildHasherDefault<IndexHasher>>;

/// The odd multiplier of `FxHash`, whose high bits mix every input bit.
const MULTIPLIER: u64 = 0x517c_c1b7_2722_0a95;

/// Hashes integers by rotating, mixing in and multiplying each word.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct IndexHasher(u64);

impl IndexHasher {
    fn add(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for IndexHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.add(u64::from_le_bytes(word));
        }
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_of_either_direction_are_distinct_keys() {
        let mut map = EdgeMap::default();
        map.insert((1, 2), 'a');
        map.insert((2, 1), 'b');
        map.insert((1, 2), 'c');
        assert_eq!(map.len(), 2);
        assert_eq!(map.remove(&(1, 2)), Some('c'));
        assert_eq!(map.get(&(2, 1)), Some(&'b'));
    }

    #[test]
    fn hashing_bytes_and_words_mixes_every_word() {
        let hash = |bytes: &[u8]| {
            let mut h = IndexHasher::default();
            h.write(bytes);
            h.finish()
        };
        assert_ne!(hash(&[1; 9]), hash(&[1; 8]));
        let mut words = IndexHasher::default();
        words.write_usize(7);
        assert_ne!(words.finish(), IndexHasher::default().finish());
    }
}
