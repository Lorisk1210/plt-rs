//! Measuring and laying out the lines of a text block, with mathtext for
//! the lines matplotlib treats as math.
//!
//! Lines are measured without the text's features and language
//! (matplotlib's `get_text_width_height_descent` never sees them); a text
//! drawn as a single plain line is laid out again with them for drawing,
//! as matplotlib's backends receive them with single-line texts only.

use pltrs_core::{Affine2D, Path, Point};
use pltrs_text::{
    BlockLayout, BlockStyle, FontHeightMetrics, Language, LineMetrics, LineSpacing, PlacedGlyph,
    Shaping, TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE, TextEngine, TextError, TextRun, TextSource,
    layout_block,
};

use super::font_notes;
use super::math::{Line, math_style};
use super::metrics::without_hinting;
use super::metrics_cache;
use super::overrides::{self, LineKind};
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
    /// The font the block was laid out in.
    pub font: FontProperties,
    /// How its lines were typeset.
    pub(crate) typesetting: Typesetting,
}

/// matplotlib's `get_text_width_height_descent` for one line: with `math`
/// the string is typeset as mathtext (extents rounded up to whole pixels,
/// malformed math is an error), otherwise it is shaped as is. The glyphs
/// are hinted as the current `text.hinting` says, which matplotlib's Agg
/// backend reads when it draws, whatever the parameters of `rc`.
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
    line_run(&line, font, dpi, rc, true).map(|run| metrics(&run))
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
    text_path_shaped(xy, text, size, font, &Shaping::default())
}

