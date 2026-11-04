//! Match finder for the LZ77 stage.
//!
//! Every position with at least three bytes left is hashed on those bytes
//! into one of as many buckets as the window holds positions (fewer for
//! short inputs). The candidates for a match at a position are the earlier
//! positions of its bucket within the window, newest first: zlib's hash
//! chains, which a search follows for at most a level's chain length.
//!
//! zlib links each position to the previous one of its bucket and walks
//! those links, one dependent load per candidate. Here the input is cut
//! into segments at least as long as the window, and each segment's
//! positions are laid out by bucket, every bucket's after the one before (a
//! counting sort): a chain is a run of adjacent entries in the current
//! segment and one in the segment before, which a search reads in sequence
//! while the processor compares several candidates at once. The buckets,
//! and so the candidates, their order and every match found, are those of
//! the hash chains.
//!
//! The parser inserts positions in increasing order as it passes them; the
//! positions it does not insert are no candidates (the greedy parser does
//! not hash the inside of long matches, as zlib's `deflate_fast` does not).

use std::ops::Range;

use super::input::View;
use super::level::LevelParams;
use super::tables::{MAX_MATCH, MIN_MATCH, WINDOW_SIZE};

/// A back-reference found by the match finder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Match {
    pub(crate) length: usize,
    pub(crate) distance: usize,
}

/// The number of positions of a segment: as many as 16-bit entries can
/// count, and more than the window, so that the window of every position
/// lies within its segment and the one before.
pub(crate) const SEGMENT: usize = u16::MAX as usize;

/// The number of bytes compared at once.
const WORD: usize = 8;

#[derive(Debug)]
pub(crate) struct Matcher {
    /// The length of the input.
    len: usize,
    /// The number of positions with at least three bytes left, the only
    /// ones inserted.
    hashable: usize,
    hash_shift: u32,
    /// The bucket of each position of the current segment.
    buckets: Vec<u16>,
    /// The segment of the positions inserted last.
    current: Segment,
    /// The segment before it.
    previous: Segment,
}

/// The positions of a segment, laid out by bucket.
#[derive(Debug)]
struct Segment {
    /// Its first position.
    start: usize,
    /// Its end.
    end: usize,
    /// The positions inserted less `start`, grouped by bucket and ascending
    /// within each. Each bucket has room for all of its positions, but only
    /// those inserted take an entry.
    order: Vec<u16>,
    /// Per bucket, where its entries start in `order` and where they end
    /// so far (the entry its next position takes), side by side: inserting
    /// and searching read both.
    bounds: Vec<[u16; 2]>,
}

impl Segment {
    /// An empty segment for `buckets` buckets.
    fn new(buckets: usize) -> Self {
        Self {
            start: 0,
            end: 0,
            order: Vec::new(),
            bounds: vec![[0, 0]; buckets],
        }
    }

    /// The positions inserted in `bucket`, before `end` among its entries.
    fn run(&self, bucket: usize, end: usize) -> Run<'_> {
        Run {
            start: self.start,
            entries: &self.order[usize::from(self.bounds[bucket][0])..end],
        }
    }

    /// Every position inserted in `bucket`.
    fn inserted(&self, bucket: usize) -> Run<'_> {
        self.run(bucket, usize::from(self.bounds[bucket][1]))
    }
}

/// The candidates of a position: its bucket, and its entry in the current
/// segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Chain {
    bucket: usize,
    entry: usize,
}

impl Matcher {
    /// Creates a match finder over an input `len` bytes long, sizing its
    /// tables to it. Each call reads the input in a [`View`] holding the
    /// window before the position and a segment after it.
    pub(crate) fn new(len: usize) -> Self {
        let buckets = len.next_power_of_two().clamp(256, WINDOW_SIZE);
        Self {
            len,
            hashable: len.saturating_sub(MIN_MATCH - 1),
            hash_shift: 32 - buckets.trailing_zeros(),
            buckets: Vec::new(),
            current: Segment::new(buckets),
            previous: Segment::new(buckets),
        }
    }

    /// Inserts `position`, later than those inserted before, as a candidate
    /// for later positions and returns its chain, for
    /// [`longest_match`](Self::longest_match). Positions with fewer than
    /// three bytes left are not inserted.
    pub(crate) fn insert(&mut self, data: View<'_>, position: usize) -> Option<Chain> {
        if position >= self.hashable {
            return None;
        }
        if position >= self.current.end {
            self.start_segment(data, position);
        }
        let segment = &mut self.current;
        let offset = position - segment.start;
        let bucket = usize::from(self.buckets[offset]);
        let end = &mut segment.bounds[bucket][1];
        let entry = *end;
        *end += 1;
        segment.order[usize::from(entry)] = offset as u16;
        Some(Chain {
            bucket,
            entry: usize::from(entry),
        })
    }

