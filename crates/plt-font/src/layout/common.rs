//! Structures shared by `GSUB` and `GPOS`: script/feature/lookup lists,
//! coverage tables and class definitions.

use crate::error::{FontError, Result};
use crate::reader::{Array, FromBytes, Reader, peek, tail};
use crate::types::{GlyphId, Tag};

/// Lookup flag bits.
pub(crate) mod lookup_flag {
    pub(crate) const RIGHT_TO_LEFT: u16 = 0x0001;
    pub(crate) const IGNORE_BASE_GLYPHS: u16 = 0x0002;
    pub(crate) const IGNORE_LIGATURES: u16 = 0x0004;
    pub(crate) const IGNORE_MARKS: u16 = 0x0008;
    pub(crate) const IGNORE_FLAGS: u16 = IGNORE_BASE_GLYPHS | IGNORE_LIGATURES | IGNORE_MARKS;
    pub(crate) const USE_MARK_FILTERING_SET: u16 = 0x0010;
    pub(crate) const MARK_ATTACHMENT_TYPE: u16 = 0xFF00;
}

#[derive(Clone, Copy, Debug)]
struct TagRecord {
    tag: Tag,
    offset: u16,
}

impl FromBytes for TagRecord {
    const SIZE: usize = 6;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            tag: Tag::from_bytes(&bytes[0..4]),
            offset: u16::from_bytes(&bytes[4..6]),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RangeRecord {
    start: u16,
    end: u16,
    value: u16,
}

impl FromBytes for RangeRecord {
    const SIZE: usize = 6;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            start: u16::from_bytes(&bytes[0..2]),
            end: u16::from_bytes(&bytes[2..4]),
            value: u16::from_bytes(&bytes[4..6]),
        }
    }
}

fn find_range(ranges: &Array<'_, RangeRecord>, glyph: GlyphId) -> Option<RangeRecord> {
    ranges
        .harfbuzz_search_by(|range| {
            if range.end < glyph.0 {
                std::cmp::Ordering::Less
            } else if range.start > glyph.0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .map(|(_, range)| range)
}

/// A coverage table: the set of glyphs a subtable applies to, each with an
/// index.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Coverage<'a> {
    Glyphs(Array<'a, GlyphId>),
    Ranges(Array<'a, RangeRecord>),
}

impl<'a> Coverage<'a> {
    pub(crate) fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        match r.u16()? {
            1 => {
                let count = r.u16()?;
                Ok(Self::Glyphs(r.array(usize::from(count))?))
            }
            2 => {
                let count = r.u16()?;
                Ok(Self::Ranges(r.array(usize::from(count))?))
            }
            format => Err(FontError::UnsupportedFormat {
                table: tag,
                format: u32::from(format),
            }),
        }
    }

    /// Parses the coverage table at `offset` inside `parent`.
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Result<Self> {
        Self::parse(tail(parent, usize::from(offset), tag)?, tag)
    }

    /// Coverage index of `glyph`, if covered (with HarfBuzz's choice among
    /// duplicate entries).
    pub(crate) fn index(&self, glyph: GlyphId) -> Option<u16> {
        match self {
            Self::Glyphs(glyphs) => glyphs
                .harfbuzz_search_by(|g| g.cmp(&glyph))
                .map(|(i, _)| i as u16),
            Self::Ranges(ranges) => find_range(ranges, glyph)
                .map(|range| range.value.wrapping_add(glyph.0 - range.start)),
        }
    }

    pub(crate) fn contains(&self, glyph: GlyphId) -> bool {
        self.index(glyph).is_some()
    }
}

/// A class definition table mapping glyphs to small integers.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum ClassDef<'a> {
    Array {
        start: u16,
        classes: Array<'a, u16>,
    },
    Ranges(Array<'a, RangeRecord>),
    #[default]
    Empty,
}

impl<'a> ClassDef<'a> {
    pub(crate) fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        match r.u16()? {
            1 => {
                let start = r.u16()?;
                let count = r.u16()?;
                Ok(Self::Array {
                    start,
                    classes: r.array(usize::from(count))?,
                })
            }
            2 => {
                let count = r.u16()?;
                Ok(Self::Ranges(r.array(usize::from(count))?))
            }
            format => Err(FontError::UnsupportedFormat {
                table: tag,
                format: u32::from(format),
            }),
        }
    }

    /// Parses the class definition at `offset` inside `parent`; a zero
    /// offset means "every glyph is class 0".
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Result<Self> {
        if offset == 0 {
            return Ok(Self::Empty);
        }
        Self::parse(tail(parent, usize::from(offset), tag)?, tag)
    }

    /// Class of `glyph` (0 when not listed).
    pub(crate) fn class(&self, glyph: GlyphId) -> u16 {
        match self {
            Self::Array { start, classes } => glyph
                .0
                .checked_sub(*start)
                .and_then(|i| classes.get(usize::from(i)))
                .unwrap_or(0),
            Self::Ranges(ranges) => find_range(ranges, glyph).map_or(0, |range| range.value),
            Self::Empty => 0,
        }
    }
}

/// A language system: the features enabled for one script/language.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LangSys<'a> {
    pub(crate) required_feature: Option<u16>,
    pub(crate) feature_indices: Array<'a, u16>,
}

