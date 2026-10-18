//! The `GSUB` single and alternate substitutions the diacritic
//! adjustments follow (FreeType's `afgsub.c`): FreeType validates every
//! lookup itself and merges the mappings of the usable ones into one
//! hash table, with its own coverage iterator (which stops after the
//! first glyph of a format 2 table's last range).

use super::hash::NumHash;
use crate::reader::peek;

/// A `u16` at `at`, if inside `data`.
fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    peek::<u16>(data, at)
}

/// A position `offset` bytes after `base`, if it lies at least `need`
/// bytes before the end of `data` (FreeType's `table_limit < p + need`
/// checks).
fn within(data: &[u8], base: usize, offset: usize, need: usize) -> Option<usize> {
    let at = base.checked_add(offset)?;
    (at.checked_add(need)? <= data.len()).then_some(at)
}

/// `af_validate_coverage`: the coverage at `at` is readable; returns its
/// glyph count.
fn validate_coverage(gsub: &[u8], at: usize) -> Option<usize> {
    within(gsub, at, 0, 4)?;
    let count = usize::from(u16_at(gsub, at + 2)?);
    match u16_at(gsub, at)? {
        1 => within(gsub, at + 4, 0, count * 2).map(|_| count),
        2 => {
            within(gsub, at + 4, 0, count * 6)?;
            (0..count).try_fold(0, |total, range| {
                let record = at + 4 + range * 6;
                let (start, end) = (u16_at(gsub, record)?, u16_at(gsub, record + 2)?);
                (start <= end).then(|| total + usize::from(end - start) + 1)
            })
        }
        _ => None,
    }
}

/// `af_validate_single_subst1`, `..._subst2` and `af_validate_alternate`
/// for the subtable at `at` of type `kind` (format already readable).
fn validate_subtable(gsub: &[u8], at: usize, kind: u16) -> Option<()> {
    let coverage = at + usize::from(u16_at(gsub, at + 2)?);
    match (kind, u16_at(gsub, at)?) {
        (1, 1) => validate_coverage(gsub, coverage).map(|_| ()),
        (1, 2) => {
            let glyphs = validate_coverage(gsub, coverage)?;
            let count = usize::from(u16_at(gsub, at + 4)?);
            within(gsub, at + 6, 0, count * 2)?;
            (count == glyphs).then_some(())
        }
        (3, 1) => {
            let glyphs = validate_coverage(gsub, coverage)?;
            let count = usize::from(u16_at(gsub, at + 4)?);
            within(gsub, at + 6, 0, count * 2)?;
            if count != glyphs {
                return None;
            }
            (0..count).try_for_each(|index| {
                let set = at + usize::from(u16_at(gsub, at + 6 + index * 2)?);
                within(gsub, set, 0, 2)?;
                let alternates = usize::from(u16_at(gsub, set)?);
                within(gsub, set + 2, 0, alternates * 2).map(|_| ())
            })
        }
        _ => None,
    }
}

/// `af_validate_lookup_table`: a lookup made of single or alternate
/// substitutions only (directly or through extensions of one type).
fn validate_lookup(gsub: &[u8], at: usize) -> bool {
    let check = || -> Option<()> {
        within(gsub, at, 0, 6)?;
        let kind = u16_at(gsub, at)?;
        let count = usize::from(u16_at(gsub, at + 4)?);
        within(gsub, at + 6, 0, count * 2)?;
        let mut real_kind = 0;
        for index in 0..count {
            let mut subtable = at + usize::from(u16_at(gsub, at + 6 + index * 2)?);
            if kind == 7 {
                within(gsub, subtable, 0, 8)?;
                if u16_at(gsub, subtable)? != 1 {
                    return None;
                }
                let extension_kind = u16_at(gsub, subtable + 2)?;
                if real_kind == 0 {
                    real_kind = extension_kind;
                } else if real_kind != extension_kind {
                    return None;
                }
                subtable = subtable.checked_add(peek::<u32>(gsub, subtable + 4)? as usize)?;
            } else {
                real_kind = kind;
            }
            within(gsub, subtable, 0, 6)?;
            validate_subtable(gsub, subtable, real_kind)?;
        }
        Some(())
    };
    check().is_some()
}

/// The single and alternate substitution lookups of a `GSUB` table
/// (`af_parse_gsub`): for every lookup, its offset in the table when
/// FreeType accepts it.
#[derive(Clone, Debug, Default)]
pub(crate) struct SubstitutionLookups {
    offsets: Vec<Option<usize>>,
}

impl SubstitutionLookups {
    /// Validates the lookups of `gsub` (the raw table).
    pub(crate) fn new(gsub: &[u8]) -> Self {
        let parse = || -> Option<Vec<Option<usize>>> {
            if gsub.len() < 10 {
                return None;
            }
            let list = usize::from(u16_at(gsub, 8)?);
            let count = usize::from(u16_at(gsub, list)?);
            within(gsub, list + 2, 0, count * 2)?;
            (0..count)
                .map(|index| {
                    let offset = list + usize::from(u16_at(gsub, list + 2 + index * 2)?);
                    Some(validate_lookup(gsub, offset).then_some(offset))
                })
                .collect()
        };
        Self {
            offsets: parse().unwrap_or_default(),
        }
    }

