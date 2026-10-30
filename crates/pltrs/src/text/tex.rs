//! `text.usetex`: texts typeset by LaTeX (matplotlib's `Text` with
//! `usetex=True` and the renderers' `get_text_width_height_descent(...,
//! ismath="TeX")`). Every line is typeset on its own, measured from the
//! DVI file, and handed to the backend as a TeX run, which each backend
//! draws as matplotlib's `draw_tex` does.

use pltrs_text::tex::{TexError, TexFonts, TexManager, TexText};
use pltrs_text::{FontHeightMetrics, TextError, TextRun, TextSource};

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
    if !pltrs_text::tex::on_path("dvips") {
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
/// TeX text backends draw.
pub(crate) fn tex_run(
    line: &str,
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<TextRun, TextError> {
    let tex = TexText::new(tex_line(line), font.request(rc), tex_manager(rc)?);
    let measured = if tex.tex.is_empty() {
        None
    } else {
        super::overrides::measure(&tex.tex, font, LineKind::Tex, dpi)
    };
    let (width, height, descent) = match measured {
        Some(metrics) => (metrics.width, metrics.height, metrics.descent),
        None if tex.tex.is_empty() => (0.0, 0.0, 0.0),
        None => tex
            .manager
            .text_width_height_descent(&tex.tex, font.size, dpi / 72.0)?,
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

/// The smallest ascent and descent of a TeX line (`Text._get_layout`
/// measures `"lp"` with TeX for usetex texts), with no line gap.
pub(crate) fn tex_height_metrics(
    font: &FontProperties,
    dpi: f64,
    rc: &RcParams,
) -> Result<FontHeightMetrics, TextError> {
    let (_, height, descent) =
        tex_manager(rc)?.text_width_height_descent("lp", font.size, dpi / 72.0)?;
    Ok(FontHeightMetrics {
        ascent: height - descent,
        descent,
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
        let run = tex_run("", &FontProperties::from_rc(&rc), 100.0, &rc).expect("not typeset");
        assert_eq!((run.width, run.height, run.descent), (0.0, 0.0, 0.0));
        assert!(run.source.as_ref().is_some_and(TextSource::is_tex));
    }
}