    /// Inserts `positions`, later than those inserted before, as candidates
    /// for later positions, like [`insert`](Self::insert) one by one.
    pub(crate) fn insert_all(&mut self, data: View<'_>, positions: Range<usize>) {
        let end = positions.end.min(self.hashable);
        let mut position = positions.start;
        while position < end {
            if position >= self.current.end {
                self.start_segment(data, position);
            }
            let segment = &mut self.current;
            let offsets = position - segment.start..end.min(segment.end) - segment.start;
            position = segment.start + offsets.end;
            // Runs of one bucket (runs of one byte, inside long matches)
            // take consecutive entries without a trip through memory.
            let mut last = usize::from(self.buckets[offsets.start]);
            let mut entry = segment.bounds[last][1];
            for (offset, &bucket) in offsets.clone().zip(&self.buckets[offsets]) {
                let bucket = usize::from(bucket);
                if bucket != last {
                    segment.bounds[last][1] = entry;
                    (last, entry) = (bucket, segment.bounds[bucket][1]);
                }
                segment.order[usize::from(entry)] = offset as u16;
                entry += 1;
            }
            segment.bounds[last][1] = entry;
        }
    }

    /// Starts a segment at `start`, laying out the buckets of its positions.
    #[cold]
    fn start_segment(&mut self, data: View<'_>, start: usize) {
        std::mem::swap(&mut self.current, &mut self.previous);
        let segment = &mut self.current;
        segment.start = start;
        segment.end = (start + SEGMENT).min(self.hashable);
        segment.order.resize(segment.end - start, 0);
        // Each position's three bytes, from three staggered slices: no step
        // waits for another, so the compiler hashes several at once.
        let shift = self.hash_shift;
        let bytes = data.at(start..segment.end + MIN_MATCH - 1);
        self.buckets.clear();
        self.buckets
            .extend(bytes.iter().zip(&bytes[1..]).zip(&bytes[2..]).map(
                |((&first, &second), &third)| {
                    let key = u32::from(first) | u32::from(second) << 8 | u32::from(third) << 16;
                    hash(key, shift)
                },
            ));
        // A counting sort: each bucket's entries follow those before it.
        // Runs of one bucket (runs of one byte) are counted at once, into
        // the first bound before it becomes the start.
        let bounds = &mut segment.bounds;
        bounds.fill([0, 0]);
        let mut last = usize::from(self.buckets[0]);
        let mut run = 0;
        for &bucket in &self.buckets {
            let bucket = usize::from(bucket);
            if bucket != last {
                bounds[last][0] += run;
                (last, run) = (bucket, 0);
            }
            run += 1;
        }
        bounds[last][0] += run;
        let mut total = 0;
        for bound in bounds.iter_mut() {
            let count = bound[0];
            *bound = [total, total];
            total += count;
        }
    }

    /// Finds the longest match for `position` that is longer than
    /// `prev_length` among the candidates of `chain`, which
    /// [`insert`](Self::insert) returned for it: the first of the longest
    /// ones, or the first to reach the level's nice length.
    pub(crate) fn longest_match(
        &self,
        data: View<'_>,
        position: usize,
        chain: Chain,
        prev_length: usize,
        params: &LevelParams,
    ) -> Option<Match> {
        let max_length = (self.len - position).min(MAX_MATCH);
        let mut best = Best {
            length: prev_length.max(MIN_MATCH - 1),
            distance: 0,
        };
        if best.length >= max_length {
            return None;
        }
        let mut budget = if prev_length >= params.good_length {
            params.max_chain / 4
        } else {
            params.max_chain
        }
        .max(1);
        let search = Search {
            position,
            max_length,
            nice_length: params.nice_length.min(max_length),
            // The earliest position within the window.
            limit: position.saturating_sub(WINDOW_SIZE),
        };
        let Chain { bucket, entry } = chain;
        // The segment before lies out of the window once `position` is a
        // window past its end.
        let earlier = if self.previous.end > search.limit {
            self.previous.inserted(bucket)
        } else {
            Run::EMPTY
        };
        let mut runs = [self.current.run(bucket, entry), earlier];
        if max_length <= WORD {
            return search_long(data, &search, best, budget, &runs);
        }
        // Most candidates differ within their first word, which one
        // comparison settles; the first that does not hands the search over
        // to the long comparisons. Every level's nice length is a word or
        // more, so no candidate here ends the search.
        debug_assert!(search.nice_length >= WORD);
        let head = data.word(position);
        for index in 0..runs.len() {
            let run = runs[index];
            for (left, &entry) in run.entries.iter().enumerate().rev() {
                let candidate = run.start + usize::from(entry);
                if candidate < search.limit {
                    return best.found();
                }
                let differing = data.word(candidate) ^ head;
                if differing == 0 {
                    runs[index].entries = &run.entries[..=left];
                    return search_long(data, &search, best, budget, &runs[index..]);
                }
                best.offer(
                    (differing.trailing_zeros() / 8) as usize,
                    position - candidate,
                );
                budget -= 1;
                if budget == 0 {
                    return best.found();
                }
            }
        }
        best.found()
    }
}

