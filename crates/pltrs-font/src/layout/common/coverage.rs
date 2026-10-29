//! Coverage tables and class definitions: the glyph sets and glyph
//! classes `GSUB`, `GPOS` and `GDEF` subtables refer to.

use crate::error::{FontError, Result};
use crate::reader::{Array, FromBytes, Reader, tail};
use crate::types::{GlyphId, Tag};

#[derive(Clone, Copy, Debug)]
pub(crate) struct RangeRecord {
    start: u16,
    end: u16,
    value: u16,
}

impl FromBytes for RangeRecord {
    const SIZE: usize = 6;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            start: u16::from_bytes(&bytes[0..2]),
            end: u16::from_bytes(&bytes[2..4]),
            value: u16::from_bytes(&bytes[4..6]),
        }
    }
}

fn find_range(ranges: &Array<'_, RangeRecord>, glyph: GlyphId) -> Option<RangeRecord> {
    ranges
        .harfbuzz_search_by(|range| {
            if range.end < glyph.0 {
                std::cmp::Ordering::Less
            } else if range.start > glyph.0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .map(|(_, range)| range)
}

/// A coverage table: the set of glyphs a subtable applies to, each with an
/// index.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Coverage<'a> {
    Glyphs(Array<'a, GlyphId>),
    Ranges(Array<'a, RangeRecord>),
}

impl<'a> Coverage<'a> {
    pub(crate) fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        match r.u16()? {
            1 => {
                let count = r.u16()?;
                Ok(Self::Glyphs(r.array(usize::from(count))?))
            }
            2 => {
                let count = r.u16()?;
                Ok(Self::Ranges(r.array(usize::from(count))?))
            }
            format => Err(FontError::UnsupportedFormat {
                table: tag,
                format: u32::from(format),
            }),
        }
    }

    /// Parses the coverage table at `offset` inside `parent`.
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Result<Self> {
        Self::parse(tail(parent, usize::from(offset), tag)?, tag)
    }

    /// Coverage index of `glyph`, if covered (with HarfBuzz's choice among
    /// duplicate entries).
    pub(crate) fn index(&self, glyph: GlyphId) -> Option<u16> {
        match self {
            Self::Glyphs(glyphs) => glyphs
                .harfbuzz_search_by(|g| g.cmp(&glyph))
                .map(|(i, _)| i as u16),
            Self::Ranges(ranges) => find_range(ranges, glyph)
                .map(|range| range.value.wrapping_add(glyph.0 - range.start)),
        }
    }

    pub(crate) fn contains(&self, glyph: GlyphId) -> bool {
        self.index(glyph).is_some()
    }

    /// Every covered glyph with its coverage index, in table order.
    pub(crate) fn entries(&self) -> Vec<(u16, GlyphId)> {
        match self {
            Self::Glyphs(glyphs) => (0..).zip(glyphs.iter()).collect(),
            Self::Ranges(ranges) => ranges
                .iter()
                .flat_map(|range| {
                    (range.start..=range.end)
                        .map(move |g| (range.value.wrapping_add(g - range.start), GlyphId(g)))
                })
                .collect(),
        }
    }
}

/// A class definition table mapping glyphs to small integers.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) enum ClassDef<'a> {
    Array {
        start: u16,
        classes: Array<'a, u16>,
    },
    Ranges(Array<'a, RangeRecord>),
    #[default]
    Empty,
}

impl<'a> ClassDef<'a> {
    pub(crate) fn parse(data: &'a [u8], tag: Tag) -> Result<Self> {
        let mut r = Reader::new(data, tag);
        match r.u16()? {
            1 => {
                let start = r.u16()?;
                let count = r.u16()?;
                Ok(Self::Array {
                    start,
                    classes: r.array(usize::from(count))?,
                })
            }
            2 => {
                let count = r.u16()?;
                Ok(Self::Ranges(r.array(usize::from(count))?))
            }
            format => Err(FontError::UnsupportedFormat {
                table: tag,
                format: u32::from(format),
            }),
        }
    }

    /// Parses the class definition at `offset` inside `parent`; a zero
    /// offset means "every glyph is class 0".
    pub(crate) fn at(parent: &'a [u8], offset: u16, tag: Tag) -> Result<Self> {
        if offset == 0 {
            return Ok(Self::Empty);
        }
        Self::parse(tail(parent, usize::from(offset), tag)?, tag)
    }

    /// Class of `glyph` (0 when not listed).
    pub(crate) fn class(&self, glyph: GlyphId) -> u16 {
        match self {
            Self::Array { start, classes } => glyph
                .0
                .checked_sub(*start)
                .and_then(|i| classes.get(usize::from(i)))
                .unwrap_or(0),
            Self::Ranges(ranges) => find_range(ranges, glyph).map_or(0, |range| range.value),
            Self::Empty => 0,
        }
    }
}
