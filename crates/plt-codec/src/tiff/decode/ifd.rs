//! The TIFF header and the first image file directory, read as Pillow's
//! `ImageFileDirectory_v2` reads them: entries of unknown types or
//! without values are skipped, and reading stops at the first entry whose
//! values lie outside the file, keeping the entries before it.

use super::error::DecodeError;

/// Field types whose values are integers, with their sizes in bytes.
const INTEGER_TYPES: [(u16, usize); 8] = [
    (1, 1),  // BYTE
    (3, 2),  // SHORT
    (4, 4),  // LONG
    (6, 1),  // SBYTE
    (8, 2),  // SSHORT
    (9, 4),  // SLONG
    (13, 4), // IFD
    (16, 8), // LONG8
];
/// Other field types Pillow reads, with their sizes.
const OTHER_TYPES: [(u16, usize); 6] = [
    (2, 1),  // ASCII
    (5, 8),  // RATIONAL
    (7, 1),  // UNDEFINED
    (10, 8), // SRATIONAL
    (11, 4), // FLOAT
    (12, 8), // DOUBLE
];
/// The BYTE type, whose values Pillow turns into a single number.
const BYTE: u16 = 1;

/// The byte order of a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ByteOrder {
    /// `II`: least significant byte first.
    Little,
    /// `MM`: most significant byte first.
    Big,
}

impl ByteOrder {
    /// An unsigned number from up to eight bytes.
    pub(super) fn read(self, bytes: &[u8]) -> u64 {
        let fold = |value: u64, &byte: &u8| value << 8 | u64::from(byte);
        match self {
            Self::Little => bytes.iter().rev().fold(0, fold),
            Self::Big => bytes.iter().fold(0, fold),
        }
    }
}

/// An entry: its field type, value count and where its values are.
#[derive(Clone, Copy, Debug)]
struct Entry {
    tag: u16,
    kind: u16,
    count: usize,
    at: usize,
}

/// The first image file directory of a file.
#[derive(Debug)]
pub(super) struct Directory<'a> {
    data: &'a [u8],
    order: ByteOrder,
    entries: Vec<Entry>,
}

impl<'a> Directory<'a> {
    /// Reads the header and the first directory of `data`.
    pub(super) fn read(data: &'a [u8]) -> Result<Self, DecodeError> {
        let order = match data.get(..4) {
            Some(b"II*\0" | b"II\0*" | b"II+\0") => ByteOrder::Little,
            Some(b"MM\0*" | b"MM*\0" | b"MM\0+") => ByteOrder::Big,
            _ => return Err(DecodeError::NotTiff),
        };
        // Pillow takes only `II+` for BigTIFF.
        let big = data[2] == 43;
        let (offset_size, header) = if big { (8, 16) } else { (4, 8) };
        let header = data.get(..header).ok_or(DecodeError::NotTiff)?;
        let first = order.read(&header[header.len() - offset_size..]);
        if first == 0 {
            return Err(DecodeError::NoImage);
        }
        let mut directory = Self {
            data,
            order,
            entries: Vec::new(),
        };
        let start = usize::try_from(first).map_err(|_| DecodeError::NoImage)?;
        let (count_size, entry_size) = if big { (8, 20) } else { (2, 12) };
        let count = directory
            .bytes(start, count_size)
            .map(|bytes| order.read(bytes))
            .ok_or(DecodeError::NoImage)?;
        for index in 0..count {
            let Some(at) = usize::try_from(index)
                .ok()
                .and_then(|i| (start + count_size).checked_add(i.checked_mul(entry_size)?))
            else {
                break;
            };
            let Some(raw) = directory.bytes(at, entry_size) else {
                break;
            };
            let tag = u16::try_from(order.read(&raw[..2])).expect("two bytes");
            let kind = u16::try_from(order.read(&raw[2..4])).expect("two bytes");
            let (count, value) = raw[4..].split_at(offset_size);
            let Some(size) = type_size(kind) else {
                continue;
            };
            let Ok(count) = usize::try_from(order.read(count)) else {
                break;
            };
            let Some(total) = count.checked_mul(size) else {
                break;
            };
            if total == 0 {
                continue;
            }
            let at = if total <= offset_size {
                at + 4 + offset_size
            } else {
                match usize::try_from(order.read(value)) {
                    Ok(at) if directory.bytes(at, total).is_some() => at,
                    _ => break,
                }
            };
            directory.entries.retain(|entry| entry.tag != tag);
            directory.entries.push(Entry {
                tag,
                kind,
                count,
                at,
            });
        }
        Ok(directory)
    }

    /// The file's byte order.
    pub(super) fn order(&self) -> ByteOrder {
        self.order
    }

