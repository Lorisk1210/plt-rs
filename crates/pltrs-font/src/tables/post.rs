//! `post`: italic angle, underline metrics and PostScript glyph names.

use crate::error::Result;
use crate::reader::{Array, Reader};
use crate::types::{GlyphId, Tag};

mod mac_names;

pub(crate) use mac_names::MAC_GLYPH_NAMES;

const POST: Tag = Tag::new(b"post");

/// Values from the `post` table header.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Post {
    /// Table format as 16.16 fixed point (`0x00020000` for format 2).
    pub format: u32,
    /// Italic angle in degrees counter-clockwise from vertical.
    pub italic_angle: f64,
    /// Underline position in font units (top of the underline).
    pub underline_position: i16,
    /// Underline thickness in font units.
    pub underline_thickness: i16,
    /// Whether the font is monospaced.
    pub is_fixed_pitch: bool,
    /// Least memory a printer needs to download the font as Type 42.
    pub min_mem_type42: u32,
    /// Most memory a printer needs to download the font as Type 42.
    pub max_mem_type42: u32,
    /// Least memory a printer needs to download the font as Type 1.
    pub min_mem_type1: u32,
    /// Most memory a printer needs to download the font as Type 1.
    pub max_mem_type1: u32,
}

/// Glyph names stored in the table.
#[derive(Clone, Copy, Debug)]
enum Names<'a> {
    /// Format 1: the 258 standard Macintosh names, in order.
    Standard,
    /// Format 2: per-glyph indices into standard + custom Pascal strings.
    Indexed {
        indices: Array<'a, u16>,
        strings: &'a [u8],
    },
    /// Format 2.5: per-glyph signed offsets into the standard order.
    Offsets(&'a [u8]),
    /// No names (format 3 and others).
    None,
}

/// Parsed `post` table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PostTable<'a> {
    pub(crate) header: Post,
    names: Names<'a>,
}

impl<'a> PostTable<'a> {
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader::new(data, POST);
        let format = r.u32()?;
        let italic_angle = r.fixed()?;
        let underline_position = r.i16()?;
        let underline_thickness = r.i16()?;
        let is_fixed_pitch = r.u32()? != 0;
        let [min_mem_type42, max_mem_type42, min_mem_type1, max_mem_type1] =
            [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
        let names = match format {
            0x0001_0000 => Names::Standard,
            0x0002_0000 => {
                let count = r.u16()?;
                let indices = r.array(usize::from(count))?;
                Names::Indexed {
                    indices,
                    strings: r.bytes(r.remaining())?,
                }
            }
            0x0002_5000 => {
                let count = r.u16()?;
                Names::Offsets(r.bytes(usize::from(count))?)
            }
            _ => Names::None,
        };
        let header = Post {
            format,
            italic_angle,
            underline_position,
            underline_thickness,
            is_fixed_pitch,
            min_mem_type42,
            max_mem_type42,
            min_mem_type1,
            max_mem_type1,
        };
        Ok(Self { header, names })
    }

    /// Whether glyph names are available (FreeType's `FT_HAS_GLYPH_NAMES`).
    pub(crate) const fn has_names(&self) -> bool {
        !matches!(self.names, Names::None)
    }

    /// Name of `glyph`, or `None` when unavailable.
    pub(crate) fn glyph_name(&self, glyph: GlyphId) -> Option<&'a str> {
        let index = usize::from(glyph.0);
        match self.names {
            Names::Standard => MAC_GLYPH_NAMES.get(index).copied(),
            Names::Indexed { indices, strings } => {
                let name_index = usize::from(indices.get(index)?);
                match name_index.checked_sub(MAC_GLYPH_NAMES.len()) {
                    None => MAC_GLYPH_NAMES.get(name_index).copied(),
                    Some(custom) => pascal_strings(strings).nth(custom),
                }
            }
            Names::Offsets(offsets) => {
                let offset = i64::from(*offsets.get(index)? as i8);
                let standard = usize::try_from(index as i64 + offset).ok()?;
                MAC_GLYPH_NAMES.get(standard).copied()
            }
            Names::None => None,
        }
    }
}

