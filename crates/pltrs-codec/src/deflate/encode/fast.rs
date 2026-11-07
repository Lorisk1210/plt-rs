//! Level 2: zlib-ng's `deflate_fast`, greedy parsing. Each match is taken
//! as found; the strings inside short matches are hashed, those inside
//! long ones are not.

use super::{
    BlockState, Deflater, Flush, MAX_DIST, MIN_LOOKAHEAD, MIN_MATCH, Output, WANT_MIN_MATCH,
};

impl Deflater {
    /// Compresses at level 2 (`deflate_fast`).
    pub(super) fn deflate_fast(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        let mut match_len = 0;
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
            if self.lookahead >= WANT_MIN_MATCH {
                let head = u32::from(self.quick_insert_string(self.strstart));
                if head != 0 && self.strstart > head && self.strstart - head <= MAX_DIST {
                    match_len = self.longest_match(head);
                }
            }
            let full = if match_len >= WANT_MIN_MATCH {
                let full = self.trees.tally_dist(
                    &mut self.syms,
                    self.strstart - self.match_start,
                    match_len - MIN_MATCH,
                );
                self.lookahead -= match_len;
                if match_len <= self.config.max_lazy && self.lookahead >= WANT_MIN_MATCH {
                    // The string at `strstart` is hashed already.
                    self.insert_string(self.strstart + 1, match_len - 1);
                    self.strstart += match_len;
                } else {
                    self.strstart += match_len;
                    self.quick_insert_string(self.strstart + 2 - MIN_MATCH);
                }
                match_len = 0;
                full
            } else {
                let byte = self.win.bytes[self.strstart as usize];
                self.lookahead -= 1;
                self.strstart += 1;
                self.trees.tally_lit(&mut self.syms, byte)
            };
            if full && let Some(state) = self.flush_block(out, false) {
                return state;
            }
        }
        self.finish_blocks(out, flush)
    }
}