    /// The offset of lookup `index`, if it is a usable substitution.
    fn offset(&self, index: u16) -> Option<usize> {
        self.offsets.get(usize::from(index)).copied().flatten()
    }

    /// Adds the mappings of lookup `index` to `map` (`af_map_lookup`):
    /// a glyph maps to its first substitute, and its further substitutes
    /// go under the keys `glyph + (n << 16)`, with their number in the high
    /// bits of the first entry.
    pub(crate) fn map_lookup(&self, gsub: &[u8], index: u16, map: &mut NumHash) {
        let Some(table) = self.offset(index) else {
            return;
        };
        let kind = u16_at(gsub, table).unwrap_or(0);
        let count = usize::from(u16_at(gsub, table + 4).unwrap_or(0));
        for entry in 0..count {
            let Some(mut subtable) =
                u16_at(gsub, table + 6 + entry * 2).map(|offset| table + usize::from(offset))
            else {
                continue;
            };
            let mut real_kind = kind;
            if kind == 7 {
                real_kind = u16_at(gsub, subtable + 2).unwrap_or(0);
                subtable += peek::<u32>(gsub, subtable + 4).unwrap_or(0) as usize;
            }
            map_subtable(gsub, subtable, real_kind, map);
        }
    }
}

/// Maps one validated subtable.
fn map_subtable(gsub: &[u8], at: usize, kind: u16, map: &mut NumHash) {
    let coverage = at + usize::from(u16_at(gsub, at + 2).unwrap_or(0));
    let glyphs = coverage_glyphs(gsub, coverage);
    match (kind, u16_at(gsub, at).unwrap_or(0)) {
        (1, 1) => {
            let delta = u16_at(gsub, at + 4).unwrap_or(0);
            for glyph in glyphs {
                insert(map, glyph, glyph.wrapping_add(delta));
            }
        }
        (1, _) => {
            for (index, glyph) in glyphs.into_iter().enumerate() {
                insert(map, glyph, u16_at(gsub, at + 6 + index * 2).unwrap_or(0));
            }
        }
        _ => {
            for (index, glyph) in glyphs.into_iter().enumerate() {
                let set = at + usize::from(u16_at(gsub, at + 6 + index * 2).unwrap_or(0));
                let count = usize::from(u16_at(gsub, set).unwrap_or(0));
                for alternate in 0..count {
                    insert(
                        map,
                        glyph,
                        u16_at(gsub, set + 2 + alternate * 2).unwrap_or(0),
                    );
                }
            }
        }
    }
}

/// FreeType's coverage iteration (`af_coverage_iterator`): all glyphs of
/// format 1; for format 2 every range but only the first glyph of the
/// last one.
fn coverage_glyphs(gsub: &[u8], at: usize) -> Vec<u16> {
    let count = usize::from(u16_at(gsub, at + 2).unwrap_or(0));
    match u16_at(gsub, at) {
        Some(1) => (0..count)
            .filter_map(|index| u16_at(gsub, at + 4 + index * 2))
            .collect(),
        Some(_) => {
            let mut glyphs = Vec::new();
            for range in 0..count {
                let record = at + 4 + range * 6;
                let start = u16_at(gsub, record).unwrap_or(0);
                let end = u16_at(gsub, record + 2).unwrap_or(0);
                if range + 1 == count {
                    glyphs.push(start);
                } else {
                    glyphs.extend(start..=end);
                }
            }
            glyphs
        }
        None => Vec::new(),
    }
}

/// `af_hash_insert`.
fn insert(map: &mut NumHash, glyph: u16, substitute: u16) {
    let key = i32::from(glyph);
    match map.get(key) {
        None => map.insert(key, usize::from(substitute), true),
        Some(value) => {
            let mask = (value & 0xFFFF_0000) + 0x1_0000;
            map.insert(key | mask as i32, usize::from(substitute), true);
            map.insert(key, value + 0x1_0000, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_range_yields_one_glyph() {
        let coverage = [0, 2, 0, 2, 0, 5, 0, 6, 0, 0, 0, 9, 0, 11, 0, 2];
        assert_eq!(coverage_glyphs(&coverage, 0), [5, 6, 9]);
        let format1 = [0, 1, 0, 2, 0, 4, 0, 3];
        assert_eq!(coverage_glyphs(&format1, 0), [4, 3]);
        assert_eq!(validate_coverage(&coverage, 0), Some(5));
        assert_eq!(validate_coverage(&coverage[..10], 0), None);
    }

    #[test]
    fn multiple_substitutes_chain_through_high_keys() {
        let mut map = NumHash::new();
        insert(&mut map, 7, 20);
        insert(&mut map, 7, 21);
        insert(&mut map, 7, 22);
        assert_eq!(map.get(7), Some(20 + 0x2_0000));
        assert_eq!(map.get(7 + 0x1_0000), Some(21));
        assert_eq!(map.get(7 + 0x2_0000), Some(22));
    }

    #[test]
    fn broken_tables_have_no_lookups() {
        assert_eq!(SubstitutionLookups::new(&[0; 4]).offsets, []);
        let header = [0, 1, 0, 0, 0, 10, 0, 10, 0, 10, 0, 0];
        let lookups = SubstitutionLookups::new(&header);
        assert_eq!(lookups.offsets, []);
        assert_eq!(lookups.offset(0), None);
    }
}
