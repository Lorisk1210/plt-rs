//! The sliding window and the hash chains (zlib-ng's `fill_window`,
//! `slide_hash` and `insert_string`).
//!
//! The window holds the last 64 KiB of input and moves its upper half
//! down when the parser nears its end, as zlib-ng's does: positions are
//! window offsets, a match may not start at offset 0, and the bytes past
//! the input's end hold what zlib-ng's do (zeros at first, then what the
//! window held before), since its four-byte hashes of the last positions
//! read them.

use pltrs_core::alloc::{self, OutOfMemory};

use super::{Deflater, MAX_DIST, MAX_MATCH, MIN_LOOKAHEAD, MIN_MATCH, W_MASK, W_SIZE, WINDOW_SIZE};

/// Entries of the hash table of chain heads (`HASH_SIZE`).
const HASH_SIZE: usize = 1 << 16;
/// The rolling hash of level 9 keeps 15 bits.
const ROLL_MASK: u32 = (1 << 15) - 1;
/// How far past the data `fill_window` zeroes the window (`WIN_INIT`).
const WIN_INIT: u32 = MAX_MATCH;
/// Bytes past the window that word reads may touch; never read in
/// practice, kept zero.
const PADDING: usize = 8;

/// The window and its hash chains.
#[derive(Debug)]
pub(super) struct Window {
    /// The window, `WINDOW_SIZE` bytes and the padding.
    pub(super) bytes: Vec<u8>,
    /// The most recent position of each hash (`head`), 0 for none.
    pub(super) head: Vec<u16>,
    /// The previous position with the same hash, by position modulo the
    /// window (`prev`).
    pub(super) prev: Vec<u16>,
    /// The end of the bytes ever written or zeroed (`high_water`).
    high_water: u32,
}

impl Window {
    /// An empty window.
    ///
    /// # Errors
    ///
    /// When its memory cannot be allocated.
    pub(super) fn new() -> Result<Self, OutOfMemory> {
        Ok(Self {
            bytes: zeros(WINDOW_SIZE as usize + PADDING)?,
            head: zeros(HASH_SIZE)?,
            prev: zeros(W_SIZE as usize)?,
            high_water: 0,
        })
    }

    /// Makes the window that of a new stream of at most `len` bytes, as
    /// far as the stream reads it: no chain heads, no links for the
    /// positions it reaches, and nothing zeroed past its data yet, so
    /// that the bytes it reads past its end are zeroed again before it
    /// reads them. The bytes of earlier streams stay where it never reads.
    pub(super) fn restart(&mut self, len: usize) {
        self.head.fill(0);
        let reached = len.min(self.prev.len());
        self.prev[..reached].fill(0);
        self.high_water = 0;
    }

    /// The four bytes at `position`, little-endian.
    #[inline]
    pub(super) fn word(&self, position: u32) -> u32 {
        let p = position as usize;
        u32::from_le_bytes([
            self.bytes[p],
            self.bytes[p + 1],
            self.bytes[p + 2],
            self.bytes[p + 3],
        ])
    }

    /// Moves the hash entries down with the window, forgetting those that
    /// leave it (`slide_hash`).
    fn slide_hash(&mut self) {
        let slide = |table: &mut [u16]| {
            for entry in table {
                *entry = entry.saturating_sub(W_SIZE as u16);
            }
        };
        slide(&mut self.head);
        slide(&mut self.prev);
    }
}

/// `len` zeros, allocated fallibly.
fn zeros<T: Clone + Default>(len: usize) -> Result<Vec<T>, OutOfMemory> {
    let mut values = alloc::try_with_capacity(len)?;
    values.resize(len, T::default());
    Ok(values)
}

/// The multiplicative hash of four bytes (`insert_string.c`'s
/// `HASH_CALC`).
#[inline]
fn hash(word: u32) -> usize {
    (word.wrapping_mul(2_654_435_761) >> 16) as usize
}

/// The rolling hash of level 9, extended by one byte
/// (`update_hash_roll`).
#[inline]
pub(super) fn roll(hash: u32, byte: u8) -> u32 {
    (hash << 5 ^ u32::from(byte)) & ROLL_MASK
}

