//! `avar`: the remapping of normalized coordinates (`ft_var_load_avar`
//! and the second stage of `ft_var_to_normalized`), version 1's segment
//! maps and version 2's item variation store.

use super::store::{IndexMap, ItemStore, f2dot14_to_fixed};
use crate::error::{FontError, Result};
use crate::fixed::mul_div;
use crate::reader::Reader;
use crate::types::Tag;

const AVAR: Tag = Tag::new(b"avar");

/// An `avar` table.
#[derive(Clone, Debug)]
pub(crate) struct Avar<'a> {
    /// Each axis' segment map: `(from, to)` pairs in 16.16.
    segments: Vec<Vec<(i64, i64)>>,
    /// Version 2's store and axis map.
    store: Option<(ItemStore<'a>, IndexMap)>,
}

impl<'a> Avar<'a> {
    /// Parses an `avar` table of a font of `axis_count` axes.
    pub(crate) fn parse(data: &'a [u8], axis_count: usize) -> Result<Self> {
        let mut r = Reader::new(data, AVAR);
        let version = r.u32()?;
        if version != 0x0001_0000 && version != 0x0002_0000 {
            return Err(FontError::malformed(AVAR, "unknown avar version"));
        }
        if r.u32()? as usize != axis_count {
            return Err(FontError::malformed(AVAR, "avar of other axes than fvar"));
        }
        let segments = (0..axis_count)
            .map(|_| {
                let count = usize::from(r.u16()?);
                if count * 4 > data.len() {
                    return Err(FontError::malformed(AVAR, "too many avar pairs"));
                }
                (0..count)
                    .map(|_| Ok((f2dot14_to_fixed(r.i16()?), f2dot14_to_fixed(r.i16()?))))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let store = if version == 0x0002_0000 {
            let axis_map = r.u32()? as usize;
            let store_at = r.u32()? as usize;
            if store_at == 0 {
                None
            } else {
                let store = ItemStore::parse(data, store_at, axis_count, AVAR)?;
                let map = if axis_map == 0 {
                    IndexMap::default()
                } else {
                    IndexMap::parse(data, axis_map, &store, AVAR)?
                };
                Some((store, map))
            }
        } else {
            None
        };
        Ok(Self { segments, store })
    }

    /// Remaps the normalized coordinates `coords` (16.16), as FreeType
    /// does after normalizing them by the axes' ranges.
    pub(crate) fn apply(&self, coords: &mut [i64]) {
        for (coord, map) in coords.iter_mut().zip(&self.segments) {
            for pair in map.windows(2) {
                let [(from0, to0), (from1, to1)] = [pair[0], pair[1]];
                if *coord < from1 {
                    *coord = mul_div(*coord - from0, to1 - to0, from1 - from0) + to0;
                    break;
                }
            }
        }
        let Some((store, map)) = &self.store else {
            return;
        };
        let half: Vec<i64> = coords.to_vec();
        for (index, coord) in coords.iter_mut().enumerate() {
            let (outer, inner) = if map.is_empty() {
                (0, index)
            } else {
                map.item(index)
            };
            let delta = store.delta(outer, inner, &half);
            *coord = (half[index] + delta * 4).clamp(-0x10000, 0x10000);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_maps() {
        // One axis: -1 -> -1, 0 -> 0, 0.5 -> 0.75, 1 -> 1.
        let mut data = vec![0, 1, 0, 0, 0, 0, 0, 1, 0, 4];
        for (from, to) in [
            (-16384i16, -16384i16),
            (0, 0),
            (8192, 12288),
            (16384, 16384),
        ] {
            data.extend(from.to_be_bytes());
            data.extend(to.to_be_bytes());
        }
        let avar = Avar::parse(&data, 1).unwrap();
        let mut coords = [0x4000];
        avar.apply(&mut coords);
        assert_eq!(coords, [0x6000]);
        let mut coords = [0xC000];
        avar.apply(&mut coords);
        assert_eq!(coords, [0xE000]);
        let mut coords = [0x10000];
        avar.apply(&mut coords);
        assert_eq!(coords, [0x10000]);
        assert!(Avar::parse(&data, 2).is_err());
        assert!(Avar::parse(&data[..12], 1).is_err());
    }
}
