//! Font files not all in memory. A face of a file held for drawing (see
//! [`FontDatabase`](crate::FontDatabase)) has in memory the tables it
//! parses, but reads its glyph programs (`glyf`, the CFF `CharStrings`)
//! glyph by glyph when they are used, and the tables pltrs draws without
//! (bitmaps, variations) when they are asked for, as FreeType reads a
//! file through matplotlib's file stream: a large font costs the memory of
//! the glyphs drawn, not that of its file.

use std::fmt;
use std::ops::Range;

use crate::cff::CffParts;
use crate::error::Result;
use crate::types::Tag;

/// A font file held for drawing: some of its bytes in memory, the others
/// read when they are asked for, each range once, and kept while the file
/// is.
pub(crate) trait LazyFile: fmt::Debug + Sync {
    /// The bytes its faces are parsed from: their directories and the
    /// tables held.
    fn held(&self) -> &[u8];

    /// Where table `tag` of face `face` is in the file, when it is not
    /// held.
    fn table(&self, face: u32, tag: Tag) -> Option<Range<usize>>;

    /// The CFF table of face `face`, when it is held but for its
    /// `CharStrings` data.
    fn cff(&self, face: u32) -> Option<CffParts<'_>>;

    /// The bytes of `range` of the file, read now unless they were
    /// before; `None` when they lie outside the file or cannot be read.
    fn read(&self, range: Range<usize>) -> Option<&[u8]>;

    /// The whole file, read the first time it is asked for.
    fn whole(&self) -> Result<&[u8]>;
}

/// The bytes of a region of a font file: in memory, or read from the
/// file a part at a time.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Region<'a> {
    /// Bytes in memory.
    Held(&'a [u8]),
    /// The `len` bytes at `start` of `file`.
    Lazy {
        /// The file.
        file: &'a dyn LazyFile,
        /// Where the region starts in the file.
        start: usize,
        /// Its length.
        len: usize,
    },
}

impl Default for Region<'_> {
    fn default() -> Self {
        Self::Held(&[])
    }
}

impl<'a> Region<'a> {
    /// The region `range` of `file`.
    pub(crate) fn lazy(file: &'a dyn LazyFile, range: Range<usize>) -> Self {
        Self::Lazy {
            file,
            start: range.start,
            len: range.len(),
        }
    }

    /// The length of the region.
    pub(crate) const fn len(&self) -> usize {
        match self {
            Self::Held(bytes) => bytes.len(),
            Self::Lazy { len, .. } => *len,
        }
    }

    /// The bytes `range` of the region; `None` when the range is not in
    /// it or cannot be read.
    pub(crate) fn get(&self, range: Range<usize>) -> Option<&'a [u8]> {
        match *self {
            Self::Held(bytes) => bytes.get(range),
            Self::Lazy { file, start, len } => {
                if range.start > range.end || range.end > len {
                    return None;
                }
                file.read(start + range.start..start + range.end)
            }
        }
    }
}