impl Deflater {
    /// Hashes the string at `position` into its chain and returns the
    /// chain's previous head (`quick_insert_string`, or its rolling
    /// variant at level 9).
    #[inline]
    pub(super) fn quick_insert_string(&mut self, position: u32) -> u16 {
        let key = if self.config.rolls() {
            self.ins_h = roll(self.ins_h, self.win.bytes[position as usize + 2]);
            self.ins_h as usize
        } else {
            hash(self.win.word(position))
        };
        self.link(key, position)
    }

    /// Hashes the `count` strings from `position` on (`insert_string`).
    #[inline]
    pub(super) fn insert_string(&mut self, position: u32, count: u32) {
        for position in position..position + count {
            self.quick_insert_string(position);
        }
    }

    /// Makes `position` the head of chain `key`, unless it is already.
    #[inline]
    fn link(&mut self, key: usize, position: u32) -> u16 {
        let head = self.win.head[key];
        if u32::from(head) != position {
            self.win.prev[(position & W_MASK) as usize] = head;
            self.win.head[key] = position as u16;
        }
        head
    }

    /// Reads input until the parser has `MIN_LOOKAHEAD` bytes ahead or the
    /// input runs out, first moving the window down if the parser is near
    /// its end (`fill_window`).
    pub(super) fn fill_window(&mut self, input: &mut &[u8]) {
        loop {
            let mut more = WINDOW_SIZE - self.lookahead - self.strstart;
            if self.strstart >= W_SIZE + MAX_DIST {
                let (low, high) = self.win.bytes.split_at_mut(W_SIZE as usize);
                low.copy_from_slice(&high[..W_SIZE as usize]);
                if self.match_start >= W_SIZE {
                    self.match_start -= W_SIZE;
                } else {
                    self.match_start = 0;
                    self.prev_length = 0;
                }
                self.strstart -= W_SIZE;
                self.block_start -= i64::from(W_SIZE);
                self.win.slide_hash();
                more += W_SIZE;
            }
            if input.is_empty() {
                break;
            }
            let read = self.read_buf(input, self.strstart + self.lookahead, more);
            self.lookahead += read;
            // Hash the string before the new bytes, which may have been
            // left out for lack of lookahead.
            if self.lookahead >= MIN_MATCH {
                let start = self.strstart as usize;
                if self.config.rolls() {
                    self.ins_h = roll(u32::from(self.win.bytes[start]), self.win.bytes[start + 1]);
                } else if self.strstart >= 1 {
                    self.quick_insert_string(self.strstart + 2 - MIN_MATCH);
                }
            }
            if self.lookahead >= MIN_LOOKAHEAD || input.is_empty() {
                break;
            }
        }
        self.zero_past_data();
    }

    /// Zeroes up to `WIN_INIT` bytes past the data the first time the data
    /// reaches them, as zlib-ng does for its memory checkers.
    fn zero_past_data(&mut self) {
        let high_water = self.win.high_water;
        if high_water >= WINDOW_SIZE {
            return;
        }
        let end = self.strstart + self.lookahead;
        let zeroed = if high_water < end {
            end..end + (WINDOW_SIZE - end).min(WIN_INIT)
        } else if high_water < end + WIN_INIT {
            high_water..high_water + (end + WIN_INIT - high_water).min(WINDOW_SIZE - high_water)
        } else {
            return;
        };
        self.win.bytes[zeroed.start as usize..zeroed.end as usize].fill(0);
        self.win.high_water = zeroed.end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_follow_zlib_ng() {
        assert_eq!(hash(0), 0);
        assert_eq!(hash(u32::from_le_bytes(*b"abcd")), 0x630D);
        assert_eq!(
            roll(roll(roll(0, b'a'), b'b'), b'c'),
            (0x61 << 10 ^ 0x62 << 5 ^ 0x63) & ROLL_MASK
        );
    }

    #[test]
    fn sliding_forgets_old_positions() {
        let mut window = Window::new().unwrap();
        window.head[1] = 40_000;
        window.head[2] = 100;
        window.prev[3] = W_SIZE as u16;
        window.slide_hash();
        assert_eq!(
            (window.head[1], window.head[2], window.prev[3]),
            (40_000 - W_SIZE as u16, 0, 0)
        );
    }
}
