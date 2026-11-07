//! Levels 3 to 6: zlib-ng's `deflate_medium` (by Intel's Arjan van de
//! Ven). Greedy parsing that, from level 5 on, also finds the match after
//! the current one and moves their boundary back when the next match
//! extends to the left ("fizzling" the current one into literals).
//!
//! The look-ahead match lives only as long as the call: it is dropped
//! whenever the parser stops for input or output room, or refills its
//! window, as zlib-ng drops its local `next_match`.

use super::{
    BlockState, Deflater, Flush, MAX_DIST, MIN_LOOKAHEAD, MIN_MATCH, Output, WANT_MIN_MATCH,
    WINDOW_SIZE,
};

/// Below this level the match after the current one is not looked for.
const LOOK_AHEAD_LEVEL: u8 = 5;

/// A match or literal of the medium parser (`struct match`), its fields
/// 16-bit window offsets as zlib-ng's are.
#[derive(Clone, Copy, Debug, Default)]
struct Match {
    /// Where the match's bytes are copied from (`match_start`).
    source: u16,
    /// Its length, 0 for none, 1 for a literal (`match_length`).
    length: u16,
    /// Where it begins (`strstart`).
    start: u16,
    /// Where hashing of the match may begin (`orgstart`).
    orgstart: u16,
}

impl Deflater {
    /// Emits `m` as a match or as literals and returns whether the block
    /// is full (`emit_match`).
    fn emit_match(&mut self, m: Match) -> bool {
        if u32::from(m.length) < WANT_MIN_MATCH {
            let mut full = false;
            for position in m.start..m.start + m.length {
                full |= self
                    .trees
                    .tally_lit(&mut self.syms, self.win.bytes[usize::from(position)]);
                self.lookahead -= 1;
            }
            return full;
        }
        let full = self.trees.tally_dist(
            &mut self.syms,
            u32::from(m.start - m.source),
            u32::from(m.length) - MIN_MATCH,
        );
        self.lookahead -= u32::from(m.length);
        full
    }

    /// Hashes the strings of `m` after its first, as far as worthwhile
    /// (`insert_match`).
    fn insert_match(&mut self, mut m: Match) {
        if self.lookahead <= u32::from(m.length) + WANT_MIN_MATCH {
            return;
        }
        let literal = u32::from(m.length) < WANT_MIN_MATCH;
        if !literal
            && (u32::from(m.length) > 16 * self.config.max_lazy || self.lookahead < WANT_MIN_MATCH)
        {
            // A long match: only its last string.
            let end = u32::from(m.start) + u32::from(m.length);
            if end >= MIN_MATCH - 2 {
                self.quick_insert_string(end + 2 - MIN_MATCH);
            }
            return;
        }
        // The string at `strstart` is hashed already.
        m.start = m.start.wrapping_add(1);
        m.length = m.length.wrapping_sub(1);
        if literal && m.length == 0 {
            return;
        }
        let (start, len, org) = (
            i64::from(m.start),
            i64::from(m.length),
            i64::from(m.orgstart),
        );
        if start >= org {
            let count = if start + len > org {
                len
            } else {
                org - start + 1
            };
            self.insert_string(start as u32, count as u32);
        } else if !literal && org < start + len {
            self.insert_string(org as u32, (start + len - org) as u32);
        }
    }

    /// Moves the boundary between `current` and the following match `next`
    /// back while the bytes before `next` repeat before its source, when
    /// that leaves at most a literal of `current` (`fizzle_matches`).
    fn fizzle_matches(&self, current: &mut Match, next: &mut Match) {
        let window = &self.win.bytes;
        let (len, next_start, next_match) = (
            usize::from(current.length),
            usize::from(next.start),
            usize::from(next.source),
        );
        if len <= 1 || len > 1 + next_match || len > 1 + next_start {
            return;
        }
        if window[next_match + 1 - len] != window[next_start + 1 - len] {
            return;
        }
        let (mut c, mut n) = (*current, *next);
        let limit = u32::from(next.start).saturating_sub(MAX_DIST);
        let mut changed = false;
        while window[usize::from(n.source) - 1] == window[usize::from(n.start) - 1] {
            if c.length < 1 || u32::from(n.start) <= limit || n.length >= 256 || n.source <= 1 {
                break;
            }
            n.start -= 1;
            n.source -= 1;
            n.length += 1;
            c.length -= 1;
            changed = true;
        }
        if changed && c.length <= 1 && n.length != 2 {
            n.orgstart += 1;
            *current = c;
            *next = n;
        }
    }

    /// The match for the string at `strstart` along the chain from
    /// `head`: the longest if it has at least four bytes and starts
    /// before, else a literal.
    fn find_match(&mut self, head: u32) -> Match {
        let strstart = self.strstart as u16;
        let mut m = Match {
            source: 0,
            length: 1,
            start: strstart,
            orgstart: strstart,
        };
        if head != 0 && self.strstart > head && self.strstart - head <= MAX_DIST {
            m.length = self.longest_match(head) as u16;
            m.source = self.match_start as u16;
            if u32::from(m.length) < WANT_MIN_MATCH || m.source >= m.start {
                m.length = 1;
            }
        }
        m
    }

    /// Compresses at levels 3 to 6 (`deflate_medium`).
    pub(super) fn deflate_medium(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        let early_exit = self.level < LOOK_AHEAD_LEVEL;
        let mut current;
        let mut next = Match::default();
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(input);
                if self.lookahead < MIN_LOOKAHEAD && flush == Flush::None {
                    return BlockState::NeedMore;
                }
                if self.lookahead == 0 {
                    break;
                }
                next.length = 0;
            }
            if !early_exit && next.length > 0 {
                current = next;
                next.length = 0;
            } else {
                let head = if self.lookahead >= WANT_MIN_MATCH {
                    u32::from(self.quick_insert_string(self.strstart))
                } else {
                    0
                };
                current = self.find_match(head);
            }
            self.insert_match(current);
            let end = u32::from(current.start) + u32::from(current.length);
            if !early_exit && self.lookahead > MIN_LOOKAHEAD && end < WINDOW_SIZE - MIN_LOOKAHEAD {
                self.strstart = end;
                let head = u32::from(self.quick_insert_string(self.strstart));
                next = self.find_match(head);
                if u32::from(next.length) >= WANT_MIN_MATCH {
                    self.fizzle_matches(&mut current, &mut next);
                }
                self.strstart = u32::from(current.start);
            } else {
                next.length = 0;
            }
            let full = self.emit_match(current);
            self.strstart += u32::from(current.length);
            if full && let Some(state) = self.flush_block(out, false) {
                return state;
            }
        }
        self.finish_blocks(out, flush)
    }
}
