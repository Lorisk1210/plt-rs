//! What the compressor reads: a whole input in memory, or bytes a source
//! produces on demand, of which only a window around the parser's
//! position is kept (the filtered scanlines of a PNG image, which need not
//! be filtered into one buffer first).
//!
//! Positions are always those of the whole input; a [`View`] maps them
//! into the bytes held.

use std::ops::Range;

use pltrs_core::alloc::{self, OutOfMemory};

use super::matcher::SEGMENT;
use super::tables::{MAX_MATCH, MIN_MATCH, WINDOW_SIZE};
use crate::checksum::Adler32;

/// How far behind the parser's position the parsers read: the window of
/// back-references, and the byte before the position the lazy parser
/// emits.
const BEHIND: usize = WINDOW_SIZE + 1;

/// How far ahead of the parser's position the parsers read: a whole
/// segment hashed from the end of the longest match (see
/// [`Matcher`](super::matcher::Matcher)), plus the word compared at once.
const AHEAD: usize = MAX_MATCH + SEGMENT + MIN_MATCH + 8;

/// How many bytes a window reads from its source at once beyond what the
/// parsers need, so that it moves its bytes rarely.
const REFILL: usize = 1 << 17;

/// Bytes produced in order, on demand, for compression.
pub(crate) trait ByteSource {
    /// The number of bytes the source produces.
    fn len(&self) -> usize;

    /// Appends the `count` bytes from `start` on (all that are left, if
    /// fewer) to `out`, which has room for them.
    fn append(&mut self, start: usize, count: usize, out: &mut Vec<u8>);
}

/// The bytes the parsers read, as they read them: from position `base`
/// of an input `len` bytes long.
#[derive(Clone, Copy, Debug)]
pub(crate) struct View<'a> {
    bytes: &'a [u8],
    base: usize,
    len: usize,
}

impl<'a> View<'a> {
    /// All of `bytes`.
    pub(crate) fn whole(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            base: 0,
            len: bytes.len(),
        }
    }

    /// The length of the whole input.
    pub(crate) fn len(self) -> usize {
        self.len
    }

    /// The bytes of the input at `positions`, which the view holds.
    pub(crate) fn at(self, positions: Range<usize>) -> &'a [u8] {
        &self.bytes[positions.start - self.base..positions.end - self.base]
    }

    /// The byte at `position`.
    pub(crate) fn byte(self, position: usize) -> u8 {
        self.bytes[position - self.base]
    }

    /// The eight bytes from `position` on, little-endian, which the view
    /// holds.
    pub(crate) fn word(self, position: usize) -> u64 {
        let bytes = self.bytes[position - self.base..].first_chunk();
        u64::from_le_bytes(*bytes.expect("the parsers read whole words"))
    }
}

/// The input of the parsers.
pub(crate) trait Input {
    /// The bytes held now.
    fn view(&self) -> View<'_>;

    /// Holds what the parsers read at `position`: the window behind it
    /// and the bytes ahead of it.
    fn reach(&mut self, position: usize);

    /// Calls `write` with the bytes at `positions`, held or not.
    fn with_range(&mut self, positions: Range<usize>, write: impl FnOnce(&[u8]));
}

/// A whole input in memory.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Whole<'a>(pub(crate) &'a [u8]);

impl Input for Whole<'_> {
    fn view(&self) -> View<'_> {
        View::whole(self.0)
    }

    fn reach(&mut self, _position: usize) {}

    fn with_range(&mut self, positions: Range<usize>, write: impl FnOnce(&[u8])) {
        write(&self.0[positions]);
    }
}

/// The bytes of a source around the parser's position, with the Adler-32
/// of every byte read from it.
#[derive(Debug)]
pub(crate) struct Window<S> {
    source: S,
    len: usize,
    /// The bytes from `base` on.
    bytes: Vec<u8>,
    base: usize,
    /// Bytes behind the window, made again for a stored block.
    again: Vec<u8>,
    checksum: Adler32,
}

impl<S: ByteSource> Window<S> {
    /// A window over `source`, holding nothing yet.
    ///
    /// # Errors
    ///
    /// When the memory for the window cannot be allocated.
    pub(crate) fn new(source: S) -> Result<Self, OutOfMemory> {
        let len = source.len();
        Ok(Self {
            source,
            len,
            bytes: alloc::try_with_capacity(len.min(BEHIND + AHEAD + REFILL))?,
            base: 0,
            again: Vec::new(),
            checksum: Adler32::new(),
        })
    }

    /// The Adler-32 of the bytes read so far: of the whole input once it
    /// is compressed.
    pub(crate) fn checksum(&self) -> u32 {
        self.checksum.finish()
    }
}

impl<S: ByteSource> Input for Window<S> {
    fn view(&self) -> View<'_> {
        View {
            bytes: &self.bytes,
            base: self.base,
            len: self.len,
        }
    }

    fn reach(&mut self, position: usize) {
        let end = self.base + self.bytes.len();
        if end == self.len || position + AHEAD <= end {
            return;
        }
        let keep = position.saturating_sub(BEHIND).max(self.base);
        self.bytes.drain(..keep - self.base);
        self.base = keep;
        let held = self.bytes.len();
        let count = (BEHIND + AHEAD + REFILL - held).min(self.len - end);
        self.source.append(end, count, &mut self.bytes);
        self.checksum.update(&self.bytes[held..]);
    }

    fn with_range(&mut self, positions: Range<usize>, write: impl FnOnce(&[u8])) {
        if positions.start >= self.base {
            write(self.view().at(positions));
            return;
        }
        self.again.clear();
        self.source
            .append(positions.start, positions.len(), &mut self.again);
        write(&self.again);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes `0, 1, 2, ...` (mod 251), counting what it is asked for.
    struct Counting {
        len: usize,
        produced: usize,
    }

    impl ByteSource for Counting {
        fn len(&self) -> usize {
            self.len
        }

        fn append(&mut self, start: usize, count: usize, out: &mut Vec<u8>) {
            let end = (start + count).min(self.len);
            out.extend((start..end).map(|i| (i % 251) as u8));
            self.produced += end - start;
        }
    }

    #[test]
    fn a_window_holds_what_the_parsers_read_and_little_more() {
        let len = 1_000_000;
        let mut window = Window::new(Counting { len, produced: 0 }).unwrap();
        let mut checksum = Adler32::new();
        for position in (0..len).step_by(1000) {
            window.reach(position);
            let view = window.view();
            let ahead = (position + AHEAD).min(len);
            let behind = position.saturating_sub(BEHIND);
            let held = view.at(behind..ahead);
            assert!(
                held.iter()
                    .zip(behind..)
                    .all(|(&b, i)| b == (i % 251) as u8)
            );
            assert!(window.bytes.len() <= BEHIND + AHEAD + REFILL);
        }
        window.reach(len - 1);
        checksum.update(&(0..len).map(|i| (i % 251) as u8).collect::<Vec<_>>());
        assert_eq!(
            window.checksum(),
            checksum.finish(),
            "every byte read once, in order"
        );
        assert_eq!(window.source.produced, len);
        let mut copied = Vec::new();
        window.with_range(10..20, |bytes| copied.extend_from_slice(bytes));
        assert_eq!(
            copied,
            (10..20).map(|i| i as u8).collect::<Vec<_>>(),
            "made again"
        );
    }
}
