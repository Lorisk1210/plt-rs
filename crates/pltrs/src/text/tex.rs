//! `text.usetex`: texts typeset by LaTeX (matplotlib's `Text` with
//! `usetex=True` and the renderers' `get_text_width_height_descent(...,
//! ismath="TeX")`). Every line is typeset on its own, measured from the
//! DVI file, and handed to the backend as a TeX run, which each backend
//! draws as matplotlib's `draw_tex` does.

use pltrs_text::tex::{DviBox, TexError, TexFonts, TexGlyph, TexManager, TexText};
use pltrs_text::{
    FontHeightMetrics, LineMetrics, TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE, TextError, TextRun,
    TextSource,
};

use super::metrics_cache;
use super::overrides::LineKind;
use crate::fonts::FontProperties;
use crate::rc::{Distiller, RcParams};

/// The TeX typesetter of the rcParams `rc`: the fonts of `font.family`
/// and the family lists, `text.latex.preamble`, `text.latex.engine`,
/// caching under the [cache directory](crate::config::cache_dir).
///
/// # Errors
///
/// When there is no usable cache directory.
pub fn tex_manager(rc: &RcParams) -> Result<TexManager, TexError> {
    let cache_dir = crate::config::cache_dir().map_err(|err| TexError::Io {
        path: "the cache directory".to_owned(),
        message: err.to_string(),
    })?;
    let fonts = TexFonts {
        family: rc.font_family.clone(),
        serif: rc.font_serif.clone(),
        sans_serif: rc.font_sans_serif.clone(),
        cursive: rc.font_cursive.clone(),
        monospace: rc.font_monospace.clone(),
    };
    Ok(TexManager::new(
        &cache_dir,
        fonts,
        &rc.text_latex_preamble,
        rc.text_latex_engine,
    ))
}

/// Whether TeX text can be drawn here: `tex`, `latex` and `dvipng` are
/// installed (matplotlib's test suite checks the same, and Ghostscript,
/// before running its usetex tests; see [`check_usetex_postscript`]).
///
/// # Errors
///
/// Which program is missing.
pub fn check_usetex() -> Result<(), TexError> {
    TexManager::check_installation()
}

/// Whether TeX text can be saved as PostScript here: TeX
/// ([`check_usetex`]), `dvips`, LaTeX's `psfrag` package (found as
/// `matplotlib.testing._has_tex_package` finds it) and Ghostscript
/// (matplotlib's `_checkdep_usetex` warns "usetex mode requires
/// ghostscript.").
///
/// # Errors
///
/// What is missing.
pub fn check_usetex_postscript() -> Result<(), TexError> {
    check_usetex()?;
    let missing = |program: &str| TexError::NotInstalled {
        program: program.to_owned(),
    };
    if pltrs_core::os::which("dvips").is_none() {
        return Err(missing("dvips"));
    }
    let rc = crate::rc::params();
    if tex_manager(&rc)?.files().find("psfrag.sty").is_err() {
        return Err(missing("the psfrag package"));
    }
    crate::figure::check_distiller(Distiller::Ghostscript).map_err(|_| missing("ghostscript"))
}

/// A line as TeX receives it (`Text._preprocess_math` with usetex): a
/// lone space is escaped, as TeX would drop it.
pub(crate) fn tex_line(line: &str) -> &str {
    if line == " " { r"\ " } else { line }
}

/// One line typeset by TeX at `dpi`: a run without glyphs whose extent
/// is the typeset line's (none for an empty line) and whose source is the
/// TeX text backends draw. TeX's fonts, preamble and engine are those of
/// the rcParams in effect now, as matplotlib's `TexManager` reads them
/// whenever it runs TeX (a text made under a style keeps its usetex, not
/// the style's preamble).
pub(crate) fn tex_run(line: &str, font: &FontProperties, dpi: f64) -> Result<TextRun, TextError> {
    let rc = crate::rc::params();
    let tex = TexText::new(tex_line(line), font.drawn_request(), tex_manager(&rc)?);
    let measured = if tex.tex.is_empty() {
        None
    } else {
        super::overrides::measure(&tex.tex, font, LineKind::Tex, dpi)
    };
    let (width, height, descent) = match measured {
        Some(metrics) => (metrics.width, metrics.height, metrics.descent),
        None if tex.tex.is_empty() => (0.0, 0.0, 0.0),
        None => {
            let metrics = metrics_cache::cached(&tex.tex, font, LineKind::Tex, dpi, || {
                tex_metrics(&tex.manager, &tex.tex, font, dpi)
            })?;
            (metrics.width, metrics.height, metrics.descent)
        }
    };
    Ok(TextRun {
        width,
        height,
        descent,
        dpi,
        source: Some(TextSource::Tex(tex)),
        ..TextRun::default()
    })
}

