//! The text of PGF pictures measured by LaTeX, as matplotlib's PGF backend
//! measures it (`RendererPgf.get_text_width_height_descent`): each line is
//! typeset in a box of the preamble the picture goes into by the TeX
//! system of `pgf.texsystem`, which runs in the background
//! ([`LatexBoxes`], matplotlib's `LatexManager`), one per thread and
//! preamble.
//!
//! A save measures its text so when its options ask for it
//! ([`SaveOptions::pgf_text_metrics`](super::SaveOptions::pgf_text_metrics));
//! by default the text engine measures it, as for every other format, and
//! no TeX system is needed.

use std::cell::RefCell;

use pltrs_text::tex::{LatexBoxes, TexError};
use pltrs_text::{LineMetrics, TextError};

use super::pgf::{pgf_preamble, pgf_text_logged};
use super::pgf_latex::DOCUMENTCLASS;
use crate::fonts::FontProperties;
use crate::rc::RcParams;

/// How the text of a PGF picture is measured while its figure is laid
/// out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PgfTextMetrics {
    /// By pltrs' text engine, as every other format measures it: no TeX
    /// system is needed, and text is placed after the metrics of the
    /// fonts pltrs finds.
    #[default]
    TextEngine,
    /// By the TeX system of `pgf.texsystem` typesetting each line in the
    /// picture's preamble, as matplotlib's PGF backend measures it: text
    /// is placed after the metrics of the fonts LaTeX typesets it in.
    Latex,
}

thread_local! {
    /// The thread's TeX system measuring texts, for the header it was
    /// started with (matplotlib keeps one, `_get_cached_or_new`).
    static BOXES: RefCell<Option<LatexBoxes>> = const { RefCell::new(None) };
}

/// The document texts are measured in, as far as its first query
/// (matplotlib's `LatexManager._build_latex_header`): the class, the TeX
/// system named for the cache, `graphicx` and the picture's preamble.
fn latex_header(rc: &RcParams) -> String {
    [
        DOCUMENTCLASS.to_owned(),
        format!("% !TeX program = {}", rc.pgf_texsystem.name()),
        r"\usepackage{graphicx}".to_owned(),
        pgf_preamble(rc),
        r"\begin{document}".to_owned(),
        r"\typeout{pgf_backend_query_start}".to_owned(),
    ]
    .join("\n")
}

/// The width, height and descent of `text` set in `font`, in pixels at
/// `dpi`, as matplotlib's PGF backend measures them: typeset by the TeX
/// system of `pgf.texsystem` in the picture's preamble, LaTeX's points
/// taken as 1/72 inch (as matplotlib takes them, leaving text a little
/// room). A font no family of which is installed is logged as matplotlib
/// logs it.
///
/// # Errors
///
/// When the TeX system cannot be started, stops on the preamble, or
/// cannot typeset the text (matplotlib's `ValueError`, "Error measuring").
pub fn measure_pgf_text(
    text: &str,
    font: &FontProperties,
    dpi: f64,
) -> Result<LineMetrics, TextError> {
    let rc = crate::rc::params();
    let tex = pgf_text_logged(text, font, &rc);
    let header = latex_header(&rc);
    let measured = BOXES.with_borrow_mut(|slot| -> Result<_, TexError> {
        if slot.as_ref().is_none_or(|boxes| boxes.header() != header) {
            // The TeX system of another preamble stops first.
            *slot = None;
            let program = rc.pgf_texsystem.name();
            *slot = Some(LatexBoxes::new(program, &header, &std::env::temp_dir())?);
        }
        slot.as_mut()
            .expect("a TeX system was started for the header")
            .measure(&tex)
    })?;
    let scale = dpi / 72.0;
    Ok(LineMetrics {
        width: measured.width * scale,
        height: (measured.height + measured.depth) * scale,
        descent: measured.depth * scale,
    })
}

/// Runs `f`, which lays out and draws a PGF picture, with its text
/// measured as `metrics` says.
pub(super) fn measuring<R>(metrics: PgfTextMetrics, f: impl FnOnce() -> R) -> R {
    match metrics {
        PgfTextMetrics::TextEngine => f(),
        PgfTextMetrics::Latex => crate::text::with_line_measure(measure_pgf_text, f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_header_names_the_tex_system_and_ends_with_the_query_start() {
        let mut rc = RcParams::matplotlib();
        rc.set("pgf.texsystem", "pdflatex").expect("valid");
        let header = latex_header(&rc);
        let lines: Vec<&str> = header.lines().collect();
        assert_eq!(
            lines[..3],
            [
                r"\documentclass{article}",
                "% !TeX program = pdflatex",
                r"\usepackage{graphicx}",
            ]
        );
        assert_eq!(
            lines[lines.len() - 2..],
            [r"\begin{document}", r"\typeout{pgf_backend_query_start}"]
        );
    }

    #[test]
    fn latex_measures_in_points_of_a_72nd_of_an_inch() {
        if pltrs_core::os::which("pdflatex").is_none() {
            return;
        }
        crate::rc::context(|| {
            crate::rc::set("pgf.texsystem", "pdflatex").expect("valid");
            let font = FontProperties::from_rc(&crate::rc::params());
            let at_72 = measure_pgf_text(r"\rule{36pt}{9pt}", &font, 72.0).expect("measured");
            assert_eq!((at_72.width, at_72.height, at_72.descent), (36.0, 9.0, 0.0));
            let at_144 = measure_pgf_text(r"\rule{36pt}{9pt}", &font, 144.0).expect("measured");
            assert_eq!(at_144.width, 72.0);
            let err = measure_pgf_text(r"\oops", &font, 72.0).expect_err("undefined");
            assert!(err.to_string().starts_with(r"Error measuring {"), "{err}");
            assert!(measure_pgf_text("x", &font, 72.0).is_ok(), "started again");
        });
    }
}