    fn bytes(&self, at: usize, len: usize) -> Option<&'a [u8]> {
        self.data.get(at..at.checked_add(len)?)
    }

    fn entry(&self, tag: u16) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.tag == tag)
    }

    /// Whether the directory has `tag`.
    pub(super) fn has(&self, tag: u16) -> bool {
        self.entry(tag).is_some()
    }

    /// The integer values of `tag`, or `None` when it is absent; values
    /// of other types, and negative ones, are invalid.
    fn integers(&self, tag: u16) -> Result<Option<Vec<u64>>, DecodeError> {
        let Some(entry) = self.entry(tag) else {
            return Ok(None);
        };
        let invalid = DecodeError::InvalidTag(tag);
        let &(_, size) = INTEGER_TYPES
            .iter()
            .find(|(kind, _)| *kind == entry.kind)
            .ok_or(invalid.clone())?;
        let signed = matches!(entry.kind, 6 | 8 | 9);
        let bytes = self
            .bytes(entry.at, entry.count * size)
            .expect("checked when read");
        bytes
            .chunks_exact(size)
            .map(|chunk| {
                let value = self.order.read(chunk);
                let negative = signed && value >> (size * 8 - 1) & 1 == 1;
                if negative {
                    Err(invalid.clone())
                } else {
                    Ok(value)
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    /// The values of an array tag such as `BitsPerSample`; a BYTE array
    /// is invalid, as Pillow reads it as a single number.
    pub(super) fn array(&self, tag: u16) -> Result<Option<Vec<u64>>, DecodeError> {
        if self.entry(tag).is_some_and(|entry| entry.kind == BYTE) {
            return Err(DecodeError::InvalidTag(tag));
        }
        self.integers(tag)
    }

    /// The first value of a single-valued tag such as `ImageWidth`.
    pub(super) fn number(&self, tag: u16) -> Result<Option<u64>, DecodeError> {
        Ok(self
            .integers(tag)?
            .and_then(|values| values.first().copied()))
    }
}

/// The size of a value of field type `kind`, if Pillow reads that type.
fn type_size(kind: u16) -> Option<usize> {
    INTEGER_TYPES
        .iter()
        .chain(&OTHER_TYPES)
        .find(|(k, _)| *k == kind)
        .map(|&(_, size)| size)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A little-endian file with one directory of `entries` (tag, type,
    /// count, value field) at offset 8.
    fn file(entries: &[(u16, u16, u32, [u8; 4])], tail: &[u8]) -> Vec<u8> {
        let mut data = b"II*\0\x08\0\0\0".to_vec();
        data.extend_from_slice(&u16::try_from(entries.len()).unwrap().to_le_bytes());
        for (tag, kind, count, value) in entries {
            data.extend_from_slice(&tag.to_le_bytes());
            data.extend_from_slice(&kind.to_le_bytes());
            data.extend_from_slice(&count.to_le_bytes());
            data.extend_from_slice(value);
        }
        data.extend_from_slice(&[0; 4]);
        data.extend_from_slice(tail);
        data
    }

    #[test]
    fn values_inline_and_elsewhere() {
        let data = file(
            &[
                (256, 3, 1, [7, 0, 0, 0]),
                (258, 3, 3, [50, 0, 0, 0]),
                (273, 4, 1, [9, 1, 0, 0]),
            ],
            &[8, 0, 16, 0, 8, 0],
        );
        let ifd = Directory::read(&data).unwrap();
        assert_eq!(ifd.number(256), Ok(Some(7)));
        assert_eq!(ifd.array(258), Ok(Some(vec![8, 16, 8])));
        assert_eq!(ifd.number(273), Ok(Some(265)));
        assert_eq!(ifd.number(999), Ok(None));
        assert_eq!(ifd.order(), ByteOrder::Little);
    }

    #[test]
    fn reading_stops_at_values_outside_the_file() {
        let data = file(
            &[
                (256, 3, 1, [7, 0, 0, 0]),
                (258, 3, 9, [200, 0, 0, 0]),
                (257, 3, 1, [5, 0, 0, 0]),
            ],
            &[],
        );
        let ifd = Directory::read(&data).unwrap();
        assert!(ifd.has(256));
        assert!(!ifd.has(258) && !ifd.has(257));
    }

    #[test]
    fn types_are_checked() {
        let data = file(
            &[
                (256, 5, 1, [40, 0, 0, 0]),
                (257, 8, 1, [0xFF, 0xFF, 0, 0]),
                (258, 1, 2, [8, 8, 0, 0]),
                (259, 99, 1, [1, 0, 0, 0]),
            ],
            &[0; 8],
        );
        let ifd = Directory::read(&data).unwrap();
        assert_eq!(ifd.number(256), Err(DecodeError::InvalidTag(256)));
        assert_eq!(
            ifd.number(257),
            Err(DecodeError::InvalidTag(257)),
            "negative"
        );
        assert_eq!(
            ifd.array(258),
            Err(DecodeError::InvalidTag(258)),
            "BYTE array"
        );
        assert!(!ifd.has(259), "unknown types are skipped");
    }

    #[test]
    fn headers() {
        assert_eq!(Directory::read(b"II*\0").err(), Some(DecodeError::NotTiff));
        assert_eq!(
            Directory::read(b"XX*\0\x08\0\0\0").err(),
            Some(DecodeError::NotTiff)
        );
        assert_eq!(
            Directory::read(b"MM\0*\0\0\0\0").err(),
            Some(DecodeError::NoImage)
        );
        let big = b"MM\0*\0\0\0\x08\0\x01\x01\x00\0\x03\0\0\0\x01\0\x05\0\0\0\0\0\0";
        let ifd = Directory::read(big).unwrap();
        assert_eq!(ifd.order(), ByteOrder::Big);
        assert_eq!(ifd.number(256), Ok(Some(5)));
        let huge_count = b"II*\0\x08\0\0\0\xff\xff";
        assert_eq!(Directory::read(huge_count).unwrap().entries.len(), 0);
    }
}
