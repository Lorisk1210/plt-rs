//! HarfBuzz's feature map (`hb_ot_map_t`): which `GSUB`/`GPOS` lookups a
//! script, language and feature list select, grouped into stages separated
//! by shaper pauses, each lookup restricted to the glyphs carrying its
//! feature's mask bits.

use std::ops::BitOr;

use crate::face::Face;
use crate::layout::apply::TableKind;
use crate::layout::buffer::Mask;
use crate::layout::common::{LangSys, LayoutTable};
use crate::layout::feature::{find_feature, select_lang_sys};
use crate::types::Tag;

/// Mask bit shared by all global features with value 1.
const GLOBAL_BIT_SHIFT: u32 = 31;
/// First bit available to features (bits below hold HarfBuzz's glyph
/// flags).
const FIRST_FEATURE_BIT: u32 = 4;
/// Largest number of mask bits one feature may use.
const MAX_FEATURE_BITS: u32 = 8;

/// How a feature is applied (HarfBuzz's `hb_ot_map_feature_flags_t`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct FeatureFlags(u8);

impl FeatureFlags {
    pub(crate) const NONE: Self = Self(0);
    /// Applies to all characters (no mask bits of its own for value 1).
    pub(crate) const GLOBAL: Self = Self(1);
    /// Keeps a mask bit even when the font lacks the feature, because the
    /// shaper implements a fallback.
    pub(crate) const HAS_FALLBACK: Self = Self(2);
    /// Doesn't skip ZWNJ when matching context.
    pub(crate) const MANUAL_ZWNJ: Self = Self(4);
    /// Doesn't skip ZWJ when matching input.
    pub(crate) const MANUAL_ZWJ: Self = Self(8);
    pub(crate) const MANUAL_JOINERS: Self = Self(4 | 8);

    const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for FeatureFlags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// One lookup of a stage, with the mask of the features that selected it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LookupMap {
    pub(crate) index: u16,
    pub(crate) mask: Mask,
    pub(crate) auto_zwnj: bool,
    pub(crate) auto_zwj: bool,
}

impl LookupMap {
    /// A lookup applied to every glyph with default joiner handling.
    pub(crate) const fn global(index: u16) -> Self {
        Self {
            index,
            mask: Mask::MAX,
            auto_zwnj: true,
            auto_zwj: true,
        }
    }
}

/// A run of lookups followed by an optional shaper pause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Stage<P> {
    pub(crate) lookups: Vec<LookupMap>,
    pub(crate) pause: Option<P>,
}

#[derive(Clone, Copy, Debug)]
struct FeatureInfo {
    tag: Tag,
    seq: usize,
    max_value: u32,
    flags: FeatureFlags,
    default_value: u32,
    stage: [usize; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FeatureMap {
    tag: Tag,
    index: [Option<u16>; 2],
    stage: [usize; 2],
    mask: Mask,
    shift: u32,
    one_mask: Mask,
    needs_fallback: bool,
    auto_zwnj: bool,
    auto_zwj: bool,
}

/// A layout table with the language system chosen for the run.
#[derive(Clone, Copy, Debug)]
struct Selected<'a> {
    table: LayoutTable<'a>,
    script: Tag,
    lang_sys: LangSys<'a>,
}

impl Selected<'_> {
    fn feature(&self, tag: Tag) -> Option<u16> {
        find_feature(&self.table, &self.lang_sys, tag)
    }
}

/// Collects features and pauses, then compiles them into a [`Map`].
#[derive(Debug)]
pub(crate) struct MapBuilder<'a, P> {
    tables: [Option<Selected<'a>>; 2],
    features: Vec<FeatureInfo>,
    current_stage: [usize; 2],
    pauses: [Vec<Option<P>>; 2],
}

