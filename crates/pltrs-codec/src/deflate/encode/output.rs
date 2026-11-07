//! The compressor's output: bits written least-significant first, and
//! zlib's accounting of which bytes are still pending and how much room
//! the caller's output buffer has left.
//!
//! zlib keeps compressed bytes in a pending buffer and copies them into
//! the caller's buffer (`next_out`, `avail_out` bytes of room) when it
//! flushes. Here every byte goes straight into one vector, in order;
//! `delivered` counts those zlib would have copied out, and the rest are
//! pending. The parsers' decisions depend on when the caller's buffer
//! fills up, so the accounting is kept exactly.

use pltrs_core::alloc::OutOfMemory;

/// The bytes written so far and the bits of the byte being filled.
#[derive(Debug)]
pub(crate) struct Output<'o> {
    bytes: &'o mut Vec<u8>,
    /// Bits not yet in `bytes`, the first in the lowest bit.
    bits: u64,
    /// How many of `bits` are valid (below 64 between calls).
    bit_count: u32,
    /// The bytes of `bytes` the caller has taken, from `start` on.
    delivered: usize,
    /// Where this stream's bytes begin in `bytes`.
    start: usize,
    /// Room left in the caller's buffer (`avail_out`).
    room: usize,
    /// The allocation refused, if one was.
    refused: Option<OutOfMemory>,
}

impl<'o> Output<'o> {
    /// Output appended to `bytes`, into a caller's buffer of unlimited
    /// room.
    pub(crate) fn new(bytes: &'o mut Vec<u8>) -> Self {
        let start = bytes.len();
        Self {
            bytes,
            bits: 0,
            bit_count: 0,
            delivered: 0,
            start,
            room: usize::MAX,
            refused: None,
        }
    }

    /// Gives the caller a new buffer of `room` bytes (a new `next_out`).
    pub(crate) fn give_room(&mut self, room: usize) {
        self.room = room;
    }

    /// Room left in the caller's buffer.
    pub(crate) fn room(&self) -> usize {
        self.room
    }

    /// The bytes the caller has taken so far.
    pub(crate) fn delivered(&self) -> usize {
        self.delivered
    }

    /// Bytes written and not yet taken (zlib's `pending`), not counting
    /// those still in the bit buffer.
    pub(crate) fn pending(&self) -> usize {
        self.bytes.len() - self.start - self.delivered
    }

    /// Bits in the bit buffer (zlib's `bi_valid` once `flush_bits` ran).
    pub(crate) fn bit_count(&self) -> u32 {
        self.bit_count
    }

    /// The end of writing: an error when memory for the output was refused.
    pub(crate) fn finish(self) -> Result<(), OutOfMemory> {
        self.refused.map_or(Ok(()), Err)
    }

    /// Whether `bytes` has room for `count` more, growing it as a `Vec`
    /// does (doubling) if it can, else by `count` alone.
    fn reserve(&mut self, count: usize) -> bool {
        if self.refused.is_some() {
            return false;
        }
        if self.bytes.capacity() - self.bytes.len() >= count {
            return true;
        }
        let doubled = count.max(self.bytes.len());
        if self.bytes.try_reserve(doubled).is_ok() || self.bytes.try_reserve_exact(count).is_ok() {
            return true;
        }
        self.refused = Some(OutOfMemory::of::<u8>(count));
        false
    }

    /// Appends bytes after the whole bytes of the bit buffer, which must
    /// be empty (zlib's `put_byte`, `put_short` and `memcpy` into the
    /// pending buffer).
    pub(crate) fn put_bytes(&mut self, bytes: &[u8]) {
        debug_assert_eq!(self.bit_count, 0, "put_bytes needs byte alignment");
        if self.reserve(bytes.len()) {
            self.bytes.extend_from_slice(bytes);
        }
    }

