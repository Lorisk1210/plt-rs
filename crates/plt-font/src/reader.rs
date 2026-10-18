//! Bounds-checked big-endian reading primitives.
//!
//! Every multi-byte read goes through [`Reader`] or [`Array`], which never
//! index out of bounds: running off the end of the data yields
//! [`FontError::Truncated`] (tagged with the table being read) or `None`.

use std::cmp::Ordering;
use std::fmt;
use std::marker::PhantomData;

use crate::error::{FontError, Result};
use crate::types::{GlyphId, Tag};

/// A fixed-size big-endian value that can be decoded from raw bytes.
pub(crate) trait FromBytes: Sized + Copy + 'static {
    /// Encoded size in bytes.
    const SIZE: usize;

    /// Decodes the value; `bytes` is exactly [`Self::SIZE`] bytes long.
    fn from_bytes(bytes: &[u8]) -> Self;
}

impl FromBytes for u8 {
    const SIZE: usize = 1;
    fn from_bytes(bytes: &[u8]) -> Self {
        bytes[0]
    }
}

impl FromBytes for u16 {
    const SIZE: usize = 2;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1]])
    }
}

impl FromBytes for i16 {
    const SIZE: usize = 2;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1]])
    }
}

impl FromBytes for u32 {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl FromBytes for i32 {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl FromBytes for GlyphId {
    const SIZE: usize = 2;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self(u16::from_bytes(bytes))
    }
}

impl FromBytes for Tag {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

/// Returns `data[offset..offset + len]`, or a truncation error for `tag`.
pub(crate) fn slice(data: &[u8], offset: usize, len: usize, tag: Tag) -> Result<&[u8]> {
    offset
        .checked_add(len)
        .and_then(|end| data.get(offset..end))
        .ok_or(FontError::Truncated(tag))
}

/// Returns `data[offset..]`, or a truncation error for `tag`.
pub(crate) fn tail(data: &[u8], offset: usize, tag: Tag) -> Result<&[u8]> {
    data.get(offset..).ok_or(FontError::Truncated(tag))
}

/// Reads a value at an absolute offset, returning `None` when out of bounds.
pub(crate) fn peek<T: FromBytes>(data: &[u8], offset: usize) -> Option<T> {
    let end = offset.checked_add(T::SIZE)?;
    data.get(offset..end).map(T::from_bytes)
}

/// A cursor reading big-endian values from a byte slice.
#[derive(Clone, Copy)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    tag: Tag,
}

impl fmt::Debug for Reader<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Reader")
            .field("len", &self.data.len())
            .field("pos", &self.pos)
            .field("tag", &self.tag)
            .finish()
    }
}

impl<'a> Reader<'a> {
    /// Starts reading `data` (belonging to table `tag`) at offset zero.
    pub(crate) const fn new(data: &'a [u8], tag: Tag) -> Self {
        Self { data, pos: 0, tag }
    }

    /// Starts reading `data` at `offset`.
    pub(crate) fn at(data: &'a [u8], offset: usize, tag: Tag) -> Result<Self> {
        if offset > data.len() {
            return Err(FontError::Truncated(tag));
        }
        Ok(Self {
            data,
            pos: offset,
            tag,
        })
    }

    /// The error reported when the data runs out.
    pub(crate) const fn truncated(&self) -> FontError {
        FontError::Truncated(self.tag)
    }

    /// Current position relative to the start of the data.
    pub(crate) const fn pos(&self) -> usize {
        self.pos
    }

    /// Number of unread bytes.
    pub(crate) const fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Moves forward by `count` bytes.
    pub(crate) fn skip(&mut self, count: usize) -> Result<()> {
        self.bytes(count).map(|_| ())
    }

    /// Reads `count` raw bytes.
    pub(crate) fn bytes(&mut self, count: usize) -> Result<&'a [u8]> {
        let bytes = slice(self.data, self.pos, count, self.tag)?;
        self.pos += count;
        Ok(bytes)
    }

    /// Reads one fixed-size value.
    pub(crate) fn read<T: FromBytes>(&mut self) -> Result<T> {
        self.bytes(T::SIZE).map(T::from_bytes)
    }

    /// Reads an unsigned byte.
    pub(crate) fn u8(&mut self) -> Result<u8> {
        self.read()
    }

    /// Reads a big-endian `u16`.
    pub(crate) fn u16(&mut self) -> Result<u16> {
        self.read()
    }

    /// Reads a big-endian `i16`.
    pub(crate) fn i16(&mut self) -> Result<i16> {
        self.read()
    }

    /// Reads a big-endian `u32`.
    pub(crate) fn u32(&mut self) -> Result<u32> {
        self.read()
    }

    /// Reads a big-endian `i32`.
    pub(crate) fn i32(&mut self) -> Result<i32> {
        self.read()
    }

    /// Reads a 16.16 fixed-point number.
    pub(crate) fn fixed(&mut self) -> Result<f64> {
        self.i32().map(|raw| f64::from(raw) / 65536.0)
    }

    /// Reads a 2.14 fixed-point number as its raw value.
    pub(crate) fn f2dot14(&mut self) -> Result<i16> {
        self.i16()
    }

    /// Reads a glyph id.
    pub(crate) fn glyph(&mut self) -> Result<GlyphId> {
        self.read()
    }

    /// Reads a table tag.
    pub(crate) fn tag(&mut self) -> Result<Tag> {
        self.read()
    }

    /// Reads an array of `count` fixed-size records.
    pub(crate) fn array<T: FromBytes>(&mut self, count: usize) -> Result<Array<'a, T>> {
        let len = count.checked_mul(T::SIZE).ok_or_else(|| self.truncated())?;
        self.bytes(len).map(Array::new)
    }
}