/// [`text_path`] with the OpenType features and language of `shaping`
/// for a line that is not math (matplotlib's `TextToPath.get_text_path`
/// with its `features` and `language`).
///
/// # Errors
///
/// When the math cannot be typeset, no font is found, or a language
/// range reaches outside the line.
pub fn text_path_shaped(
    xy: Point,
    text: &str,
    size: Option<f64>,
    font: &FontProperties,
    shaping: &Shaping,
) -> Result<Path, TextError> {
    let rc = crate::rc::params();
    let mut typeset = font.clone();
    typeset.size = TEXT_PATH_FONT_SCALE;
    let line = Line::classify(text, rc.text_parse_math);
    let run = match &line {
        Line::Plain(plain) if !shaping.is_default() => TextSource::Plain {
            text: plain.to_string(),
            font: typeset.request(&rc),
            shaping: shaping.clone(),
        }
        .layout(TextEngine::global(), TEXT_PATH_FONT_SCALE, TEXT_PATH_DPI),
        _ => without_hinting(|| line_run(&line, &typeset, TEXT_PATH_DPI, &rc, true)),
    }?;
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
                    // Drawing the line notes its lookup (see `font_notes`).
                    _ => without_hinting(|| {
                        line_run(
                            &Line::classify(line, parse_math),
                            &typeset,
                            TEXT_PATH_DPI,
                            rc,
                            false,
                        )
                    }),
                }?;
                Ok(scaled_run(&path_run, scale, dpi, run.source.clone()))
            })
            .collect::<Result<Vec<_>, TextError>>()?;
        Ok(Self {
            runs,
            ..self.clone()
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

/// One (non-empty) line typeset in `font` at `dpi`, with the extents the
/// engine gives it.
fn typeset(
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<TextRun, TextError> {
    font_file_opens(font)?;
    let engine = TextEngine::global();
    match line {
        Line::Plain(text) => engine.layout_line(text, &font.drawn_request(), dpi),
        Line::Math(source) => engine.layout_math(source, &math_style(font, dpi, rc)),
    }
}

/// A line as it is measured: the string a renderer's
/// `get_text_width_height_descent` receives and how it is typeset.
pub(super) fn measured_as<'a>(line: &'a Line<'_>) -> (&'a str, LineKind) {
    match line {
        Line::Plain(text) => (text, LineKind::Plain),
        Line::Math(source) => (source, LineKind::Math),
    }
}

/// The extent of one (non-empty) line as pltrs measures it in the
/// thread's measuring mode, without the thread's measurer or metrics
/// cache: what a miss of the cache measures.
///
/// # Errors
///
/// When the math is malformed or no font is found.
pub(super) fn measure_uncached(
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<LineMetrics, TextError> {
    let run = measured_line(typeset(line, font, dpi, rc)?, line, font, dpi, true)?;
    Ok(metrics(&run))
}

/// `run`, `line` laid out in `font` at `dpi`, with the extent the
/// thread's measuring mode reports: measured by the backend's program in
/// its mode (see [`with_line_measure`](super::with_line_measure)), else
/// by the text engine, which, when `noting`, notes what measuring the
/// line notes in matplotlib.
fn measured_line(
    run: TextRun,
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    noting: bool,
) -> Result<TextRun, TextError> {
    if let Some(measure) = super::metrics::line_measure() {
        let (text, _) = measured_as(line);
        return Ok(with_metrics(run, measure(text, font, dpi)?));
    }
    let run = super::metrics::measured(run, dpi)?;
    if noting {
        font_notes::note_measured(line, font, &run)?;
    }
    Ok(run)
}

/// Lays out one (non-empty) line, measured by the thread's
/// [`LineMeasurer`](super::LineMeasurer) if it has one, else through the
/// thread's [`MetricsCache`](super::MetricsCache) if one is installed; a
/// line `measuring` that the cache does not hold notes what matplotlib's
/// measuring notes (see [`font_notes`]). The line is measured at the
/// resolution [`set_measured_at`](crate::artist::set_measured_at) set,
/// cached under `dpi` (matplotlib's cache key is the figure's).
fn line_run(
    line: &Line<'_>,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
    measuring: bool,
) -> Result<TextRun, TextError> {
    let run = typeset(line, font, dpi, rc)?;
    let (text, kind) = measured_as(line);
    let apart = crate::artist::measured_apart(dpi);
    if let Some(metrics) = overrides::measure(text, font, kind, apart.unwrap_or(dpi)) {
        return Ok(with_metrics(run, metrics));
    }
    // The run measured on a miss of the renderer's cache comes back to
    // its slot; on a hit, the run takes the cached metrics.
    let mut slot = Some(run);
    let metrics = metrics_cache::cached(text, font, kind, dpi, || {
        let run = slot.take().expect("measured at most once");
        if let Some(at) = apart {
            slot = Some(run);
            let other = measured_line(typeset(line, font, at, rc)?, line, font, at, measuring)?;
            return Ok(metrics(&other));
        }
        let run = measured_line(run, line, font, dpi, measuring)?;
        let measured = metrics(&run);
        slot = Some(run);
        Ok(measured)
    })?;
    let run = slot.expect("the run is back in its slot");
    Ok(with_metrics(run, metrics))
}

/// One line of `text` typeset as `kind` in `font` at `dpi`, as a
/// renderer's `draw_text` and `draw_tex` receive it (their `s`, `prop` and
/// `ismath`), noting the font lookup drawing it makes in matplotlib.
///
/// # Errors
///
/// When the math is malformed, no font is found or TeX fails.
pub fn typeset_line(
    text: &str,
    font: &FontProperties,
    kind: LineKind,
    dpi: f64,
    rc: &RcParams,
) -> Result<TextRun, TextError> {
    let run = match kind {
        LineKind::Plain => line_run(&Line::Plain(text.into()), font, dpi, rc, false),
        LineKind::Math => line_run(&Line::Math(text), font, dpi, rc, false),
        LineKind::Tex => tex_run(text, font, dpi, rc),
    }?;
    font_notes::note_drawn(&run)?;
    Ok(run)
}

/// `run` with the width, height and descent of `metrics`.
fn with_metrics(run: TextRun, metrics: LineMetrics) -> TextRun {
    TextRun {
        width: metrics.width,
        height: metrics.height,
        descent: metrics.descent,
        ..run
    }
}

/// Lays out `text` (lines separated by `\n`) at `dpi` with matplotlib's
/// line metrics, stacking and alignment rules; lines with math (see
/// [`Line::classify`]) are typeset as mathtext when `rc` enables
/// `text.parse_math`, every line by TeX when it enables `text.usetex`. A
/// single plain line is drawn in the language of `text.language`, which
/// matplotlib gives every text it creates. The glyphs are hinted as the
/// current `text.hinting` says (see [`measure_text`]).
pub(crate) fn lay_out(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
) -> Result<LaidOutText, TextError> {
    lay_out_as(text, font, style, dpi, rc, Typesetting::from_rc(rc))
}

/// [`lay_out`] typeset as `typesetting` says (a text's own `parse_math`
/// and `usetex` properties).
pub(crate) fn lay_out_as(
    text: &str,
    font: &FontProperties,
    style: &BlockStyle,
    dpi: f64,
    rc: &RcParams,
    typesetting: Typesetting,
) -> Result<LaidOutText, TextError> {
    let shaping = Shaping {
        features: Vec::new(),
        language: rc_language(rc),
    };
    lay_out_with(text, font, style, dpi, rc, typesetting)?.shaped(&shaping)
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
                    font: font.drawn_request(),
                    shaping: Shaping::default(),
                }),
                ..TextRun::default()
            }),
            Typesetting::Mathtext { parse_math } => {
                line_run(&Line::classify(line, parse_math), font, dpi, rc, true)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    font_notes::note_layout(text, font, typesetting)?;
    let line_metrics: Vec<LineMetrics> = runs.iter().map(metrics).collect();
    let (font_metrics, style) = match typesetting {
        Typesetting::Tex => (
            match overrides::font_height(font, typesetting, dpi) {
                Some(measured) => measured,
                None => tex_height_metrics(font, dpi, rc)?,
            },
            BlockStyle {
                linespacing: LineSpacing::Normal,
                ..*style
            },
        ),
        Typesetting::Mathtext { .. } => (mathtext_height_metrics(font, typesetting, dpi)?, *style),
    };
    Ok(LaidOutText {
        lines,
        block: layout_block(&line_metrics, font_metrics, &style),
        runs,
        style,
        font: font.clone(),
        typesetting,
    })
}

