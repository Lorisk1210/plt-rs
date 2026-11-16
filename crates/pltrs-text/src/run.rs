//! Laid-out text ready for any backend.

use std::borrow::Cow;

use pltrs_core::{Affine2D, Bbox, Path, Point};
use pltrs_font::{FontId, GlyphId, Hinting};

use crate::block::{HAlign, RotationMode, VAlign};
use crate::engine::{FontRequest, TextEngine};
use crate::error::TextError;
use crate::mathtext::MathStyle;
use crate::shaping::Shaping;
use crate::tex::TexText;

/// The characters a glyph renders (matplotlib's `LayoutItem.char` for
/// shaped text, the character code of a mathtext glyph).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlyphChars {
    /// Not known (the run was not laid out by the engine).
    #[default]
    Unknown,
    /// The cluster of shaped text spanning these byte offsets of the
    /// run's [`TextSource`] text; every glyph of a cluster renders all of
    /// it.
    Cluster {
        /// Offset of the cluster's first byte.
        start: usize,
        /// Offset just past the cluster's last byte.
        end: usize,
    },
    /// The character code mathtext looked the glyph up by.
    Code(u32),
}

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
    /// The characters the glyph renders.
    pub chars: GlyphChars,
}

/// What a run was laid out from: the `s`, `prop` and `ismath` arguments
/// matplotlib's backends receive in `draw_text`, and the features and
/// language of the `mtext` artist. Vector backends use it to name the
/// text, to embed fonts by character and to lay the text out again at
/// another size.
#[derive(Clone, Debug, PartialEq)]
pub enum TextSource {
    /// Shaped text (`ismath=False`).
    Plain {
        /// The line as laid out.
        text: String,
        /// Its font.
        font: FontRequest,
        /// Its features and language.
        shaping: Shaping,
    },
    /// Mathtext (`ismath=True`).
    Math {
        /// The line, `$` delimiters included.
        text: String,
        /// Its fonts and resolution.
        style: MathStyle,
        /// What matplotlib's mathtext says parsing it (see
        /// [`MathLayout::logged`](crate::mathtext::MathLayout::logged)).
        logged: Vec<crate::MathNote>,
    },
    /// TeX text (`ismath="TeX"`, drawn by `draw_tex`): the run carries
    /// no glyphs, only the extent of the typeset line; backends typeset
    /// it themselves.
    Tex(TexText),
}

impl TextSource {
    /// The string that was laid out.
    pub fn text(&self) -> &str {
        match self {
            Self::Plain { text, .. } | Self::Math { text, .. } => text,
            Self::Tex(tex) => &tex.tex,
        }
    }

    /// Whether the run is typeset mathtext.
    pub fn is_math(&self) -> bool {
        matches!(self, Self::Math { .. })
    }

    /// Whether the run is TeX text, typeset by `latex`.
    pub fn is_tex(&self) -> bool {
        matches!(self, Self::Tex(_))
    }

    /// The text font (for mathtext, the font of text outside `$...$`,
    /// whose size is the size of the math).
    pub fn font(&self) -> &FontRequest {
        match self {
            Self::Plain { font, .. } => font,
            Self::Math { style, .. } => &style.font,
            Self::Tex(tex) => &tex.font,
        }
    }

    /// The font size in points (matplotlib's `prop.get_size_in_points()`).
    pub fn size(&self) -> f64 {
        self.font().query.size
    }

    /// Lays the text out again, unhinted, at `points` and `dpi` with the
    /// same features and language (with `points` 100 and `dpi` 72 this is
    /// matplotlib's `TextToPath`, which vector backends and path effects
    /// use). TeX text is typeset at `points` and measured at `dpi`; its
    /// run has no glyphs.
    ///
    /// # Errors
    ///
    /// When the fonts cannot be loaded (or, for mathtext, the math does
    /// not parse, which cannot happen for a source that was laid out
    /// before), or TeX fails.
    pub fn layout(&self, engine: &TextEngine, points: f64, dpi: f64) -> Result<TextRun, TextError> {
        match self {
            Self::Plain {
                text,
                font,
                shaping,
            } => {
                let mut font = font.clone();
                font.query.size = points;
                font.hinting = Hinting::None;
                engine.layout_line_with(text, &font, shaping, dpi)
            }
            Self::Math { text, style, .. } => {
                let mut style = style.clone().with_size(points).with_dpi(dpi);
                style.font.hinting = Hinting::None;
                engine.layout_math(text, &style)
            }
            Self::Tex(tex) => {
                let (width, height, descent) =
                    tex.manager
                        .text_width_height_descent(&tex.tex, points, dpi / 72.0)?;
                let mut font = tex.font.clone();
                font.query.size = points;
                Ok(TextRun {
                    width,
                    height,
                    descent,
                    dpi,
                    source: Some(Self::Tex(TexText {
                        font,
                        ..tex.clone()
                    })),
                    ..TextRun::default()
                })
            }
        }
    }
}

