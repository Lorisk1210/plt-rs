//! Measuring and laying out the lines of a text block, with mathtext for
//! the lines matplotlib treats as math.
//!
//! Lines are measured without the text's features and language
//! (matplotlib's `get_text_width_height_descent` never sees them); a text
//! drawn as a single plain line is laid out again with them for drawing,
//! as matplotlib's backends receive them with single-line texts only.

use pltrs_core::{Affine2D, Path, Point};
use pltrs_text::{
    BlockLayout, BlockStyle, Language, LineMetrics, LineSpacing, PlacedGlyph, Shaping,
    TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE, TextEngine, TextError, TextRun, TextSource, layout_block,
};

use super::math::{Line, math_style};
use super::metrics::without_hinting;
use super::tex::{tex_height_metrics, tex_line, tex_manager, tex_run};
use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// How the lines of a text are typeset: by mathtext where they hold math
/// (when math parsing is on), or all by TeX (matplotlib's `Text` with
/// `parse_math` and `usetex`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Typesetting {
    /// Plain text, with mathtext for the lines holding `$...$` math when
    /// `parse_math` is set.
    Mathtext {
        /// Whether `$...$` is typeset.
        parse_math: bool,
    },
    /// Every line typeset by LaTeX (`usetex`).
    Tex,
}

impl Typesetting {
    /// The typesetting of a text with these `parse_math` and `usetex`
    /// properties.
    pub(crate) fn of(parse_math: bool, usetex: bool) -> Self {
        if usetex {
            Self::Tex
        } else {
            Self::Mathtext { parse_math }
        }
    }

    /// The typesetting `rc` gives texts (`text.parse_math`,
    /// `text.usetex`).
    pub(crate) fn from_rc(rc: &RcParams) -> Self {
        Self::of(rc.text_parse_math, rc.text_usetex)
    }
}

/// A text block laid out for drawing: per-line runs plus their placement.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidOutText {
    /// The lines of the block as given (before any math processing).
    pub lines: Vec<String>,
    /// Placement of every line relative to the anchor (pixels).
    pub block: BlockLayout,
    /// The glyph run of every line, in the same order; its origin is the
    /// left end of the line's baseline.
    pub runs: Vec<TextRun>,
    /// The alignment and rotation the block was laid out with.
    pub style: BlockStyle,
}

/// matplotlib's `get_text_width_height_descent` for one line: with `math`
/// the string is typeset as mathtext (extents rounded up to whole pixels,
/// malformed math is an error), otherwise it is shaped as is.
pub fn measure_text(
    text: &str,
    font: &FontProperties,
    math: bool,
    dpi: f64,
    rc: &RcParams,
) -> Result<LineMetrics, TextError> {
    let line = if math {
        Line::Math(text)
    } else {
        Line::Plain(text.into())
    };
    line_run(&line, font, dpi, rc).map(|run| metrics(&run))
}

/// The outline of one line of `text` with its baseline starting at `xy`,
/// in the units of `xy` with the font `size` units tall (`None` = the
/// font's size): typeset with `font` (as mathtext when it holds math and
/// `text.parse_math` is on) at 100 points and 72 dpi, unhinted, then
/// scaled (matplotlib's `TextPath(xy, text, size, prop)`).
///
/// # Errors
///
/// When the math cannot be typeset or no font is found.
pub fn text_path(
    xy: Point,
    text: &str,
    size: Option<f64>,
    font: &FontProperties,
) -> Result<Path, TextError> {
    let rc = crate::rc::params();
    let mut typeset = font.clone();
    typeset.size = TEXT_PATH_FONT_SCALE;
    let run = without_hinting(|| {
        line_run(
            &Line::classify(text, rc.text_parse_math),
            &typeset,
            TEXT_PATH_DPI,
            &rc,
        )
    })?;
    let scale = size.unwrap_or(font.size) / TEXT_PATH_FONT_SCALE;
    Ok(run
        .outline(TextEngine::global())
        .transformed(&Affine2D::scaling(scale, scale).translate(xy.x, xy.y)))
}

