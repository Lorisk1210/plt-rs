//! The process-wide text engine: fonts, line layout and glyph outlines.

mod outline;
mod request;

pub use outline::PathSink;
pub use outline::{TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE};
pub use request::FontRequest;

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use pltrs_core::Path;
use pltrs_font::{FontDatabase, FontEntry, FontId, ScanOutline};

use crate::block::FontHeightMetrics;
use crate::error::TextError;
use crate::mathtext::MathStyle;
use crate::memo::{self, Memo};
use crate::missing::{self, MissingGlyph};
use crate::run::{GlyphChars, PlacedGlyph, TextRun, TextSource};
use crate::shaping::Shaping;

/// Fonts plus layout services shared by every figure and backend; the
/// layouts it makes are kept and given again for the same inputs.
#[derive(Debug)]
pub struct TextEngine {
    db: FontDatabase,
    outlines: Mutex<HashMap<outline::OutlineKey, Path>>,
    scan_outlines: Mutex<HashMap<outline::OutlineKey, ScanOutline>>,
    lines: Memo<LineKey, TextRun>,
    pub(crate) formulas: Memo<FormulaKey, TextRun>,
    heights: Memo<(FontRequest, u64), FontHeightMetrics>,
}

/// The inputs of a single-line layout.
#[derive(Debug)]
struct LineKey {
    text: String,
    request: FontRequest,
    shaping: Shaping,
    dpi_bits: u64,
}

/// The inputs of a mathtext layout.
#[derive(Debug)]
pub(crate) struct FormulaKey {
    pub(crate) source: String,
    pub(crate) style: MathStyle,
}

impl TextEngine {
    /// An engine over `db`.
    pub fn new(db: FontDatabase) -> Self {
        Self {
            db,
            outlines: Mutex::new(HashMap::new()),
            scan_outlines: Mutex::new(HashMap::new()),
            lines: Memo::new(),
            formulas: Memo::new(),
            heights: Memo::new(),
        }
    }