impl<'a, P: Copy> MapBuilder<'a, P> {
    /// A builder for text whose script and language look for the OpenType
    /// tags `scripts` and `languages`, most preferred first.
    pub(crate) fn new(face: &Face<'a>, scripts: &[Tag], languages: &[Tag]) -> Self {
        let select = |table: Option<&LayoutTable<'a>>| {
            let table = *table?;
            let (script, lang_sys) = select_lang_sys(&table, scripts, languages)?;
            Some(Selected {
                table,
                script,
                lang_sys,
            })
        };
        Self {
            tables: [select(face.gsub()), select(face.gpos())],
            features: Vec::new(),
            current_stage: [0, 0],
            pauses: [Vec::new(), Vec::new()],
        }
    }

    /// The script tag chosen in `table` (`DFLT` etc. when the font lacks
    /// the run's script).
    pub(crate) fn chosen_script(&self, table: TableKind) -> Option<Tag> {
        self.tables[table.index()].map(|selected| selected.script)
    }

    /// Whether either table's language system has feature `tag`.
    pub(crate) fn has_feature(&self, tag: Tag) -> bool {
        self.tables
            .iter()
            .flatten()
            .any(|selected| selected.feature(tag).is_some())
    }

    /// Adds a feature in the current stage.
    pub(crate) fn add_feature(&mut self, tag: Tag, flags: FeatureFlags, value: u32) {
        self.features.push(FeatureInfo {
            tag,
            seq: self.features.len() + 1,
            max_value: value,
            flags,
            default_value: if flags.contains(FeatureFlags::GLOBAL) {
                value
            } else {
                0
            },
            stage: self.current_stage,
        });
    }

    /// Adds a feature enabled for the whole text.
    pub(crate) fn enable_feature(&mut self, tag: Tag, flags: FeatureFlags) {
        self.add_feature(tag, flags | FeatureFlags::GLOBAL, 1);
    }

    /// Ends the current `GSUB` stage; `pause` runs after its lookups.
    pub(crate) fn add_gsub_pause(&mut self, pause: Option<P>) {
        self.add_pause(TableKind::Gsub, pause);
    }

    fn add_pause(&mut self, table: TableKind, pause: Option<P>) {
        self.pauses[table.index()].push(pause);
        self.current_stage[table.index()] += 1;
    }

    /// Sorts the features and merges duplicates like HarfBuzz: global
    /// settings override, the earliest stage wins.
    fn merged_features(&mut self) -> Vec<FeatureInfo> {
        let mut features = std::mem::take(&mut self.features);
        features.sort_by_key(|f| (f.tag, f.seq));
        let mut merged: Vec<FeatureInfo> = Vec::with_capacity(features.len());
        for feature in features {
            match merged.last_mut() {
                Some(last) if last.tag == feature.tag => {
                    if feature.flags.contains(FeatureFlags::GLOBAL) {
                        last.flags = last.flags | FeatureFlags::GLOBAL;
                        last.max_value = feature.max_value;
                        last.default_value = feature.default_value;
                    } else {
                        last.flags = FeatureFlags(last.flags.0 & !FeatureFlags::GLOBAL.0);
                        last.max_value = last.max_value.max(feature.max_value);
                    }
                    last.flags = FeatureFlags(
                        last.flags.0 | (feature.flags.0 & FeatureFlags::HAS_FALLBACK.0),
                    );
                    last.stage = [
                        last.stage[0].min(feature.stage[0]),
                        last.stage[1].min(feature.stage[1]),
                    ];
                }
                _ => merged.push(feature),
            }
        }
        merged
    }

    /// Allocates mask bits and resolves the lookups of every stage.
    pub(crate) fn compile(mut self) -> Map<P> {
        let features = self.merged_features();
        let required: [Option<(Tag, u16)>; 2] = self.tables.map(|selected| {
            let selected = selected?;
            let index = selected.lang_sys.required_feature?;
            Some((selected.table.feature(index)?.0, index))
        });
        let mut required_stage = [0usize; 2];
        let mut global_mask: Mask = 1 << GLOBAL_BIT_SHIFT;
        let mut next_bit = FIRST_FEATURE_BIT;
        let mut maps = Vec::new();
        for info in &features {
            let global = info.flags.contains(FeatureFlags::GLOBAL) && info.max_value == 1;
            let bits = if global {
                0
            } else {
                info.max_value.bit_width().min(MAX_FEATURE_BITS)
            };
            if info.max_value == 0 || next_bit + bits >= GLOBAL_BIT_SHIFT {
                continue;
            }
            for (table, slot) in required.iter().enumerate() {
                if slot.is_some_and(|(tag, _)| tag == info.tag) {
                    required_stage[table] = info.stage[table];
                }
            }
            let index = self
                .tables
                .map(|selected| selected.and_then(|s| s.feature(info.tag)));
            let found = index.iter().any(Option::is_some);
            if !found && !info.flags.contains(FeatureFlags::HAS_FALLBACK) {
                continue;
            }
            let (shift, mask) = if global {
                (GLOBAL_BIT_SHIFT, 1 << GLOBAL_BIT_SHIFT)
            } else {
                let mask = (1 << (next_bit + bits)) - (1 << next_bit);
                let shift = next_bit;
                next_bit += bits;
                global_mask |= (info.default_value << shift) & mask;
                (shift, mask)
            };
            maps.push(FeatureMap {
                tag: info.tag,
                index,
                stage: info.stage,
                mask,
                shift,
                one_mask: (1 << shift) & mask,
                needs_fallback: !found,
                auto_zwnj: !info.flags.contains(FeatureFlags::MANUAL_ZWNJ),
                auto_zwj: !info.flags.contains(FeatureFlags::MANUAL_ZWJ),
            });
        }
        self.add_gsub_pause(None);
        self.add_pause(TableKind::Gpos, None);
        // Stages exist (for their pauses) even when the font lacks the table.
        let stages = [TableKind::Gsub, TableKind::Gpos].map(|table| {
            let t = table.index();
            let required = required[t].map(|(_, index)| (index, required_stage[t]));
            self.pauses[t]
                .iter()
                .enumerate()
                .map(|(stage, &pause)| Stage {
                    lookups: self.tables[t].map_or_else(Vec::new, |selected| {
                        stage_lookups(&selected, &maps, t, stage, required)
                    }),
                    pause,
                })
                .collect()
        });
        Map {
            global_mask,
            features: maps,
            stages,
        }
    }
}

