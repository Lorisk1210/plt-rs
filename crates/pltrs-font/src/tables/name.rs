//! `name`: naming strings, decoded the way FreeType exposes them.

use crate::error::Result;
use crate::reader::{Array, FromBytes, Reader, slice};
use crate::types::Tag;

const NAME: Tag = Tag::new(b"name");

/// Well-known name ids.
pub(crate) mod id {
    pub(crate) const FAMILY: u16 = 1;
    pub(crate) const SUBFAMILY: u16 = 2;
    pub(crate) const FULL_NAME: u16 = 4;
    pub(crate) const POSTSCRIPT: u16 = 6;
    pub(crate) const TYPOGRAPHIC_FAMILY: u16 = 16;
    pub(crate) const TYPOGRAPHIC_SUBFAMILY: u16 = 17;
    pub(crate) const WWS_FAMILY: u16 = 21;
    pub(crate) const WWS_SUBFAMILY: u16 = 22;
}

/// Platform / encoding / language of a name record.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NameKey {
    /// Platform id (1 Macintosh, 3 Windows, 0 Unicode).
    pub platform_id: u16,
    /// Platform-specific encoding id.
    pub encoding_id: u16,
    /// Language id.
    pub language_id: u16,
}

impl NameKey {
    /// Macintosh, Roman, English: the first key matplotlib consults.
    pub const MAC_ENGLISH: Self = Self {
        platform_id: 1,
        encoding_id: 0,
        language_id: 0,
    };
    /// Windows, Unicode BMP, US English: matplotlib's second key.
    pub const WINDOWS_ENGLISH: Self = Self {
        platform_id: 3,
        encoding_id: 1,
        language_id: 0x0409,
    };

    /// Whether strings under this key are UTF-16BE encoded.
    const fn is_utf16(self) -> bool {
        matches!(self.platform_id, 0 | 2 | 3)
    }
}

/// One record of the `name` table: its key, name id and raw string
/// bytes (FreeType's `FT_SfntName`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NameEntry<'a> {
    /// Platform, encoding and language.
    pub key: NameKey,
    /// What the string names (1 family, 2 subfamily, ...).
    pub name_id: u16,
    /// The string as stored: Macintosh text or UTF-16BE.
    pub bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug)]
struct NameRecord {
    key: NameKey,
    name_id: u16,
    length: u16,
    offset: u16,
}

impl FromBytes for NameRecord {
    const SIZE: usize = 12;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            key: NameKey {
                platform_id: u16::from_bytes(&bytes[0..2]),
                encoding_id: u16::from_bytes(&bytes[2..4]),
                language_id: u16::from_bytes(&bytes[4..6]),
            },
            name_id: u16::from_bytes(&bytes[6..8]),
            length: u16::from_bytes(&bytes[8..10]),
            offset: u16::from_bytes(&bytes[10..12]),
        }
    }
}

/// The `name` table.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Name<'a> {
    records: Array<'a, NameRecord>,
    storage: &'a [u8],
}

