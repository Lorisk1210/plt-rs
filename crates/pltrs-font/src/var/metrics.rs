//! Varied metrics: `HVAR` advance widths (`tt_hadvance_adjust`) and the
//! face metrics `MVAR` varies (`tt_apply_mvar`).

use super::store::{IndexMap, ItemStore};
use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::types::{GlyphId, Tag};

const HVAR: Tag = Tag::new(b"HVAR");
const MVAR: Tag = Tag::new(b"MVAR");

/// An `HVAR` table: advance width deltas.
#[derive(Clone, Debug)]
pub(crate) struct Hvar<'a> {
    store: ItemStore<'a>,
    widths: IndexMap,
}

impl<'a> Hvar<'a> {
    /// Parses an `HVAR` table of a font of `axis_count` axes.
    pub(crate) fn parse(data: &'a [u8], axis_count: usize) -> Result<Self> {
        let mut r = Reader::new(data, HVAR);
        if r.u16()? != 1 {
            return Err(FontError::malformed(HVAR, "unknown HVAR version"));
        }
        r.skip(2)?;
        let store_at = r.u32()? as usize;
        let widths_at = r.u32()? as usize;
        let store = ItemStore::parse(data, store_at, axis_count, HVAR)?;
        let widths = if widths_at == 0 {
            IndexMap::default()
        } else {
            IndexMap::parse(data, widths_at, &store, HVAR)?
        };
        Ok(Self { store, widths })
    }

    /// The delta of `glyph`'s advance width at `coords`.
    pub(crate) fn advance_delta(&self, glyph: GlyphId, coords: &[i64]) -> i64 {
        let (outer, inner) = self.widths.item(usize::from(glyph.0));
        self.store.delta(outer, inner, coords)
    }
}

/// The face metrics `MVAR` varies, which FreeType applies to the face's
/// ascender, descender and height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct LineMetricDeltas {
    /// `hasc`: the typographic ascender.
    pub(crate) ascender: i16,
    /// `hdsc`: the typographic descender.
    pub(crate) descender: i16,
    /// `hlgp`: the typographic line gap.
    pub(crate) line_gap: i16,
}

/// An `MVAR` table.
#[derive(Clone, Debug)]
pub(crate) struct Mvar<'a> {
    store: ItemStore<'a>,
    /// Each value record: its tag and item.
    values: Vec<(Tag, usize, usize)>,
}

impl<'a> Mvar<'a> {
    /// Parses an `MVAR` table of a font of `axis_count` axes.
    pub(crate) fn parse(data: &'a [u8], axis_count: usize) -> Result<Self> {
        let mut r = Reader::new(data, MVAR);
        if r.u16()? != 1 {
            return Err(FontError::malformed(MVAR, "unknown MVAR version"));
        }
        r.skip(6)?;
        let count = usize::from(r.u16()?);
        let store_at = usize::from(r.u16()?);
        let store = ItemStore::parse(data, store_at, axis_count, MVAR)?;
        let values = (0..count)
            .map(|_| {
                let (tag, outer, inner) = (r.tag()?, usize::from(r.u16()?), usize::from(r.u16()?));
                if !store.has_item(outer, inner) {
                    return Err(FontError::malformed(MVAR, "MVAR item out of range"));
                }
                Ok((tag, outer, inner))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { store, values })
    }

    /// The delta of the value tagged `tag` at `coords` (as a 16-bit value,
    /// as FreeType keeps it), if the table varies it.
    pub(crate) fn delta(&self, tag: Tag, coords: &[i64]) -> Option<i16> {
        self.values
            .iter()
            .find(|(value, _, _)| *value == tag)
            .map(|&(_, outer, inner)| self.store.delta(outer, inner, coords) as i16)
    }

    /// The deltas FreeType applies to the face's line metrics.
    pub(crate) fn line_metrics(&self, coords: &[i64]) -> LineMetricDeltas {
        let delta = |tag: &[u8; 4]| self.delta(Tag::new(tag), coords).unwrap_or(0);
        LineMetricDeltas {
            ascender: delta(b"hasc"),
            descender: delta(b"hdsc"),
            line_gap: delta(b"hlgp"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An item variation store of one axis and region (peak 1), one data
    /// of `items` items with one byte delta each.
    fn store(items: &[i8]) -> Vec<u8> {
        let mut data = vec![0, 1, 0, 0, 0, 12, 0, 1, 0, 0, 0, 22];
        data.extend([0, 1, 0, 1, 0, 0, 0x40, 0, 0x40, 0]);
        data.extend([0, items.len() as u8, 0, 0, 0, 1, 0, 0]);
        data.extend(items.iter().map(|&delta| delta as u8));
        data
    }

    #[test]
    fn advance_and_metric_deltas() {
        let mut hvar = vec![0, 1, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0];
        hvar.extend(store(&[10, -20]));
        let hvar = Hvar::parse(&hvar, 1).unwrap();
        assert_eq!(hvar.advance_delta(GlyphId(1), &[0x10000]), -20);
        assert_eq!(hvar.advance_delta(GlyphId(0), &[0x8000]), 5);
        assert_eq!(hvar.advance_delta(GlyphId(0), &[0]), 0);
        let mut mvar = vec![0, 1, 0, 0, 0, 0, 0, 8, 0, 1, 0, 20];
        mvar.extend(b"hasc");
        mvar.extend([0, 0, 0, 0]);
        mvar.extend(store(&[15]));
        let mvar = Mvar::parse(&mvar, 1).unwrap();
        let deltas = mvar.line_metrics(&[0x10000]);
        assert_eq!((deltas.ascender, deltas.descender), (15, 0));
        assert_eq!(mvar.delta(Tag::new(b"xhgt"), &[0x10000]), None);
    }
}
