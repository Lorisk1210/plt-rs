//! Hash-chain match finder for the LZ77 stage.
//!
//! Every position with at least three bytes left is hashed on those bytes;
//! `head` holds the latest position per hash and `prev` links each position
//! to the previous one with the same hash, forming chains that are walked
//! newest-first when looking for matches.
//!
//! Positions are stored as `u32` tags (`position + 1`, wrapping, 0 = empty).
//! Wrapping on inputs beyond 4 GiB can only make a chain point at an
//! unrelated position, which is harmless: candidates are always verified
//! byte by byte and must lie within the window.

use super::level::LevelParams;
use super::tables::{MAX_MATCH, MIN_MATCH, WINDOW_SIZE};

/// A back-reference found by the match finder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) length: usize,
    pub(crate) distance: usize,
}

#[derive(Debug)]
pub(crate) struct Matcher<'a> {
    data: &'a [u8],
    head: Vec<u32>,
    prev: Vec<u32>,
    hash_shift: u32,
    window_mask: usize,
}

impl<'a> Matcher<'a> {
    /// Creates a match finder over `data`, sizing its tables to the input.
    pub(crate) fn new(data: &'a [u8]) -> Self {
        let window = data.len().next_power_of_two().clamp(256, WINDOW_SIZE);
        let hash_bits = window.trailing_zeros();
        Self {
            data,
            head: vec![0; window],
            prev: vec![0; window],
            hash_shift: 32 - hash_bits,
            window_mask: window - 1,
        }
    }

    /// Records `position` in its hash chain and returns the most recent
    /// earlier position with the same hash, if it is within the window.
    /// Positions with fewer than three bytes left are not recorded.
    pub(crate) fn insert(&mut self, position: usize) -> Option<usize> {
        let bytes = self.data.get(position..position + MIN_MATCH)?;
        let key = u32::from(bytes[0]) | u32::from(bytes[1]) << 8 | u32::from(bytes[2]) << 16;
        let slot = &mut self.head[(key.wrapping_mul(0x9E37_79B1) >> self.hash_shift) as usize];
        let previous = std::mem::replace(slot, tag(position));
        self.prev[position & self.window_mask] = previous;
        resolve(previous, position)
    }

    /// Finds the longest match for `position` that is longer than
    /// `prev_length`, starting at chain candidate `candidate`.
    pub(crate) fn longest_match(
        &self,
        position: usize,
        candidate: usize,
        prev_length: usize,
        params: &LevelParams,
    ) -> Option<Match> {
        let max_length = (self.data.len() - position).min(MAX_MATCH);
        let mut best_length = prev_length.max(MIN_MATCH - 1);
        if best_length >= max_length {
            return None;
        }
        let nice_length = params.nice_length.min(max_length);
        let mut chain = if prev_length >= params.good_length {
            params.max_chain / 4
        } else {
            params.max_chain
        }
        .max(1);
        let current = &self.data[position..position + max_length];
        let mut best = None;
        let mut candidate = candidate;
        loop {
            let earlier = &self.data[candidate..candidate + max_length];
            if earlier[best_length] == current[best_length] && earlier[0] == current[0] {
                let length = common_prefix(earlier, current);
                if length > best_length {
                    best_length = length;
                    best = Some(Match {
                        length,
                        distance: position - candidate,
                    });
                    if length >= nice_length {
                        break;
                    }
                }
            }
            chain -= 1;
            if chain == 0 {
                break;
            }
            match resolve(self.prev[candidate & self.window_mask], position) {
                Some(next) if next < candidate => candidate = next,
                _ => break,
            }
        }
        best
    }
}

/// The chain tag of `position`.
fn tag(position: usize) -> u32 {
    // Deliberately wrapping: see the module documentation.
    (position as u32).wrapping_add(1)
}

/// Turns a chain tag back into a position, if it lies within the window
/// before `position`.
fn resolve(tag_value: u32, position: usize) -> Option<usize> {
    if tag_value == 0 {
        return None;
    }
    let distance = tag(position).wrapping_sub(tag_value) as usize;
    (distance >= 1 && distance <= WINDOW_SIZE.min(position)).then(|| position - distance)
}

/// Length of the common prefix of two equally long slices.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    let (a_words, _) = a.as_chunks::<8>();
    let (b_words, _) = b.as_chunks::<8>();
    for (index, (x, y)) in a_words.iter().zip(b_words).enumerate() {
        let difference = u64::from_le_bytes(*x) ^ u64::from_le_bytes(*y);
        if difference != 0 {
            return index * 8 + (difference.trailing_zeros() / 8) as usize;
        }
    }
    let done = a_words.len() * 8;
    done + a[done..]
        .iter()
        .zip(&b[done..])
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompressionLevel;

    #[test]
    fn common_prefix_handles_word_and_tail_differences() {
        let a = b"abcdefghijklmnopq";
        assert_eq!(common_prefix(a, a), 17);
        assert_eq!(common_prefix(b"abcdefgX", b"abcdefgY"), 7);
        assert_eq!(common_prefix(b"abcdefghiX", b"abcdefghiY"), 9);
        assert_eq!(common_prefix(b"", b""), 0);
    }

    #[test]
    fn finds_repeated_strings() {
        let data = b"abcdefabcdefabcdefXYZ";
        let params = CompressionLevel::BEST.params();
        let mut matcher = Matcher::new(data);
        for position in 0..6 {
            assert_eq!(matcher.insert(position), None);
        }
        let candidate = matcher.insert(6).expect("abc seen before");
        assert_eq!(candidate, 0);
        assert_eq!(
            matcher.longest_match(6, candidate, 2, &params),
            Some(Match {
                length: 12,
                distance: 6
            })
        );
        assert_eq!(matcher.longest_match(6, candidate, 12, &params), None);
    }

    #[test]
    fn tags_outside_the_window_are_ignored() {
        assert_eq!(resolve(0, 10), None);
        assert_eq!(resolve(tag(5), 10), Some(5));
        assert_eq!(resolve(tag(10), 10), None);
        assert_eq!(resolve(tag(0), WINDOW_SIZE), Some(0));
        assert_eq!(resolve(tag(0), WINDOW_SIZE + 1), None);
    }
}