impl<'a> Name<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, NAME);
        r.skip(2)?;
        let count = r.u16()?;
        let storage_offset = usize::from(r.u16()?);
        let records = r.array(usize::from(count))?;
        let storage = data.get(storage_offset..).unwrap_or(&[]);
        Ok(Self { records, storage })
    }

    fn bytes(&self, record: &NameRecord) -> Option<&'a [u8]> {
        slice(
            self.storage,
            usize::from(record.offset),
            usize::from(record.length),
            NAME,
        )
        .ok()
    }

    /// The non-empty records whose strings lie inside the table, in file
    /// order: the names FreeType lists (`FT_Get_Sfnt_Name`).
    pub(crate) fn entries(&self) -> impl Iterator<Item = NameEntry<'a>> + '_ {
        self.records
            .iter()
            .filter(|record| record.length > 0)
            .filter_map(|record| {
                Some(NameEntry {
                    key: record.key,
                    name_id: record.name_id,
                    bytes: self.bytes(&record)?,
                })
            })
    }

    /// Raw bytes of the record with exactly this key and name id.
    pub(crate) fn raw(&self, key: NameKey, name_id: u16) -> Option<&'a [u8]> {
        let record = self
            .records
            .iter()
            .find(|r| r.key == key && r.name_id == name_id)?;
        self.bytes(&record)
    }

    /// The record with exactly this key and name id, decoded as Latin-1
    /// (Macintosh) or UTF-16BE (Unicode/Windows), like matplotlib does.
    pub(crate) fn decoded(&self, key: NameKey, name_id: u16) -> Option<String> {
        let bytes = self.raw(key, name_id)?;
        Some(if key.is_utf16() {
            decode_utf16be(bytes)
        } else {
            decode_latin1(bytes)
        })
    }

    /// FreeType's `tt_face_get_name`: picks the best English record for
    /// `name_id` and converts it to ASCII, replacing other characters by `?`.
    pub(crate) fn freetype_string(&self, name_id: u16) -> Option<String> {
        let (mut win, mut apple_english, mut apple_roman, mut unicode) = (None, None, None, None);
        let mut win_is_english = false;
        for record in self
            .records
            .iter()
            .filter(|r| r.name_id == name_id && r.length > 0)
        {
            let english = record.key.language_id & 0x3FF == 0x009;
            match record.key.platform_id {
                0 | 2 => unicode = Some(record),
                1 if record.key.language_id == 0 => apple_english = Some(record),
                1 if record.key.encoding_id == 0 => apple_roman = Some(record),
                3 if (win.is_none() || english) && matches!(record.key.encoding_id, 0 | 1 | 10) => {
                    win_is_english = english;
                    win = Some(record);
                }
                _ => {}
            }
        }
        let apple = apple_english.or(apple_roman);
        let (record, utf16) = match (win, apple) {
            (Some(w), a) if a.is_none() || win_is_english => (w, true),
            (_, Some(a)) => (a, false),
            _ => (unicode?, true),
        };
        let bytes = self.bytes(&record)?;
        let ascii = |code: u32| {
            if (32..=127).contains(&code) {
                char::from(code as u8)
            } else {
                '?'
            }
        };
        Some(if utf16 {
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| u32::from(u16::from_be_bytes(pair)))
                .take_while(|&code| code != 0)
                .map(ascii)
                .collect()
        } else {
            bytes
                .iter()
                .map(|&b| u32::from(b))
                .take_while(|&code| code != 0)
                .map(ascii)
                .collect()
        })
    }

    /// FreeType's PostScript name: name id 6 from a Windows record
    /// (preferring US English), else from a Macintosh Roman record, keeping
    /// only characters valid in PostScript names.
    pub(crate) fn postscript_name(&self) -> Option<String> {
        let (mut win, mut apple) = (None, None);
        for record in self
            .records
            .iter()
            .filter(|r| r.name_id == id::POSTSCRIPT && r.length > 0)
        {
            let key = record.key;
            if key.platform_id == 3
                && matches!(key.encoding_id, 0 | 1)
                && (key.language_id == 0x409 || win.is_none())
            {
                win = Some(record);
            }
            if key.platform_id == 1
                && key.encoding_id == 0
                && (key.language_id == 0 || apple.is_none())
            {
                apple = Some(record);
            }
        }
        let valid = |c: u8| (33..=126).contains(&c) && !b"[](){}<>/%".contains(&c);
        let from_win = win.and_then(|record| self.bytes(&record)).map(|bytes| {
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .filter(|pair| pair[0] == 0 && valid(pair[1]))
                .map(|pair| char::from(pair[1]))
                .collect::<String>()
        });
        from_win.or_else(|| {
            let bytes = self.bytes(&apple?)?;
            Some(
                bytes
                    .iter()
                    .copied()
                    .filter(|&b| valid(b))
                    .map(char::from)
                    .collect(),
            )
        })
    }
}

