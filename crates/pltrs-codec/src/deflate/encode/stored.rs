//! Level 0: zlib-ng's `deflate_stored` (by Mark Adler), stored blocks
//! copied straight from the input to the caller's buffer when they are
//! large enough, else gathered in the window first.
//!
//! Level 0 never hashes, so the bookkeeping zlib-ng keeps for a later
//! change of level (`matches`, `insert`, `high_water`) is left out.

use super::trees::send_stored_header;
use super::{BlockState, Deflater, Flush, MAX_STORED, Output, W_SIZE, WINDOW_SIZE};

/// Bytes of a stored block's header, with what the bit buffer holds.
fn header_bytes(out: &Output<'_>) -> u32 {
    (out.bit_count() + 42) >> 3
}

impl Deflater {
    /// Copies as many stored blocks as the caller's buffer takes, then
    /// keeps the rest of the input in the window (`deflate_stored`).
    pub(super) fn deflate_stored(
        &mut self,
        input: &mut &[u8],
        out: &mut Output<'_>,
        flush: Flush,
    ) -> BlockState {
        let pending_size = u32::try_from(self.pending_buf_size).unwrap_or(u32::MAX);
        let min_block = (pending_size - 5).min(W_SIZE);
        let original = *input;
        let mut last = false;
        loop {
            let header = header_bytes(out);
            let room = u32::try_from(out.room()).unwrap_or(u32::MAX);
            if room < header {
                break;
            }
            let have = room - header;
            let left = (i64::from(self.strstart) - self.block_start) as u32;
            let available = u64::from(left) + input.len() as u64;
            let len = u64::from(MAX_STORED).min(available).min(u64::from(have)) as u32;
            if len < min_block
                && ((len == 0 && flush != Flush::Finish)
                    || flush == Flush::None
                    || u64::from(len) != available)
            {
                break;
            }
            last = flush == Flush::Finish && u64::from(len) == available;
            send_stored_header(out, len as u16, last);
            out.flush_pending();
            let from_window = left.min(len);
            if from_window > 0 {
                let start = self.block_start as usize;
                out.deliver(&self.win.bytes[start..start + from_window as usize]);
                self.block_start += i64::from(from_window);
            }
            let from_input = (len - from_window) as usize;
            if from_input > 0 {
                let (taken, rest) = input.split_at(from_input);
                self.adler.update(taken);
                out.deliver(taken);
                *input = rest;
            }
            if last {
                break;
            }
        }
        let used = original.len() - input.len();
        if used > 0 {
            let copied = &original[..used];
            if used >= W_SIZE as usize {
                self.win.bytes[..W_SIZE as usize]
                    .copy_from_slice(&copied[used - W_SIZE as usize..]);
                self.strstart = W_SIZE;
            } else {
                if WINDOW_SIZE - self.strstart <= used as u32 {
                    self.slide_stored();
                }
                let start = self.strstart as usize;
                self.win.bytes[start..start + used].copy_from_slice(copied);
                self.strstart += used as u32;
            }
            self.block_start = i64::from(self.strstart);
        }
        if last {
            return BlockState::FinishDone;
        }
        let mut have = WINDOW_SIZE - self.strstart;
        if input.len() > have as usize && self.block_start >= i64::from(W_SIZE) {
            self.block_start -= i64::from(W_SIZE);
            self.slide_stored();
            have += W_SIZE;
        }
        let have = have.min(u32::try_from(input.len()).unwrap_or(u32::MAX));
        if have > 0 {
            let read = self.read_buf(input, self.strstart, have);
            self.strstart += read;
        }
        let have = (pending_size - header_bytes(out)).min(MAX_STORED);
        let min_block = have.min(W_SIZE);
        let left = (i64::from(self.strstart) - self.block_start) as u32;
        if left >= min_block
            || ((left > 0 || flush == Flush::Finish)
                && flush != Flush::None
                && input.is_empty()
                && left <= have)
        {
            let len = left.min(have);
            last = flush == Flush::Finish && input.is_empty() && len == left;
            let start = self.block_start as usize;
            send_stored_header(out, len as u16, last);
            out.put_bytes(&self.win.bytes[start..start + len as usize]);
            self.block_start += i64::from(len);
            out.flush_pending();
        }
        if last {
            BlockState::FinishStarted
        } else {
            BlockState::NeedMore
        }
    }

    /// Moves the window's upper half down, for level 0.
    fn slide_stored(&mut self) {
        self.strstart -= W_SIZE;
        let kept = self.strstart as usize;
        self.win
            .bytes
            .copy_within(W_SIZE as usize..W_SIZE as usize + kept, 0);
    }
}
