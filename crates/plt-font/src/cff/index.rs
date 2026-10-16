//! CFF INDEX structures and DICT operand decoding.

use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::types::Tag;

pub(crate) const CFF: Tag = Tag::new(b"CFF ");

/// An INDEX: a counted list of variable-length byte strings.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Index<'a> {
    count: usize,
    offset_size: u8,
    offsets: &'a [u8],
    data: &'a [u8],
}

impl<'a> Index<'a> {
    /// Reads an INDEX at the reader position, leaving the reader after it.
    pub(crate) fn read(r: &mut Reader<'a>) -> Result<Self> {
        let count = usize::from(r.u16()?);
        if count == 0 {
            return Ok(Self::default());
        }
        let offset_size = r.u8()?;
        if !(1..=4).contains(&offset_size) {
            return Err(FontError::malformed(CFF, "INDEX offset size outside 1..=4"));
        }
        let offsets = r.bytes((count + 1) * usize::from(offset_size))?;
        let mut index = Self {
            count,
            offset_size,
            offsets,
            data: &[],
        };
        let last = index.offset(count).ok_or(FontError::Truncated(CFF))?;
        // Offsets are 1-based relative to the byte before the data.
        index.data = r.bytes(
            last.checked_sub(1)
                .ok_or(FontError::malformed(CFF, "INDEX offset zero"))?,
        )?;
        Ok(index)
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

/// A DICT operator: one byte, or `12 x` encoded as `1200 + x`.
pub(crate) type Operator = u16;

/// Decodes a DICT into `(operator, operands)` pairs.
pub(crate) fn parse_dict(data: &[u8]) -> Result<Vec<(Operator, Vec<f64>)>> {
    let mut r = Reader::new(data, CFF);
    let mut entries = Vec::new();
    let mut operands = Vec::new();
    while r.remaining() > 0 {
        let b0 = r.u8()?;
        match b0 {
            0..=21 => {
                let operator = if b0 == 12 {
                    1200 + u16::from(r.u8()?)
                } else {
                    u16::from(b0)
                };
                entries.push((operator, std::mem::take(&mut operands)));
            }
            28 => operands.push(f64::from(r.i16()?)),
            29 => operands.push(f64::from(r.i32()?)),
            30 => operands.push(read_real(&mut r)?),
            32..=246 => operands.push(f64::from(i32::from(b0) - 139)),
            247..=250 => operands.push(f64::from(
                (i32::from(b0) - 247) * 256 + i32::from(r.u8()?) + 108,
            )),
            251..=254 => operands.push(f64::from(
                -(i32::from(b0) - 251) * 256 - i32::from(r.u8()?) - 108,
            )),
            _ => return Err(FontError::malformed(CFF, "reserved DICT byte")),
        }
        if operands.len() > 48 {
            return Err(FontError::malformed(CFF, "too many DICT operands"));
        }
    }
    Ok(entries)
}

/// Reads a BCD-encoded real number (operator 30).
fn read_real(r: &mut Reader<'_>) -> Result<f64> {
    let mut text = String::new();
    loop {
        let byte = r.u8()?;
        for nibble in [byte >> 4, byte & 0x0F] {
            match nibble {
                0..=9 => text.push(char::from(b'0' + nibble)),
                0xA => text.push('.'),
                0xB => text.push('E'),
                0xC => text.push_str("E-"),
                0xE => text.push('-'),
                0xF => return Ok(text.parse().unwrap_or(0.0)),
                _ => return Err(FontError::malformed(CFF, "reserved real nibble")),
            }
        }
    }
}

/// Looks up the operands of `operator` in a decoded DICT.
pub(crate) fn dict_get(dict: &[(Operator, Vec<f64>)], operator: Operator) -> Option<&[f64]> {
    dict.iter()
        .find(|(op, _)| *op == operator)
        .map(|(_, operands)| operands.as_slice())
}

/// The single operand of `operator` as a non-negative offset or count.
pub(crate) fn dict_usize(dict: &[(Operator, Vec<f64>)], operator: Operator) -> Option<usize> {
    let value = *dict_get(dict, operator)?.first()?;
    (value >= 0.0 && value.fract() == 0.0).then_some(value as usize)
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
    }

    #[test]
    fn malformed_indexes() {
        assert!(Index::read(&mut Reader::new(&[0, 1, 5, 1, 2], CFF)).is_err());
        assert!(Index::read(&mut Reader::new(&[0, 1, 1, 1, 9, b'a'], CFF)).is_err());
        assert!(Index::read(&mut Reader::new(&[0, 1, 1, 0, 0], CFF)).is_err());
    }

    #[test]
    fn dict_operands() {
        // 100 (1-byte), 1000 (2-byte), -1000, 28-short, real -2.25, op 17,
        // then op 12 7.
        let data = [
            239, 250, 124, 254, 124, 28, 0x30, 0x39, 30, 0xE2, 0xA2, 0x5F, 17, 139, 12, 7,
        ];
        let dict = parse_dict(&data).unwrap();
        assert_eq!(dict[0].0, 17);
        assert_eq!(dict[0].1, vec![100.0, 1000.0, -1000.0, 12345.0, -2.25]);
        assert_eq!(dict_get(&dict, 1207), Some(&[0.0][..]));
        assert_eq!(dict_usize(&dict, 1207), Some(0));
        assert_eq!(dict_usize(&dict, 17), Some(100));
        assert!(parse_dict(&[255]).is_err());
        assert!(parse_dict(&[28, 1]).is_err());
    }
}
