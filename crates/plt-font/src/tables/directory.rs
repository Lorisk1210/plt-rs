//! The sfnt table directory, including TrueType/OpenType collections.

use crate::error::{DIRECTORY, FontError, Result};
use crate::reader::{Array, FromBytes, Reader};
use crate::types::Tag;

const TRUETYPE: u32 = 0x0001_0000;
const APPLE_TRUE: u32 = u32::from_be_bytes(*b"true");
const OPENTYPE_CFF: u32 = u32::from_be_bytes(*b"OTTO");
const COLLECTION: u32 = u32::from_be_bytes(*b"ttcf");

/// Outline flavour announced by the sfnt version field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SfntFlavor {
    /// `0x00010000` or `true`: TrueType outlines.
    TrueType,
    /// `OTTO`: CFF outlines.
    Cff,
}

#[derive(Clone, Copy, Debug)]
struct TableRecord {
    tag: Tag,
    offset: u32,
    length: u32,
}

impl FromBytes for TableRecord {
    const SIZE: usize = 16;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            tag: Tag::from_bytes(&bytes[0..4]),
            offset: u32::from_bytes(&bytes[8..12]),
            length: u32::from_bytes(&bytes[12..16]),
        }
    }
}

/// Number of faces stored in `data`: the collection size for `.ttc` files,
/// one for plain sfnt files.
pub fn face_count(data: &[u8]) -> Result<u32> {
    let mut reader = Reader::new(data, DIRECTORY);
    match reader.u32()? {
        COLLECTION => {
            reader.skip(4)?;
            reader.u32()
        }
        TRUETYPE | APPLE_TRUE | OPENTYPE_CFF => Ok(1),
        other => Err(FontError::UnknownFormat(other)),
    }
}

/// The table directory of one face.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TableDirectory<'a> {
    data: &'a [u8],
    records: Array<'a, TableRecord>,
    flavor: SfntFlavor,
}

impl<'a> TableDirectory<'a> {
    /// Parses the directory of face `index` (0 for non-collections).
    pub(crate) fn parse(data: &'a [u8], index: u32) -> Result<Self> {
        let mut reader = Reader::new(data, DIRECTORY);
        let magic = reader.u32()?;
        let offset = if magic == COLLECTION {
            reader.skip(4)?;
            let count = reader.u32()?;
            if index >= count {
                return Err(FontError::FaceIndexOutOfRange { index, count });
            }
            let offsets: Array<'_, u32> = reader.array(count as usize)?;
            offsets
                .get(index as usize)
                .ok_or(FontError::Truncated(DIRECTORY))? as usize
        } else if index != 0 {
            return Err(FontError::FaceIndexOutOfRange { index, count: 1 });
        } else {
            0
        };
        let mut reader = Reader::at(data, offset, DIRECTORY)?;
        let flavor = match reader.u32()? {
            TRUETYPE | APPLE_TRUE => SfntFlavor::TrueType,
            OPENTYPE_CFF => SfntFlavor::Cff,
            other => return Err(FontError::UnknownFormat(other)),
        };
        let num_tables = reader.u16()?;
        reader.skip(6)?;
        let records = reader.array(usize::from(num_tables))?;
        Ok(Self {
            data,
            records,
            flavor,
        })
    }

    /// The outline flavour announced by the face header.
    pub(crate) const fn flavor(&self) -> SfntFlavor {
        self.flavor
    }

