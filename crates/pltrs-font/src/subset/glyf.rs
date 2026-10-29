//! Raw `glyf` glyph records: locating them through `loca`, finding the
//! components of composite glyphs and renumbering those components.

use crate::error::{FontError, Result};
use crate::face::Face;
use crate::reader::{Reader, peek};
use crate::types::{GlyphId, Tag};

const GLYF: Tag = Tag::new(b"glyf");
const LOCA: Tag = Tag::new(b"loca");
const HEAD: Tag = Tag::new(b"head");

/// Composite glyph flags (`glyf` component records).
mod flag {
    pub(super) const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
    pub(super) const WE_HAVE_A_SCALE: u16 = 0x0008;
    pub(super) const MORE_COMPONENTS: u16 = 0x0020;
    pub(super) const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
    pub(super) const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;
}

/// The glyph records of a TrueType face.
#[derive(Clone, Copy, Debug)]
pub(super) struct Glyf<'a> {
    records: &'a [u8],
    loca: &'a [u8],
    long_offsets: bool,
    num_glyphs: u16,
}

impl<'a> Glyf<'a> {
    /// The `glyf` and `loca` tables of `face`.
    pub(super) fn new(face: &Face<'a>) -> Result<Self> {
        let glyf = face.table(GLYF).ok_or(FontError::MissingTable(GLYF))?;
        let loca = face.table(LOCA).ok_or(FontError::MissingTable(LOCA))?;
        let head = face.table(HEAD).ok_or(FontError::MissingTable(HEAD))?;
        let format: i16 = peek(head, 50).ok_or(FontError::Truncated(HEAD))?;
        Ok(Self {
            records: glyf,
            loca,
            long_offsets: format != 0,
            num_glyphs: face.num_glyphs(),
        })
    }

    /// Byte offset of glyph `index`'s record (`index` may be the glyph
    /// count, for the end of the last record).
    fn offset(&self, index: usize) -> Option<usize> {
        if self.long_offsets {
            peek::<u32>(self.loca, index * 4).map(|o| o as usize)
        } else {
            peek::<u16>(self.loca, index * 2).map(|o| usize::from(o) * 2)
        }
    }

    /// The record of `glyph` (empty for glyphs without outline).
    pub(super) fn record(&self, glyph: GlyphId) -> Result<&'a [u8]> {
        if glyph.0 >= self.num_glyphs {
            return Ok(&[]);
        }
        let index = usize::from(glyph.0);
        let (Some(start), Some(end)) = (self.offset(index), self.offset(index + 1)) else {
            return Err(FontError::Truncated(LOCA));
        };
        if end <= start {
            return Ok(&[]);
        }
        self.records
            .get(start..end)
            .ok_or(FontError::Truncated(GLYF))
    }
}

/// The byte offsets of the glyph index of every component of a composite
/// glyph record (none for simple glyphs).
fn component_offsets(record: &[u8]) -> Result<Vec<usize>> {
    let contours: i16 = peek(record, 0).unwrap_or(0);
    if contours >= 0 {
        return Ok(Vec::new());
    }
    let mut offsets = Vec::new();
    let mut at = 10;
    loop {
        let mut r = Reader::at(record, at, GLYF)?;
        let flags = r.u16()?;
        offsets.push(at + 2);
        let arguments = if flags & flag::ARG_1_AND_2_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        let transform = if flags & flag::WE_HAVE_A_SCALE != 0 {
            2
        } else if flags & flag::WE_HAVE_AN_X_AND_Y_SCALE != 0 {
            4
        } else if flags & flag::WE_HAVE_A_TWO_BY_TWO != 0 {
            8
        } else {
            0
        };
        at += 4 + arguments + transform;
        if flags & flag::MORE_COMPONENTS == 0 {
            return Ok(offsets);
        }
    }
}

/// The glyphs a composite glyph record is built from.
pub(super) fn components(record: &[u8]) -> Result<Vec<GlyphId>> {
    component_offsets(record)?
        .into_iter()
        .map(|at| {
            peek::<u16>(record, at)
                .map(GlyphId)
                .ok_or(FontError::Truncated(GLYF))
        })
        .collect()
}

/// `record` with every component glyph renumbered by `renumber`.
pub(super) fn renumbered(record: &[u8], renumber: impl Fn(GlyphId) -> GlyphId) -> Result<Vec<u8>> {
    let mut out = record.to_vec();
    for at in component_offsets(record)? {
        let glyph = peek::<u16>(record, at).ok_or(FontError::Truncated(GLYF))?;
        out[at..at + 2].copy_from_slice(&renumber(GlyphId(glyph)).0.to_be_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A composite record of two components: glyph 7 with byte offsets and
    /// glyph 9 with word offsets and a scale.
    fn composite() -> Vec<u8> {
        let mut record = vec![0xFF, 0xFF];
        record.extend([0; 8]);
        record.extend(0x0020u16.to_be_bytes());
        record.extend(7u16.to_be_bytes());
        record.extend([1, 2]);
        record.extend((0x0001u16 | 0x0008).to_be_bytes());
        record.extend(9u16.to_be_bytes());
        record.extend([0, 1, 0, 2, 0x40, 0]);
        record
    }

    #[test]
    fn composite_components_are_found_and_renumbered() {
        let record = composite();
        assert_eq!(components(&record).unwrap(), [GlyphId(7), GlyphId(9)]);
        let renamed = renumbered(&record, |g| GlyphId(g.0 + 100)).unwrap();
        assert_eq!(components(&renamed).unwrap(), [GlyphId(107), GlyphId(109)]);
        assert_eq!(components(&[0, 1, 0, 0]).unwrap(), []);
    }
}