impl<'a> LangSys<'a> {
    fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        r.skip(2)?;
        let required = r.u16()?;
        let count = r.u16()?;
        let feature_indices = r.array(usize::from(count))?;
        Ok(Self {
            required_feature: (required != 0xFFFF).then_some(required),
            feature_indices,
        })
    }
}

/// A lookup: a typed list of subtables plus flags.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Lookup<'a> {
    /// Lookup type, with extension lookups already resolved.
    pub(crate) kind: u16,
    pub(crate) flags: u16,
    pub(crate) mark_filtering_set: Option<u16>,
    data: &'a [u8],
    offsets: Array<'a, u16>,
    extension_kind: u16,
    tag: Tag,
}

impl<'a> Lookup<'a> {
    fn parse(data: &'a [u8], tag: Tag, extension_kind: u16) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        let declared_kind = r.u16()?;
        let flags = r.u16()?;
        let count = r.u16()?;
        let offsets: Array<'_, u16> = r.array(usize::from(count))?;
        let mark_filtering_set = if flags & lookup_flag::USE_MARK_FILTERING_SET != 0 {
            Some(r.u16()?)
        } else {
            None
        };
        let mut lookup = Self {
            kind: declared_kind,
            flags,
            mark_filtering_set,
            data,
            offsets,
            extension_kind,
            tag,
        };
        if declared_kind == extension_kind {
            // All extension subtables of a lookup share one real type.
            let first = offsets
                .get(0)
                .and_then(|offset| peek::<u16>(data, usize::from(offset) + 2));
            lookup.kind = first.unwrap_or(0);
        }
        Ok(lookup)
    }

    /// Number of subtables.
    pub(crate) fn len(&self) -> usize {
        self.offsets.len()
    }

    /// Bytes of subtable `index`, following extension indirections.
    pub(crate) fn subtable(&self, index: usize) -> Option<&'a [u8]> {
        let data = self.data.get(usize::from(self.offsets.get(index)?)..)?;
        if peek::<u16>(self.data, 0)? != self.extension_kind {
            return Some(data);
        }
        let mut r = Reader::new(data, self.tag);
        let format = r.u16().ok()?;
        let kind = r.u16().ok()?;
        let offset = r.u32().ok()?;
        (format == 1 && kind == self.kind)
            .then(|| data.get(offset as usize..))
            .flatten()
    }

    /// Iterates over all subtables.
    pub(crate) fn subtables(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
        (0..self.len()).filter_map(|index| self.subtable(index))
    }
}

/// A `GSUB` or `GPOS` table header with its three lists.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LayoutTable<'a> {
    tag: Tag,
    scripts: &'a [u8],
    features: &'a [u8],
    lookups: &'a [u8],
    extension_kind: u16,
}

impl<'a> LayoutTable<'a> {
    /// Parses a layout table; `extension_kind` is 7 for `GSUB`, 9 for `GPOS`.
    pub(crate) fn parse(data: &'a [u8], tag: Tag, extension_kind: u16) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        let major = r.u16()?;
        r.skip(2)?;
        if major != 1 {
            return Err(FontError::UnsupportedFormat {
                table: tag,
                format: u32::from(major),
            });
        }
        let scripts = tail(data, usize::from(r.u16()?), tag)?;
        let features = tail(data, usize::from(r.u16()?), tag)?;
        let lookups = tail(data, usize::from(r.u16()?), tag)?;
        Ok(Self {
            tag,
            scripts,
            features,
            lookups,
            extension_kind,
        })
    }

    fn records(list: &'a [u8], tag: Tag) -> Result<Array<'a, TagRecord>> {
        let mut r = Reader::new(list, tag);
        let count = r.u16()?;
        r.array(usize::from(count))
    }

    /// Script tags present in the table.
    pub(crate) fn script_tags(&self) -> impl Iterator<Item = Tag> + 'a {
        Self::records(self.scripts, self.tag)
            .unwrap_or_default()
            .iter()
            .map(|record| record.tag)
    }

