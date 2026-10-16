//! `GDEF`: glyph classes, mark attachment classes and mark glyph sets.

use crate::error::{FontError, Result};
use crate::layout::common::{ClassDef, Coverage};
use crate::reader::{Array, Reader, tail};
use crate::types::{GlyphId, Tag};

const GDEF: Tag = Tag::new(b"GDEF");

/// GDEF glyph class values.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum GlyphClass {
    Unclassified,
    Base,
    Ligature,
    Mark,
    Component,
}

impl GlyphClass {
    fn from_raw(raw: u16) -> Self {
        match raw {
            1 => Self::Base,
            2 => Self::Ligature,
            3 => Self::Mark,
            4 => Self::Component,
            _ => Self::Unclassified,
        }
    }
}

/// The parsed `GDEF` table.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Gdef<'a> {
    glyph_classes: ClassDef<'a>,
    has_glyph_classes: bool,
    mark_attach_classes: ClassDef<'a>,
    mark_sets: Option<(&'a [u8], Array<'a, u32>)>,
}

impl<'a> Gdef<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, GDEF);
        let major = r.u16()?;
        let minor = r.u16()?;
        if major != 1 {
            return Err(FontError::UnsupportedFormat {
                table: GDEF,
                format: u32::from(major),
            });
        }
        let glyph_class_offset = r.u16()?;
        r.skip(4)?;
        let mark_attach_offset = r.u16()?;
        let mark_sets = if minor >= 2 {
            match r.u16()? {
                0 => None,
                offset => {
                    let sets = tail(data, usize::from(offset), GDEF)?;
                    let mut sr = Reader::new(sets, GDEF);
                    sr.skip(2)?;
                    let count = sr.u16()?;
                    Some((sets, sr.array(usize::from(count))?))
                }
            }
        } else {
            None
        };
        Ok(Self {
            glyph_classes: ClassDef::at(data, glyph_class_offset, GDEF)?,
            has_glyph_classes: glyph_class_offset != 0,
            mark_attach_classes: ClassDef::at(data, mark_attach_offset, GDEF)?,
            mark_sets,
        })
    }

    /// Whether glyph classes are defined (otherwise they are synthesized
    /// from Unicode properties during shaping).
    pub(crate) const fn has_glyph_classes(&self) -> bool {
        self.has_glyph_classes
    }

    pub(crate) fn glyph_class(&self, glyph: GlyphId) -> GlyphClass {
        GlyphClass::from_raw(self.glyph_classes.class(glyph))
    }

    pub(crate) fn mark_attach_class(&self, glyph: GlyphId) -> u16 {
        self.mark_attach_classes.class(glyph)
    }

    /// Whether `glyph` belongs to mark glyph set `set`.
    pub(crate) fn is_in_mark_set(&self, set: u16, glyph: GlyphId) -> bool {
        let Some((data, offsets)) = self.mark_sets else {
            return false;
        };
        offsets
            .get(usize::from(set))
            .and_then(|offset| Coverage::parse(data.get(offset as usize..)?, GDEF).ok())
            .is_some_and(|coverage| coverage.contains(glyph))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{coverage_format1, ranges};

    #[test]
    fn classes_and_mark_sets() {
        let classes = ranges(2, &[(1, 5, 1), (6, 6, 2), (7, 9, 3)]);
        let attach = ranges(2, &[(7, 8, 1)]);
        let sets_offset = 14 + classes.len() + attach.len();
        let mut data = vec![0, 1, 0, 2, 0, 14, 0, 0, 0, 0];
        data.extend_from_slice(&((14 + classes.len()) as u16).to_be_bytes());
        data.extend_from_slice(&(sets_offset as u16).to_be_bytes());
        data.extend_from_slice(&classes);
        data.extend_from_slice(&attach);
        data.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0, 8]);
        data.extend_from_slice(&coverage_format1(&[9]));
        let gdef = Gdef::parse(&data).unwrap();
        assert!(gdef.has_glyph_classes());
        assert_eq!(gdef.glyph_class(GlyphId(3)), GlyphClass::Base);
        assert_eq!(gdef.glyph_class(GlyphId(6)), GlyphClass::Ligature);
        assert_eq!(gdef.glyph_class(GlyphId(8)), GlyphClass::Mark);
        assert_eq!(gdef.glyph_class(GlyphId(10)), GlyphClass::Unclassified);
        assert_eq!(gdef.mark_attach_class(GlyphId(7)), 1);
        assert_eq!(gdef.mark_attach_class(GlyphId(9)), 0);
        assert!(gdef.is_in_mark_set(0, GlyphId(9)));
        assert!(!gdef.is_in_mark_set(0, GlyphId(8)));
        assert!(!gdef.is_in_mark_set(1, GlyphId(9)));
        assert!(Gdef::parse(&data[..12]).is_err());
        assert!(Gdef::parse(&[0, 2, 0, 0]).is_err());
    }
}
