//! Laid-out text ready for any backend.

use plt_core::{Affine2D, Bbox, Path, Point};
use plt_font::{FontId, GlyphId};

use crate::engine::TextEngine;

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

    /// The glyph outlines and rules as one compound path in pixels
    /// relative to the origin, glyphs first (matplotlib's
    /// `TextToPath.get_text_path`); glyphs `engine` has no outline for
    /// are left out.
    pub fn outline(&self, engine: &TextEngine) -> Path {
        let glyphs = self.glyphs.iter().filter_map(|glyph| {
            let outline = engine
                .glyph_outline(glyph.font, glyph.glyph, glyph.size, self.dpi)
                .ok()?;
            Some(outline.transformed(&Affine2D::translation(glyph.x, glyph.y)))
        });
        let rects = self.rects.iter().map(|rect| {
            let (x, y, w, h) = rect.bounds();
            Path::polygon([
                Point::new(x, y),
                Point::new(x + w, y),
                Point::new(x + w, y + h),
                Point::new(x, y + h),
            ])
        });
        let parts: Vec<Path> = glyphs.chain(rects).collect();
        Path::compound(&parts)
    }
}