/// The font height metrics of lines that are not typeset by TeX: those of
/// the PDF and PostScript backends' AFM fonts while they measure with
/// them, else those of the thread's measurer if it has one, else the
/// font's own (matplotlib's `Text._get_layout`).
fn mathtext_height_metrics(
    font: &FontProperties,
    typesetting: Typesetting,
    dpi: f64,
) -> Result<FontHeightMetrics, TextError> {
    if !super::metrics::measures_afm()
        && let Some(measured) = overrides::font_height(font, typesetting, dpi)
    {
        return Ok(measured);
    }
    font_file_opens(font)?;
    super::metrics::font_height_metrics(&font.drawn_request(), dpi)
}

/// Fails as matplotlib's `FT2Font` fails to open the font file `font`
/// names, when it names one that does not exist or holds no font (a
/// request for it sets the text in the font its families find instead).
fn font_file_opens(font: &FontProperties) -> Result<(), TextError> {
    font.file
        .as_deref()
        .map_or(Ok(()), |file| TextEngine::global().open_font_file(file))
}

#[cfg(test)]
mod tests {
    use pltrs_text::Feature;

    use super::*;
    use crate::text::TextHinting;

    /// The spec's reference configuration (`matplotlib.testing.setup`).
    fn reference() -> RcParams {
        let mut rc = RcParams::matplotlib();
        rc.font_family = vec!["DejaVu Sans".to_owned()];
        rc.text_hinting = TextHinting::None;
        rc
    }

    /// Runs `f` with the reference configuration as the current
    /// parameters, whose `text.hinting` text is laid out with.
    fn under_reference(f: impl FnOnce(&RcParams)) {
        crate::rc::context(|| {
            let rc = reference();
            crate::rc::replace(rc.clone());
            f(&rc);
        });
    }

    #[test]
    fn math_lines_round_up_to_whole_pixels() {
        under_reference(|rc| {
            let font = FontProperties::from_rc(rc);
            // matplotlib: get_text_width_height_descent('$x^2$', ismath=True)
            // at 100 dpi is (16, 13, 0) with dejavusans.
            let math = measure_text("$x^2$", &font, true, 100.0, rc).expect("valid math");
            assert_eq!((math.width, math.height, math.descent), (16.0, 13.0, 0.0));
            assert!(measure_text("$x^$", &font, true, 100.0, rc).is_err());
            let plain = measure_text("$x^$", &font, false, 100.0, rc).expect("plain text");
            assert!(plain.width.fract() != 0.0, "unhinted widths are fractional");
        });
    }

    #[test]
    fn text_is_hinted_as_the_current_parameters_say() {
        let mut hinted = reference();
        hinted.text_hinting = TextHinting::ForceAutohint;
        let font = FontProperties::from_rc(&hinted);
        under_reference(|_| {
            // Hinted, the line is a whole number of pixels wide.
            let width = measure_text("$x^$", &font, false, 100.0, &hinted)
                .expect("plain text")
                .width;
            assert!(
                width.fract() != 0.0,
                "unhinted, as the current parameters say"
            );
        });
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
    fn text_paths_follow_their_features() {
        let font = FontProperties::from_rc(&reference());
        let path = |features: Vec<Feature>| {
            let shaping = Shaping {
                features,
                language: None,
            };
            text_path_shaped(Point::ORIGIN, "fi", None, &font, &shaping).expect("plain text")
        };
        let ligature = path(Vec::new());
        assert_eq!(
            ligature,
            text_path(Point::ORIGIN, "fi", None, &font).expect("plain text")
        );
        let no_ligature: Feature = "-liga".parse().expect("a feature");
        assert_ne!(path(vec![no_ligature]), ligature);
    }

    #[test]
    fn text_path_runs_keep_the_lines_and_rescale_the_glyphs() {
        under_reference(|rc| {
            let font = FontProperties::from_rc(rc);
            let laid_out = lay_out("AVA\n\nWAV", &font, &BlockStyle::default(), 100.0, rc)
                .expect("valid text");
            let paths = laid_out
                .as_text_paths(&font, true, 100.0, rc)
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
        });
    }
}
