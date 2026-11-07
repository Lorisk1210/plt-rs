//! Level 1: zlib-ng's `deflate_quick` (by Intel's Wajdi Feghali, Jim
//! Guilford, Vinodh Gopal, Erdinc Ozturk and Jim Kukunas). One fixed-code
//! block, written as it goes, and the first candidate of each hash taken
//! if it matches at least four bytes.

use super::trees::{send_fixed_end, send_fixed_header, send_fixed_literal, send_fixed_match};
use super::{
    BlockState, Deflater, Flush, MAX_DIST, MAX_MATCH, MIN_LOOKAHEAD, Output, WANT_MIN_MATCH,
};

/// The most bytes the bit buffer adds to the pending ones at once.
const BIT_BUFFER_BYTES: usize = 8;

impl Deflater {
    /// Opens a fixed-code block (`QUICK_START_BLOCK`).
    fn quick_start_block(&mut self, out: &mut Output<'_>, last: bool) {
        send_fixed_header(out, last);
        self.block_open = 1 + u8::from(last);
        self.block_start = i64::from(self.strstart);
    }

    /// Closes the open block, if any, and returns the state to stop with
    /// if the caller's buffer is full (`QUICK_END_BLOCK`).
    fn quick_end_block(&mut self, out: &mut Output<'_>, last: bool) -> Option<BlockState> {
        if self.block_open == 0 {
            return None;
        }
        send_fixed_end(out);
        if last {
            out.align();
        }
        self.block_open = 0;
        self.block_start = i64::from(self.strstart);
        out.flush_pending();
        (out.room() == 0).then_some(if last {
            BlockState::FinishStarted
        } else {
            BlockState::NeedMore
        })
    }

    /// Compresses at level 1 (`deflate_quick`).
    pub(super) fn deflate_quick(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        let last = flush == Flush::Finish;
        if last && self.block_open != 2 {
            if let Some(state) = self.quick_end_block(out, false) {
                return state;
            }
            self.quick_start_block(out, last);
        } else if self.block_open == 0 && self.lookahead > 0 {
            self.quick_start_block(out, last);
        }
        loop {
            if out.pending() + BIT_BUFFER_BYTES >= self.pending_buf_size {
                out.flush_pending();
                if out.room() == 0 {
                    return BlockState::NeedMore;
                }
            }
            if self.lookahead < MIN_LOOKAHEAD {
                self.fill_window(input);
                if self.lookahead < MIN_LOOKAHEAD && flush == Flush::None {
                    return BlockState::NeedMore;
                }
                if self.lookahead == 0 {
                    break;
                }
                if self.block_open == 0 {
                    self.quick_start_block(out, last);
                }
            }
            if self.lookahead >= WANT_MIN_MATCH {
                let head = u32::from(self.quick_insert_string(self.strstart));
                let dist = self.strstart.wrapping_sub(head);
                if self.strstart > head
                    && dist <= MAX_DIST
                    && let Some(len) = self.quick_match(head)
                {
                    send_fixed_match(out, len - 3, dist);
                    self.lookahead -= len;
                    self.strstart += len;
                    continue;
                }
            }
            send_fixed_literal(out, self.win.bytes[self.strstart as usize]);
            self.strstart += 1;
            self.lookahead -= 1;
        }
        debug_assert!(last, "the loop ends early unless finishing");
        self.quick_end_block(out, true)
            .unwrap_or(BlockState::FinishDone)
    }

    /// The match at `head` for the string at `strstart`, if it has at
    /// least four bytes, cut to the lookahead.
    fn quick_match(&self, head: u32) -> Option<u32> {
        let window = &self.win.bytes;
        let (scan, candidate) = (self.strstart as usize, head as usize);
        if window[scan..scan + 2] != window[candidate..candidate + 2] {
            return None;
        }
        let len = super::matcher::compare256(window, scan + 2, candidate + 2) + 2;
        (len >= WANT_MIN_MATCH).then(|| len.min(self.lookahead).min(MAX_MATCH))
    }
}
