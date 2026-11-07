//! Item variation stores and delta-set index maps (`HVAR`, `MVAR`, `avar`
//! version 2), read and evaluated as FreeType does
//! (`tt_var_load_item_variation_store`, `tt_var_get_item_delta`).

use crate::error::{FontError, Result};
use crate::fixed::mul_div;
use crate::reader::Reader;
use crate::types::Tag;

/// One axis of a region: start, peak and end in 16.16; a peak of zero
/// marks an axis the region ignores (FreeType tags invalid ranges so).
pub(crate) type AxisRange = [i64; 3];

/// `FT_fdot14ToFixed`.
pub(crate) fn f2dot14_to_fixed(value: i16) -> i64 {
    i64::from(value) << 2
}

/// A region's axis range from three `F2Dot14` values, tagged as FreeType
/// tags invalid ones.
pub(crate) fn axis_range(start: i16, peak: i16, end: i16) -> AxisRange {
    let invalid = (start < 0 && end > 0) || start > peak || peak > end;
    let peak = if invalid { 0 } else { peak };
    [start, peak, end].map(f2dot14_to_fixed)
}

/// `tt_calculate_scalar`: the weight of a region at the normalized
/// coordinates `coords`.
pub(crate) fn region_scalar(region: &[AxisRange], coords: &[i64]) -> i64 {
    let mut scalar: i64 = 0x10000;
    for (&[start, peak, end], &coord) in region.iter().zip(coords) {
        if peak == coord || peak == 0 {
            continue;
        }
        if coord <= start || coord >= end {
            return 0;
        }
        scalar = if coord < peak {
            mul_div(scalar, coord - start, peak - start)
        } else {
            mul_div(scalar, end - coord, end - peak)
        };
    }
    scalar
}

/// The deltas of one item variation data.
#[derive(Clone, Debug)]
struct ItemData<'a> {
    item_count: usize,
    word_count: usize,
    long_words: bool,
    regions: Vec<usize>,
    deltas: &'a [u8],
}

/// An item variation store.
#[derive(Clone, Debug)]
pub(crate) struct ItemStore<'a> {
    regions: Vec<Vec<AxisRange>>,
    data: Vec<ItemData<'a>>,
}