/// Lookups of `stage` in table `t`, sorted by index with duplicates merged
/// (masks combined, joiner handling manual if any feature wants it).
fn stage_lookups(
    selected: &Selected<'_>,
    maps: &[FeatureMap],
    t: usize,
    stage: usize,
    required: Option<(u16, usize)>,
) -> Vec<LookupMap> {
    let mut lookups = Vec::new();
    let mut add = |feature: u16, mask: Mask, auto_zwnj: bool, auto_zwj: bool| {
        if let Some((_, indices)) = selected.table.feature(feature) {
            lookups.extend(
                indices
                    .iter()
                    .filter(|&index| usize::from(index) < selected.table.lookup_count())
                    .map(|index| LookupMap {
                        index,
                        mask,
                        auto_zwnj,
                        auto_zwj,
                    }),
            );
        }
    };
    if let Some((index, _)) = required.filter(|&(_, s)| s == stage) {
        add(index, 1 << GLOBAL_BIT_SHIFT, true, true);
    }
    for map in maps.iter().filter(|map| map.stage[t] == stage) {
        if let Some(index) = map.index[t] {
            add(index, map.mask, map.auto_zwnj, map.auto_zwj);
        }
    }
    lookups.sort_by_key(|lookup| lookup.index);
    let mut merged: Vec<LookupMap> = Vec::with_capacity(lookups.len());
    for lookup in lookups {
        match merged.last_mut() {
            Some(last) if last.index == lookup.index => {
                last.mask |= lookup.mask;
                last.auto_zwnj &= lookup.auto_zwnj;
                last.auto_zwj &= lookup.auto_zwj;
            }
            _ => merged.push(lookup),
        }
    }
    merged
}

/// The compiled feature map of one shaping run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Map<P> {
    global_mask: Mask,
    features: Vec<FeatureMap>,
    stages: [Vec<Stage<P>>; 2],
}

impl<P> Map<P> {
    /// Mask every glyph starts with: global features plus the default
    /// values of the others.
    pub(crate) const fn global_mask(&self) -> Mask {
        self.global_mask
    }

    fn feature(&self, tag: Tag) -> Option<&FeatureMap> {
        self.features.iter().find(|map| map.tag == tag)
    }

    /// The mask bits of feature `tag` and `value` in them (HarfBuzz's
    /// `get_mask` with the value shifted into place), `None` when the
    /// feature is not in the map.
    pub(crate) fn value_bits(&self, tag: Tag, value: u32) -> Option<(Mask, Mask)> {
        self.feature(tag)
            .map(|map| (map.mask, (value << map.shift) & map.mask))
    }

    /// Mask bits enabling feature `tag` with value 1 (0 when the feature is
    /// not in the map).
    pub(crate) fn one_mask(&self, tag: Tag) -> Mask {
        self.feature(tag).map_or(0, |map| map.one_mask)
    }

    /// Whether feature `tag` is in the map only through its fallback.
    pub(crate) fn needs_fallback(&self, tag: Tag) -> bool {
        self.feature(tag).is_some_and(|map| map.needs_fallback)
    }

    /// Whether `table` has feature `tag` for the run.
    pub(crate) fn has_lookups(&self, table: TableKind, tag: Tag) -> bool {
        self.feature(tag)
            .is_some_and(|map| map.index[table.index()].is_some())
    }