    /// The shared engine: the bundled fonts (matplotlib's and Inter) plus
    /// the fonts installed on the system.
    pub fn global() -> &'static Self {
        static ENGINE: OnceLock<TextEngine> = OnceLock::new();
        ENGINE.get_or_init(|| {
            let db = FontDatabase::with_bundled_fonts();
            db.add_system_fonts();
            Self::new(db)
        })
    }

    /// An engine over the bundled fonts only, ignoring the fonts installed
    /// on the system (for host-independent results).
    pub fn bundled() -> &'static Self {
        static ENGINE: OnceLock<TextEngine> = OnceLock::new();
        ENGINE.get_or_init(|| Self::new(FontDatabase::with_bundled_fonts()))
    }

    /// The font catalogue.
    pub fn database(&self) -> &FontDatabase {
        &self.db
    }

    /// Adds the fonts of the file at `path` to the catalogue (matplotlib's
    /// `fontManager.addfont`); layouts made before are forgotten, as the
    /// fonts they matched may have changed.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or parsed.
    pub fn add_font_file(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<FontId>, TextError> {
        let faces = self.db.add_file(path)?;
        self.forget_layouts();
        Ok(faces)
    }

    /// Appends `entry` to the catalogue (matplotlib's `ttflist.append`);
    /// layouts made before are forgotten.
    pub fn add_font_entry(&self, entry: FontEntry) {
        self.db.add_entry(entry);
        self.forget_layouts();
    }

    /// Removes the first catalogue entry equal to `entry` (matplotlib's
    /// `ttflist.remove`), forgetting the layouts made before; whether there
    /// was one.
    pub fn remove_font_entry(&self, entry: &FontEntry) -> bool {
        let removed = self.db.remove_entry(entry);
        if removed {
            self.forget_layouts();
        }
        removed
    }

    /// The faces of the font file `path` (or bundled file name) for text
    /// set in it whatever its query (matplotlib's
    /// `FontProperties(fname=...)`), read on first use.
    ///
    /// # Errors
    ///
    /// When the file cannot be read or parsed.
    pub fn file_faces(&self, path: impl AsRef<std::path::Path>) -> Result<Vec<FontId>, TextError> {
        Ok(self.db.file_faces(path)?)
    }

    /// Forgets every memoised layout.
    fn forget_layouts(&self) {
        self.lines.clear();
        self.formulas.clear();
        self.heights.clear();
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
        let dpi_bits = dpi.to_bits();
        let extra = (
            text,
            shaping.features.len(),
            shaping.language.is_some(),
            dpi_bits,
        );
        let run = self.lines.get_or_compute(
            memo::fingerprint(request, extra),
            |key| {
                key.dpi_bits == dpi_bits
                    && key.text == text
                    && key.request == *request
                    && key.shaping == *shaping
            },
            || LineKey {
                text: text.to_owned(),
                request: request.clone(),
                shaping: shaping.clone(),
                dpi_bits,
            },
            || self.shape_line(text, request, shaping, dpi),
        )?;
        self.record_missing(&run, text, request)?;
        Ok(run)
    }

    /// Records the characters of `run`, laid out from `text`, that no font
    /// of `request` has: those drawn from the Last Resort font, or as the
    /// missing-glyph box (matplotlib warns about them at every layout).
    fn record_missing(
        &self,
        run: &TextRun,
        text: &str,
        request: &FontRequest,
    ) -> Result<(), TextError> {
        let resort = self.db.last_resort();
        if !run
            .glyphs
            .iter()
            .any(|g| g.glyph.0 == 0 || Some(g.font) == resort)
        {
            return Ok(());
        }
        let fonts = request.fonts(&self.db)?;
        let mut families: Vec<String> = Vec::new();
        for &font in &fonts {
            if let Some(name) = self.db.face(font)?.family_name()
                && !families.contains(&name)
            {
                families.push(name);
            }
        }
        missing::record(
            run.glyphs
                .iter()
                .filter(|g| g.glyph.0 == 0 || !fonts.contains(&g.font))
                .filter_map(|g| match g.chars {
                    GlyphChars::Cluster { start, .. } => text.get(start..)?.chars().next(),
                    GlyphChars::Unknown | GlyphChars::Code(_) => None,
                })
                .map(|character| MissingGlyph {
                    character,
                    families: families.clone(),
                }),
        );
        Ok(())
    }

    /// [`layout_line_with`](Self::layout_line_with) without the memo.
    fn shape_line(
        &self,
        text: &str,
        request: &FontRequest,
        shaping: &Shaping,
        dpi: f64,
    ) -> Result<TextRun, TextError> {
        let options = shaping.shape_options(text)?;
        let query = &request.query;
        let fonts = request.fonts(&self.db)?;
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
        let dpi_bits = dpi.to_bits();
        self.heights.get_or_compute(
            memo::fingerprint(request, dpi_bits),
            |(stored, bits)| *bits == dpi_bits && stored == request,
            || (request.clone(), dpi_bits),
            || self.measure_font_height(request, dpi),
        )
    }

    /// [`font_height_metrics`](Self::font_height_metrics) without the memo.
    fn measure_font_height(
        &self,
        request: &FontRequest,
        dpi: f64,
    ) -> Result<FontHeightMetrics, TextError> {
        let query = &request.query;
        let face = self.db.face(request.font(&self.db)?)?;
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

#[cfg(test)]
mod tests {
    use pltrs_font::{FontQuery, Hinting};

    use super::*;

    #[test]
    fn added_fonts_are_matched_and_old_layouts_forgotten() {
        let engine = TextEngine::new(FontDatabase::new());
        let request = FontRequest::new(FontQuery::families(["DejaVu Sans"]));
        assert!(
            engine
                .layout_line_with("A", &request, &Shaping::default(), 72.0)
                .is_err()
        );
        let path =
            std::env::temp_dir().join(format!("pltrs-text-addfont-{}.ttf", std::process::id()));
        std::fs::write(
            &path,
            pltrs_font::bundled_font_data("DejaVuSans.ttf").unwrap(),
        )
        .unwrap();
        assert_eq!(engine.add_font_file(&path).unwrap().len(), 1);
        let run = engine
            .layout_line_with("A", &request, &Shaping::default(), 72.0)
            .unwrap();
        assert_eq!(run.glyphs.len(), 1);
        std::fs::remove_file(&path).unwrap();
        assert!(engine.add_font_file(&path).is_err());
    }

    #[test]
    fn missing_characters_fall_back_as_asked() {
        let engine = TextEngine::global();
        let resort = engine.db.last_resort();
        let font_of = |last_resort: bool| {
            let request = FontRequest {
                last_resort,
                ..FontRequest::dejavu_sans(12.0)
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
    fn missing_characters_are_recorded_at_every_layout() {
        let engine = TextEngine::global();
        let request = FontRequest::dejavu_sans(12.0);
        crate::take_missing_glyphs();
        engine.layout_line("ab", &request, 72.0).unwrap();
        assert_eq!(crate::take_missing_glyphs(), Vec::new());
        let expected = MissingGlyph {
            character: '\u{1f643}',
            families: vec!["DejaVu Sans".to_owned()],
        };
        for _ in 0..2 {
            engine.layout_line("a\u{1f643}b", &request, 72.0).unwrap();
            assert_eq!(
                crate::take_missing_glyphs(),
                std::slice::from_ref(&expected)
            );
        }
    }

    #[test]
    fn matches_matplotlib_text_extent() {
        // matplotlib: fig.text(0.5, 0.5, 'AVA To') at 10 pt / 100 dpi is
        // 45.765625 px wide (spec text_metrics probes).
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let run = engine
            .layout_line("AVA To", &FontRequest::dejavu_sans(10.0), 100.0)
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
            ..FontRequest::dejavu_sans(10.0)
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
    fn glyphs_know_their_clusters() {
        let engine = TextEngine::bundled();
        let chars = |text: &str| -> Vec<String> {
            let run = engine
                .layout_line(text, &FontRequest::dejavu_sans(10.0), 72.0)
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
    fn layouts_are_kept_by_their_inputs() {
        let engine = TextEngine::new(FontDatabase::with_bundled_fonts());
        let request = FontRequest::new(FontQuery {
            size: 10.0,
            ..FontQuery::families(["DejaVu Sans"])
        });
        let first = engine.layout_line("Kept", &request, 100.0).unwrap();
        let again = engine.layout_line("Kept", &request, 100.0).unwrap();
        let fresh = engine
            .shape_line("Kept", &request, &Shaping::default(), 100.0)
            .unwrap();
        assert_eq!((&first, &again), (&fresh, &fresh));
        let denser = engine.layout_line("Kept", &request, 200.0).unwrap();
        assert!(
            denser.width > first.width,
            "another resolution is another layout"
        );
        let heights = engine.font_height_metrics(&request, 100.0).unwrap();
        assert_eq!(
            heights,
            engine.measure_font_height(&request, 100.0).unwrap()
        );
    }
}
