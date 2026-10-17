//! Font data for embedded fonts: what matplotlib's PDF and PostScript
//! backends read through `FreeType` (`FT2Font`) when they write Type 3 fonts,
//! and the glyph outlines the SVG backend defines.

use std::collections::HashMap;

use plt_core::{Path, PathBuilder, Point};
use plt_font::{Face, FontSource, GlyphId, OutlineSink, Rect, Size, Tag};
use plt_text::{FontId, TextEngine, TextError};

/// One face as the vector backends see it.
#[derive(Debug)]
pub(crate) struct FontInfo {
    face: Face<'static>,
    postscript_name: String,
    sort_key: String,
    /// The smallest character code mapped to each glyph.
    chars: HashMap<GlyphId, u32>,
}

impl FontInfo {
    fn new(engine: &'static TextEngine, id: FontId) -> Result<Self, TextError> {
        let db = engine.database();
        let face = db.face(id)?;
        let postscript_name = face
            .postscript_name()
            .unwrap_or_else(|| "UNAVAILABLE".to_owned());
        let sort_key = match db.source(id) {
            Some(FontSource::File(path)) => path.to_string_lossy().into_owned(),
            Some(FontSource::Bundled(name)) => (*name).to_owned(),
            None => postscript_name.clone(),
        };
        let mut chars = HashMap::new();
        for (code, glyph) in face.mappings() {
            chars
                .entry(glyph)
                .and_modify(|c: &mut u32| *c = (*c).min(code))
                .or_insert(code);
        }
        Ok(Self {
            face,
            postscript_name,
            sort_key,
            chars,
        })
    }

    /// The face itself.
    pub(crate) const fn face(&self) -> &Face<'static> {
        &self.face
    }

    /// `FreeType`'s PostScript name (`"UNAVAILABLE"` when the font has none).
    pub(crate) fn postscript_name(&self) -> &str {
        &self.postscript_name
    }

    /// The font file path matplotlib sorts embedded fonts by.
    pub(crate) fn sort_key(&self) -> &str {
        &self.sort_key
    }

    /// Design units per em.
    pub(crate) fn units_per_em(&self) -> f64 {
        f64::from(self.face.units_per_em())
    }

    /// The union of all glyph boxes from `head` (`FreeType`'s `face->bbox`).
    pub(crate) fn bbox(&self) -> [i32; 4] {
        rect_values(self.face.bbox())
    }

    /// `post.isFixedPitch` (`FreeType`'s `FT_FACE_FLAG_FIXED_WIDTH`).
    pub(crate) fn is_fixed_pitch(&self) -> bool {
        self.face.post().is_some_and(|post| post.is_fixed_pitch)
    }

    /// The low 16 bits of `post.italicAngle`, which matplotlib writes as
    /// the PDF `ItalicAngle`.
    pub(crate) fn italic_angle_fraction(&self) -> u16 {
        let raw = (self.face.italic_angle() * 65536.0).round() as i64;
        (raw & 0xffff) as u16
    }

    /// `capHeight` and `xHeight` from the `PCLT` table (zero without one).
    pub(crate) fn pclt_heights(&self) -> (u16, u16) {
        let read = |table: &[u8], at: usize| {
            table
                .get(at..at + 2)
                .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]))
        };
        self.face
            .table(Tag::new(b"PCLT"))
            .map_or((0, 0), |table| (read(table, 16), read(table, 10)))
    }

    /// matplotlib's `get_glyph_name`: the `post`/`CFF` name, or
    /// `uni%08x` of the glyph index for fonts without names.
    pub(crate) fn glyph_name(&self, glyph: GlyphId) -> String {
        if self.face.has_glyph_names()
            && let Some(name) = self.face.glyph_name(glyph)
        {
            return name.to_owned();
        }
        format!("uni{:08x}", glyph.0)
    }

    /// Horizontal advance in font units.
    pub(crate) fn advance(&self, glyph: GlyphId) -> u16 {
        self.face.glyph_advance(glyph)
    }

    /// Unhinted linear advance at `points` and `dpi`, in pixels.
    pub(crate) fn linear_advance(&self, glyph: GlyphId, points: f64, dpi: f64) -> f64 {
        Size::new(&self.face, points, dpi).linear_advance(&self.face, glyph) as f64 / 65536.0
    }

    /// The unscaled control box (`FreeType`'s `FT_Glyph_Get_CBox` after
    /// loading with `NO_SCALE`), zero for empty glyphs.
    pub(crate) fn control_box(&self, glyph: GlyphId) -> [i32; 4] {
        self.face
            .glyph_bbox(glyph)
            .ok()
            .flatten()
            .map_or([0; 4], rect_values)
    }

    /// The outline loaded with `NO_SCALE`, in font units: `FreeType`
    /// decomposes integer coordinates, so implied on-curve points between
    /// quadratic control points are truncated towards zero.
    pub(crate) fn unscaled_outline(&self, glyph: GlyphId) -> Path {
        let truncate = self.face.table(Tag::new(b"glyf")).is_some();
        let mut sink = PathSink::new(truncate);
        // A glyph that cannot be loaded is drawn as nothing.
        let _ = self.face.outline(glyph, &mut sink);
        sink.finish()
    }

    /// The character code a glyph renders, recovered from the character
    /// map (the smallest code when several map to the glyph).
    pub(crate) fn char_of(&self, glyph: GlyphId) -> Option<char> {
        self.chars
            .get(&glyph)
            .and_then(|&code| char::from_u32(code))
    }
}

