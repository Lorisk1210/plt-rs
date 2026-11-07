//! CFF and CFF2 INDEX structures.

use std::ops::Range;

use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::region::Region;
use crate::types::Tag;

pub(crate) const CFF: Tag = Tag::new(b"CFF ");

/// The width of an INDEX's count: 16 bits in CFF, 32 in CFF2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CountSize {
    Short,
    Long,
}

/// An INDEX: a counted list of variable-length byte strings.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Index<'a> {
    count: usize,
    offset_size: u8,
    offsets: &'a [u8],
    data: Region<'a>,
}

impl<'a> Index<'a> {
    /// Reads a CFF INDEX at the reader position, leaving the reader after
    /// it.
    pub(crate) fn read(r: &mut Reader<'a>) -> Result<Self> {
        Self::read_sized(r, CountSize::Short)
    }

    /// Reads a CFF or CFF2 INDEX at the reader position, leaving the
    /// reader after it.
    pub(crate) fn read_sized(r: &mut Reader<'a>, count: CountSize) -> Result<Self> {
        let Some(mut index) = Self::read_head(r, count)? else {
            return Ok(Self::default());
        };
        index.data = Region::Held(r.bytes(index.data_len()?)?);
        Ok(index)
    }

    /// Reads the count, offset size and offsets of an INDEX at the reader
    /// position, whose data are `data`, held elsewhere.
    pub(crate) fn read_offsets(
        r: &mut Reader<'a>,
        data: Region<'a>,
        count: CountSize,
    ) -> Result<Self> {
        let Some(index) = Self::read_head(r, count)? else {
            return Ok(Self::default());
        };
        if index.data_len()? != data.len() {
            return Err(FontError::malformed(CFF, "INDEX data elsewhere"));
        }
        Ok(Self { data, ..index })
    }

    /// Where the data of the INDEX at the start of `bytes` lie in them,
    /// when they hold its count, offset size and offsets (but not
    /// necessarily its data): what a file held for drawing leaves to read
    /// entry by entry.
    pub(crate) fn data_range(bytes: &'a [u8], count: CountSize) -> Result<Range<usize>> {
        let mut r = Reader::new(bytes, CFF);
        let head = Self::read_head(&mut r, count)?;
        let start = r.pos();
        Ok(start..start + head.map_or(Ok(0), |index| index.data_len())?)
    }

    /// Reads the count, offset size and offsets at the reader position:
    /// the INDEX without its data, `None` when it is empty.
    fn read_head(r: &mut Reader<'a>, count: CountSize) -> Result<Option<Self>> {
        let count = match count {
            CountSize::Short => usize::from(r.u16()?),
            CountSize::Long => r.u32()? as usize,
        };
        if count == 0 {
            return Ok(None);
        }
        let offset_size = r.u8()?;
        if !(1..=4).contains(&offset_size) {
            return Err(FontError::malformed(CFF, "INDEX offset size outside 1..=4"));
        }
        let length = count
            .checked_add(1)
            .and_then(|entries| entries.checked_mul(usize::from(offset_size)))
            .ok_or(FontError::Truncated(CFF))?;
        Ok(Some(Self {
            count,
            offset_size,
            offsets: r.bytes(length)?,
            data: Region::default(),
        }))
    }

    /// The length of the data the offsets describe.
    fn data_len(&self) -> Result<usize> {
        let last = self.offset(self.count).ok_or(FontError::Truncated(CFF))?;
        // Offsets are 1-based relative to the byte before the data.
        last.checked_sub(1)
            .ok_or(FontError::malformed(CFF, "INDEX offset zero"))
    }

    fn offset(&self, index: usize) -> Option<usize> {
        let size = usize::from(self.offset_size);
        let bytes = self.offsets.get(index * size..(index + 1) * size)?;
        Some(
            bytes
                .iter()
                .fold(0usize, |acc, &b| acc << 8 | usize::from(b)),
        )
    }

    /// Number of entries.
    pub(crate) const fn len(&self) -> usize {
        self.count
    }

    /// Entry `index`, or `None` when out of range or malformed.
    pub(crate) fn get(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.count {
            return None;
        }
        let start = self.offset(index)?.checked_sub(1)?;
        let end = self.offset(index + 1)?.checked_sub(1)?;
        self.data.get(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_entries() {
        let data = [0, 3, 1, 1, 2, 2, 5, b'a', b'b', b'c', b'd', 9];
        let mut r = Reader::new(&data, CFF);
        let index = Index::read(&mut r).unwrap();
        assert_eq!(index.len(), 3);
        assert_eq!(index.get(0), Some(&b"a"[..]));
        assert_eq!(index.get(1), Some(&b""[..]));
        assert_eq!(index.get(2), Some(&b"bcd"[..]));
        assert_eq!(index.get(3), None);
        assert_eq!(r.u8().unwrap(), 9);
        let mut empty = Reader::new(&[0, 0], CFF);
        assert_eq!(Index::read(&mut empty).unwrap().len(), 0);
        let long = [0, 0, 0, 1, 1, 1, 2, b'x'];
        let index = Index::read_sized(&mut Reader::new(&long, CFF), CountSize::Long).unwrap();
        assert_eq!(index.get(0), Some(&b"x"[..]));
    }

    #[test]
    fn malformed_indexes() {
        assert!(Index::read(&mut Reader::new(&[0, 1, 5, 1, 2], CFF)).is_err());
        assert!(Index::read(&mut Reader::new(&[0, 1, 1, 1, 9, b'a'], CFF)).is_err());
        assert!(Index::read(&mut Reader::new(&[0, 1, 1, 0, 0], CFF)).is_err());
    }
}