/// The outline of one line of `text` typeset by LaTeX with its baseline
/// starting at `xy`, in the units of `xy` with the font `size` units tall
/// (`None` = the font's size): typeset at 100 points, every glyph loaded
/// from its Type 1 font with light hinting, then scaled (matplotlib's
/// `TextPath(xy, text, size, prop, usetex=True)`). Only the size of `font`
/// matters to TeX; the fonts come from the rcParams (`font.family` and the
/// family lists, `text.latex.preamble`).
///
/// # Errors
///
/// When TeX is missing or fails on the text, or a font it uses has no
/// Type 1 version.
pub fn text_path_tex(
    xy: Point,
    text: &str,
    size: Option<f64>,
    font: &FontProperties,
) -> Result<Path, TextError> {
    let rc = crate::rc::params();
    let tex = pltrs_text::tex::TexText::new(tex_line(text), font.request(&rc), tex_manager(&rc)?);
    let scale = size.unwrap_or(font.size) / TEXT_PATH_FONT_SCALE;
    Ok(tex
        .text_path()?
        .transformed(&Affine2D::scaling(scale, scale).translate(xy.x, xy.y)))
}

impl LaidOutText {
    /// The same text as drawn with `shaping`: a block of a single plain
    /// line has that line laid out again, in the same font, with the
    /// features and language (matplotlib's backends lay out the line they
    /// draw with the artist's settings, and only receive the artist for
    /// single-line texts). The placement of the lines does not change: it
    /// comes from the measurement, which never sees the settings.
    ///
    /// # Errors
    ///
    /// When a language range reaches outside the line.
    pub(crate) fn shaped(mut self, shaping: &Shaping) -> Result<Self, TextError> {
        if shaping.is_default() {
            return Ok(self);
        }
        if let [run] = self.runs.as_mut_slice()
            && let Some(TextSource::Plain { text, font, .. }) = &run.source
        {
            *run = TextEngine::global().layout_line_with(text, font, shaping, run.dpi)?;
        }
        Ok(self)
    }

    /// The same text with the glyphs of each line placed as matplotlib
    /// draws text through path effects (`RendererBase._draw_text_as_path`):
    /// typeset by `TextToPath` at 100 points and 72 dpi, unhinted (with the
    /// features and language a line was drawn with), then scaled to
    /// `font`'s size at `dpi`. The lines stay where they are; only the
    /// glyph positions within them change, as advances and kerning round
    /// differently at that size.
    ///
    /// # Errors
    ///
    /// When a line's math cannot be typeset.
    pub(crate) fn as_text_paths(
        &self,
        font: &FontProperties,
        parse_math: bool,
        dpi: f64,
        rc: &RcParams,
    ) -> Result<Self, TextError> {
        let mut typeset = font.clone();
        typeset.size = TEXT_PATH_FONT_SCALE;
        let scale = font.size * dpi / 72.0 / TEXT_PATH_FONT_SCALE;
        let runs = self
            .lines
            .iter()
            .zip(&self.runs)
            .map(|(line, run)| {
                // TeX lines are drawn as paths from their own typesetting.
                if line.is_empty() || run.source.as_ref().is_some_and(TextSource::is_tex) {
                    return Ok(run.clone());
                }
                let path_run = match &run.source {
                    Some(source @ TextSource::Plain { shaping, .. }) if !shaping.is_default() => {
                        source.layout(TextEngine::global(), TEXT_PATH_FONT_SCALE, TEXT_PATH_DPI)
                    }
                    _ => without_hinting(|| {
                        line_run(
                            &Line::classify(line, parse_math),
                            &typeset,
                            TEXT_PATH_DPI,
                            rc,
                        )
                    }),
                }?;
                Ok(scaled_run(&path_run, scale, dpi, run.source.clone()))
            })
            .collect::<Result<Vec<_>, TextError>>()?;
        Ok(Self {
            lines: self.lines.clone(),
            block: self.block.clone(),
            runs,
            style: self.style,
        })
    }
}

/// `run` (laid out at the text-path resolution) scaled by `scale` and
/// expressed at `dpi`: positions, glyph sizes and rules grow by `scale`;
/// `source` is what the scaled run shows.
fn scaled_run(run: &TextRun, scale: f64, dpi: f64, source: Option<TextSource>) -> TextRun {
    TextRun {
        glyphs: run
            .glyphs
            .iter()
            .map(|glyph| PlacedGlyph {
                size: glyph.size * scale * TEXT_PATH_DPI / dpi,
                x: glyph.x * scale,
                y: glyph.y * scale,
                ..*glyph
            })
            .collect(),
        rects: run
            .rects
            .iter()
            .map(|rect| rect.transformed(&Affine2D::scaling(scale, scale)))
            .collect(),
        width: run.width * scale,
        height: run.height * scale,
        descent: run.descent * scale,
        dpi,
        source,
        anchor: None,
    }
}

