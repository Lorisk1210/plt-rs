//! The match finder: zlib-ng's `longest_match` (`match_tpl.h`), and the
//! slower variant level 9 uses, which also jumps to the chains of the
//! strings inside a match to reach older candidates sooner.

use super::window::roll;
use super::{Deflater, MAX_DIST, MIN_MATCH, W_MASK};

/// Below this level the search stops at the first candidate that is not
/// longer than the best (`EARLY_EXIT_TRIGGER_LEVEL`).
const EARLY_EXIT_LEVEL: u8 = 5;

/// The eight bytes at `at`, little-endian.
#[inline]
fn word(window: &[u8], at: usize) -> u64 {
    let bytes: [u8; 8] = window[at..at + 8]
        .try_into()
        .expect("eight bytes are eight bytes");
    u64::from_le_bytes(bytes)
}

/// How many of the 256 bytes from `a` and from `b` agree before the first
/// difference (`compare256`).
#[inline]
pub(super) fn compare256(window: &[u8], a: usize, b: usize) -> u32 {
    let (a, b) = (&window[a..a + 256], &window[b..b + 256]);
    let (a, b) = (a.as_chunks::<8>().0, b.as_chunks::<8>().0);
    for (offset, (&x, &y)) in (0..).step_by(8).zip(a.iter().zip(b)) {
        let difference = u64::from_le_bytes(x) ^ u64::from_le_bytes(y);
        if difference != 0 {
            return offset + difference.trailing_zeros() / 8;
        }
    }
    256
}

/// How many of the 256 bytes from `at` equal the byte at `byte`
/// (`compare256_rle`).
#[inline]
pub(super) fn compare256_rle(window: &[u8], byte: usize, at: usize) -> u32 {
    let run = u64::from_le_bytes([window[byte]; 8]);
    for (offset, &chunk) in (0..)
        .step_by(8)
        .zip(window[at..at + 256].as_chunks::<8>().0)
    {
        let difference = run ^ u64::from_le_bytes(chunk);
        if difference != 0 {
            return offset + difference.trailing_zeros() / 8;
        }
    }
    256
}

/// How far into a match the candidates are compared: the word ending at
/// the byte that would make a match longer than `best`, of the width the
/// comparison uses.
#[inline]
fn end_offset(best: u32) -> u32 {
    match best {
        0..4 => best - 1,
        4..8 => best - 3,
        _ => best - 7,
    }
}

/// Whether the candidate at `candidate` agrees with the string at `scan`
/// in the bytes that matter for beating a match of `best` bytes: two,
/// four or eight bytes at the start and at the end offset.
#[inline]
fn could_beat(window: &[u8], best: u32, scan: (u64, u64), candidate: usize, end: usize) -> bool {
    let mask = match best {
        0..4 => 0xFFFF,
        4..8 => 0xFFFF_FFFF,
        _ => u64::MAX,
    };
    (word(window, candidate + end) ^ scan.1) & mask == 0
        && (word(window, candidate) ^ scan.0) & mask == 0
}

impl Deflater {
    /// The search's limits: the chain length (cut to a quarter after a
    /// good match), the nice length and the lowest position a match may
    /// start after.
    fn search_limits(&self, best: u32) -> (u32, u32, u32) {
        let mut chain = self.config.max_chain;
        if best >= self.config.good_length {
            chain >>= 2;
        }
        let limit = self.strstart.saturating_sub(MAX_DIST);
        (chain, self.config.nice_length, limit)
    }

    /// The longest match for the string at `strstart` along the chain from
    /// `cur_match`, longer than `prev_length`: its length (at most the
    /// lookahead), its start left in `match_start` (`longest_match`).
    pub(super) fn longest_match(&mut self, mut cur_match: u32) -> u32 {
        let strstart = self.strstart;
        let lookahead = self.lookahead;
        let window = &self.win.bytes;
        let prev = &self.win.prev;
        let scan = strstart as usize;
        let mut best = if self.prev_length != 0 {
            self.prev_length
        } else {
            MIN_MATCH - 1
        };
        let mut offset = end_offset(best) as usize;
        let mut scan_words = (word(window, scan), word(window, scan + offset));
        let (mut chain, nice, limit) = self.search_limits(best);
        let early_exit = self.level < EARLY_EXIT_LEVEL;
        let next_in_chain = |chain: &mut u32, cur_match: &mut u32| {
            *chain -= 1;
            if *chain == 0 {
                return false;
            }
            *cur_match = u32::from(prev[(*cur_match & W_MASK) as usize]);
            *cur_match > limit
        };
        loop {
            if cur_match >= strstart {
                return best;
            }
            while !could_beat(window, best, scan_words, cur_match as usize, offset) {
                if !next_in_chain(&mut chain, &mut cur_match) {
                    return best;
                }
            }
            let len = compare256(window, scan + 2, cur_match as usize + 2) + 2;
            if len > best {
                self.match_start = cur_match;
                if len >= lookahead {
                    return lookahead;
                }
                best = len;
                if best >= nice {
                    return best;
                }
                offset = end_offset(best) as usize;
                scan_words.1 = word(window, scan + offset);
            } else if early_exit {
                return best;
            }
            if !next_in_chain(&mut chain, &mut cur_match) {
                return best;
            }
        }
    }