fn decode_utf16be(bytes: &[u8]) -> String {
    let units = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_be_bytes(pair));
    char::decode_utf16(units)
        .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// `bytes` decoded as Latin-1 (every byte is the code point of its value).
pub(crate) fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| char::from(b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(records: &[(NameKey, u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = vec![0, 0];
        out.extend_from_slice(&(records.len() as u16).to_be_bytes());
        out.extend_from_slice(&((6 + 12 * records.len()) as u16).to_be_bytes());
        let mut storage = Vec::new();
        for (key, name_id, bytes) in records {
            for v in [
                key.platform_id,
                key.encoding_id,
                key.language_id,
                *name_id,
                bytes.len() as u16,
            ] {
                out.extend_from_slice(&v.to_be_bytes());
            }
            out.extend_from_slice(&(storage.len() as u16).to_be_bytes());
            storage.extend_from_slice(bytes);
        }
        out.extend_from_slice(&storage);
        out
    }

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    #[test]
    fn decodes_platform_strings() {
        let data = table(&[
            (NameKey::MAC_ENGLISH, 1, b"Caf\xe9".to_vec()),
            (NameKey::WINDOWS_ENGLISH, 1, utf16("Café")),
        ]);
        let name = Name::parse(&data).unwrap();
        assert_eq!(
            name.decoded(NameKey::MAC_ENGLISH, 1).as_deref(),
            Some("Café")
        );
        assert_eq!(
            name.decoded(NameKey::WINDOWS_ENGLISH, 1).as_deref(),
            Some("Café")
        );
        assert_eq!(name.decoded(NameKey::WINDOWS_ENGLISH, 2), None);
        // FreeType prefers the English Windows record and ASCII-fies it.
        assert_eq!(name.freetype_string(1).as_deref(), Some("Caf?"));
    }

    #[test]
    fn freetype_prefers_mac_over_non_english_windows() {
        let french = NameKey {
            platform_id: 3,
            encoding_id: 1,
            language_id: 0x040C,
        };
        let data = table(&[
            (NameKey::MAC_ENGLISH, 1, b"Mac".to_vec()),
            (french, 1, utf16("Win")),
        ]);
        let name = Name::parse(&data).unwrap();
        assert_eq!(name.freetype_string(1).as_deref(), Some("Mac"));
        let data = table(&[(french, 1, utf16("Win"))]);
        assert_eq!(
            Name::parse(&data).unwrap().freetype_string(1).as_deref(),
            Some("Win")
        );
        let unicode = NameKey {
            platform_id: 0,
            encoding_id: 3,
            language_id: 0,
        };
        let data = table(&[(unicode, 1, utf16("Uni"))]);
        assert_eq!(
            Name::parse(&data).unwrap().freetype_string(1).as_deref(),
            Some("Uni")
        );
        assert_eq!(Name::parse(&data).unwrap().freetype_string(4), None);
    }

    #[test]
    fn postscript_names_drop_invalid_characters() {
        let data = table(&[
            (NameKey::MAC_ENGLISH, 6, b"Mac-Name".to_vec()),
            (NameKey::WINDOWS_ENGLISH, 6, utf16("Dé jà(Vu)")),
        ]);
        assert_eq!(
            Name::parse(&data).unwrap().postscript_name().as_deref(),
            Some("DjVu")
        );
        let data = table(&[(NameKey::MAC_ENGLISH, 6, b"Mac Name".to_vec())]);
        assert_eq!(
            Name::parse(&data).unwrap().postscript_name().as_deref(),
            Some("MacName")
        );
        assert_eq!(Name::parse(&table(&[])).unwrap().postscript_name(), None);
    }

    #[test]
    fn truncated_tables() {
        let data = table(&[(NameKey::MAC_ENGLISH, 1, b"Name".to_vec())]);
        assert!(Name::parse(&data[..10]).is_err());
        // Storage cut short: the record is unreadable but parsing succeeds.
        let name = Name::parse(&data[..data.len() - 1]).unwrap();
        assert_eq!(name.decoded(NameKey::MAC_ENGLISH, 1), None);
    }
}
