//! The variation store of a CFF2 table (`cff_vstore_load`) and the blend
//! vectors `blend` weighs its operands with (`cff_blend_build_vector`).

use super::bytes::CffBytes;
use super::index::CFF;
use crate::error::{FontError, Result};
use crate::psaux::CharstringError;
use crate::var::{AxisRange, axis_range, region_scalar};

/// A CFF2 variation store: the regions and, for each item variation
/// data, the regions its deltas are for.
#[derive(Clone, Debug, Default)]
pub(super) struct VariationStore {
    axis_count: usize,
    regions: Vec<Vec<AxisRange>>,
    data: Vec<Vec<u16>>,
}

impl VariationStore {
    /// The variation store at `offset` of the table (after its length).
    pub(super) fn parse(table: CffBytes<'_>, offset: usize) -> Result<Self> {
        let mut r = table.at(offset)?;
        r.skip(2)?;
        let start = offset + 2;
        let format = r.u16()?;
        if format != 1 {
            return Err(FontError::UnsupportedFormat {
                table: CFF,
                format: u32::from(format),
            });
        }
        let region_list = r.u32()? as usize;
        let data_offsets = (0..r.u16()?)
            .map(|_| r.u32().map(|at| at as usize))
            .collect::<Result<Vec<_>>>()?;
        let mut r = table.at(start + region_list)?;
        let axis_count = usize::from(r.u16()?);
        let regions = (0..r.u16()?)
            .map(|_| {
                (0..axis_count)
                    .map(|_| Ok(axis_range(r.f2dot14()?, r.f2dot14()?, r.f2dot14()?)))
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let data = data_offsets
            .into_iter()
            .map(|at| {
                let mut r = table.at(start + at)?;
                // CFF2 has no delta sets: the item count and the count of
                // short deltas are skipped.
                r.skip(4)?;
                let count = usize::from(r.u16()?);
                (0..count).map(|_| r.u16()).collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            axis_count,
            regions,
            data,
        })
    }

    /// Whether the store has item variation data (FreeType's test of a
    /// variable CFF2 font).
    pub(super) fn has_data(&self) -> bool {
        !self.data.is_empty()
    }

    /// How many regions item variation data `vsindex` blends (the deltas a
    /// `blend` reads per value).
    pub(super) fn region_count(&self, vsindex: u32) -> Option<usize> {
        self.data.get(usize::try_from(vsindex).ok()?).map(Vec::len)
    }

    /// The blend vector of item variation data `vsindex` at the normalized
    /// coordinates `coords` (16.16, none for the default instance): one for
    /// the default, then each region's scalar.
    pub(super) fn blend_vector(
        &self,
        vsindex: u32,
        coords: &[i64],
    ) -> std::result::Result<Vec<i32>, CharstringError> {
        let invalid = CharstringError::Malformed("invalid blend");
        if !coords.is_empty() && coords.len() != self.axis_count {
            return Err(invalid);
        }
        let data = usize::try_from(vsindex)
            .ok()
            .and_then(|index| self.data.get(index))
            .ok_or(invalid.clone())?;
        let mut vector = Vec::with_capacity(data.len() + 1);
        vector.push(1 << 16);
        for &index in data {
            let region = self
                .regions
                .get(usize::from(index))
                .ok_or(invalid.clone())?;
            // `cff_blend_build_vector` weighs as `tt_calculate_scalar`.
            vector.push(if coords.is_empty() {
                0
            } else {
                region_scalar(region, coords) as i32
            });
        }
        Ok(vector)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store of one axis with two regions (peaks at 1 and -1) and one
    /// item variation data using both.
    fn store() -> Vec<u8> {
        let mut data = vec![0, 0, 0, 1, 0, 0, 0, 12, 0, 1, 0, 0, 0, 28];
        // Region list at 12 (after the length): 1 axis, 2 regions.
        data.extend([0, 1, 0, 2, 0, 0, 0x40, 0, 0x40, 0, 0xC0, 0, 0xC0, 0, 0, 0]);
        // Item variation data at 28: item count, short deltas, 2 regions.
        data.extend([0, 0, 0, 0, 0, 2, 0, 0, 0, 1]);
        data
    }

    #[test]
    fn blend_vectors() {
        let data = store();
        let vstore = VariationStore::parse(CffBytes::Whole(&data), 0).unwrap();
        assert!(vstore.has_data());
        assert_eq!(vstore.blend_vector(0, &[]), Ok(vec![0x10000, 0, 0]));
        assert_eq!(vstore.blend_vector(0, &[0]), Ok(vec![0x10000, 0, 0]));
        assert_eq!(
            vstore.blend_vector(0, &[0x8000]),
            Ok(vec![0x10000, 0x8000, 0])
        );
        assert_eq!(
            vstore.blend_vector(0, &[-0x10000]),
            Ok(vec![0x10000, 0, 0x10000])
        );
        assert!(vstore.blend_vector(1, &[]).is_err());
        assert!(vstore.blend_vector(0, &[0, 0]).is_err());
        assert!(!VariationStore::default().has_data());
        for len in 0..data.len() {
            assert!(VariationStore::parse(CffBytes::Whole(&data[..len]), 0).is_err());
        }
    }
}