    /// [`longest_match`](Self::longest_match) as level 9 runs it
    /// (`longest_match_slow`): the search follows the chain of the string
    /// at some offset into the match so far whose chain reaches farthest
    /// back, comparing its candidates moved back by that offset.
    pub(super) fn longest_match_slow(&mut self, mut cur_match: u32) -> u32 {
        let strstart = self.strstart;
        let lookahead = self.lookahead;
        let window = &self.win.bytes;
        let head = &self.win.head;
        let prev = &self.win.prev;
        let scan = strstart as usize;
        let mut best = if self.prev_length != 0 {
            self.prev_length
        } else {
            MIN_MATCH - 1
        };
        let mut offset = end_offset(best) as usize;
        let mut scan_words = (word(window, scan), word(window, scan + offset));
        let (mut chain, nice, limit_base) = self.search_limits(best);
        let capped = |best: u32| best.min(lookahead);
        // Candidates are positions of the string `match_offset` bytes into
        // the scan; matches start that much before them.
        let mut match_offset = 0;
        if best >= MIN_MATCH {
            let mut hash = roll(roll(0, window[scan + 1]), window[scan + 2]);
            for i in MIN_MATCH..=best {
                hash = roll(hash, window[scan + i as usize]);
                let position = u32::from(head[hash as usize]);
                if position < cur_match {
                    match_offset = i - 2;
                    cur_match = position;
                }
            }
            if cur_match <= limit_base + match_offset {
                return capped(best);
            }
        }
        let mut limit = limit_base + match_offset;
        let next_in_chain = |chain: &mut u32, cur_match: &mut u32, limit: u32| {
            *chain -= 1;
            if *chain == 0 {
                return false;
            }
            *cur_match = u32::from(prev[(*cur_match & W_MASK) as usize]);
            *cur_match > limit
        };
        loop {
            if cur_match >= strstart {
                return best;
            }
            while !could_beat(
                window,
                best,
                scan_words,
                (cur_match - match_offset) as usize,
                offset,
            ) {
                if !next_in_chain(&mut chain, &mut cur_match, limit) {
                    return best;
                }
            }
            let match_start = cur_match - match_offset;
            let len = compare256(window, scan + 2, match_start as usize + 2) + 2;
            if len > best {
                self.match_start = match_start;
                if len >= lookahead {
                    return lookahead;
                }
                best = len;
                if best >= nice {
                    return best;
                }
                offset = end_offset(best) as usize;
                scan_words.1 = word(window, scan + offset);
                if len > MIN_MATCH && match_start + len < strstart {
                    // Follow instead the chain of the string in the match
                    // that goes back farthest, or of the string ending it.
                    let mut next = match_start;
                    match_offset = 0;
                    for i in 0..=len - MIN_MATCH {
                        let position = u32::from(prev[((match_start + i) & W_MASK) as usize]);
                        if position < next {
                            if position <= limit_base + i {
                                return capped(best);
                            }
                            next = position;
                            match_offset = i;
                        }
                    }
                    cur_match = next;
                    let end = scan + (len - (MIN_MATCH + 1)) as usize;
                    let hash = roll(roll(roll(0, window[end]), window[end + 1]), window[end + 2]);
                    let position = u32::from(head[hash as usize]);
                    if position < cur_match {
                        match_offset = len - (MIN_MATCH + 1);
                        if position <= limit_base + match_offset {
                            return capped(best);
                        }
                        cur_match = position;
                    }
                    limit = limit_base + match_offset;
                    continue;
                }
            }
            if !next_in_chain(&mut chain, &mut cur_match, limit) {
                return best;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparisons_count_equal_bytes() {
        let mut window = vec![0_u8; 1024];
        window[300..600].copy_from_slice(&[7; 300]);
        window[12] = 1;
        assert_eq!(compare256(&window, 0, 600), 12);
        assert_eq!(compare256(&window, 13, 613), 256);
        assert_eq!(compare256_rle(&window, 300, 301), 256);
        assert_eq!(compare256_rle(&window, 300, 500), 100);
        assert_eq!(compare256_rle(&window, 0, 1), 11);
    }

    #[test]
    fn end_offsets_reach_the_byte_after_the_best() {
        for best in 2..258 {
            let width = match best {
                0..4 => 2,
                4..8 => 4,
                _ => 8,
            };
            assert_eq!(end_offset(best) + width - 1, best, "best {best}");
        }
    }
}