/// The width, height and descent of a run.
fn metrics(run: &TextRun) -> LineMetrics {
    LineMetrics {
        width: run.width,
        height: run.height,
        descent: run.descent,
    }
}

/// Lays out one (non-empty) line.
fn line_run(
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<TextRun, TextError> {
    let engine = TextEngine::global();
    let run = match line {
        Line::Plain(text) => engine.layout_line(text, &font.request(rc), dpi),
        Line::Math(source) => engine.layout_math(source, &math_style(font, dpi, rc)),
    }?;
    super::metrics::measured(run, dpi)
}

/// Lays out `text` (lines separated by `\n`) at `dpi` with matplotlib's
/// line metrics, stacking and alignment rules; lines with math (see
/// [`Line::classify`]) are typeset as mathtext when `rc` enables
/// `text.parse_math`, every line by TeX when it enables `text.usetex`. A
/// single plain line is drawn in the language of `text.language`, which
/// matplotlib gives every text it creates.
pub(crate) fn lay_out(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
) -> Result<LaidOutText, TextError> {
    let shaping = Shaping {
        features: Vec::new(),
        language: rc_language(rc),
    };
    lay_out_with(text, font, style, dpi, rc, Typesetting::from_rc(rc))?.shaped(&shaping)
}

/// The language matplotlib gives the texts it creates: `text.language`.
pub(crate) fn rc_language(rc: &RcParams) -> Option<Language> {
    rc.text_language.as_deref().map(Language::from)
}

/// [`lay_out`] with the typesetting chosen explicitly (a text's
/// `parse_math` and `usetex` properties). TeX lines all stack with the
/// normal spacing, at least as tall as TeX's `"lp"`, as matplotlib's
/// `Text._get_layout` stacks them.
pub(crate) fn lay_out_with(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
    typesetting: Typesetting,
) -> Result<LaidOutText, TextError> {
    let lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    let runs = lines
        .iter()
        .map(|line| match typesetting {
            Typesetting::Tex => tex_run(line, font, dpi, rc),
            Typesetting::Mathtext { .. } if line.is_empty() => Ok(TextRun {
                dpi,
                source: Some(TextSource::Plain {
                    text: String::new(),
                    font: font.request(rc),
                    shaping: Shaping::default(),
                }),
                ..TextRun::default()
            }),
            Typesetting::Mathtext { parse_math } => {
                line_run(&Line::classify(line, parse_math), font, dpi, rc)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let line_metrics: Vec<LineMetrics> = runs.iter().map(metrics).collect();
    let (font_metrics, style) = match typesetting {
        Typesetting::Tex => (
            tex_height_metrics(font, dpi, rc)?,
            BlockStyle {
                linespacing: LineSpacing::Normal,
                ..*style
            },
        ),
        Typesetting::Mathtext { .. } => (
            super::metrics::font_height_metrics(&font.request(rc), dpi)?,
            *style,
        ),
    };
    Ok(LaidOutText {
        lines,
        block: layout_block(&line_metrics, font_metrics, &style),
        runs,
        style,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::TextHinting;

    /// The spec's reference configuration (`matplotlib.testing.setup`).
    fn reference() -> RcParams {
        let mut rc = RcParams::matplotlib();
        rc.font_family = vec!["DejaVu Sans".to_owned()];
        rc.text_hinting = TextHinting::None;
        rc
    }

    #[test]
    fn math_lines_round_up_to_whole_pixels() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        // matplotlib: get_text_width_height_descent('$x^2$', ismath=True)
        // at 100 dpi is (16, 13, 0) with dejavusans.
        let math = measure_text("$x^2$", &font, true, 100.0, &rc).expect("valid math");
        assert_eq!((math.width, math.height, math.descent), (16.0, 13.0, 0.0));
        assert!(measure_text("$x^$", &font, true, 100.0, &rc).is_err());
        let plain = measure_text("$x^$", &font, false, 100.0, &rc).expect("plain text");
        assert!(plain.width.fract() != 0.0);
    }

    #[test]
    fn lines_are_classified_independently() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let laid_out = lay_out(
            "$x^2$\nprice \\$5\n",
            &font,
            &BlockStyle::default(),
            100.0,
            &rc,
        )
        .expect("valid text");
        assert_eq!(laid_out.lines, ["$x^2$", "price \\$5", ""]);
        assert_eq!(laid_out.runs[0].width, 16.0);
        let dollars = measure_text("price $5", &font, false, 100.0, &rc).expect("plain");
        assert_eq!(laid_out.runs[1].width, dollars.width);
        assert!(laid_out.runs[2].is_empty());
    }

    #[test]
    fn shaping_relays_out_single_plain_lines_only() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let style = BlockStyle::default();
        let lay = |text: &str| lay_out(text, &font, &style, 100.0, &rc).expect("valid text");
        let shaping = Shaping {
            features: vec!["-liga".parse().expect("valid feature")],
            language: None,
        };
        let single = lay("office");
        assert_eq!(single.runs[0].glyphs.len(), 4, "o, ffi, c, e");
        let shaped = single.clone().shaped(&shaping).expect("valid text");
        assert_eq!(shaped.runs[0].glyphs.len(), 6);
        assert_eq!(shaped.block, single.block, "placed as measured");
        let paths = shaped
            .as_text_paths(&font, true, 100.0, &rc)
            .expect("valid text");
        assert_eq!(paths.runs[0].glyphs.len(), 6, "paths keep the features");
        for unshaped in [lay("office\noffice"), lay("$office$")] {
            let again = unshaped.clone().shaped(&shaping);
            assert_eq!(again, Ok(unshaped));
        }
        let outside = Shaping {
            features: Vec::new(),
            language: Some(vec![pltrs_text::LanguageRange::new("sr", 0, 9)].into()),
        };
        assert!(matches!(
            single.shaped(&outside),
            Err(TextError::LanguageRange { .. })
        ));
    }

    #[test]
    fn the_rc_language_shapes_laid_out_lines() {
        let mut rc = reference();
        rc.text_language = Some("ca".to_owned());
        let font = FontProperties::from_rc(&rc);
        let laid_out =
            lay_out("office", &font, &BlockStyle::default(), 100.0, &rc).expect("valid text");
        // DejaVu Sans' Catalan language system has no ligatures.
        assert_eq!(laid_out.runs[0].glyphs.len(), 6);
        assert_eq!(rc_language(&rc), Some(Language::from("ca")));
        assert_eq!(rc_language(&reference()), None);
    }

    #[test]
    fn text_paths_scale_with_their_size_from_their_origin() {
        let font = FontProperties::from_rc(&reference());
        let small = text_path(Point::ORIGIN, "Ag", Some(1.0), &font).expect("plain text");
        let large = text_path(Point::new(5.0, 2.0), "Ag", Some(3.0), &font).expect("plain text");
        let (small, large) = (small.extents(), large.extents());
        assert!(small.height() > 0.5 && small.height() < 1.5, "{small:?}");
        assert!((large.width() - 3.0 * small.width()).abs() < 1e-9);
        assert!((large.x0() - (5.0 + 3.0 * small.x0())).abs() < 1e-9);
        assert!(small.y0() < 0.0, "the g descends below the baseline");
        assert!(text_path(Point::ORIGIN, r"$\frac$", None, &font).is_err());
    }

    #[test]
    fn text_path_runs_keep_the_lines_and_rescale_the_glyphs() {
        let rc = reference();
        let font = FontProperties::from_rc(&rc);
        let laid_out =
            lay_out("AVA\n\nWAV", &font, &BlockStyle::default(), 100.0, &rc).expect("valid text");
        let paths = laid_out
            .as_text_paths(&font, true, 100.0, &rc)
            .expect("valid text");
        assert_eq!(paths.block, laid_out.block);
        assert!(paths.runs[1].is_empty());
        for (path_run, run) in paths.runs.iter().zip(&laid_out.runs) {
            assert_eq!(path_run.glyphs.len(), run.glyphs.len());
            assert_eq!(path_run.dpi, 100.0);
            for (path_glyph, glyph) in path_run.glyphs.iter().zip(&run.glyphs) {
                assert!((path_glyph.size - glyph.size).abs() < 1e-9);
                assert!(
                    (path_glyph.x - glyph.x).abs() < 0.5,
                    "only the rounding differs"
                );
            }
        }
    }
}