    /// Returns the bytes of table `tag`, or `None` when the table is absent
    /// or lies outside the file. Like FreeType, metrics tables that run past
    /// the end of the file are clamped rather than dropped.
    pub(crate) fn table(&self, tag: Tag) -> Option<&'a [u8]> {
        let record = self.records.iter().find(|record| record.tag == tag)?;
        let start = record.offset as usize;
        let available = self.data.len().checked_sub(start)?;
        let length = record.length as usize;
        let clamp = tag == Tag::new(b"hmtx") || tag == Tag::new(b"vmtx");
        let length = if length > available && clamp {
            available
        } else {
            length
        };
        self.data.get(start..start.checked_add(length)?)
    }

    /// Returns table `tag` or a [`FontError::MissingTable`] error.
    pub(crate) fn required(&self, tag: Tag) -> Result<&'a [u8]> {
        self.table(tag).ok_or(FontError::MissingTable(tag))
    }

    /// Tags of every table present in the directory.
    pub(crate) fn tags(&self) -> impl Iterator<Item = Tag> + 'a {
        self.records.iter().map(|record| record.tag)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a minimal sfnt file containing the given tables.
    pub(crate) fn build_sfnt(tables: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&TRUETYPE.to_be_bytes());
        out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        let mut offset = 12 + 16 * tables.len();
        let mut body = Vec::new();
        for (tag, data) in tables {
            out.extend_from_slice(*tag);
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            body.extend_from_slice(data);
            while body.len() % 4 != 0 {
                body.push(0);
            }
            offset = 12 + 16 * tables.len() + body.len();
        }
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn finds_tables() {
        let font = build_sfnt(&[(b"abcd", &[1, 2, 3]), (b"wxyz", &[9])]);
        let dir = TableDirectory::parse(&font, 0).unwrap();
        assert_eq!(dir.table(Tag::new(b"abcd")), Some(&[1u8, 2, 3][..]));
        assert_eq!(dir.table(Tag::new(b"wxyz")), Some(&[9u8][..]));
        assert_eq!(dir.table(Tag::new(b"none")), None);
        assert_eq!(dir.tags().count(), 2);
        assert_eq!(dir.flavor(), SfntFlavor::TrueType);
        assert_eq!(face_count(&font), Ok(1));
        assert!(matches!(
            TableDirectory::parse(&font, 1),
            Err(FontError::FaceIndexOutOfRange { index: 1, count: 1 })
        ));
    }

    #[test]
    fn rejects_unknown_and_truncated_data() {
        assert!(matches!(
            TableDirectory::parse(b"wOFF0000", 0),
            Err(FontError::UnknownFormat(_))
        ));
        assert!(TableDirectory::parse(&[0, 1], 0).is_err());
        let mut font = build_sfnt(&[(b"abcd", &[1, 2, 3])]);
        font.truncate(20);
        assert!(TableDirectory::parse(&font, 0).is_err());
    }

    #[test]
    fn out_of_file_tables_are_absent_except_metrics() {
        let mut font = build_sfnt(&[(b"abcd", &[1, 2, 3, 4]), (b"hmtx", &[5, 6, 7, 8])]);
        font.truncate(font.len() - 2);
        let dir = TableDirectory::parse(&font, 0).unwrap();
        assert_eq!(dir.table(Tag::new(b"abcd")), Some(&[1u8, 2, 3, 4][..]));
        assert_eq!(dir.table(Tag::new(b"hmtx")), Some(&[5u8, 6][..]));
        font.truncate(font.len() - 4);
        let dir = TableDirectory::parse(&font, 0).unwrap();
        assert_eq!(dir.table(Tag::new(b"abcd")), None);
    }

    #[test]
    fn reads_collections() {
        let face = build_sfnt(&[(b"abcd", &[1])]);
        let mut ttc = Vec::new();
        ttc.extend_from_slice(b"ttcf");
        ttc.extend_from_slice(&[0, 1, 0, 0]);
        ttc.extend_from_slice(&2u32.to_be_bytes());
        let first = 20u32;
        let second = first + face.len() as u32;
        ttc.extend_from_slice(&first.to_be_bytes());
        ttc.extend_from_slice(&second.to_be_bytes());
        // Table offsets inside each face are relative to the file start.
        for base in [first, second] {
            let mut copy = face.clone();
            let old = u32::from_be_bytes(copy[20..24].try_into().unwrap());
            copy[20..24].copy_from_slice(&(old + base).to_be_bytes());
            ttc.extend_from_slice(&copy);
        }
        assert_eq!(face_count(&ttc), Ok(2));
        for index in 0..2 {
            let dir = TableDirectory::parse(&ttc, index).unwrap();
            assert_eq!(dir.table(Tag::new(b"abcd")), Some(&[1u8][..]));
        }
        assert!(matches!(
            TableDirectory::parse(&ttc, 2),
            Err(FontError::FaceIndexOutOfRange { index: 2, count: 2 })
        ));
    }
}
