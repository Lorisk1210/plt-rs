//! HarfBuzz's layout queries, as FreeType's autohinter makes them:
//! the lookups of a set of features (`hb_ot_layout_collect_lookups`), the
//! glyphs a lookup reads or writes (`hb_ot_layout_lookup_collect_glyphs`)
//! and whether a lookup would substitute a glyph
//! (`hb_ot_layout_lookup_would_substitute`).
//!
//! The quirks of HarfBuzz 14.1 are kept: coverage iteration stops at the
//! first broken range, class definitions collect one glyph too many,
//! contextual lookups contribute the output of every nested lookup, and
//! contextual "would apply" tests trust the lookup's set digest instead of
//! the first coverage. Feature variations are not consulted (plt-rs reads
//! no variable fonts).

mod glyphs;
mod sets;

use std::collections::{BTreeSet, HashSet};

pub(crate) use glyphs::{gpos_input_glyphs, gsub_output_glyphs, would_substitute};

use crate::layout::common::{LangSys, LayoutTable, TagRecord};
use crate::reader::{Array, Reader};
use crate::types::Tag;

/// HarfBuzz's limits on what one feature collection visits
/// (`HB_MAX_SCRIPTS`, `HB_MAX_LANGSYS`, `HB_MAX_FEATURE_INDICES`).
const MAX_SCRIPTS: usize = 500;
const MAX_LANGSYS: usize = 2000;
const MAX_FEATURE_INDICES: usize = 1500;

/// HarfBuzz's `hb_collect_features_context_t`.
struct FeatureCollector<'t, 'a> {
    table: &'t LayoutTable<'a>,
    /// Feature indices whose tag was asked for; `None` collects all.
    filter: Option<BTreeSet<u16>>,
    features: BTreeSet<u16>,
    visited_scripts: HashSet<usize>,
    visited_lang_systems: HashSet<usize>,
    script_count: usize,
    lang_sys_count: usize,
    feature_index_count: usize,
}

impl FeatureCollector<'_, '_> {
    /// Whether the script at `offset` (in the script list) was visited
    /// already, or must be skipped as empty or beyond the limits.
    fn visited_script(&mut self, offset: usize, empty: bool) -> bool {
        if empty {
            return true;
        }
        self.script_count += 1;
        if self.script_count > MAX_SCRIPTS + 1 {
            return true;
        }
        !self.visited_scripts.insert(offset)
    }

    fn visited_lang_sys(&mut self, offset: usize, lang_sys: &LangSys<'_>) -> bool {
        if lang_sys.required_feature.is_none() && lang_sys.feature_indices.is_empty() {
            return true;
        }
        self.lang_sys_count += 1;
        if self.lang_sys_count > MAX_LANGSYS + 1 {
            return true;
        }
        !self.visited_lang_systems.insert(offset)
    }

    /// Counts `count` more feature indices; true past the limit.
    fn too_many_features(&mut self, count: usize) -> bool {
        self.feature_index_count += count;
        self.feature_index_count > MAX_FEATURE_INDICES
    }

    /// `langsys_collect_features`.
    fn lang_sys(&mut self, offset: usize, lang_sys: &LangSys<'_>) {
        if self.visited_lang_sys(offset, lang_sys) {
            return;
        }
        match &mut self.filter {
            None => {
                if let Some(required) = lang_sys.required_feature
                    && !self.too_many_features(1)
                {
                    self.features.insert(required);
                }
                if !self.too_many_features(lang_sys.feature_indices.len()) {
                    self.features.extend(lang_sys.feature_indices.iter());
                }
            }
            Some(filter) => {
                for index in lang_sys.feature_indices.iter() {
                    if filter.remove(&index) {
                        self.features.insert(index);
                    }
                }
            }
        }
    }

    /// `script_collect_features` for every language of the script `tag`.
    fn script(&mut self, tag: Tag) {
        let list = self.table.scripts;
        let Ok(records) = LayoutTable::records(list, self.table.tag) else {
            return;
        };
        let Some((_, record)) = records.harfbuzz_search_by(|record| record.tag.cmp(&tag)) else {
            return;
        };
        let script_offset = usize::from(record.offset);
        let Some(script) = list.get(script_offset..) else {
            return;
        };
        let mut r = Reader::new(script, self.table.tag);
        let default = r.u16().unwrap_or(0);
        let languages: Array<'_, TagRecord> = r
            .u16()
            .and_then(|count| r.array(usize::from(count)))
            .unwrap_or_default();
        if self.visited_script(script_offset, default == 0 && languages.is_empty()) {
            return;
        }
        let offsets = (default != 0)
            .then_some(default)
            .into_iter()
            .chain(languages.iter().map(|language| language.offset));
        for offset in offsets {
            let parsed = script
                .get(usize::from(offset)..)
                .and_then(|data| LangSys::parse(data, self.table.tag).ok());
            if let Some(lang_sys) = parsed {
                self.lang_sys(script_offset + usize::from(offset), &lang_sys);
            }
        }
    }
}

/// The lookups of the features tagged `features` (all features when
/// `None`) in every language system of `scripts`
/// (`hb_ot_layout_collect_lookups` with all languages). Lookup indices are
/// not checked against the lookup list.
pub(crate) fn collect_lookups(
    table: &LayoutTable<'_>,
    scripts: &[Tag],
    features: Option<&[Tag]>,
) -> BTreeSet<u16> {
    let filter = features.map(|tags| {
        let count = LayoutTable::records(table.features, table.tag).map_or(0, |r| r.len());
        (0..count)
            .filter_map(|index| u16::try_from(index).ok())
            .filter(|&index| {
                table
                    .feature(index)
                    .is_some_and(|(tag, _)| tags.contains(&tag))
            })
            .collect()
    });
    let mut collector = FeatureCollector {
        table,
        filter,
        features: BTreeSet::new(),
        visited_scripts: HashSet::new(),
        visited_lang_systems: HashSet::new(),
        script_count: 0,
        lang_sys_count: 0,
        feature_index_count: 0,
    };
    if collector
        .filter
        .as_ref()
        .is_none_or(|filter| !filter.is_empty())
    {
        for &script in scripts {
            collector.script(script);
        }
    }
    collector
        .features
        .iter()
        .filter_map(|&index| table.feature(index))
        .flat_map(|(_, lookups)| lookups.iter())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{layout_table, lookup};

    #[test]
    fn lookups_of_features() {
        let single = vec![0, 1, 0, 6, 0, 1, 0, 1, 0, 1, 0, 5];
        let data = layout_table(
            *b"latn",
            &[(b"smcp", &[1]), (b"liga", &[0, 2])],
            &[
                lookup(1, 0, std::slice::from_ref(&single)),
                lookup(1, 0, std::slice::from_ref(&single)),
            ],
        );
        let table = LayoutTable::parse(&data, Tag::new(b"GSUB"), 7).unwrap();
        let latn = [Tag::new(b"latn")];
        let all: Vec<u16> = collect_lookups(&table, &latn, None).into_iter().collect();
        assert_eq!(all, [0, 1, 2], "unchecked indices included");
        let smcp = [Tag::new(b"smcp")];
        let some: Vec<u16> = collect_lookups(&table, &latn, Some(&smcp))
            .into_iter()
            .collect();
        assert_eq!(some, [1]);
        assert!(collect_lookups(&table, &[Tag::new(b"grek")], None).is_empty());
        assert!(collect_lookups(&table, &latn, Some(&[Tag::new(b"kern")])).is_empty());
    }
}