    /// Writes the low `count` bits of `value` (zlib-ng's `send_bits`);
    /// `count` is at most 64 and the bits above it are zero.
    #[inline]
    pub(crate) fn send_bits(&mut self, value: u64, count: u32) {
        debug_assert!(count <= 64 && (count == 64 || value >> count == 0));
        let total = self.bit_count + count;
        if total < 64 {
            self.bits |= value << self.bit_count;
            self.bit_count = total;
            return;
        }
        let full = self.bits | value << self.bit_count;
        if self.reserve(8) {
            self.bytes.extend_from_slice(&full.to_le_bytes());
        }
        // The bits of `value` that did not fit (none if it began a word).
        self.bits = value.checked_shr(64 - self.bit_count).unwrap_or(0);
        self.bit_count = total - 64;
    }

    /// Moves the whole bytes of the bit buffer into the pending bytes,
    /// keeping at most seven bits (`zng_tr_flush_bits`).
    pub(crate) fn flush_bits(&mut self) {
        let whole = self.bit_count / 8;
        if whole > 0 && self.reserve(whole as usize) {
            self.bytes
                .extend_from_slice(&self.bits.to_le_bytes()[..whole as usize]);
        }
        self.bits >>= 8 * whole;
        self.bit_count %= 8;
    }

    /// Pads the bit buffer to a whole byte and moves it into the pending
    /// bytes (`bi_windup`).
    pub(crate) fn align(&mut self) {
        let whole = self.bit_count.div_ceil(8) as usize;
        if whole > 0 && self.reserve(whole) {
            self.bytes
                .extend_from_slice(&self.bits.to_le_bytes()[..whole]);
        }
        self.bits = 0;
        self.bit_count = 0;
    }

    /// Hands the caller as many pending bytes as its buffer takes
    /// (`flush_pending`).
    pub(crate) fn flush_pending(&mut self) {
        self.flush_bits();
        let taken = self.pending().min(self.room);
        self.delivered += taken;
        self.room -= taken;
    }

    /// Appends `bytes` straight into the caller's buffer, which has room
    /// for them, with nothing pending (`deflate_stored`'s copies to
    /// `next_out`).
    pub(crate) fn deliver(&mut self, bytes: &[u8]) {
        debug_assert!(self.pending() == 0 && self.room >= bytes.len());
        self.put_bytes(bytes);
        self.delivered += bytes.len();
        self.room -= bytes.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_bits_least_significant_first() {
        let mut bytes = vec![0xAA];
        let mut out = Output::new(&mut bytes);
        out.send_bits(0b1, 1);
        out.send_bits(0b01, 2);
        out.send_bits(0b10110, 5);
        out.send_bits(0x0123_4567_89AB_CDEF, 64);
        out.send_bits(0b11, 2);
        out.align();
        out.flush_pending();
        assert_eq!(out.delivered(), 10);
        assert_eq!(out.finish(), Ok(()));
        assert_eq!(
            bytes,
            [
                0xAA,
                0b1011_0011,
                0xEF,
                0xCD,
                0xAB,
                0x89,
                0x67,
                0x45,
                0x23,
                0x01,
                0b11
            ]
        );
    }

    #[test]
    fn long_codes_straddle_the_buffer() {
        let mut bytes = Vec::new();
        let mut out = Output::new(&mut bytes);
        for _ in 0..7 {
            out.send_bits(0x1_5555, 17);
        }
        out.align();
        assert_eq!(out.finish(), Ok(()));
        let value = bytes
            .iter()
            .rev()
            .fold(0_u128, |acc, &byte| acc << 8 | u128::from(byte));
        let expected = (0..7).fold(0_u128, |acc, i| acc | 0x1_5555 << (17 * i));
        assert_eq!(value, expected);
    }

    #[test]
    fn the_callers_room_limits_what_is_delivered() {
        let mut bytes = Vec::new();
        let mut out = Output::new(&mut bytes);
        out.give_room(3);
        out.put_bytes(&[1, 2, 3, 4, 5]);
        out.send_bits(0b1_0000_0001, 9);
        out.flush_pending();
        assert_eq!((out.delivered(), out.pending(), out.room()), (3, 3, 0));
        assert_eq!(out.bit_count(), 1);
        out.give_room(10);
        out.align();
        out.flush_pending();
        out.deliver(&[9]);
        assert_eq!((out.delivered(), out.room()), (8, 5));
        assert_eq!(out.finish(), Ok(()));
        assert_eq!(bytes, [1, 2, 3, 4, 5, 1, 1, 9]);
    }
}
