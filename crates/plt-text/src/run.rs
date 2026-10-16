//! Laid-out text ready for any backend.

use plt_core::Bbox;
use plt_font::{FontId, GlyphId};

/// One glyph placed relative to the text origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedGlyph {
    /// The face the glyph comes from.
    pub font: FontId,
    /// The glyph within that face.
    pub glyph: GlyphId,
    /// Font size in points.
    pub size: f64,
    /// Horizontal position of the glyph origin, in pixels.
    pub x: f64,
    /// Vertical position of the glyph origin (baseline), in pixels, y up.
    pub y: f64,
}

/// Text laid out in pixels around its origin (the left end of the
/// baseline): glyphs plus solid rectangles (mathtext rules such as
/// fraction bars).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextRun {
    /// Glyphs in drawing order.
    pub glyphs: Vec<PlacedGlyph>,
    /// Filled rectangles, relative to the origin.
    pub rects: Vec<Bbox>,
    /// Advance width.
    pub width: f64,
    /// Height above the bottom of the descent (matplotlib's `h`).
    pub height: f64,
    /// Depth below the baseline.
    pub descent: f64,
    /// Resolution the run was laid out for.
    pub dpi: f64,
}

impl TextRun {
    /// Whether nothing would be drawn.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty() && self.rects.is_empty()
    }
}