impl<'a> ItemStore<'a> {
    /// The store at `offset` of `table` (tagged `tag`), for a font of
    /// `axis_count` axes.
    pub(crate) fn parse(
        table: &'a [u8],
        offset: usize,
        axis_count: usize,
        tag: Tag,
    ) -> Result<Self> {
        let invalid = |why| FontError::malformed(tag, why);
        let mut r = Reader::at(table, offset, tag)?;
        if r.u16()? != 1 {
            return Err(invalid("unknown item variation store format"));
        }
        let region_list = r.u32()? as usize;
        let data_count = usize::from(r.u16()?);
        if data_count == 0 {
            return Err(invalid("item variation store without data"));
        }
        let data_offsets = (0..data_count)
            .map(|_| r.u32().map(|at| at as usize))
            .collect::<Result<Vec<_>>>()?;
        let mut r = Reader::at(table, offset.saturating_add(region_list), tag)?;
        let axes = usize::from(r.u16()?);
        let region_count = usize::from(r.u16()?);
        if axes != axis_count {
            return Err(invalid("item variation store of other axes than fvar"));
        }
        if region_count >= 32768 {
            return Err(invalid("too many variation regions"));
        }
        let regions = (0..region_count)
            .map(|_| {
                (0..axes)
                    .map(|_| Ok(axis_range(r.i16()?, r.i16()?, r.i16()?)))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let data = data_offsets
            .into_iter()
            .map(|at| {
                let mut r = Reader::at(table, offset.saturating_add(at), tag)?;
                let item_count = usize::from(r.u16()?);
                let word_field = r.u16()?;
                let region_count = usize::from(r.u16()?);
                let long_words = word_field & 0x8000 != 0;
                let word_count = usize::from(word_field & 0x7FFF);
                if word_count > region_count || region_count > regions.len() {
                    return Err(invalid("item variation data of bad region counts"));
                }
                let indices = (0..region_count)
                    .map(|_| {
                        let index = usize::from(r.u16()?);
                        if index >= regions.len() {
                            return Err(invalid("region index out of range"));
                        }
                        Ok(index)
                    })
                    .collect::<Result<Vec<_>>>()?;
                let row = (word_count + region_count) * if long_words { 2 } else { 1 };
                let deltas = r.bytes(item_count * row)?;
                Ok(ItemData {
                    item_count,
                    word_count,
                    long_words,
                    regions: indices,
                    deltas,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { regions, data })
    }

    /// Whether item `(outer, inner)` exists (`0xFFFF, 0xFFFF` stands for
    /// none and is accepted).
    pub(crate) fn has_item(&self, outer: usize, inner: usize) -> bool {
        (outer == 0xFFFF && inner == 0xFFFF)
            || self
                .data
                .get(outer)
                .is_some_and(|data| inner < data.item_count)
    }

    /// The number of item variation data.
    pub(crate) fn data_count(&self) -> usize {
        self.data.len()
    }

    /// The number of items of data `outer`.
    pub(crate) fn item_count(&self, outer: usize) -> usize {
        self.data.get(outer).map_or(0, |data| data.item_count)
    }

    /// `tt_var_get_item_delta`: the delta of item `(outer, inner)` at the
    /// normalized coordinates `coords`, rounded; zero for items out of
    /// range.
    pub(crate) fn delta(&self, outer: usize, inner: usize, coords: &[i64]) -> i64 {
        let Some(data) = self.data.get(outer).filter(|data| inner < data.item_count) else {
            return 0;
        };
        let size = if data.long_words { 2 } else { 1 };
        let row = (data.word_count + data.regions.len()) * size;
        let mut r = Reader::new(
            &data.deltas[row * inner..row * (inner + 1)],
            Tag::new(b"VARC"),
        );
        let mut sum: i64 = 0;
        for (master, &region) in data.regions.iter().enumerate() {
            let word = master < data.word_count;
            let delta = match (data.long_words, word) {
                (true, true) => r.i32().map(i64::from),
                (true, false) | (false, true) => r.i16().map(i64::from),
                (false, false) => r.u8().map(|byte| i64::from(byte as i8)),
            }
            .unwrap_or(0);
            let scalar = region_scalar(&self.regions[region], coords);
            sum += delta * scalar;
        }
        (sum + 0x8000) >> 16
    }
}

/// A delta-set index map: glyph (or axis) to item.
#[derive(Clone, Debug, Default)]
pub(crate) struct IndexMap {
    entries: Vec<(usize, usize)>,
}

impl IndexMap {
    /// The map at `offset` of `table`, whose items are in `store`.
    pub(crate) fn parse(
        table: &[u8],
        offset: usize,
        store: &ItemStore<'_>,
        tag: Tag,
    ) -> Result<Self> {
        let invalid = |why| FontError::malformed(tag, why);
        let mut r = Reader::at(table, offset, tag)?;
        let format = r.u8()?;
        let entry_format = r.u8()?;
        let count = match format {
            0 => usize::from(r.u16()?),
            1 => r.u32()? as usize,
            _ => return Err(invalid("unknown delta-set index map format")),
        };
        if entry_format & 0xC0 != 0 {
            return Err(invalid("bad delta-set index map entry format"));
        }
        let size = usize::from((entry_format & 0x30) >> 4) + 1;
        let inner_bits = u32::from(entry_format & 0x0F) + 1;
        if count.saturating_mul(size) > table.len() {
            return Err(invalid("too many delta-set index map entries"));
        }
        let entries = (0..count)
            .map(|_| {
                let bytes = r.bytes(size)?;
                let value = bytes.iter().fold(0u32, |acc, &b| acc << 8 | u32::from(b));
                if value == 0xFFFF_FFFF {
                    return Ok((0xFFFF, 0xFFFF));
                }
                let outer = (value >> inner_bits) as usize;
                let inner = (value & ((1 << inner_bits) - 1)) as usize;
                if outer >= store.data_count() || inner >= store.item_count(outer) {
                    return Err(invalid("delta-set index out of range"));
                }
                Ok((outer, inner))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { entries })
    }

    /// The item of `index`: the map's last entry for indices past it, and
    /// `(0, index)` without a map.
    pub(crate) fn item(&self, index: usize) -> (usize, usize) {
        if self.entries.is_empty() {
            return (0, index);
        }
        self.entries[index.min(self.entries.len() - 1)]
    }

    /// Whether the map has entries.
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: Tag = Tag::new(b"HVAR");

    /// A store of one axis, two regions (peaks 1 and -1), one data of two
    /// items with one word delta and one byte delta each.
    fn store() -> Vec<u8> {
        let mut data = vec![0, 1, 0, 0, 0, 12, 0, 1, 0, 0, 0, 28];
        data.extend([0, 1, 0, 2, 0, 0, 0x40, 0, 0x40, 0, 0xC0, 0, 0xC0, 0, 0, 0]);
        data.extend([0, 2, 0, 1, 0, 2, 0, 0, 0, 1]);
        data.extend([0x01, 0x00, 0xF6, 0xFF, 0x9C, 20]);
        data
    }

    #[test]
    fn deltas_at_coordinates() {
        let data = store();
        let store = ItemStore::parse(&data, 0, 1, TAG).unwrap();
        assert_eq!(store.delta(0, 0, &[0]), 0);
        assert_eq!(store.delta(0, 0, &[0x10000]), 256);
        assert_eq!(store.delta(0, 0, &[0x8000]), 128);
        assert_eq!(store.delta(0, 1, &[-0x10000]), 20);
        assert_eq!(store.delta(0, 1, &[0x10000]), -100);
        assert_eq!(store.delta(0, 2, &[0x10000]), 0);
        assert_eq!(store.delta(1, 0, &[0x10000]), 0);
        assert!(store.has_item(0, 1) && store.has_item(0xFFFF, 0xFFFF) && !store.has_item(0, 2));
        assert!(ItemStore::parse(&data, 0, 2, TAG).is_err());
        for len in 0..data.len() {
            assert!(ItemStore::parse(&data[..len], 0, 1, TAG).is_err());
        }
        // A map of 1-byte entries, one inner bit: items (0, 1) and (0, 0).
        let map_data = [0, 0x00, 0, 2, 1, 0];
        let map = IndexMap::parse(&map_data, 0, &store, TAG).unwrap();
        assert_eq!((map.item(0), map.item(5)), ((0, 1), (0, 0)));
        assert_eq!(IndexMap::default().item(7), (0, 7));
        assert!(IndexMap::parse(&[0, 0, 0, 1, 4], 0, &store, TAG).is_err());
    }

    #[test]
    fn region_scalars() {
        let region = [axis_range(0, 0x4000, 0x4000)];
        assert_eq!(region_scalar(&region, &[0x8000]), 0x8000);
        assert_eq!(region_scalar(&region, &[0]), 0);
        // An invalid range ignores its axis.
        let ignored = [axis_range(-0x4000, 0x2000, 0x4000)];
        assert_eq!(region_scalar(&ignored, &[0x1234]), 0x10000);
    }
}
