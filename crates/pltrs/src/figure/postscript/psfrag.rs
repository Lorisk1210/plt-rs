//! usetex PostScript (matplotlib's `_print_figure_tex` and
//! `_convert_psfrags`): LaTeX includes the figure the renderer made with
//! `\includegraphics`, replacing its tags by the typeset texts with
//! `psfrag`; `dvips` makes PostScript of that, and Ghostscript distills
//! it.

use std::ffi::OsString;

use pltrs_core::pyfmt;
use pltrs_render::{PageOrientation, Paper, PsfragFigure};
use pltrs_text::tex::usepackage_if_not_loaded;

use super::distill::{Distillation, WorkFile, gs_distill};
use super::programs::ProgramError;
use crate::figure::save::SaveError;
use crate::rc::RcParams;

/// How much of `dvips`' output is searched for a landscape page
/// (`fh.read(1000)`).
const LANDSCAPE_SEARCH: usize = 1000;

/// How the document is laid out: the format, the paper and the
/// orientation of the save.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layout {
    /// Encapsulated PostScript.
    pub(super) eps: bool,
    /// The paper type.
    pub(super) paper: Paper,
    /// The orientation.
    pub(super) orientation: PageOrientation,
}

/// The page LaTeX lays the figure out on, its width and height in
/// inches as Python writes them: the figure's size (turned in landscape)
/// for EPS and the `figure` paper type, else the paper's (always
/// upright), whose whole numbers are Python integers.
fn latex_page(figure: &PsfragFigure, layout: Layout) -> (String, String) {
    if let (false, Paper::Named { width, height, .. }) = (layout.eps, layout.paper) {
        let inches = |value: f64| {
            if value.fract() == 0.0 {
                pyfmt::integer(value)
            } else {
                pyfmt::repr(value)
            }
        };
        return (inches(width), inches(height));
    }
    let (width, height) = match layout.orientation {
        PageOrientation::Portrait => (figure.width, figure.height),
        PageOrientation::Landscape => (figure.height, figure.width),
    };
    (pyfmt::repr(width), pyfmt::repr(height))
}

/// The preamble of the LaTeX document: the user's, then `color`,
/// `graphicx` and `psfrag` unless it loads them, then a page of
/// `width × height` inches without margins.
fn psfrag_preamble(preamble: &str, width: &str, height: &str) -> String {
    format!(
        "{preamble}{}{}{}\\geometry{{papersize={{{width}in,{height}in}},margin=0in}}",
        usepackage_if_not_loaded("color", None),
        usepackage_if_not_loaded("graphicx", None),
        usepackage_if_not_loaded("psfrag", None),
    )
}

/// The body of the LaTeX document: the psfrag commands, then the figure
/// at `epsfile`, rotated by 90° in landscape.
fn psfrag_body(psfrags: &[String], orientation: PageOrientation, epsfile: &str) -> String {
    let angle = match orientation {
        PageOrientation::Portrait => 0,
        PageOrientation::Landscape => 90,
    };
    format!(
        "\n\\begin{{figure}}\n  \\centering\\leavevmode\n  {}\n  \
         \\includegraphics*[angle={angle}]{{{epsfile}}}\n\\end{{figure}}",
        psfrags.join("\n")
    )
}

/// `_convert_psfrags`: LaTeX typesets the figure in `file` with its
/// texts in place (at TeX's default size, 10 points, with the fonts and
/// preamble of `rc`), and `dvips` turns the DVI file into the PostScript
/// that replaces the file. Returns whether `dvips` made a landscape page,
/// which it does for some figure sizes.
fn convert_psfrags(
    file: &WorkFile,
    figure: &PsfragFigure,
    layout: Layout,
    rc: &RcParams,
) -> Result<bool, SaveError> {
    let (width, height) = latex_page(figure, layout);
    let mut rc = rc.clone();
    rc.text_latex_preamble = psfrag_preamble(&rc.text_latex_preamble, &width, &height);
    let tex = crate::text::tex_manager(&rc)?;
    let epsfile = file
        .path()
        .canonicalize()
        .map_err(|err| ProgramError::io(file.path(), &err))?;
    let body = psfrag_body(
        &figure.psfrags,
        layout.orientation,
        &epsfile.to_string_lossy(),
    );
    let dvi = tex.make_dvi(&body, 10.0)?;
    let directory = file.workspace().subdirectory()?;
    let psfile = directory.path().join("tmp.ps");
    // -R1 keeps the DVI file from running shell commands.
    let args: Vec<OsString> = ["-q", "-R1", "-o"]
        .map(OsString::from)
        .into_iter()
        .chain([psfile.clone().into(), dvi.into()])
        .collect();
    file.workspace().run("dvips", &args)?;
    file.replace_with(&psfile)?;
    let converted = file.contents()?;
    let head = &converted[..converted.len().min(LANDSCAPE_SEARCH)];
    Ok(String::from_utf8_lossy(head).contains("Landscape"))
}

