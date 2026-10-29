//! `hmtx`: horizontal advances and left side bearings.

use crate::error::Result;
use crate::reader::{Array, FromBytes, Reader};
use crate::types::{GlyphId, Tag};

const HMTX: Tag = Tag::new(b"hmtx");

#[derive(Clone, Copy, Debug)]
struct LongMetric {
    advance: u16,
    lsb: i16,
}

impl FromBytes for LongMetric {
    const SIZE: usize = 4;
    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            advance: u16::from_bytes(&bytes[0..2]),
            lsb: i16::from_bytes(&bytes[2..4]),
        }
    }
}

/// Horizontal metrics of every glyph.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Hmtx<'a> {
    metrics: Array<'a, LongMetric>,
    bearings: Array<'a, i16>,
}

impl<'a> Hmtx<'a> {
    /// Parses `hmtx` with `long_count` full records. Like FreeType, a table
    /// shorter than announced is accepted and the missing values read as the
    /// last available ones (advances) or zero (bearings).
    pub(crate) fn parse(data: &'a [u8], long_count: u16, num_glyphs: u16) -> Result<Self> {
        let mut r = Reader::new(data, HMTX);
        let long_len = (usize::from(long_count) * LongMetric::SIZE).min(r.remaining());
        let metrics = Array::new(r.bytes(long_len)?);
        let short_count = usize::from(num_glyphs.saturating_sub(long_count));
        let short_len = (short_count * 2).min(r.remaining());
        let bearings = Array::new(r.bytes(short_len)?);
        Ok(Self { metrics, bearings })
    }

    /// Advance width of `glyph` in font units.
    pub(crate) fn advance(&self, glyph: GlyphId) -> u16 {
        let index = usize::from(glyph.0);
        self.metrics
            .get(index)
            .or_else(|| self.metrics.last())
            .map_or(0, |m| m.advance)
    }

    /// Left side bearing of `glyph` in font units.
    pub(crate) fn left_side_bearing(&self, glyph: GlyphId) -> i16 {
        let index = usize::from(glyph.0);
        match self.metrics.get(index) {
            Some(metric) => metric.lsb,
            None => self.bearings.get(index - self.metrics.len()).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_and_short_metrics() {
        let data = [0, 100, 0, 5, 0, 200, 0xFF, 0xFF, 0, 7, 0, 9];
        let hmtx = Hmtx::parse(&data, 2, 4).unwrap();
        assert_eq!(hmtx.advance(GlyphId(0)), 100);
        assert_eq!(hmtx.advance(GlyphId(1)), 200);
        assert_eq!(hmtx.advance(GlyphId(3)), 200);
        assert_eq!(hmtx.left_side_bearing(GlyphId(1)), -1);
        assert_eq!(hmtx.left_side_bearing(GlyphId(2)), 7);
        assert_eq!(hmtx.left_side_bearing(GlyphId(3)), 9);
        assert_eq!(hmtx.left_side_bearing(GlyphId(9)), 0);
    }

    #[test]
    fn truncated_tables_degrade_gracefully() {
        let data = [0, 100, 0, 5, 0];
        let hmtx = Hmtx::parse(&data, 2, 4).unwrap();
        assert_eq!(hmtx.advance(GlyphId(3)), 100);
        assert_eq!(hmtx.left_side_bearing(GlyphId(3)), 0);
        let empty = Hmtx::parse(&[], 0, 4).unwrap();
        assert_eq!(empty.advance(GlyphId(1)), 0);
    }
}