/// Goes on with the search for `search` from `best` among the first
/// `budget` candidates of `runs`, comparing them byte by byte.
fn search_long(
    data: View<'_>,
    search: &Search,
    mut best: Best,
    budget: usize,
    runs: &[Run<'_>],
) -> Option<Match> {
    let &Search {
        position,
        max_length,
        nice_length,
        limit,
    } = search;
    let candidates = runs
        .iter()
        .flat_map(Run::newest_first)
        .take_while(|&candidate| candidate >= limit)
        .take(budget);
    let current = data.at(position..position + max_length);
    for candidate in candidates {
        let earlier = data.at(candidate..candidate + max_length);
        // A longer match agrees on the bytes up to the best one's end:
        // compare the words that start and end them first.
        if best.length >= WORD {
            let end = best.length + 1 - WORD;
            if word_at(earlier, end) != word_at(current, end)
                || word_at(earlier, 0) != word_at(current, 0)
            {
                continue;
            }
        }
        let length = common_prefix(earlier, current);
        if length > best.length {
            best = Best {
                length,
                distance: position - candidate,
            };
            if length >= nice_length {
                break;
            }
        }
    }
    best.found()
}

/// What a match is searched for.
#[derive(Debug)]
struct Search {
    /// Where the match starts.
    position: usize,
    /// The longest match possible there.
    max_length: usize,
    /// A match this long ends the search.
    nice_length: usize,
    /// The earliest candidate within the window.
    limit: usize,
}

/// Positions of one bucket, of one segment.
#[derive(Clone, Copy, Debug)]
struct Run<'s> {
    /// The segment's first position.
    start: usize,
    /// The positions less `start`, ascending.
    entries: &'s [u16],
}

impl Run<'_> {
    /// No positions.
    const EMPTY: Self = Self {
        start: 0,
        entries: &[],
    };

    /// The positions, newest first.
    fn newest_first(&self) -> impl Iterator<Item = usize> {
        let start = self.start;
        self.entries
            .iter()
            .rev()
            .map(move |&entry| start + usize::from(entry))
    }
}

/// The best match of a search so far.
#[derive(Clone, Copy, Debug)]
struct Best {
    /// Its length, or the length a match must exceed while there is none.
    length: usize,
    /// Its distance; 0 while there is none.
    distance: usize,
}

impl Best {
    /// Takes a match of `length` at `distance` if it is longer. Whether it
    /// is longer is a coin toss for the branch predictor, so it selects
    /// without branching.
    fn offer(&mut self, length: usize, distance: usize) {
        let longer = usize::from(length > self.length).wrapping_neg();
        self.distance = distance & longer | self.distance & !longer;
        self.length = self.length.max(length);
    }

    /// The match found, if any.
    fn found(self) -> Option<Match> {
        (self.distance != 0).then_some(Match {
            length: self.length,
            distance: self.distance,
        })
    }
}

/// The bucket of a position whose three bytes are `key` (little-endian),
/// among `2^(32 - shift)` buckets.
fn hash(key: u32, shift: u32) -> u16 {
    (key.wrapping_mul(0x9E37_79B1) >> shift) as u16
}

/// The [`WORD`] bytes of `bytes` from `at` on, little-endian, which the
/// caller knows are there.
fn word_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(*bytes[at..].first_chunk().expect("a whole word"))
}