    /// The stages of `table` in order.
    pub(crate) fn stages(&self, table: TableKind) -> &[Stage<P>] {
        &self.stages[table.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::common::tests::{layout_table, lookup};

    const SINGLE: [u8; 12] = [0, 1, 0, 6, 0, 1, 0, 1, 0, 1, 0, 5];

    fn selected(data: &[u8]) -> Selected<'_> {
        let table = LayoutTable::parse(data, Tag::new(b"GSUB"), 7).unwrap();
        let (script, lang_sys) = select_lang_sys(&table, &[Tag::new(b"arab")], &[]).unwrap();
        Selected {
            table,
            script,
            lang_sys,
        }
    }

    fn builder(data: &[u8]) -> MapBuilder<'_, u8> {
        MapBuilder {
            tables: [Some(selected(data)), None],
            features: Vec::new(),
            current_stage: [0, 0],
            pauses: [Vec::new(), Vec::new()],
        }
    }

    #[test]
    fn stages_masks_and_pauses() {
        let lookups: Vec<Vec<u8>> = (0..4).map(|_| lookup(1, 0, &[SINGLE.to_vec()])).collect();
        let data = layout_table(
            *b"arab",
            &[
                (b"ccmp", &[1]),
                (b"init", &[2]),
                (b"liga", &[3, 1]),
                (b"rlig", &[0]),
            ],
            &lookups,
        );
        let mut map = builder(&data);
        map.enable_feature(Tag::new(b"ccmp"), FeatureFlags::NONE);
        map.add_gsub_pause(Some(7));
        map.add_feature(Tag::new(b"init"), FeatureFlags::MANUAL_ZWJ, 1);
        map.add_feature(Tag::new(b"isol"), FeatureFlags::HAS_FALLBACK, 1);
        map.add_feature(Tag::new(b"fina"), FeatureFlags::NONE, 1);
        map.add_gsub_pause(None);
        map.enable_feature(Tag::new(b"liga"), FeatureFlags::NONE);
        map.enable_feature(Tag::new(b"ccmp"), FeatureFlags::NONE);
        map.add_feature(Tag::new(b"kern"), FeatureFlags::GLOBAL, 0);
        assert!(map.has_feature(Tag::new(b"rlig")) && !map.has_feature(Tag::new(b"kern")));
        assert_eq!(map.chosen_script(TableKind::Gsub), Some(Tag::new(b"arab")));
        let map = map.compile();
        let global = 1 << GLOBAL_BIT_SHIFT;
        assert_eq!(map.global_mask(), global);
        let init = map.one_mask(Tag::new(b"init"));
        let isol = map.one_mask(Tag::new(b"isol"));
        assert!(init != 0 && isol != 0 && init != isol && init & global == 0);
        assert_eq!(map.one_mask(Tag::new(b"fina")), 0);
        assert_eq!(map.one_mask(Tag::new(b"liga")), global);
        assert!(map.needs_fallback(Tag::new(b"isol")) && !map.needs_fallback(Tag::new(b"init")));
        assert!(map.has_lookups(TableKind::Gsub, Tag::new(b"init")));
        assert!(!map.has_lookups(TableKind::Gpos, Tag::new(b"init")));
        let stages = map.stages(TableKind::Gsub);
        assert_eq!(stages.len(), 3);
        assert_eq!(stages[0].pause, Some(7));
        assert_eq!(
            stages[0].lookups,
            vec![LookupMap {
                mask: global,
                ..LookupMap::global(1)
            }]
        );
        assert_eq!(
            stages[1].lookups,
            vec![LookupMap {
                index: 2,
                mask: init,
                auto_zwnj: true,
                auto_zwj: false
            }]
        );
        let indices: Vec<u16> = stages[2].lookups.iter().map(|l| l.index).collect();
        assert_eq!(indices, vec![1, 3]);
        let gpos = map.stages(TableKind::Gpos);
        assert!(gpos.len() == 1 && gpos[0].lookups.is_empty());
    }

    #[test]
    fn feature_values_take_bits() {
        let data = layout_table(
            *b"latn",
            &[(b"aalt", &[0])],
            &[lookup(1, 0, &[SINGLE.to_vec()])],
        );
        let mut map = builder(&data);
        map.add_feature(Tag::new(b"aalt"), FeatureFlags::GLOBAL, 3);
        let map = map.compile();
        let mask = map.one_mask(Tag::new(b"aalt"));
        assert_eq!(mask, 1 << FIRST_FEATURE_BIT);
        assert_eq!(map.global_mask() & mask, mask);
        let bits = 0b11 << FIRST_FEATURE_BIT;
        assert_eq!(
            map.value_bits(Tag::new(b"aalt"), 2),
            Some((bits, 0b10 << FIRST_FEATURE_BIT))
        );
        assert_eq!(map.value_bits(Tag::new(b"aalt"), 7), Some((bits, bits)));
        assert_eq!(map.value_bits(Tag::new(b"kern"), 1), None);
    }
}
