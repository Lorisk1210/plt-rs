//! Levels 7 to 9: zlib-ng's `deflate_slow`, lazy parsing. A match is
//! only taken once the next position offers none longer.

use super::{
    BlockState, Deflater, Flush, MAX_DIST, MIN_LOOKAHEAD, MIN_MATCH, Output, Strategy,
    WANT_MIN_MATCH,
};

/// With [`Strategy::Filtered`], matches this short are dropped.
const FILTERED_MAX: u32 = 5;

impl Deflater {
    /// Compresses at levels 7 to 9 (`deflate_slow`).
    pub(super) fn deflate_slow(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        let slow_matcher = self.config.rolls();
        loop {
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(input);
                if self.lookahead < MIN_LOOKAHEAD && flush == Flush::None {
                    return BlockState::NeedMore;
                }
                if self.lookahead == 0 {
                    break;
                }
            }
            let head = if self.lookahead >= WANT_MIN_MATCH {
                u32::from(self.quick_insert_string(self.strstart))
            } else {
                0
            };
            self.prev_match = self.match_start;
            let mut match_len = MIN_MATCH - 1;
            if head != 0
                && self.strstart > head
                && self.strstart - head <= MAX_DIST
                && self.prev_length < self.config.max_lazy
            {
                match_len = if slow_matcher {
                    self.longest_match_slow(head)
                } else {
                    self.longest_match(head)
                };
                if match_len <= FILTERED_MAX && self.strategy == Strategy::Filtered {
                    match_len = MIN_MATCH - 1;
                }
            }
            if self.prev_length >= MIN_MATCH && match_len <= self.prev_length {
                // The match at the previous position is at least as good:
                // emit it and hash the strings it covers, as far as the
                // lookahead allows.
                let max_insert = self.strstart + self.lookahead - MIN_MATCH;
                let full = self.trees.tally_dist(
                    &mut self.syms,
                    self.strstart - 1 - self.prev_match,
                    self.prev_length - MIN_MATCH,
                );
                self.prev_length -= 1;
                self.lookahead -= self.prev_length;
                let mov_fwd = self.prev_length - 1;
                if max_insert > self.strstart {
                    let count = mov_fwd.min(max_insert - self.strstart);
                    self.insert_string(self.strstart + 1, count);
                }
                self.prev_length = 0;
                self.match_available = false;
                self.strstart += mov_fwd + 1;
                if full && let Some(state) = self.flush_block(out, false) {
                    return state;
                }
            } else if self.match_available {
                // No match here, or a longer one than at the previous
                // position: that position becomes a literal.
                let byte = self.win.bytes[self.strstart as usize - 1];
                if self.trees.tally_lit(&mut self.syms, byte) {
                    self.flush_block_only(out, false);
                }
                self.prev_length = match_len;
                self.strstart += 1;
                self.lookahead -= 1;
                if out.room() == 0 {
                    return BlockState::NeedMore;
                }
            } else {
                self.prev_length = match_len;
                self.match_available = true;
                self.strstart += 1;
                self.lookahead -= 1;
            }
        }
        if self.match_available {
            let byte = self.win.bytes[self.strstart as usize - 1];
            self.trees.tally_lit(&mut self.syms, byte);
            self.match_available = false;
        }
        self.finish_blocks(out, flush)
    }
}
