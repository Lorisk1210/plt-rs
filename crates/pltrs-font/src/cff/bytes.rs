//! The bytes of a CFF table: all in memory, or held in two parts around
//! its `CharStrings` data, which a face of a file held for drawing reads
//! glyph by glyph (most of a large CFF font is its `CharStrings`).

use std::ops::Range;

use super::Version;
use super::index::CFF;
use crate::error::{FontError, Result};
use crate::reader::{Reader, tail};
use crate::region::Region;

/// A CFF table held but for its `CharStrings` data: the bytes before
/// them, those after them, and the data themselves, read when used.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CffParts<'a> {
    /// The table's version.
    pub(crate) version: Version,
    /// The bytes before the `CharStrings` data (the table's first bytes).
    pub(crate) head: &'a [u8],
    /// The bytes after the `CharStrings` data, to the end of the table.
    pub(crate) tail: &'a [u8],
    /// The `CharStrings` data.
    pub(crate) charstrings: Region<'a>,
}

impl CffParts<'_> {
    /// Where the tail starts in the table.
    const fn tail_at(&self) -> usize {
        self.head.len() + self.charstrings.len()
    }
}

/// The bytes of a CFF table, which the parser reads at offsets.
#[derive(Clone, Copy, Debug)]
pub(super) enum CffBytes<'a> {
    /// The whole table.
    Whole(&'a [u8]),
    /// The table without its `CharStrings` data.
    Parts(CffParts<'a>),
}

impl<'a> CffBytes<'a> {
    /// The part holding `offset` of the table and where it starts; an
    /// offset in the `CharStrings` data of [`Parts`](Self::Parts) is in
    /// none.
    fn part(self, offset: usize) -> Result<(&'a [u8], usize)> {
        match self {
            Self::Whole(data) => Ok((data, 0)),
            Self::Parts(parts) if offset < parts.head.len() => Ok((parts.head, 0)),
            Self::Parts(parts) if offset >= parts.tail_at() => Ok((parts.tail, parts.tail_at())),
            Self::Parts(_) => Err(FontError::malformed(CFF, "structure inside CharStrings")),
        }
    }

    /// A reader at `offset` of the table, to the end of its part.
    pub(super) fn at(self, offset: usize) -> Result<Reader<'a>> {
        let (part, start) = self.part(offset)?;
        Reader::at(part, offset - start, CFF)
    }

    /// The bytes from `offset` to the end of its part.
    pub(super) fn tail(self, offset: usize) -> Result<&'a [u8]> {
        let (part, start) = self.part(offset)?;
        tail(part, offset - start, CFF)
    }

    /// The bytes of `range`, when they lie in one part.
    pub(super) fn get(self, range: Range<usize>) -> Option<&'a [u8]> {
        let (part, start) = self.part(range.start).ok()?;
        part.get(range.start - start..range.end.checked_sub(start)?)
    }

    /// The `CharStrings` data of [`Parts`](Self::Parts), held elsewhere.
    pub(super) const fn charstrings(self) -> Option<Region<'a>> {
        match self {
            Self::Whole(_) => None,
            Self::Parts(parts) => Some(parts.charstrings),
        }
    }
}
