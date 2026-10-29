//! Which glyphs get diacritic adjustments (FreeType's `afadjust.c`): the
//! adjustment database looked up by code point, and the reverse character
//! map that leads from glyphs back to the code points of the database,
//! through the character map and `GSUB` single and alternate
//! substitutions of the style's script.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use super::cmap::UnicodeMap;
use super::gsub::SubstitutionLookups;
use super::hash::NumHash;
use super::shaper::script_tags;
use super::tables::{ADJUSTMENTS, ScriptClass};
use crate::layout::collect::collect_lookups;
use crate::layout::common::LayoutTable;
use crate::types::GlyphId;

/// The adjustment flags of `code_point` (`af_adjustment_database_lookup`).
///
/// This is FreeType's binary search over its database, which is not
/// quite sorted (`Q` comes before `?`), so that the entry for `Q` is
/// never found.
pub(crate) fn adjustment(code_point: u32) -> u16 {
    let (mut low, mut high) = (0usize, ADJUSTMENTS.len() - 1);
    while high >= low {
        let mid = usize::midpoint(low, high);
        let (code, flags) = ADJUSTMENTS[mid];
        match code.cmp(&code_point) {
            Ordering::Less => low = mid + 1,
            // Below the first entry FreeType's unsigned index would wrap;
            // code points of the database never get there.
            Ordering::Greater => match mid.checked_sub(1) {
                Some(below) => high = below,
                None => return 0,
            },
            Ordering::Equal => return flags,
        }
    }
    0
}

/// Glyphs mapped to the code points of the adjustment database
/// (`af_reverse_character_map_new`).
#[derive(Clone, Debug, Default)]
pub(crate) struct ReverseCharmap {
    code_points: HashMap<u16, u32>,
}

impl ReverseCharmap {
    /// The map for a style of `script`: the database code points' glyphs,
    /// plus every glyph the script's `GSUB` single and alternate
    /// substitutions lead to from them.
    pub(crate) fn new(
        unicode: &UnicodeMap,
        script: &ScriptClass,
        gsub: Option<(&[u8], LayoutTable<'_>)>,
        substitutions: &SubstitutionLookups,
    ) -> Self {
        let mut code_points = HashMap::new();
        for &(code_point, _) in &ADJUSTMENTS {
            if let Some(glyph) = unicode.glyph(code_point) {
                code_points.insert(glyph.0, code_point);
            }
        }
        let mut map = Self { code_points };
        if let Some((data, table)) = gsub {
            map.follow_substitutions(data, &table, script, substitutions);
        }
        map
    }

    fn follow_substitutions(
        &mut self,
        data: &[u8],
        table: &LayoutTable<'_>,
        script: &ScriptClass,
        substitutions: &SubstitutionLookups,
    ) {
        // FreeType asks HarfBuzz for a single script tag here.
        let scripts = script_tags(script.harfbuzz, 1);
        let mut substitutes = NumHash::new();
        for lookup in collect_lookups(table, &scripts, None) {
            substitutions.map_lookup(data, lookup, &mut substitutes);
        }
        for (glyph, value) in substitutes.iter() {
            if glyph as u32 & 0xFFFF_0000 != 0 {
                continue;
            }
            let Some(&code_point) = self.code_points.get(&(glyph as u16)) else {
                continue;
            };
            self.add_substitutes(glyph, value, code_point, &substitutes);
        }
    }

    /// Records `code_point` for the substitutes of `glyph` (whose first
    /// hash entry is `value`) not mapped yet, and depth first for theirs
    /// (FreeType's recursive `add_substitute`, with an explicit stack).
    fn add_substitutes(&mut self, glyph: i32, value: usize, code_point: u32, map: &NumHash) {
        let substitutes = |glyph: i32, value: usize| -> std::vec::IntoIter<i32> {
            let further = (1..=value >> 16).filter_map(|n| map.get(glyph + ((n as i32) << 16)));
            std::iter::once((value & 0xFFFF) as i32)
                .chain(further.map(|substitute| substitute as i32))
                .collect::<Vec<_>>()
                .into_iter()
        };
        let mut stack = vec![substitutes(glyph, value)];
        while let Some(pending) = stack.last_mut() {
            let Some(substitute) = pending.next() else {
                stack.pop();
                continue;
            };
            if self.insert_new(substitute, code_point)
                && let Some(next) = map.get(substitute)
            {
                stack.push(substitutes(substitute, next));
            }
        }
    }

    /// Maps `glyph` to `code_point` unless it is mapped; true if it was
    /// added.
    fn insert_new(&mut self, glyph: i32, code_point: u32) -> bool {
        match self.code_points.entry(glyph as u16) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(code_point);
                true
            }
        }
    }

    /// The database code point `glyph` belongs to.
    pub(crate) fn code_point(&self, glyph: GlyphId) -> Option<u32> {
        self.code_points.get(&glyph.0).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autofit::tables::adjust::{NO_HEIGHT_CHECK, TILDE_TOP, UP};

    #[test]
    fn database_lookup_follows_freetype() {
        assert_eq!(adjustment(0x21), UP | NO_HEIGHT_CHECK);
        assert_eq!(adjustment(0x69), UP);
        assert_eq!(adjustment(0xC3), UP | TILDE_TOP);
        assert_eq!(adjustment(0x3F), UP | NO_HEIGHT_CHECK);
        // The unsorted entry for `Q` is never found.
        assert_eq!(adjustment(0x51), 0);
        assert_eq!(adjustment(0x41), 0);
        assert_eq!(adjustment(0x10_FFFF), 0);
    }
}
