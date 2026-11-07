//! The strategies that ignore the hash chains: zlib-ng's `deflate_huff`
//! (literals only) and `deflate_rle` (runs of a byte, matches one back).

use super::matcher::compare256_rle;
use super::{BlockState, Deflater, Flush, MAX_MATCH, MIN_MATCH, Output};

impl Deflater {
    /// Codes every byte as a literal ([`Strategy::HuffmanOnly`],
    /// `deflate_huff`).
    ///
    /// [`Strategy::HuffmanOnly`]: super::Strategy::HuffmanOnly
    pub(super) fn deflate_huff(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        loop {
            if self.lookahead == 0 {
                self.fill_window(input);
                if self.lookahead == 0 {
                    if flush == Flush::None {
                        return BlockState::NeedMore;
                    }
                    break;
                }
            }
            let byte = self.win.bytes[self.strstart as usize];
            self.lookahead -= 1;
            self.strstart += 1;
            if self.trees.tally_lit(&mut self.syms, byte)
                && let Some(state) = self.flush_block(out, false)
            {
                return state;
            }
        }
        self.finish_blocks(out, flush)
    }

    /// Codes runs of a byte as matches one back, everything else as
    /// literals ([`Strategy::Rle`], `deflate_rle`).
    ///
    /// [`Strategy::Rle`]: super::Strategy::Rle
    pub(super) fn deflate_rle(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        loop {
            if self.lookahead <= MAX_MATCH {
                self.fill_window(input);
                if self.lookahead <= MAX_MATCH && flush == Flush::None {
                    return BlockState::NeedMore;
                }
                if self.lookahead == 0 {
                    break;
                }
            }
            let mut match_len = 0;
            if self.lookahead >= MIN_MATCH && self.strstart > 0 {
                let scan = self.strstart as usize - 1;
                let window = &self.win.bytes;
                if window[scan] == window[scan + 1] && window[scan + 1] == window[scan + 2] {
                    match_len = (compare256_rle(window, scan, scan + 3) + 2)
                        .min(self.lookahead)
                        .min(MAX_MATCH);
                }
            }
            let full = if match_len >= MIN_MATCH {
                self.lookahead -= match_len;
                self.strstart += match_len;
                self.trees
                    .tally_dist(&mut self.syms, 1, match_len - MIN_MATCH)
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
