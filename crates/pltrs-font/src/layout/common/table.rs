//! The `GSUB`/`GPOS` table header and its script, feature and lookup
//! lists: language systems, features and lookups with their subtables.

use super::{TagRecord, lookup_flag};
use crate::error::{FontError, Result};
use crate::reader::{Array, Reader, peek, tail};
use crate::types::Tag;

/// A language system: the features enabled for one script/language.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LangSys<'a> {
    pub(crate) required_feature: Option<u16>,
    pub(crate) feature_indices: Array<'a, u16>,
}

impl<'a> LangSys<'a> {
    pub(crate) fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
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
    pub(crate) tag: Tag,
    pub(crate) scripts: &'a [u8],
    pub(crate) features: &'a [u8],
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

    pub(crate) fn records(list: &'a [u8], tag: Tag) -> Result<Array<'a, TagRecord>> {
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

    /// The language system of `script` for the first of `languages` it
    /// has, else its language system tagged `dflt`, else its default
    /// language system (HarfBuzz's `hb_ot_layout_script_select_language`).
    pub(crate) fn lang_sys(&self, script: Tag, languages: &[Tag]) -> Option<LangSys<'a>> {
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
        let chosen = languages
            .iter()
            .chain(&[Tag::new(b"dflt")])
            .find_map(|&language| lang_records.iter().find(|record| record.tag == language));
        match chosen {
            Some(record) => parse_at(record.offset),
            None if default_offset == 0 => None,
            None => parse_at(default_offset),
        }
    }

    /// The features any language system of any script refers to,
    /// required features included (fontTools' `ScriptList.collect_features`).
    pub(crate) fn referenced_features(&self) -> Vec<u16> {
        let mut features = Vec::new();
        let records = Self::records(self.scripts, self.tag).unwrap_or_default();
        for record in records.iter() {
            let Some(script_data) = self.scripts.get(usize::from(record.offset)..) else {
                continue;
            };
            let mut r = Reader::new(script_data, self.tag);
            let (Ok(default_offset), Ok(count)) = (r.u16(), r.u16()) else {
                continue;
            };
            let lang_records: Array<'_, TagRecord> =
                r.array(usize::from(count)).unwrap_or_default();
            let offsets = std::iter::once(default_offset)
                .filter(|&offset| offset != 0)
                .chain(lang_records.iter().map(|lang| lang.offset));
            for offset in offsets {
                let lang_sys = script_data
                    .get(usize::from(offset)..)
                    .and_then(|d| LangSys::parse(d, self.tag).ok());
                if let Some(lang_sys) = lang_sys {
                    features.extend(lang_sys.feature_indices.iter());
                    features.extend(lang_sys.required_feature);
                }
            }
        }
        features.sort_unstable();
        features.dedup();
        features
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