/// The extent of the TeX line `tex` in `font` at `dpi`, typeset by
/// `manager`, or measured by the backend's program in its mode (see
/// [`with_line_measure`](super::with_line_measure)).
fn tex_metrics(
    manager: &TexManager,
    tex: &str,
    font: &FontProperties,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    if let Some(measure) = super::metrics::line_measure() {
        return measure(tex, font, dpi);
    }
    let (width, height, descent) = manager.text_width_height_descent(tex, font.size, dpi / 72.0)?;
    Ok(LineMetrics {
        width,
        height,
        descent,
    })
}

/// Looks the line `line`, typeset by TeX in `font` at `dpi`, up again in
/// the thread's metrics cache as [`tex_run`] does, measuring it again
/// with the rcParams in effect now if the cache no longer holds it (see
/// [`LaidOutText::measure_again`](super::LaidOutText::measure_again)).
///
/// # Errors
///
/// When the line is measured again and TeX fails.
pub(super) fn measure_tex_again(
    line: &str,
    font: &FontProperties,
    dpi: f64,
) -> Result<(), TextError> {
    let tex = tex_line(line);
    if tex.is_empty() {
        return Ok(());
    }
    metrics_cache::cached(tex, font, LineKind::Tex, dpi, || {
        tex_metrics(&tex_manager(&crate::rc::params())?, tex, font, dpi)
    })?;
    Ok(())
}

/// The glyphs and rules of `text` typeset by LaTeX at
/// [`TEXT_PATH_FONT_SCALE`] points, in points from the page's bottom-left
/// corner (matplotlib's `TextToPath.get_glyphs_tex`): each glyph resolved
/// to its Type 1 font, whose outline [`TexGlyph::path_outline`] gives.
/// Only the size of `font` matters to TeX; the fonts come from the
/// rcParams.
///
/// # Errors
///
/// When TeX is missing or fails on the text, or a font it uses has no
/// Type 1 version.
pub fn tex_glyphs(
    text: &str,
    font: &FontProperties,
) -> Result<(Vec<TexGlyph>, Vec<DviBox>), TextError> {
    let rc = crate::rc::params();
    let tex = TexText::new(text, font.request(&rc), tex_manager(&rc)?);
    let page = tex.page(TEXT_PATH_FONT_SCALE, TEXT_PATH_DPI)?;
    Ok((tex.glyphs(&page)?, page.boxes))
}

