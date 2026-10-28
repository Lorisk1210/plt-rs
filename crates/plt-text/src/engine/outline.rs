//! Glyph outlines: unhinted, hinted as the raster backend renders them,
//! and as scan outlines, cached by font, glyph, size and resolution.

use std::sync::PoisonError;

use plt_core::{Path, PathBuilder, Point};
use plt_font::{
    AutohintTarget, Face, FontId, GlyphId, HintedGlyph, Hinter, Hinting, HintingInstance,
    OutlineSink, ScanOutline, Size,
};

use super::TextEngine;
use crate::error::TextError;

/// The size text outlines are made at before scaling them to their size
/// (matplotlib's `TextToPath.FONT_SCALE`).
pub const TEXT_PATH_FONT_SCALE: f64 = 100.0;

/// The resolution text outlines are made at (`TextToPath.DPI`).
pub const TEXT_PATH_DPI: f64 = 72.0;

/// What a cached outline depends on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct OutlineKey {
    font: FontId,
    glyph: GlyphId,
    size_bits: u64,
    dpi_bits: u64,
    /// For the raster backend: the hinting asked for and whether the glyph
    /// is axis-aligned; `None` for the unhinted outline.
    raster: Option<(Hinting, bool)>,
}

impl TextEngine {
    /// The unhinted outline of a glyph at `size` points and `dpi`, in
    /// pixels with the glyph origin at `(0, 0)` (matplotlib's
    /// `FT2Font.get_path`). Outlines are cached.
    pub fn glyph_outline(
        &self,
        font: FontId,
        glyph: GlyphId,
        size: f64,
        dpi: f64,
    ) -> Result<Path, TextError> {
        self.outline(font, glyph, size, dpi, None)
    }

    /// The outline of a glyph as matplotlib's Agg backend renders it with
    /// `hinting` (`FT2Font._render_glyph`): grid-fitted by the font's
    /// bytecode or by the autohinter, as `FreeType` would hint it, or
    /// unhinted. `axis_aligned` tells whether the glyph is drawn at a
    /// rotation by a multiple of 90 degrees (only such glyphs get
    /// `FreeType`'s autohinter). Outlines are cached.
    pub fn raster_glyph_outline(
        &self,
        font: FontId,
        glyph: GlyphId,
        size: f64,
        dpi: f64,
        hinting: Hinting,
        axis_aligned: bool,
    ) -> Result<Path, TextError> {
        self.outline(font, glyph, size, dpi, Some((hinting, axis_aligned)))
    }

    /// The outline of a glyph loaded as [`raster_glyph_outline`] loads it,
    /// as `FreeType`'s monochrome rasterizer receives it: tagged points in
    /// 26.6 pixels with the drop-out rules the loader set (matplotlib's
    /// `text.antialiased: False`).
    ///
    /// [`raster_glyph_outline`]: Self::raster_glyph_outline
    ///
    /// # Errors
    ///
    /// When the font cannot be read or the glyph cannot be loaded.
    pub fn scan_glyph_outline(
        &self,
        font: FontId,
        glyph: GlyphId,
        size: f64,
        dpi: f64,
        hinting: Hinting,
        axis_aligned: bool,
    ) -> Result<ScanOutline, TextError> {
        let face = self.db.face(font)?;
        let size = Size::new(&face, size, dpi);
        Ok(
            match self.hinted_glyph(&face, &size, font, glyph, hinting, axis_aligned)? {
                Some(hinted) => hinted.scan_outline(),
                None => size.scan_outline(&face, glyph)?,
            },
        )
    }

    /// `glyph` loaded with the hinter `hinting` picks for it, or `None`
    /// when it stays unhinted.
    fn hinted_glyph(
        &self,
        face: &Face<'_>,
        size: &Size,
        font: FontId,
        glyph: GlyphId,
        hinting: Hinting,
        axis_aligned: bool,
    ) -> Result<Option<HintedGlyph>, TextError> {
        Ok(match hinting.hinter(face, axis_aligned) {
            Hinter::Bytecode => Some(HintingInstance::new(face, size).load_glyph(face, glyph)?),
            Hinter::Autohinter => Some(self.db.autohinter(font)?.load_glyph(
                face,
                size,
                glyph,
                AutohintTarget::Normal,
            )?),
            Hinter::Unhinted => None,
        })
    }

