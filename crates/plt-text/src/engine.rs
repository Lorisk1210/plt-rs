//! The process-wide text engine: fonts, line layout and glyph outlines.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use plt_core::{Path, PathBuilder, Point};
use plt_font::{
    AutohintTarget, Face, FontDatabase, FontId, FontQuery, GenericFamilies, GlyphId, HintedGlyph,
    Hinter, Hinting, HintingInstance, OutlineSink, ScanOutline, Size,
};

use crate::block::FontHeightMetrics;
use crate::error::TextError;
use crate::run::{GlyphChars, PlacedGlyph, TextRun, TextSource};
use crate::shaping::Shaping;

/// A font query plus the generic-family lists (`font.sans-serif`, ...)
/// in effect when it is resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct FontRequest {
    /// Families, style, weight, stretch and size.
    pub query: FontQuery,
    /// Preference lists for generic family names.
    pub generics: GenericFamilies,
    /// Whether characters no family covers come from the Last Resort font
    /// rather than the first family's missing-glyph box (matplotlib's
    /// `font.enable_last_resort`).
    pub last_resort: bool,
    /// How glyphs are grid-fitted when the text is laid out and rasterized
    /// (matplotlib's `text.hinting` for the Agg backend; vector output and
    /// text paths are never hinted).
    pub hinting: Hinting,
}

impl FontRequest {
    /// A request resolving generic families with matplotlib's defaults,
    /// unhinted.
    pub fn new(query: FontQuery) -> Self {
        Self {
            query,
            generics: GenericFamilies::default(),
            last_resort: true,
            hinting: Hinting::None,
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
    /// For the raster backend: the hinting asked for and whether the glyph
    /// is axis-aligned; `None` for the unhinted outline.
    raster: Option<(Hinting, bool)>,
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

    /// An engine over matplotlib's bundled fonts only, ignoring the fonts
    /// installed on the system (for host-independent results).
    pub fn bundled() -> &'static Self {
        static ENGINE: OnceLock<TextEngine> = OnceLock::new();
        ENGINE.get_or_init(|| Self::new(FontDatabase::with_bundled_fonts()))
    }

    /// The font catalogue.
    pub fn database(&self) -> &FontDatabase {
        &self.db
    }

    /// Lays out a single line of plain text (matplotlib's
    /// `FT2Font.set_text` with font fallback and the request's hinting):
    /// glyph positions and the width/height/descent matplotlib reports, all
    /// in pixels.
    ///
    /// Like matplotlib, whose layout engine (libraqm) shapes a single
    /// paragraph, only the text up to and including the first paragraph
    /// separator (`\n`, `\r`, `\r\n`, U+001C–U+001E, U+0085, U+2029) is
    /// laid out.
    pub fn layout_line(
        &self,
        text: &str,
        request: &FontRequest,
        dpi: f64,
    ) -> Result<TextRun, TextError> {
        self.layout_line_with(text, request, &Shaping::default(), dpi)
    }

    /// [`layout_line`](Self::layout_line) with OpenType features and a
    /// language (`FT2Font.set_text(text, features=..., language=...)`);
    /// language ranges count the characters of the whole `text`.
    ///
    /// # Errors
    ///
    /// When the fonts cannot be loaded, or a language range reaches
    /// outside `text` ([`TextError::LanguageRange`]).
    pub fn layout_line_with(
        &self,
        text: &str,
        request: &FontRequest,
        shaping: &Shaping,
        dpi: f64,
    ) -> Result<TextRun, TextError> {
        let options = shaping.shape_options(text)?;
        let query = &request.query;
        let fonts = self.db.find_fonts_with(query, &request.generics)?;
        let chain = self.db.font_chain(&fonts, request.last_resort)?;
        let paragraph = first_paragraph(text);
        let layout = chain.layout(paragraph, query.size, dpi, &options, request.hinting)?;
        let resort = self.db.last_resort();
        let font_of = |index: usize| {
            fonts
                .get(index)
                .copied()
                .or(resort)
                .expect("chain faces come from `fonts` or the last-resort font")
        };
        let clusters = ClusterEnds::new(layout.glyphs.iter().map(|g| g.cluster), paragraph.len());
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
                    chars: GlyphChars::Cluster {
                        start: g.cluster,
                        end: clusters.end_of(g.cluster),
                    },
                })
                .collect(),
            rects: Vec::new(),
            width: px(layout.width),
            height: px(layout.height),
            descent: px(layout.descent),
            dpi,
            source: Some(TextSource::Plain {
                text: text.to_owned(),
                font: request.clone(),
                shaping: shaping.clone(),
            }),
            anchor: None,
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

/// Where the clusters of a shaped text end: each cluster runs up to the
/// next cluster start in logical order (matplotlib's `LayoutItem.char` is
/// that whole substring for every glyph of the cluster).
struct ClusterEnds {
    starts: Vec<usize>,
    len: usize,
}

impl ClusterEnds {
    /// The clusters starting at `starts` (in any order) of a text `len`
    /// bytes long.
    fn new(starts: impl Iterator<Item = usize>, len: usize) -> Self {
        let mut starts: Vec<usize> = starts.collect();
        starts.sort_unstable();
        starts.dedup();
        Self { starts, len }
    }