/// Length of the common prefix of two equally long slices.
fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    let (a_words, _) = a.as_chunks::<WORD>();
    let (b_words, _) = b.as_chunks::<WORD>();
    for (index, (x, y)) in a_words.iter().zip(b_words).enumerate() {
        let difference = u64::from_le_bytes(*x) ^ u64::from_le_bytes(*y);
        if difference != 0 {
            return index * WORD + (difference.trailing_zeros() / 8) as usize;
        }
    }
    let done = a_words.len() * WORD;
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
    fn words_are_little_endian() {
        assert_eq!(word_at(b"abcdefghi", 1), u64::from_le_bytes(*b"bcdefghi"));
    }

    #[test]
    fn finds_repeated_strings() {
        let data = b"abcdefabcdefabcdefXYZ";
        let params = CompressionLevel::BEST.params();
        let mut matcher = Matcher::new(data.len());
        let view = View::whole(data);
        for position in 0..6 {
            let chain = matcher.insert(view, position).expect("hashable");
            assert_eq!(
                matcher.longest_match(view, position, chain, 2, &params),
                None
            );
        }
        let chain = matcher.insert(view, 6).expect("hashable");
        assert_eq!(
            matcher.longest_match(view, 6, chain, 2, &params),
            Some(Match {
                length: 12,
                distance: 6
            })
        );
        assert_eq!(matcher.longest_match(view, 6, chain, 12, &params), None);
        for position in 7..data.len() - 2 {
            assert!(matcher.insert(view, position).is_some());
        }
        assert_eq!(matcher.insert(view, data.len() - 2), None);
    }

    #[test]
    fn positions_not_inserted_are_no_candidates() {
        let data = b"abcabcabc";
        let params = CompressionLevel::BEST.params();
        let mut matcher = Matcher::new(data.len());
        let view = View::whole(data);
        matcher.insert(view, 0);
        let chain = matcher.insert(view, 6).expect("hashable");
        // `abc` at 3 was not inserted: the match reaches back to 0.
        assert_eq!(
            matcher.longest_match(view, 6, chain, 2, &params),
            Some(Match {
                length: 3,
                distance: 6
            })
        );
    }

    /// zlib's hash chains over `data`: the previous position of each
    /// position's bucket.
    fn chains(data: &[u8], shift: u32) -> Vec<Option<usize>> {
        let mut last = vec![None; 1 << (32 - shift)];
        data.windows(MIN_MATCH)
            .enumerate()
            .map(|(position, bytes)| {
                let key = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]);
                last[usize::from(hash(key, shift))].replace(position)
            })
            .collect()
    }

    /// The match zlib's hash chains find for `position`: the candidates
    /// are the inserted positions of its bucket within the window, newest
    /// first, at most the chain length of them.
    fn chain_match(
        data: &[u8],
        chains: &[Option<usize>],
        inserted: &[bool],
        position: usize,
        prev_length: usize,
        params: &LevelParams,
    ) -> Option<Match> {
        let max_length = (data.len() - position).min(MAX_MATCH);
        let mut best_length = prev_length.max(MIN_MATCH - 1);
        let budget = if prev_length >= params.good_length {
            params.max_chain / 4
        } else {
            params.max_chain
        }
        .max(1);
        let limit = position.saturating_sub(WINDOW_SIZE);
        let candidates = std::iter::successors(chains[position], |&candidate| chains[candidate])
            .take_while(|&candidate| candidate >= limit)
            .filter(|&candidate| inserted[candidate])
            .take(budget);
        let mut best = None;
        for candidate in candidates {
            let length = common_prefix(
                &data[candidate..candidate + max_length],
                &data[position..position + max_length],
            );
            if length > best_length {
                best_length = length;
                best = Some(Match {
                    length,
                    distance: position - candidate,
                });
                if length >= params.nice_length.min(max_length) {
                    break;
                }
            }
        }
        best
    }

    /// Text of few symbols, long enough for several segments, with long
    /// repeats: long chains, long matches and every distance.
    fn sample(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed;
        let mut next = move || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as usize
        };
        let mut data = Vec::with_capacity(len);
        while data.len() < len {
            if data.len() > 300 && next() % 4 == 0 {
                let from = data.len() - 1 - next() % (data.len() - 1).min(40_000);
                let count = next() % 300;
                for offset in 0..count {
                    data.push(data[from + offset % (data.len() - from)]);
                }
            } else {
                data.push(b"0123456789 .l\n"[next() % 14]);
            }
        }
        data.truncate(len);
        data
    }

    #[test]
    fn matches_are_those_of_the_hash_chains() {
        let cases = [
            (100_000, 1, 1),
            (100_000, 2, 6),
            (40_000, 3, 9),
            (2_000, 4, 4),
            (5, 5, 6),
        ];
        for (len, seed, level) in cases {
            let data = sample(len, seed);
            let params = CompressionLevel::new(level).expect("valid").params();
            let mut matcher = Matcher::new(data.len());
            let view = View::whole(&data);
            let chains = chains(&data, matcher.hash_shift);
            let mut inserted = vec![false; data.len()];
            let mut position = 0;
            while position < data.len() {
                // Leave stretches out now and then, as the greedy parser does.
                if position % 997 == 5 {
                    position += 300;
                    continue;
                }
                // Insert others at once, as both parsers do inside matches.
                if position % 1009 == 7 {
                    let end = (position + 400).min(data.len());
                    matcher.insert_all(view, position..end);
                    inserted[position..end].fill(true);
                    position = end;
                    continue;
                }
                if let Some(chain) = matcher.insert(view, position) {
                    inserted[position] = true;
                    let prev_length = position % 7;
                    assert_eq!(
                        matcher.longest_match(view, position, chain, prev_length, &params),
                        chain_match(&data, &chains, &inserted, position, prev_length, &params),
                        "level {level}, position {position} of {len}"
                    );
                }
                position += 1;
            }
        }
    }
}