    /// A glyph outline in pixels, unhinted or as the raster backend loads
    /// it, from the cache when possible.
    fn outline(
        &self,
        font: FontId,
        glyph: GlyphId,
        size: f64,
        dpi: f64,
        raster: Option<(Hinting, bool)>,
    ) -> Result<Path, TextError> {
        let key = OutlineKey {
            font,
            glyph,
            size_bits: size.to_bits(),
            dpi_bits: dpi.to_bits(),
            raster,
        };
        let mut cache = self.outlines.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(path) = cache.get(&key) {
            return Ok(path.clone());
        }
        let face = self.db.face(font)?;
        let size = Size::new(&face, size, dpi);
        let mut sink = PathSink::default();
        let hinted = match raster {
            Some((hinting, axis_aligned)) => {
                self.hinted_glyph(&face, &size, font, glyph, hinting, axis_aligned)?
            }
            None => None,
        };
        match hinted {
            Some(hinted) => hinted.outline(&mut sink),
            None => size.outline(&face, glyph, &mut sink)?,
        }
        let path = sink.finish();
        cache.insert(key, path.clone());
        Ok(path)
    }
}

/// Collects font outline commands into a [`Path`].
#[derive(Debug, Default)]
pub(crate) struct PathSink {
    builder: PathBuilder,
}

impl PathSink {
    pub(crate) fn finish(self) -> Path {
        self.builder.build()
    }

    fn update(&mut self, f: impl FnOnce(PathBuilder) -> PathBuilder) {
        self.builder = f(std::mem::take(&mut self.builder));
    }
}

fn point(x: f32, y: f32) -> Point {
    Point::new(f64::from(x), f64::from(y))
}

impl OutlineSink for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.update(|b| b.move_to(point(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.update(|b| b.line_to(point(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.update(|b| b.quad_to(point(x1, y1), point(x, y)));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.update(|b| b.cubic_to(point(x1, y1), point(x2, y2), point(x, y)));
    }

    /// Closes the contour with a vertex at the glyph origin, as
    /// matplotlib's `FT2Font.get_path` does.
    fn close(&mut self) {
        self.update(|b| b.close_at(Point::ORIGIN));
    }
}

#[cfg(test)]
mod tests {
    use plt_font::FontDatabase;

    use super::*;
    use crate::engine::FontRequest;

    #[test]
    fn raster_outlines_follow_the_hinting() {
        let engine = TextEngine::bundled();
        let run = engine
            .layout_line("o", &FontRequest::dejavu_sans(10.0), 100.0)
            .expect("bundled font");
        let glyph = run.glyphs[0];
        let outline = |hinting, axis_aligned| {
            engine
                .raster_glyph_outline(glyph.font, glyph.glyph, 10.0, 100.0, hinting, axis_aligned)
                .expect("bundled font")
        };
        let unhinted = engine
            .glyph_outline(glyph.font, glyph.glyph, 10.0, 100.0)
            .expect("bundled font");
        assert_eq!(outline(Hinting::None, true), unhinted);
        assert_ne!(outline(Hinting::Default, true), unhinted);
        // FORCE_AUTOHINT gives axis-aligned glyphs to the autohinter and
        // rotated ones to the bytecode, as FreeType does.
        let autohinted = outline(Hinting::Auto, true);
        assert_ne!(autohinted, unhinted);
        assert_ne!(autohinted, outline(Hinting::Default, true));
        assert_eq!(
            outline(Hinting::Auto, false),
            outline(Hinting::Native, true)
        );
    }

    #[test]
    fn outlines_are_cached_paths() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let run = engine
            .layout_line("o", &FontRequest::dejavu_sans(12.0), 72.0)
            .expect("bundled font");
        let glyph = run.glyphs[0];
        let outline = engine
            .glyph_outline(glyph.font, glyph.glyph, 12.0, 72.0)
            .expect("outline");
        assert!(!outline.is_empty());
        let again = engine
            .glyph_outline(glyph.font, glyph.glyph, 12.0, 72.0)
            .expect("cached");
        assert_eq!(outline, again);
    }
}