impl PostTable<'_> {
    /// Glyph whose name is `name` (a single pass over the name data).
    pub(crate) fn glyph_by_name(&self, name: &str) -> Option<GlyphId> {
        let position = match self.names {
            Names::Standard => MAC_GLYPH_NAMES.iter().position(|&n| n == name),
            Names::Indexed { indices, strings } => {
                let custom: Vec<&str> = pascal_strings(strings).collect();
                indices.iter().position(|index| {
                    let index = usize::from(index);
                    let found = match index.checked_sub(MAC_GLYPH_NAMES.len()) {
                        None => MAC_GLYPH_NAMES.get(index).copied(),
                        Some(custom_index) => custom.get(custom_index).copied(),
                    };
                    found == Some(name)
                })
            }
            Names::Offsets(offsets) => (0..offsets.len()).find(|&i| {
                u16::try_from(i).is_ok_and(|g| self.glyph_name(GlyphId(g)) == Some(name))
            }),
            Names::None => None,
        };
        position.and_then(|p| u16::try_from(p).ok()).map(GlyphId)
    }
}

/// Iterates over consecutive Pascal strings, stopping at the first
/// truncated or non-UTF-8 entry.
fn pascal_strings(mut data: &[u8]) -> impl Iterator<Item = &str> {
    std::iter::from_fn(move || {
        let (&len, rest) = data.split_first()?;
        let bytes = rest.get(..usize::from(len))?;
        data = &rest[usize::from(len)..];
        std::str::from_utf8(bytes).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(format: u32) -> Vec<u8> {
        let mut data = format.to_be_bytes().to_vec();
        data.extend_from_slice(&(-12i32 << 16 | 0x8000).to_be_bytes());
        data.extend_from_slice(&(-130i16).to_be_bytes());
        data.extend_from_slice(&90i16.to_be_bytes());
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(&[0; 16]);
        data
    }

    #[test]
    fn header_values() {
        let data = header(0x0003_0000);
        let post = PostTable::parse(&data).unwrap();
        assert!((post.header.italic_angle - -11.5).abs() < 1e-9);
        assert_eq!(post.header.underline_position, -130);
        assert_eq!(post.header.underline_thickness, 90);
        assert!(post.header.is_fixed_pitch);
        assert!(!post.has_names());
        assert_eq!(post.glyph_name(GlyphId(3)), None);
        assert!(PostTable::parse(&header(0x0003_0000)[..20]).is_err());
    }

    #[test]
    fn format1_and_25_names() {
        let standard = header(0x0001_0000);
        let post = PostTable::parse(&standard).unwrap();
        assert_eq!(post.glyph_name(GlyphId(3)), Some("space"));
        assert_eq!(post.glyph_name(GlyphId(257)), Some("dcroat"));
        assert_eq!(post.glyph_name(GlyphId(258)), None);
        let mut data = header(0x0002_5000);
        data.extend_from_slice(&[0, 2, 3, 0xFF]);
        let post = PostTable::parse(&data).unwrap();
        assert_eq!(post.glyph_name(GlyphId(0)), Some("space"));
        assert_eq!(post.glyph_name(GlyphId(1)), Some(".notdef"));
    }

    #[test]
    fn format2_names() {
        let mut data = header(0x0002_0000);
        data.extend_from_slice(&[0, 4, 0, 0, 1, 2, 1, 3, 0, 36]);
        data.extend_from_slice(b"\x03one\x03two");
        let post = PostTable::parse(&data).unwrap();
        assert_eq!(post.glyph_name(GlyphId(0)), Some(".notdef"));
        assert_eq!(post.glyph_name(GlyphId(1)), Some("one"));
        assert_eq!(post.glyph_name(GlyphId(2)), Some("two"));
        assert_eq!(post.glyph_name(GlyphId(3)), Some("A"));
        assert_eq!(post.glyph_name(GlyphId(4)), None);
        assert_eq!(post.glyph_by_name("two"), Some(GlyphId(2)));
        assert_eq!(post.glyph_by_name("A"), Some(GlyphId(3)));
        assert_eq!(post.glyph_by_name("zzz"), None);
        data.truncate(data.len() - 2);
        let post = PostTable::parse(&data).unwrap();
        assert_eq!(post.glyph_name(GlyphId(1)), Some("one"));
        assert_eq!(post.glyph_name(GlyphId(2)), None);
    }
}
