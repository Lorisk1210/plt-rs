//! `GSUB` lookups built at shaping time, serialized into an in-memory
//! table so that the regular lookup engine applies them (HarfBuzz's
//! synthesized fallback lookups, e.g. for Arabic fonts that only map the
//! presentation-form characters).

use crate::error::Result;
use crate::layout::common::LayoutTable;
use crate::types::{GlyphId, Tag};

/// Lookup type of single substitutions.
const SINGLE: u16 = 1;
/// Lookup type of ligature substitutions.
const LIGATURE: u16 = 4;

/// One ligature: the components after the first glyph and the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Ligature {
    pub(crate) components: Vec<GlyphId>,
    pub(crate) glyph: GlyphId,
}

/// A lookup to synthesize.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SyntheticLookup {
    /// Single substitution `(glyph, substitute)` pairs.
    Single {
        flags: u16,
        pairs: Vec<(GlyphId, GlyphId)>,
    },
    /// Ligatures grouped by first glyph, tried in order.
    Ligature {
        flags: u16,
        sets: Vec<(GlyphId, Vec<Ligature>)>,
    },
}

/// Big-endian serialization helpers.
trait Push {
    fn push_u16(&mut self, value: u16);
}

impl Push for Vec<u8> {
    fn push_u16(&mut self, value: u16) {
        self.extend_from_slice(&value.to_be_bytes());
    }
}

/// Converts a length or offset to the 16-bit field it is stored in.
fn short(value: usize) -> u16 {
    u16::try_from(value).expect("synthesized lookups stay far below 64 KiB")
}

/// A format 1 coverage table of `glyphs` (sorted, unique).
fn coverage(glyphs: impl ExactSizeIterator<Item = GlyphId>) -> Vec<u8> {
    let mut out = Vec::new();
    out.push_u16(1);
    out.push_u16(short(glyphs.len()));
    for glyph in glyphs {
        out.push_u16(glyph.0);
    }
    out
}

/// Single substitution format 2.
fn single_subtable(pairs: &[(GlyphId, GlyphId)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push_u16(2);
    out.push_u16(short(6 + 2 * pairs.len()));
    out.push_u16(short(pairs.len()));
    for &(_, substitute) in pairs {
        out.push_u16(substitute.0);
    }
    out.extend(coverage(pairs.iter().map(|&(glyph, _)| glyph)));
    out
}

/// A ligature set: count, offsets, then the ligature tables.
fn ligature_set(ligatures: &[Ligature]) -> Vec<u8> {
    let mut out = Vec::new();
    out.push_u16(short(ligatures.len()));
    let mut body = Vec::new();
    for ligature in ligatures {
        out.push_u16(short(2 + 2 * ligatures.len() + body.len()));
        body.push_u16(ligature.glyph.0);
        body.push_u16(short(ligature.components.len() + 1));
        for component in &ligature.components {
            body.push_u16(component.0);
        }
    }
    out.extend(body);
    out
}

/// Ligature substitution format 1.
fn ligature_subtable(sets: &[(GlyphId, Vec<Ligature>)]) -> Vec<u8> {
    let header = 6 + 2 * sets.len();
    let mut out = Vec::new();
    out.push_u16(1);
    let mut body = coverage(sets.iter().map(|(glyph, _)| *glyph));
    out.push_u16(short(header));
    out.push_u16(short(sets.len()));
    let mut offsets = Vec::new();
    for (_, ligatures) in sets {
        offsets.push(header + body.len());
        body.extend(ligature_set(ligatures));
    }
    for offset in offsets {
        out.push_u16(short(offset));
    }
    out.extend(body);
    out
}

/// A `GSUB` table without scripts or features whose lookup list holds the
/// synthesized lookups (one subtable each).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SyntheticGsub {
    data: Vec<u8>,
}

impl SyntheticGsub {
    /// Serializes `lookups`; single substitution pairs and ligature sets
    /// are sorted by glyph, stably and keeping duplicates like HarfBuzz
    /// (the coverage search then picks among them as HarfBuzz does).
    pub(crate) fn new(lookups: &[SyntheticLookup]) -> Self {
        let mut list = Vec::new();
        list.push_u16(short(lookups.len()));
        let mut body = Vec::new();
        for lookup in lookups {
            list.push_u16(short(2 + 2 * lookups.len() + body.len()));
            let (kind, flags, subtable) = match lookup {
                SyntheticLookup::Single { flags, pairs } => {
                    let mut pairs = pairs.clone();
                    pairs.sort_by_key(|&(glyph, _)| glyph);
                    (SINGLE, *flags, single_subtable(&pairs))
                }
                SyntheticLookup::Ligature { flags, sets } => {
                    let mut sets = sets.clone();
                    sets.sort_by_key(|(glyph, _)| *glyph);
                    (LIGATURE, *flags, ligature_subtable(&sets))
                }
            };
            body.push_u16(kind);
            body.push_u16(flags);
            body.push_u16(1);
            body.push_u16(8);
            body.extend(subtable);
        }
        list.extend(body);
        // Header (10 bytes), empty script list and feature list, lookups.
        let mut data = vec![0, 1, 0, 0, 0, 10, 0, 12, 0, 14, 0, 0, 0, 0];
        data.extend(list);
        Self { data }
    }

    /// The table, ready for an `Applier`.
    pub(crate) fn table(&self) -> Result<LayoutTable<'_>> {
        LayoutTable::parse(&self.data, Tag::new(b"GSUB"), 7)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_lookups() {
        let gsub = SyntheticGsub::new(&[
            SyntheticLookup::Single {
                flags: 8,
                pairs: vec![(GlyphId(9), GlyphId(90)), (GlyphId(3), GlyphId(30))],
            },
            SyntheticLookup::Ligature {
                flags: 0,
                sets: vec![(
                    GlyphId(5),
                    vec![Ligature {
                        components: vec![GlyphId(6)],
                        glyph: GlyphId(56),
                    }],
                )],
            },
        ]);
        let table = gsub.table().unwrap();
        assert_eq!(table.lookup_count(), 2);
        assert_eq!(table.script_tags().count(), 0);
        let single = table.lookup(0).unwrap();
        assert_eq!((single.kind, single.flags, single.len()), (1, 8, 1));
        let subtable = single.subtable(0).unwrap();
        // Format 2, three-glyph header, substitutes sorted with coverage.
        assert_eq!(&subtable[..10], &[0, 2, 0, 10, 0, 2, 0, 30, 0, 90]);
        assert_eq!(&subtable[10..18], &[0, 1, 0, 2, 0, 3, 0, 9]);
        let ligature = table.lookup(1).unwrap();
        assert_eq!((ligature.kind, ligature.flags), (4, 0));
        let subtable = ligature.subtable(0).unwrap();
        assert_eq!(&subtable[..8], &[0, 1, 0, 8, 0, 1, 0, 14]);
        assert_eq!(&subtable[14..24], &[0, 1, 0, 4, 0, 56, 0, 2, 0, 6]);
    }
}
