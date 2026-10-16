//! The process-wide text engine: fonts, line layout and glyph outlines.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use plt_core::{Path, PathBuilder, Point};
use plt_font::{
    FontDatabase, FontId, FontQuery, GenericFamilies, GlyphId, OutlineSink, ShapeOptions, Size,
};

use crate::block::FontHeightMetrics;
use crate::error::TextError;
use crate::run::{PlacedGlyph, TextRun};

/// A font query plus the generic-family lists (`font.sans-serif`, ...)
/// in effect when it is resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct FontRequest {
    /// Families, style, weight, stretch and size.
    pub query: FontQuery,
    /// Preference lists for generic family names.
    pub generics: GenericFamilies,
}

impl FontRequest {
    /// A request resolving generic families with matplotlib's defaults.
    pub fn new(query: FontQuery) -> Self {
        Self {
            query,
            generics: GenericFamilies::default(),
        }
    }
}

/// Fonts plus layout services shared by every figure and backend.
#[derive(Debug)]
pub struct TextEngine {
    db: FontDatabase,
    outlines: Mutex<HashMap<OutlineKey, Path>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct OutlineKey {
    font: FontId,
    glyph: GlyphId,
    size_bits: u64,
    dpi_bits: u64,
}

impl TextEngine {
    /// An engine over `db`.
    pub fn new(db: FontDatabase) -> Self {
        Self {
            db,
            outlines: Mutex::new(HashMap::new()),
        }
    }

    /// The shared engine: matplotlib's bundled fonts plus the fonts
    /// installed on the system.
    pub fn global() -> &'static Self {
        static ENGINE: OnceLock<TextEngine> = OnceLock::new();
        ENGINE.get_or_init(|| {
            let mut db = FontDatabase::with_bundled_fonts();
            db.add_system_fonts();
            Self::new(db)
        })
    }

    /// The font catalogue.
    pub fn database(&self) -> &FontDatabase {
        &self.db
    }

    /// Lays out a single line of plain text (matplotlib's
    /// `FT2Font.set_text` with font fallback): glyph positions and the
    /// width/height/descent matplotlib reports, all in pixels.
    pub fn layout_line(
        &self,
        text: &str,
        request: &FontRequest,
        dpi: f64,
    ) -> Result<TextRun, TextError> {
        let query = &request.query;
        let fonts = self.db.find_fonts_with(query, &request.generics)?;
        let chain = self.db.font_chain(&fonts, true)?;
        let layout = chain.layout(text, query.size, dpi, &ShapeOptions::default())?;
        let resort = self.db.last_resort();
        let font_of = |index: usize| {
            fonts
                .get(index)
                .copied()
                .or(resort)
                .expect("chain faces come from `fonts` or the last-resort font")
        };
        let px = |v: i32| f64::from(v) / 64.0;
        Ok(TextRun {
            glyphs: layout
                .glyphs
                .iter()
                .map(|g| PlacedGlyph {
                    font: font_of(g.face),
                    glyph: g.glyph_id,
                    size: query.size,
                    x: px(g.x),
                    y: px(g.y),
                })
                .collect(),
            rects: Vec::new(),
            width: px(layout.width),
            height: px(layout.height),
            descent: px(layout.descent),
            dpi,
        })
    }

    /// Minimum line ascent, descent and line gap for text in `query`, from
    /// the primary font's `OS/2` typo metrics (else `hhea`), as
    /// matplotlib's `Text._get_layout` computes them.
    pub fn font_height_metrics(
        &self,
        request: &FontRequest,
        dpi: f64,
    ) -> Result<FontHeightMetrics, TextError> {
        let query = &request.query;
        let fonts = self.db.find_fonts_with(query, &request.generics)?;
        let face = self.db.face(fonts[0])?;
        let scale = 1.0 / f64::from(face.units_per_em()) * query.size * dpi / 72.0;
        let (gap, ascent, descent) = face.os2().map_or_else(
            || {
                let hhea = face.hhea();
                (hhea.line_gap, hhea.ascender, hhea.descender)
            },
            |os2| (os2.typo_line_gap, os2.typo_ascender, os2.typo_descender),
        );
        Ok(FontHeightMetrics {
            ascent: f64::from(ascent) * scale,
            descent: -f64::from(descent) * scale,
            line_gap: f64::from(gap) * scale,
        })
    }

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
        let key = OutlineKey {
            font,
            glyph,
            size_bits: size.to_bits(),
            dpi_bits: dpi.to_bits(),
        };
        let mut cache = self.outlines.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(path) = cache.get(&key) {
            return Ok(path.clone());
        }
        let face = self.db.face(font)?;
        let mut sink = PathSink::default();
        Size::new(&face, size, dpi).outline(&face, glyph, &mut sink)?;
        let path = sink.finish();
        cache.insert(key, path.clone());
        Ok(path)
    }
}

/// Collects font outline commands into a [`Path`].
#[derive(Debug, Default)]
struct PathSink {
    builder: PathBuilder,
}

impl PathSink {
    fn finish(self) -> Path {
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

    fn close(&mut self) {
        self.update(PathBuilder::close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dejavu(size: f64) -> FontRequest {
        FontRequest::new(FontQuery {
            size,
            ..FontQuery::families(["DejaVu Sans"])
        })
    }

    #[test]
    fn matches_matplotlib_text_extent() {
        // matplotlib: fig.text(0.5, 0.5, 'AVA To') at 10 pt / 100 dpi is
        // 45.765625 px wide (spec text_metrics probes).
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let run = engine
            .layout_line("AVA To", &dejavu(10.0), 100.0)
            .expect("bundled font");
        assert_eq!(run.width, 45.765_625);
        assert!(run.height > 0.0 && run.descent >= 0.0);
        assert_eq!(run.glyphs.len(), 6);
    }

    #[test]
    fn outlines_are_cached_paths() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let run = engine
            .layout_line("o", &dejavu(12.0), 72.0)
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