/// Where the text artist drawing a run is anchored, as matplotlib's
/// backends learn it from the artist (`mtext`) they receive with
/// single-line texts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextAnchor {
    /// The anchor in display coordinates.
    pub point: Point,
    /// Horizontal alignment at the anchor.
    pub halign: HAlign,
    /// Vertical alignment at the anchor.
    pub valign: VAlign,
    /// How rotation and alignment combine.
    pub rotation_mode: RotationMode,
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
    /// What the run was laid out from (`None` for runs not laid out by
    /// the engine).
    pub source: Option<TextSource>,
    /// The anchor of the single-line text artist drawing the run, if it
    /// is one (set when drawing).
    pub anchor: Option<TextAnchor>,
}

impl TextRun {
    /// Whether nothing would be drawn (TeX text is typeset when drawn, so
    /// its runs count as drawn).
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
            && self.rects.is_empty()
            && !self.source.as_ref().is_some_and(TextSource::is_tex)
    }

    /// The characters `glyph` renders, if known.
    pub fn glyph_text(&self, glyph: &PlacedGlyph) -> Option<Cow<'_, str>> {
        match glyph.chars {
            GlyphChars::Unknown => None,
            GlyphChars::Cluster { start, end } => self
                .source
                .as_ref()
                .and_then(|source| source.text().get(start..end))
                .map(Cow::Borrowed),
            GlyphChars::Code(code) => Some(Cow::Owned(
                char::from_u32(code)
                    .unwrap_or(char::REPLACEMENT_CHARACTER)
                    .to_string(),
            )),
        }
    }

    /// The glyph outlines and rules as one compound path in pixels
    /// relative to the origin, glyphs first, each rule a closed rectangle
    /// starting at its bottom-left corner and going up first (matplotlib's
    /// `TextToPath.get_text_path`); glyphs `engine` has no outline for are
    /// left out. Like matplotlib, every glyph is loaded at the size the
    /// text was laid out at, and the smaller ones of mathtext (sub- and
    /// superscripts) are scaled down from there.
    pub fn outline(&self, engine: &TextEngine) -> Path {
        let laid_out_at = self.source.as_ref().map(TextSource::size);
        let glyphs = self.glyphs.iter().filter_map(|glyph| {
            let loaded_at = laid_out_at.unwrap_or(glyph.size);
            let outline = engine
                .glyph_outline(glyph.font, glyph.glyph, loaded_at, self.dpi)
                .ok()?;
            let scale = glyph.size / loaded_at;
            let placement = Affine2D::scaling(scale, scale).translate(glyph.x, glyph.y);
            Some(outline.transformed(&placement))
        });
        let rects = self.rects.iter().map(|rect| {
            let (x, y, w, h) = rect.bounds();
            Path::compound_from_polygons(&[[
                Point::new(x, y),
                Point::new(x, y + h),
                Point::new(x + w, y + h),
                Point::new(x + w, y),
                Point::new(x, y),
            ]])
        });
        let parts: Vec<Path> = glyphs.chain(rects).collect();
        Path::compound(&parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_glyphs_render_their_clusters() {
        let engine = TextEngine::bundled();
        let run = engine
            .layout_line("office", &FontRequest::dejavu_sans(10.0), 72.0)
            .expect("bundled font");
        let chars: Vec<String> = run
            .glyphs
            .iter()
            .map(|g| run.glyph_text(g).expect("known").into_owned())
            .collect();
        // matplotlib: [item.char for item in layout("office", font)].
        assert_eq!(chars, ["o", "ffi", "c", "e"]);
        let source = run.source.as_ref().expect("laid out by the engine");
        assert_eq!(source.text(), "office");
        assert!(!source.is_math());
        assert_eq!(source.size(), 10.0);
    }

    #[test]
    fn sources_lay_out_again_at_other_sizes() {
        let engine = TextEngine::bundled();
        let run = engine
            .layout_line("1.0", &FontRequest::dejavu_sans(10.0), 72.0)
            .expect("bundled font");
        let at_100 = run
            .source
            .as_ref()
            .expect("laid out by the engine")
            .layout(engine, 100.0, 72.0)
            .expect("bundled font");
        // matplotlib's SVG output places '.' at 63.625 and '0' at 95.40625.
        let xs: Vec<f64> = at_100.glyphs.iter().map(|g| g.x).collect();
        assert_eq!(xs, [0.0, 63.625, 95.406_25]);
        assert_eq!(at_100.source.map(|s| s.size()), Some(100.0));
    }

    #[test]
    fn sources_keep_their_features_and_language() {
        let engine = TextEngine::bundled();
        let shaping = Shaping {
            features: vec!["-liga".parse().expect("valid")],
            language: None,
        };
        let run = engine
            .layout_line_with("office", &FontRequest::dejavu_sans(10.0), &shaping, 72.0)
            .expect("bundled font");
        assert_eq!(run.glyphs.len(), 6, "no ffi ligature");
        let source = run.source.expect("laid out by the engine");
        let again = source.layout(engine, 100.0, 72.0).expect("bundled font");
        assert_eq!(again.glyphs.len(), 6);
        assert!(matches!(again.source, Some(TextSource::Plain { shaping: s, .. }) if s == shaping));
        let catalan = Shaping {
            features: Vec::new(),
            language: Some("ca".into()),
        };
        let run = engine
            .layout_line_with("office", &FontRequest::dejavu_sans(10.0), &catalan, 72.0)
            .expect("bundled font");
        assert_eq!(run.glyphs.len(), 6, "Catalan has no liga in DejaVu Sans");
        let outside = Shaping {
            features: Vec::new(),
            language: Some(vec![pltrs_font::LanguageRange::new("sr", 0, 7)].into()),
        };
        assert!(matches!(
            engine.layout_line_with("office", &FontRequest::dejavu_sans(10.0), &outside, 72.0),
            Err(TextError::LanguageRange {
                start: 0,
                end: 7,
                ..
            })
        ));
    }

    #[test]
    fn math_glyphs_render_their_codes_and_rules_go_up_first() {
        let engine = TextEngine::bundled();
        let style = MathStyle::default();
        let run = engine
            .layout_math(r"$\frac{1}{2}$", &style)
            .expect("valid math");
        let chars: Vec<String> = run
            .glyphs
            .iter()
            .map(|g| run.glyph_text(g).expect("known").into_owned())
            .collect();
        assert_eq!(chars, ["1", "2"]);
        assert!(run.source.as_ref().is_some_and(TextSource::is_math));
        let outline = run.outline(engine);
        let rule = &outline.vertices()[outline.vertices().len() - 6..];
        let bar = run.rects[0];
        assert_eq!(rule[0], Point::new(bar.x0(), bar.y0()));
        assert_eq!(rule[1], Point::new(bar.x0(), bar.y1()));
        assert_eq!(rule[3], Point::new(bar.x1(), bar.y0()));
    }

    #[test]
    fn math_glyphs_are_scaled_from_the_laid_out_size() {
        let engine = TextEngine::bundled();
        let run = engine
            .layout_math(r"$a_a$", &MathStyle::default())
            .expect("valid math");
        let base = run.source.as_ref().expect("laid out by the engine").size();
        let sub = run.glyphs[1];
        assert!(sub.size < base, "the subscript is smaller");
        let loaded = engine
            .glyph_outline(sub.font, sub.glyph, base, run.dpi)
            .expect("bundled glyph");
        let scale = sub.size / base;
        let expected = loaded.transformed(&Affine2D::scaling(scale, scale).translate(sub.x, sub.y));
        let outline = run.outline(engine);
        let first = run.glyphs[0];
        let skip = engine
            .glyph_outline(first.font, first.glyph, base, run.dpi)
            .expect("bundled glyph")
            .vertices()
            .len();
        assert_eq!(
            &outline.vertices()[skip..skip + expected.vertices().len()],
            expected.vertices()
        );
    }

    #[test]
    fn runs_without_source_know_no_characters() {
        let mut run = TextEngine::bundled()
            .layout_line("a", &FontRequest::dejavu_sans(10.0), 72.0)
            .expect("bundled font");
        run.source = None;
        let glyph = run.glyphs[0];
        assert_eq!(glyph.chars, GlyphChars::Cluster { start: 0, end: 1 });
        assert_eq!(run.glyph_text(&glyph), None);
        let unknown = PlacedGlyph {
            chars: GlyphChars::Unknown,
            ..glyph
        };
        assert_eq!(run.glyph_text(&unknown), None);
    }
}