/// The rest of `_print_figure_tex`: the figure converted by LaTeX and
/// `dvips`, then always distilled by Ghostscript (with its bounding box
/// for EPS) at `ps.distiller.res`; the programs date what they write
/// `source_date`.
///
/// # Errors
///
/// When LaTeX fails (`psfrag` missing included), or `dvips` or
/// Ghostscript is missing or fails.
pub(super) fn print_figure_tex(
    figure: &PsfragFigure,
    layout: Layout,
    rc: &RcParams,
    source_date: i64,
) -> Result<Vec<u8>, SaveError> {
    let file = WorkFile::new(figure.figure.as_bytes(), source_date)?;
    let rotated = convert_psfrags(&file, figure, layout, rc)?;
    let distillation = Distillation {
        eps: layout.eps,
        paper: layout.paper,
        bbox: figure.bbox,
        rotated,
    };
    gs_distill(&file, &distillation, rc.ps_distiller_res)?;
    Ok(file.contents()?)
}

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;

    use super::*;

    fn figure() -> PsfragFigure {
        PsfragFigure {
            figure: String::new(),
            psfrags: vec![
                r"\psfrag{psmarker0}{a}".to_owned(),
                r"\psfrag{psmarker1}{b}".to_owned(),
            ],
            width: 6.4,
            height: 4.8,
            bbox: Bbox::from_extents(0.0, 0.0, 460.8, 345.6),
        }
    }

    fn layout(eps: bool, paper: &str, orientation: PageOrientation) -> Layout {
        Layout {
            eps,
            paper: paper.parse().unwrap(),
            orientation,
        }
    }

    #[test]
    fn latex_pages_are_written_as_python_writes_them() {
        let page = |eps, paper, orientation| latex_page(&figure(), layout(eps, paper, orientation));
        let strings = |w: &str, h: &str| (w.to_owned(), h.to_owned());
        assert_eq!(
            page(false, "letter", PageOrientation::Landscape),
            strings("8.5", "11")
        );
        assert_eq!(
            page(false, "a4", PageOrientation::Portrait),
            strings("8.27", "11.69")
        );
        assert_eq!(
            page(false, "figure", PageOrientation::Portrait),
            strings("6.4", "4.8")
        );
        assert_eq!(
            page(true, "letter", PageOrientation::Landscape),
            strings("4.8", "6.4")
        );
    }

    #[test]
    fn the_latex_document_includes_the_figure_with_its_tags() {
        assert_eq!(
            psfrag_preamble(r"\usepackage{x}", "8.5", "11"),
            "\\usepackage{x}\
             \\makeatletter\\@ifpackageloaded{color}{}{\\usepackage{color}}\\makeatother\
             \\makeatletter\\@ifpackageloaded{graphicx}{}{\\usepackage{graphicx}}\\makeatother\
             \\makeatletter\\@ifpackageloaded{psfrag}{}{\\usepackage{psfrag}}\\makeatother\
             \\geometry{papersize={8.5in,11in},margin=0in}"
        );
        assert_eq!(
            psfrag_body(
                &figure().psfrags,
                PageOrientation::Landscape,
                "/tmp/x/tmp.ps"
            ),
            "\n\\begin{figure}\n  \\centering\\leavevmode\n  \\psfrag{psmarker0}{a}\n\
             \\psfrag{psmarker1}{b}\n  \\includegraphics*[angle=90]{/tmp/x/tmp.ps}\n\\end{figure}"
        );
    }
}