    /// The end of the cluster starting at `start`.
    fn end_of(&self, start: usize) -> usize {
        let next = self.starts.partition_point(|&s| s <= start);
        self.starts.get(next).copied().unwrap_or(self.len)
    }
}

/// Whether `c` separates paragraphs (Unicode bidi class `B`).
fn is_paragraph_separator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{1c}'..='\u{1e}' | '\u{85}' | '\u{2029}')
}

/// The first paragraph of `text`, including the separator that ends it
/// (`\r\n` is a single separator).
fn first_paragraph(text: &str) -> &str {
    let Some((index, separator)) = text
        .char_indices()
        .find(|&(_, c)| is_paragraph_separator(c))
    else {
        return text;
    };
    let mut end = index + separator.len_utf8();
    if separator == '\r' && text[end..].starts_with('\n') {
        end += 1;
    }
    &text[..end]
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
    fn missing_characters_fall_back_as_asked() {
        let engine = TextEngine::global();
        let resort = engine.db.last_resort();
        let font_of = |last_resort: bool| {
            let request = FontRequest {
                last_resort,
                ..dejavu(12.0)
            };
            let run = engine.layout_line("\u{4e00}", &request, 72.0).unwrap();
            (Some(run.glyphs[0].font), run.glyphs[0].glyph)
        };
        assert_eq!(font_of(true).0, resort, "the Last Resort font draws it");
        let (font, glyph) = font_of(false);
        assert_ne!(font, resort);
        assert_eq!(glyph.0, 0, "the missing-glyph box of DejaVu Sans");
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
    fn hinted_layout_matches_matplotlib() {
        // matplotlib with `text.hinting: default`: get_text_width_height_descent
        // and `FT2Font._layout` positions at 10 pt / 100 dpi.
        let engine = TextEngine::bundled();
        let request = FontRequest {
            hinting: Hinting::Default,
            ..dejavu(10.0)
        };
        let run = engine
            .layout_line("AVA To", &request, 100.0)
            .expect("bundled font");
        assert_eq!(
            (run.width, run.height, run.descent),
            (47.859_375, 10.0, 0.0)
        );
        let xs: Vec<f64> = run.glyphs.iter().map(|g| g.x).collect();
        assert_eq!(
            xs,
            [0.0, 9.109_375, 18.218_75, 28.218_75, 32.218_75, 38.859_375]
        );
    }

    #[test]
    fn raster_outlines_follow_the_hinting() {
        let engine = TextEngine::bundled();
        let run = engine
            .layout_line("o", &dejavu(10.0), 100.0)
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
    fn glyphs_know_their_clusters() {
        let engine = TextEngine::bundled();
        let chars = |text: &str| -> Vec<String> {
            let run = engine
                .layout_line(text, &dejavu(10.0), 72.0)
                .expect("bundled font");
            run.glyphs
                .iter()
                .map(|g| run.glyph_text(g).expect("known").into_owned())
                .collect()
        };
        // matplotlib: [item.char for item in _text_helpers.layout(s, font)].
        assert_eq!(chars("x\u{301}y"), ["x\u{301}", "x\u{301}", "y"]);
        assert_eq!(
            chars("\u{644}\u{627} b"),
            ["b", " ", "\u{644}\u{627}"],
            "visual order"
        );
        let ends = ClusterEnds::new([4, 0, 2, 2].into_iter(), 6);
        assert_eq!((ends.end_of(0), ends.end_of(2), ends.end_of(4)), (2, 4, 6));
    }

    #[test]
    fn only_the_first_paragraph_is_laid_out() {
        assert_eq!(first_paragraph("a\rb"), "a\r");
        assert_eq!(first_paragraph("a\r\nb"), "a\r\n");
        assert_eq!(first_paragraph("a\r\rb"), "a\r");
        assert_eq!(first_paragraph("a\u{2029}b"), "a\u{2029}");
        assert_eq!(first_paragraph("a\tb"), "a\tb");
        assert_eq!(first_paragraph(""), "");
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
