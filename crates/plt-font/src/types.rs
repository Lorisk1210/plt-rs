//! Small value types shared by every table parser.

use std::fmt;

/// A four-byte OpenType tag such as `b"glyf"` or `b"kern"`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    /// Builds a tag from its four ASCII bytes.
    pub const fn new(bytes: &[u8; 4]) -> Self {
        Self(*bytes)
    }

    /// Builds a tag from its big-endian `u32` representation.
    pub const fn from_u32(value: u32) -> Self {
        Self(value.to_be_bytes())
    }

    /// Returns the big-endian `u32` representation of the tag.
    pub const fn to_u32(self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// Parses a tag from a string of at most four ASCII characters, padding
    /// with spaces (`"kern"`, `"DFLT"`, `"ENG"`).
    pub fn from_str_padded(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.is_empty() || bytes.len() > 4 || !text.is_ascii() {
            return None;
        }
        let mut tag = [b' '; 4];
        tag[..bytes.len()].copy_from_slice(bytes);
        Some(Self(tag))
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tag({self})")
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &byte in &self.0 {
            let shown = if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '?'
            };
            write!(f, "{shown}")?;
        }
        Ok(())
    }
}

/// Index of a glyph inside a face.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct GlyphId(pub u16);

impl GlyphId {
    /// The `.notdef` glyph, used for characters the font cannot display.
    pub const NOTDEF: Self = Self(0);
}

/// An axis-aligned rectangle in font units (or whatever unit the producer
/// documents), with `x_min <= x_max` and `y_min <= y_max` when non-empty.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    /// Left edge.
    pub x_min: i32,
    /// Bottom edge.
    pub y_min: i32,
    /// Right edge.
    pub x_max: i32,
    /// Top edge.
    pub y_max: i32,
}

impl Rect {
    /// Width of the rectangle.
    pub const fn width(&self) -> i32 {
        self.x_max - self.x_min
    }

    /// Height of the rectangle.
    pub const fn height(&self) -> i32 {
        self.y_max - self.y_min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_round_trips_and_pads() {
        let tag = Tag::new(b"kern");
        assert_eq!(Tag::from_u32(tag.to_u32()), tag);
        assert_eq!(Tag::from_str_padded("ENG"), Some(Tag::new(b"ENG ")));
        assert_eq!(Tag::from_str_padded(""), None);
        assert_eq!(Tag::from_str_padded("toolong"), None);
        assert_eq!(format!("{}", Tag::new(b"OS/2")), "OS/2");
        assert_eq!(format!("{}", Tag([0, b'a', b'b', b'c'])), "?abc");
    }

    #[test]
    fn rect_dimensions() {
        let rect = Rect {
            x_min: -2,
            y_min: -3,
            x_max: 5,
            y_max: 7,
        };
        assert_eq!(rect.width(), 7);
        assert_eq!(rect.height(), 10);
    }
}