    /// The language system for `script`/`language`, falling back to the
    /// script's default language system.
    pub(crate) fn lang_sys(&self, script: Tag, language: Option<Tag>) -> Option<LangSys<'a>> {
        let records = Self::records(self.scripts, self.tag).ok()?;
        let record = records.iter().find(|record| record.tag == script)?;
        let script_data = self.scripts.get(usize::from(record.offset)..)?;
        let mut r = Reader::new(script_data, self.tag);
        let default_offset = r.u16().ok()?;
        let count = r.u16().ok()?;
        let lang_records: Array<'_, TagRecord> = r.array(usize::from(count)).ok()?;
        let parse_at = |offset: u16| {
            script_data
                .get(usize::from(offset)..)
                .and_then(|d| LangSys::parse(d, self.tag).ok())
        };
        if let Some(lang) =
            language.and_then(|language| lang_records.iter().find(|record| record.tag == language))
        {
            return parse_at(lang.offset);
        }
        if default_offset == 0 {
            None
        } else {
            parse_at(default_offset)
        }
    }

    /// Tag and lookup indices of feature `index`.
    pub(crate) fn feature(&self, index: u16) -> Option<(Tag, Array<'a, u16>)> {
        let records = Self::records(self.features, self.tag).ok()?;
        let record = records.get(usize::from(index))?;
        let data = self.features.get(usize::from(record.offset)..)?;
        let mut r = Reader::new(data, self.tag);
        r.skip(2).ok()?;
        let count = r.u16().ok()?;
        Some((record.tag, r.array(usize::from(count)).ok()?))
    }

    /// Number of lookups in the lookup list.
    pub(crate) fn lookup_count(&self) -> usize {
        peek::<u16>(self.lookups, 0).map_or(0, usize::from)
    }

    /// Lookup `index`.
    pub(crate) fn lookup(&self, index: u16) -> Option<Lookup<'a>> {
        let mut r = Reader::new(self.lookups, self.tag);
        let count = r.u16().ok()?;
        let offsets: Array<'_, u16> = r.array(usize::from(count)).ok()?;
        let offset = offsets.get(usize::from(index))?;
        let data = self.lookups.get(usize::from(offset)..)?;
        Lookup::parse(data, self.tag, self.extension_kind).ok()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const TAG: Tag = Tag::new(b"GSUB");

    pub(crate) fn coverage_format1(glyphs: &[u16]) -> Vec<u8> {
        let mut out = vec![0, 1];
        out.extend_from_slice(&(glyphs.len() as u16).to_be_bytes());
        for g in glyphs {
            out.extend_from_slice(&g.to_be_bytes());
        }
        out
    }

    pub(crate) fn ranges(format: u16, ranges: &[(u16, u16, u16)]) -> Vec<u8> {
        let mut out = format.to_be_bytes().to_vec();
        out.extend_from_slice(&(ranges.len() as u16).to_be_bytes());
        for r in ranges {
            for v in [r.0, r.1, r.2] {
                out.extend_from_slice(&v.to_be_bytes());
            }
        }
        out
    }

    #[test]
    fn coverage_formats() {
        let glyphs = coverage_format1(&[3, 7, 9]);
        let cov = Coverage::parse(&glyphs, TAG).unwrap();
        assert_eq!(cov.index(GlyphId(7)), Some(1));
        assert_eq!(cov.index(GlyphId(8)), None);
        let range_data = ranges(2, &[(10, 12, 0), (20, 20, 3)]);
        let cov = Coverage::parse(&range_data, TAG).unwrap();
        assert_eq!(cov.index(GlyphId(11)), Some(1));
        assert_eq!(cov.index(GlyphId(20)), Some(3));
        assert!(!cov.contains(GlyphId(13)));
        assert!(Coverage::parse(&[0, 3, 0, 0], TAG).is_err());
        assert!(Coverage::parse(&coverage_format1(&[3, 7, 9])[..6], TAG).is_err());
    }

    #[test]
    fn class_definitions() {
        let classes = ClassDef::parse(&[0, 1, 0, 5, 0, 2, 0, 3, 0, 4], TAG).unwrap();
        assert_eq!(classes.class(GlyphId(5)), 3);
        assert_eq!(classes.class(GlyphId(6)), 4);
        assert_eq!(classes.class(GlyphId(7)), 0);
        assert_eq!(classes.class(GlyphId(1)), 0);
        let class_data = ranges(2, &[(1, 4, 2)]);
        let classes = ClassDef::parse(&class_data, TAG).unwrap();
        assert_eq!(classes.class(GlyphId(3)), 2);
        assert_eq!(classes.class(GlyphId(5)), 0);
        assert_eq!(ClassDef::at(&[], 0, TAG).unwrap().class(GlyphId(1)), 0);
        assert!(ClassDef::at(&[0, 1], 4, TAG).is_err());
    }

    /// Builds a layout table with one script (with default language system
    /// enabling every feature), the given features and raw lookups.
    pub(crate) fn layout_table(
        script: [u8; 4],
        features: &[(&[u8; 4], &[u16])],
        lookups: &[Vec<u8>],
    ) -> Vec<u8> {
        let mut script_list = vec![0, 1];
        script_list.extend_from_slice(&script);
        script_list.extend_from_slice(&8u16.to_be_bytes());
        script_list.extend_from_slice(&[0, 4, 0, 0]);
        script_list.extend_from_slice(&[0, 0, 0xFF, 0xFF]);
        script_list.extend_from_slice(&(features.len() as u16).to_be_bytes());
        for index in 0..features.len() as u16 {
            script_list.extend_from_slice(&index.to_be_bytes());
        }
        let mut feature_list = (features.len() as u16).to_be_bytes().to_vec();
        let mut feature_body = Vec::new();
        let header = 2 + 6 * features.len();
        for (tag, indices) in features {
            feature_list.extend_from_slice(*tag);
            feature_list.extend_from_slice(&((header + feature_body.len()) as u16).to_be_bytes());
            feature_body.extend_from_slice(&[0, 0]);
            feature_body.extend_from_slice(&(indices.len() as u16).to_be_bytes());
            for index in *indices {
                feature_body.extend_from_slice(&index.to_be_bytes());
            }
        }
        feature_list.extend_from_slice(&feature_body);
        let mut lookup_list = (lookups.len() as u16).to_be_bytes().to_vec();
        let mut lookup_body = Vec::new();
        let header = 2 + 2 * lookups.len();
        for lookup in lookups {
            lookup_list.extend_from_slice(&((header + lookup_body.len()) as u16).to_be_bytes());
            lookup_body.extend_from_slice(lookup);
        }
        lookup_list.extend_from_slice(&lookup_body);
        let mut out = vec![0, 1, 0, 0];
        let scripts_at = 10;
        let features_at = scripts_at + script_list.len();
        let lookups_at = features_at + feature_list.len();
        for v in [scripts_at, features_at, lookups_at] {
            out.extend_from_slice(&(v as u16).to_be_bytes());
        }
        out.extend_from_slice(&script_list);
        out.extend_from_slice(&feature_list);
        out.extend_from_slice(&lookup_list);
        out
    }

    /// Wraps subtables into a lookup of `kind` with `flags`.
    pub(crate) fn lookup(kind: u16, flags: u16, subtables: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        for v in [kind, flags, subtables.len() as u16] {
            out.extend_from_slice(&v.to_be_bytes());
        }
        let mut offset = 6 + 2 * subtables.len();
        for sub in subtables {
            out.extend_from_slice(&(offset as u16).to_be_bytes());
            offset += sub.len();
        }
        for sub in subtables {
            out.extend_from_slice(sub);
        }
        out
    }

    #[test]
    fn lists_and_extensions() {
        let single = vec![0, 1, 0, 6, 0, 1, 0, 1, 0, 1, 0, 5];
        let mut extension = vec![0, 1, 0, 1, 0, 0, 0, 8];
        extension.extend_from_slice(&single);
        let data = layout_table(
            *b"latn",
            &[(b"liga", &[0]), (b"kern", &[1])],
            &[
                lookup(1, 0, std::slice::from_ref(&single)),
                lookup(7, 8, &[extension]),
            ],
        );
        let table = LayoutTable::parse(&data, TAG, 7).unwrap();
        assert_eq!(
            table.script_tags().collect::<Vec<_>>(),
            vec![Tag::new(b"latn")]
        );
        let lang = table
            .lang_sys(Tag::new(b"latn"), Some(Tag::new(b"ENG ")))
            .unwrap();
        assert_eq!(lang.feature_indices.len(), 2);
        assert!(lang.required_feature.is_none());
        assert!(table.lang_sys(Tag::new(b"grek"), None).is_none());
        let (tag, lookups) = table.feature(1).unwrap();
        assert_eq!((tag, lookups.get(0)), (Tag::new(b"kern"), Some(1)));
        let ext = table.lookup(1).unwrap();
        assert_eq!((ext.kind, ext.flags, ext.len()), (1, 8, 1));
        assert_eq!(ext.subtable(0), Some(&single[..]));
        assert_eq!(table.lookup(0).unwrap().subtables().count(), 1);
        assert_eq!(table.lookup_count(), 2);
        assert!(table.lookup(2).is_none());
        assert!(LayoutTable::parse(&data[..8], TAG, 7).is_err());
    }
}