/// The smallest ascent and descent of a TeX line (`Text._get_layout`
/// measures `"lp"` with TeX for usetex texts), with no line gap, typeset
/// with the parameters current now (matplotlib's `TexManager` reads them
/// whenever it typesets, not when the text was made).
pub(crate) fn tex_height_metrics(
    font: &FontProperties,
    dpi: f64,
) -> Result<FontHeightMetrics, TextError> {
    let lp = metrics_cache::cached("lp", font, LineKind::Tex, dpi, || {
        tex_metrics(&tex_manager(&crate::rc::params())?, "lp", font, dpi)
    })?;
    Ok(FontHeightMetrics {
        ascent: lp.height - lp.descent,
        descent: lp.descent,
        line_gap: 0.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artist::ArtistHandle as _;

    #[test]
    fn lone_spaces_are_escaped() {
        assert_eq!(tex_line(" "), r"\ ");
        assert_eq!(tex_line("  "), "  ");
        assert_eq!(tex_line(r"$\alpha$"), r"$\alpha$");
    }

    #[test]
    fn managers_follow_the_rc() {
        let mut rc = RcParams::matplotlib();
        rc.text_latex_preamble = r"\usepackage{amsmath}".to_owned();
        rc.text_latex_engine = pltrs_text::tex::TexEngine::LatexDvipng;
        let tex = tex_manager(&rc).expect("a cache directory");
        assert_eq!(tex.engine(), pltrs_text::tex::TexEngine::LatexDvipng);
        assert!(tex.source("x", 10.0).contains("\n\\usepackage{amsmath}\n"));
        assert!(tex.cache_dir().ends_with("tex.cache"));
        assert_eq!(tex.fonts().family, rc.font_family);
    }

    /// Figures with usetex text draw where TeX is installed and fail with
    /// matplotlib's message where it is not; a LaTeX error in the string
    /// fails with `latex`'s.
    #[test]
    fn usetex_texts_need_tex() {
        let fig = crate::figure::Figure::with_size(2.0, 1.0, 72.0);
        let text = fig.text(0.1, 0.5, r"$\frac{1}{2}$");
        text.set_usetex(true);
        assert!(text.usetex());
        match check_usetex() {
            Ok(()) => {
                assert!(fig.to_png().is_ok());
                assert!(text.window_extent().is_some_and(|e| e.width() > 0.0));
                text.set_text("$x");
                let err = fig.to_png().expect_err("unbalanced math");
                assert!(
                    err.to_string()
                        .contains("latex was not able to process the following string:\nb'$x'"),
                    "{err}"
                );
            }
            Err(missing) if missing.to_string().contains("requires TeX") => {
                // Without a DVI file in the cache, latex is missing; with
                // one (typeset earlier), the fonts it names are.
                let err = fig.to_png().expect_err("no TeX").to_string();
                assert!(
                    err.contains(
                        "Failed to process string with tex because latex could not be found"
                    ) || err.contains("in your texmf tree, but could not find it"),
                    "{err}"
                );
            }
            Err(_) => {}
        }
    }

    /// matplotlib's `test_text.py::test_single_artist_usenotex`: one text
    /// left out of TeX while `text.usetex` is on saves in the formats
    /// that mix the two, without TeX (`2_2_2` would fail in it).
    #[test]
    fn single_texts_may_leave_tex_out() {
        crate::rc::context(|| {
            crate::rc::set("text.usetex", "True").expect("a valid rcParam");
            let fig = crate::figure::Figure::with_size(2.0, 1.0, 72.0);
            fig.text(0.5, 0.5, "2_2_2").set_usetex(false);
            assert!(fig.to_png().is_ok());
            assert!(fig.to_pdf().is_ok());
            assert!(fig.to_svg().is_ok());
        });
    }

    #[test]
    fn empty_lines_are_not_typeset() {
        let rc = RcParams::matplotlib();
        let run = tex_run("", &FontProperties::from_rc(&rc), 100.0).expect("not typeset");
        assert_eq!((run.width, run.height, run.descent), (0.0, 0.0, 0.0));
        assert!(run.source.as_ref().is_some_and(TextSource::is_tex));
    }

    /// Records the TeX lines it is given to draw, and draws nothing.
    #[derive(Default)]
    struct Recorder {
        lines: std::cell::RefCell<Vec<(String, f64)>>,
    }

    impl crate::text::TexPainter for Recorder {
        fn paint(
            &self,
            _: &pltrs_render::GraphicsContext,
            line: &str,
            _: &FontProperties,
            origin: pltrs_core::Point,
            _: f64,
        ) -> Vec<crate::text::DrawnPath> {
            self.lines.borrow_mut().push((line.to_owned(), origin.y));
            Vec::new()
        }
    }

    /// matplotlib's `test_text.py::test_metrics_cache`: the lines of TeX
    /// texts go to the replaced `draw_tex` one by one, those of texts at
    /// the same height at the same height, and the renderer's cache finds
    /// the lines measured before.
    #[test]
    fn tex_lines_go_to_the_painter_and_through_the_cache() {
        use std::rc::Rc;

        use crate::text::{MetricsCache, set_metrics_cache, set_tex_painter};

        if check_usetex().is_err() {
            return;
        }
        let fig = crate::figure::Figure::with_size(2.0, 2.0, 100.0);
        fig.text(0.3, 0.5, "foo\nbar");
        fig.text(0.3, 0.5, "foo\nbar").set_usetex(true);
        fig.text(0.5, 0.5, "foo\nbar").set_usetex(true);
        let cache = Rc::new(MetricsCache::new());
        let recorder = Rc::new(Recorder::default());
        let before = set_metrics_cache(Some(Rc::clone(&cache)));
        let first = fig.render_raster();
        set_tex_painter(Some(Rc::clone(&recorder) as Rc<dyn crate::text::TexPainter>));
        let second = fig.render_raster();
        set_tex_painter(None);
        set_metrics_cache(before);
        assert!(first.is_ok() && second.is_ok());
        let lines = recorder.lines.borrow();
        let heights = |line: &str| {
            let mut heights: Vec<u64> = lines
                .iter()
                .filter(|(drawn, _)| drawn == line)
                .map(|(_, y)| y.to_bits())
                .collect();
            heights.dedup();
            heights
        };
        assert_eq!(lines.len(), 4);
        assert_eq!((heights("foo").len(), heights("bar").len()), (1, 1));
        let info = cache.info();
        assert!(info.hits > info.misses, "{info:?}");
    }

    #[test]
    fn tex_glyphs_are_those_of_the_text_path() {
        if check_usetex().is_err() {
            return;
        }
        let font = FontProperties::from_rc(&RcParams::matplotlib());
        let (glyphs, rules) = tex_glyphs("W", &font).expect("typeset");
        let [w] = glyphs.as_slice() else {
            panic!("one glyph")
        };
        // Set at 100 TeX points, a little smaller in PostScript points.
        let size = TEXT_PATH_FONT_SCALE * 72.0 / 72.27;
        assert!((w.size - size).abs() < 1e-9, "{}", w.size);
        assert_eq!(rules.len(), 0);
        let (_, fraction) = tex_glyphs(r"$\frac{1}{2}$", &font).expect("typeset");
        assert_eq!(fraction.len(), 1, "the fraction bar");
    }
}