/// A lazily decoded array of fixed-size big-endian records.
#[derive(Clone, Copy)]
pub(crate) struct Array<'a, T> {
    data: &'a [u8],
    marker: PhantomData<T>,
}

impl<T> fmt::Debug for Array<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Array({} bytes)", self.data.len())
    }
}

impl<T> Default for Array<'_, T> {
    fn default() -> Self {
        Self {
            data: &[],
            marker: PhantomData,
        }
    }
}

impl<'a, T: FromBytes> Array<'a, T> {
    /// Wraps raw bytes; a trailing partial record is ignored.
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            marker: PhantomData,
        }
    }

    /// Number of complete records.
    pub(crate) const fn len(&self) -> usize {
        self.data.len() / T::SIZE
    }

    /// Whether the array has no complete record.
    pub(crate) const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Decodes record `index`.
    pub(crate) fn get(&self, index: usize) -> Option<T> {
        if index >= self.len() {
            return None;
        }
        peek(self.data, index * T::SIZE)
    }

    /// The array without its first `count` records.
    pub(crate) fn skip(&self, count: usize) -> Self {
        Self::new(
            self.data
                .get(count.saturating_mul(T::SIZE)..)
                .unwrap_or(&[]),
        )
    }

    /// Decodes the last record.
    pub(crate) fn last(&self) -> Option<T> {
        self.len().checked_sub(1).and_then(|index| self.get(index))
    }

    /// Iterates over all records.
    pub(crate) fn iter(&self) -> impl Iterator<Item = T> + use<'a, T> {
        let data = self.data;
        data.chunks_exact(T::SIZE).map(T::from_bytes)
    }

    /// Binary search over records sorted by `compare`.
    pub(crate) fn binary_search_by(
        &self,
        mut compare: impl FnMut(&T) -> Ordering,
    ) -> Option<(usize, T)> {
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let mid = low + (high - low) / 2;
            let value = self.get(mid)?;
            match compare(&value) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => return Some((mid, value)),
            }
        }
        None
    }

    /// Binary search probing like HarfBuzz's `hb_bsearch` (midpoint of an
    /// inclusive range), which decides the match among duplicate keys.
    pub(crate) fn harfbuzz_search_by(
        &self,
        mut compare: impl FnMut(&T) -> Ordering,
    ) -> Option<(usize, T)> {
        let (mut low, mut high) = (0, self.len());
        while low < high {
            let mid = low + (high - 1 - low) / 2;
            let value = self.get(mid)?;
            match compare(&value) {
                Ordering::Less => low = mid + 1,
                Ordering::Greater => high = mid,
                Ordering::Equal => return Some((mid, value)),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAG: Tag = Tag::new(b"test");

    #[test]
    fn reads_big_endian_values() {
        let data = [
            0x12, 0x34, 0xFF, 0xFE, 0x00, 0x01, 0x00, 0x00, 0xAB, 0xCD, 0xEF,
        ];
        let mut reader = Reader::new(&data, TAG);
        assert_eq!(reader.u16().unwrap(), 0x1234);
        assert_eq!(reader.i16().unwrap(), -2);
        assert!((reader.fixed().unwrap() - 1.0).abs() < 1e-12);
        assert_eq!(reader.bytes(2).unwrap(), &[0xAB, 0xCD]);
        assert_eq!(reader.u8().unwrap(), 0xEF);
        assert_eq!(reader.remaining(), 0);
        assert_eq!(reader.u8(), Err(FontError::Truncated(TAG)));
    }

    #[test]
    fn truncation_is_an_error_not_a_panic() {
        let data = [1, 2, 3];
        let mut reader = Reader::new(&data, TAG);
        assert!(reader.u32().is_err());
        assert!(reader.array::<u16>(usize::MAX).is_err());
        assert!(Reader::at(&data, 4, TAG).is_err());
        assert!(slice(&data, usize::MAX, 2, TAG).is_err());
        assert_eq!(peek::<u16>(&data, 2), None);
    }

    #[test]
    fn arrays_decode_and_search() {
        let data = [0, 1, 0, 5, 0, 9, 7];
        let array: Array<'_, u16> = Array::new(&data);
        assert_eq!(array.len(), 3);
        assert_eq!(array.get(1), Some(5));
        assert_eq!(array.get(3), None);
        assert_eq!(array.last(), Some(9));
        assert_eq!(array.iter().collect::<Vec<_>>(), vec![1, 5, 9]);
        assert_eq!(array.binary_search_by(|v| v.cmp(&9)), Some((2, 9)));
        assert_eq!(array.binary_search_by(|v| v.cmp(&4)), None);
        assert_eq!(array.harfbuzz_search_by(|v| v.cmp(&9)), Some((2, 9)));
        assert_eq!(array.harfbuzz_search_by(|v| v.cmp(&4)), None);
        assert_eq!(Array::<u32>::default().len(), 0);
        // Duplicates: HarfBuzz probes (0 + 3) / 2 first.
        let duplicates = [0, 7, 0, 7, 0, 7, 0, 7];
        let array: Array<'_, u16> = Array::new(&duplicates);
        assert_eq!(array.harfbuzz_search_by(|v| v.cmp(&7)), Some((1, 7)));
        assert_eq!(array.binary_search_by(|v| v.cmp(&7)), Some((2, 7)));
    }
}