fn rect_values(rect: Rect) -> [i32; 4] {
    [rect.x_min, rect.y_min, rect.x_max, rect.y_max]
}

/// Collects outline commands into a [`Path`], optionally truncating every
/// coordinate (`FreeType`'s integer midpoints in font units).
#[derive(Debug)]
struct PathSink {
    builder: PathBuilder,
    truncate: bool,
}

impl PathSink {
    fn new(truncate: bool) -> Self {
        Self {
            builder: PathBuilder::default(),
            truncate,
        }
    }

    fn point(&self, x: f32, y: f32) -> Point {
        let (x, y) = (f64::from(x), f64::from(y));
        if self.truncate {
            Point::new(x.trunc(), y.trunc())
        } else {
            Point::new(x, y)
        }
    }

    fn update(&mut self, f: impl FnOnce(PathBuilder) -> PathBuilder) {
        self.builder = f(std::mem::take(&mut self.builder));
    }

    fn finish(self) -> Path {
        self.builder.build()
    }
}

impl OutlineSink for PathSink {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.update(|b| b.move_to(p));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.point(x, y);
        self.update(|b| b.line_to(p));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (c, p) = (self.point(x1, y1), self.point(x, y));
        self.update(|b| b.quad_to(c, p));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (self.point(x1, y1), self.point(x2, y2), self.point(x, y));
        self.update(|b| b.cubic_to(c1, c2, p));
    }

    fn close(&mut self) {
        self.update(PathBuilder::close);
    }
}

/// The faces used by one document, loaded once each.
#[derive(Debug)]
pub(crate) struct FontCache {
    engine: &'static TextEngine,
    fonts: HashMap<FontId, FontInfo>,
}

impl Default for FontCache {
    fn default() -> Self {
        Self {
            engine: TextEngine::global(),
            fonts: HashMap::new(),
        }
    }
}

impl FontCache {
    /// The text engine the faces come from (the one laying out text).
    pub(crate) const fn engine(&self) -> &'static TextEngine {
        self.engine
    }

    /// The face `id`, loading it on first use.
    pub(crate) fn get(&mut self, id: FontId) -> Result<&FontInfo, TextError> {
        if !self.fonts.contains_key(&id) {
            let info = FontInfo::new(self.engine, id)?;
            self.fonts.insert(id, info);
        }
        Ok(&self.fonts[&id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plt_text::{FontQuery, FontRequest};

    fn dejavu() -> (FontCache, FontId) {
        let cache = FontCache::default();
        let run = cache
            .engine()
            .layout_line(
                "1",
                &FontRequest::new(FontQuery::families(["DejaVu Sans"])),
                72.0,
            )
            .expect("bundled font");
        (cache, run.glyphs[0].font)
    }

    #[test]
    fn reads_what_freetype_reports() {
        let (mut cache, id) = dejavu();
        let font = cache.get(id).expect("bundled font");
        assert_eq!(font.postscript_name(), "DejaVuSans");
        assert_eq!(font.units_per_em(), 2048.0);
        assert_eq!(font.glyph_name(GlyphId(20)), "one");
        assert_eq!(font.char_of(GlyphId(20)), Some('1'));
        assert_eq!(font.advance(GlyphId(20)), 1303);
        assert_eq!(
            font.linear_advance(GlyphId(20), 10.0, 72.0),
            6.362_304_687_5
        );
        assert_eq!(font.sort_key(), "DejaVuSans.ttf");
        assert!(!font.is_fixed_pitch());
        assert_eq!(font.italic_angle_fraction(), 0);
    }

    #[test]
    fn unscaled_outlines_are_integral() {
        let (mut cache, id) = dejavu();
        let font = cache.get(id).expect("bundled font");
        let zero = font.unscaled_outline(GlyphId(19));
        assert!(!zero.is_empty());
        assert!(zero.vertices().iter().all(|p| p.x.fract() == 0.0));
        assert_eq!(font.control_box(GlyphId(3)), [0; 4], "space is empty");
    }
}
