//! PostScript and Encapsulated PostScript output (matplotlib's
//! `backend_ps`): the vector renderer placed on a page per `ps.papersize`
//! and `savefig.orientation`, with fonts embedded or taken from AFM metrics.
//!
//! Like matplotlib, pltrs then runs external programs where the rcParams
//! ask for them, in a temporary directory removed afterwards
//! (`programs`): a distiller with `ps.usedistiller` (`distill`), and
//! with `text.usetex` LaTeX with `psfrag`, `dvips` and Ghostscript
//! (`psfrag`). A missing program fails the save with matplotlib's
//! message.

mod distill;
mod programs;
mod psfrag;

use pltrs_render::{Metadata, PsOptions, PsOutput, PsRenderer};

use self::distill::{Distillation, WorkFile, gs_distill, xpdf_distill};
pub(crate) use self::programs::{Executable, Workspace};
use self::psfrag::{Layout, print_figure_tex};
use super::Figure;
use super::print::SaveOptions;
use super::save::{SaveError, embedding, simplify_settings};
use crate::rc::Distiller;
pub use distill::{DistillOptions, distill_file};
pub use programs::ProgramError;

/// Whether the distiller `distiller` can run here: Ghostscript for
/// `ghostscript`, Ghostscript and `pdftops` for `xpdf` (the checks of
/// matplotlib's `gs_distill` and `xpdf_distill`, on which its tests
/// skip).
///
/// # Errors
///
/// matplotlib's `ExecutableNotFoundError` message for the first program
/// missing.
pub fn check_distiller(distiller: Distiller) -> Result<(), ProgramError> {
    match distiller {
        Distiller::Off => Ok(()),
        Distiller::Ghostscript => Executable::Ghostscript.check().map(drop),
        Distiller::Xpdf => {
            Executable::Ghostscript.check()?;
            Executable::Pdftops.check().map(drop)
        }
    }
}

/// matplotlib's warning for usetex text in a document made without
/// `text.usetex`, logged once per document.
const PARTIAL_USETEX: &str = "The PS backend determines usetex status solely based on \
     rcParams['text.usetex'] and does not support having usetex=True only for some \
     elements; this element will thus be rendered as if usetex=False.";

/// The options of a PostScript document on the paper and in the
/// orientation of the rcParams, as the `ps.*`, `image.composite_image`
/// and `text.usetex` rcParams configure figures.
pub(super) fn ps_options() -> PsOptions {
    let rc = crate::rc::params();
    PsOptions {
        eps: false,
        paper: rc.ps_papersize,
        orientation: rc.savefig_orientation,
        metadata: Metadata::default(),
        simplify: simplify_settings(),
        composite_images: rc.image_composite_image,
        fonttype: embedding(rc.ps_fonttype),
        afm: rc.ps_useafm,
        usetex: rc.text_usetex,
    }
}

/// A document's bytes as text: matplotlib writes PostScript in Latin-1,
/// whose characters are bytes (pltrs' documents and the distillers' are
/// ASCII).
fn latin1(document: Vec<u8>) -> String {
    document.into_iter().map(char::from).collect()
}

impl Figure {
    /// A PostScript document with save options: `eps` selects
    /// Encapsulated PostScript; the paper type and orientation of the
    /// options, else `ps.papersize` and `savefig.orientation`, place the
    /// figure on the page. With `text.usetex` the document goes through
    /// LaTeX, `dvips` and Ghostscript (`_print_figure_tex`), else through
    /// the distiller of `ps.usedistiller` if any (`_print_figure`).
    pub(super) fn postscript(
        &self,
        options: &SaveOptions,
        eps: bool,
    ) -> Result<Vec<u8>, SaveError> {
        let rc = crate::rc::params();
        let ps = PsOptions {
            eps,
            paper: options.papertype.unwrap_or(rc.ps_papersize),
            orientation: options.orientation.unwrap_or(rc.savefig_orientation),
            metadata: options.metadata.clone().unwrap_or_default(),
            ..ps_options()
        };
        // The programs date what they write as the document is dated.
        let source_date = ps.metadata.date.map_or(0, |date| date.0);
        let layout = Layout {
            eps,
            paper: ps.paper,
            orientation: ps.orientation,
        };
        // Text the backend draws in AFM fonts is laid out with their metrics.
        let output = crate::text::with_afm_metrics(ps.afm_directory(), || {
            self.render_vector_with(options, |width, height, dpi| {
                PsRenderer::new(width, height, dpi, ps)
            })
        })?
        .finish()
        .map_err(SaveError::Vector)?;
        let document = match output {
            PsOutput::Psfrag(figure) => {
                return print_figure_tex(&figure, layout, &rc, source_date);
            }
            PsOutput::Document(document) => document,
        };
        if document.tex_drawn_as_plain {
            crate::warnings::log("backends.backend_ps", PARTIAL_USETEX);
        }
        if rc.ps_usedistiller == Distiller::Off {
            return Ok(document.text.into_bytes());
        }
        let file = WorkFile::new(document.text.as_bytes(), source_date)?;
        let distillation = Distillation {
            eps,
            paper: layout.paper,
            bbox: Some(document.bbox),
            rotated: false,
        };
        if rc.ps_usedistiller == Distiller::Xpdf {
            xpdf_distill(&file, &distillation)?;
        } else {
            gs_distill(&file, &distillation, rc.ps_distiller_res)?;
        }
        Ok(file.contents()?)
    }

    /// The figure as a PostScript document centred on a `ps.papersize`
    /// page (matplotlib's `savefig(format="ps")`).
    pub fn to_ps(&self) -> Result<String, SaveError> {
        self.to_ps_with(SaveOptions::default())
    }

    /// The figure as a PostScript document with save options; its bytes
    /// are Latin-1 characters, as matplotlib writes them.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or a font cannot be embedded; with
    /// `ps.usedistiller` or `text.usetex`, when a program matplotlib runs
    /// there (LaTeX, `dvips`, Ghostscript, `pdftops`) is missing or fails.
    pub fn to_ps_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.postscript(&options, false).map(latin1)
    }

    /// The figure as an Encapsulated PostScript document (matplotlib's
    /// `savefig(format="eps")`).
    pub fn to_eps(&self) -> Result<String, SaveError> {
        self.to_eps_with(SaveOptions::default())
    }

    /// The figure as an Encapsulated PostScript document with save
    /// options; its bytes are Latin-1 characters, as matplotlib writes
    /// them.
    ///
    /// # Errors
    ///
    /// As [`Figure::to_ps_with`].
    pub fn to_eps_with(&self, options: SaveOptions) -> Result<String, SaveError> {
        self.postscript(&options, true).map(latin1)
    }
}

#[cfg(test)]
mod tests {
    use pltrs_core::Bbox;
    use pltrs_text::tex::TexError;

    use super::*;
    use crate::artist::ArtistHandle as _;
    use crate::figure::{BboxInches, PageOrientation, Paper};
    use crate::text::check_usetex_postscript;

    #[test]
    fn postscript_documents() {
        let fig = Figure::with_size(2.0, 1.0, 50.0);
        let ax = fig.subplot();
        ax.plot(vec![0.0, 1.0], vec![0.0, 1.0]).unwrap();
        let ps = fig.to_ps().unwrap();
        assert!(ps.starts_with("%!PS-Adobe-3.0\n%%DocumentPaperSizes: letter"));
        let tight = SaveOptions {
            bbox_inches: Some(BboxInches::Inches(Bbox::from_bounds(0.0, 0.0, 1.0, 0.5))),
            ..SaveOptions::default()
        };
        let eps = fig.to_eps_with(tight).unwrap();
        assert!(eps.contains("%%BoundingBox: 0 0 72 36\n"));
        let scratch = crate::scratch::Scratch::new("postscript");
        fig.savefig(scratch.path("figure.eps")).unwrap();
        let saved = std::fs::read_to_string(scratch.path("figure.eps")).unwrap();
        assert!(saved.contains("%%Title: figure.eps\n"));
    }

    #[test]
    fn save_options_choose_the_paper_and_orientation() {
        let fig = Figure::with_size(8.0, 6.0, 80.0);
        let options = SaveOptions {
            orientation: Some(PageOrientation::Landscape),
            papertype: Some(Paper::Figure),
            ..SaveOptions::default()
        };
        let ps = fig.to_ps_with(options).unwrap();
        assert!(!ps.contains("%%DocumentPaperSizes"));
        assert!(ps.contains("%%Orientation: landscape\n%%BoundingBox: 0 0 432 576\n"));
        assert!(ps.contains("mpldict begin\n432 0 translate\n90 rotate\n"));
        let letter = SaveOptions {
            papertype: Some("letter".parse().unwrap()),
            ..SaveOptions::default()
        };
        let ps = fig.to_ps_with(letter).unwrap();
        assert!(ps.contains("%%DocumentPaperSizes: letter\n"));
        assert!(ps.contains("%%BoundingBox: 18 180 594 612\n"));
    }

    /// The figure of matplotlib's `test_savefig_to_stringio` (8 × 6 inches
    /// in its test style): a line titled "Déjà vu", with a minus and a euro
    /// sign unless TeX typesets it.
    fn titled_line(usetex: bool) -> Figure {
        let fig = Figure::with_size(8.0, 6.0, 100.0);
        let ax = fig.subplot();
        ax.plot([1.0, 2.0], [1.0, 2.0]).unwrap();
        ax.set_title(if usetex {
            "D\u{e9}j\u{e0} vu"
        } else {
            "D\u{e9}j\u{e0} vu \u{2212}\u{20ac}"
        });
        fig
    }

    /// `fig` saved as `format` on `paper` in `orientation`.
    fn saved(
        fig: &Figure,
        eps: bool,
        paper: &str,
        orientation: PageOrientation,
    ) -> Result<String, SaveError> {
        let options = SaveOptions {
            orientation: Some(orientation),
            papertype: Some(paper.parse().unwrap()),
            ..SaveOptions::default()
        };
        if eps {
            fig.to_eps_with(options)
        } else {
            fig.to_ps_with(options)
        }
    }

    const ORIENTATIONS: [PageOrientation; 2] =
        [PageOrientation::Portrait, PageOrientation::Landscape];

    /// matplotlib's `test_savefig_to_stringio` with the distillers: the
    /// `xpdf` document names its letter paper (or does not), Ghostscript's
    /// records the paper options it was run with; where a distiller cannot
    /// run, the save fails with matplotlib's message (where matplotlib's
    /// test skips).
    #[test]
    fn distillers_rewrite_the_document() {
        for (distiller, name) in [
            (Distiller::Ghostscript, "ghostscript"),
            (Distiller::Xpdf, "xpdf"),
        ] {
            crate::rc::context(|| {
                crate::rc::set("ps.usedistiller", name).unwrap();
                let fig = titled_line(false);
                for (paper, orientation) in ["letter", "figure"]
                    .into_iter()
                    .flat_map(|paper| ORIENTATIONS.map(|o| (paper, o)))
                {
                    let saved = saved(&fig, false, paper, orientation);
                    let document = match (check_distiller(distiller), saved) {
                        (Ok(()), Ok(document)) => document,
                        (Err(missing), Err(SaveError::Program(err))) => {
                            assert_eq!(err, missing);
                            continue;
                        }
                        (check, saved) => panic!("{distiller:?}: {check:?} but {saved:?}"),
                    };
                    let letter = document.to_lowercase().contains("letter");
                    if distiller == Distiller::Xpdf {
                        assert_eq!(letter, paper == "letter", "{paper} {orientation:?}");
                        continue;
                    }
                    let width = match orientation {
                        PageOrientation::Portrait => "576.0",
                        PageOrientation::Landscape => "432.0",
                    };
                    let wanted = match paper {
                        "figure" => format!("-dDEVICEWIDTHPOINTS={width}"),
                        _ => "-sPAPERSIZE".to_owned(),
                    };
                    assert!(document.contains(&wanted), "{paper} {orientation:?}");
                }
                if check_distiller(distiller).is_ok() {
                    let eps = saved(&fig, true, "letter", PageOrientation::Portrait).unwrap();
                    assert!(
                        eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"),
                        "{distiller:?}"
                    );
                    assert!(eps.ends_with("restore\nshowpage\n%%EOF\n"), "{distiller:?}");
                }
            });
        }
    }

    /// Where LaTeX, `psfrag`, `dvips` and Ghostscript are installed, usetex
    /// PostScript is what Ghostscript distills, its paper as asked for
    /// (matplotlib's `test_savefig_to_stringio` with usetex); where one is
    /// missing, saving fails with matplotlib's error.
    #[test]
    fn usetex_documents_go_through_latex_and_ghostscript() {
        crate::rc::context(|| {
            crate::rc::set("text.usetex", "True").unwrap();
            let fig = titled_line(true);
            let first = saved(&fig, false, "letter", PageOrientation::Portrait);
            let missing = match check_usetex_postscript() {
                Ok(()) => None,
                Err(TexError::NotInstalled { program }) => Some(program),
                Err(err) => panic!("{err}"),
            };
            let Some(program) = missing else {
                let document = first.unwrap();
                assert!(document.starts_with("%!PS-Adobe-3.0\n%%Invocation: path/gs "));
                assert!(document.contains(" -sPAPERSIZE=letter "));
                for orientation in ORIENTATIONS {
                    let figure = saved(&fig, false, "figure", orientation).unwrap();
                    let width = match orientation {
                        PageOrientation::Portrait => "576.0",
                        PageOrientation::Landscape => "432.0",
                    };
                    assert!(figure.contains(&format!("-dDEVICEWIDTHPOINTS={width}")));
                    let eps = saved(&fig, true, "letter", orientation).unwrap();
                    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 "));
                    assert!(eps.contains(" -dEPSCrop "));
                }
                return;
            };
            let err = first.expect_err("a program is missing").to_string();
            let expected = match program.as_str() {
                "dvips" => "[Errno 2] No such file or directory: 'dvips'",
                "the psfrag package" => "psfrag.sty",
                "ghostscript" => "Failed to find a Ghostscript installation",
                _ => "",
            };
            assert!(err.contains(expected), "{program}: {err}");
        });
    }

    /// matplotlib's `test_failing_latex`: a string LaTeX cannot typeset
    /// (a double subscript) fails the save.
    #[test]
    fn failing_latex_fails_the_save() {
        if crate::text::check_usetex().is_err() {
            return;
        }
        crate::rc::context(|| {
            crate::rc::set("text.usetex", "True").unwrap();
            let fig = Figure::new();
            fig.subplot().set_xlabel("$22_2_2$");
            let err = fig.to_ps().expect_err("a double subscript");
            assert!(
                err.to_string().contains("latex was not able to process"),
                "{err}"
            );
        });
    }

    /// matplotlib's `test_partial_usetex`: with `text.usetex` off, texts
    /// marked usetex (measured by TeX) are drawn as plain text.
    #[test]
    fn partial_usetex_draws_plain_text() {
        if crate::text::check_usetex().is_err() {
            return;
        }
        let fig = Figure::new();
        fig.text(0.1, 0.1, "foo").set_usetex(true);
        fig.text(0.2, 0.2, "bar").set_usetex(true);
        let ps = fig.to_ps().unwrap();
        assert!(ps.contains("/f glyphshow") && ps.contains("/r glyphshow"));
        assert!(!ps.contains("psmarker"));
    }

    /// matplotlib's `test_usetex_preamble` (a preamble loading the packages
    /// the psfrag document loads does not conflict with them),
    /// `test_transparency_tex` (fully transparent texts are left out) and
    /// `test_tilde_in_tempfilename` (a usetex EPS saved with `format="ps"`
    /// into a directory named `short-1`).
    #[test]
    fn usetex_postscript_saves_with_a_preamble_and_transparent_texts() {
        if check_usetex_postscript().is_err() {
            return;
        }
        crate::rc::context(|| {
            crate::rc::set("text.usetex", "True").unwrap();
            crate::rc::set(
                "text.latex.preamble",
                r"\usepackage{color,graphicx,textcomp}",
            )
            .unwrap();
            let fig = Figure::new();
            fig.text(0.5, 0.5, "foo");
            assert!(fig.to_ps().is_ok());
            let empty = Figure::new();
            let ax = empty.subplot();
            ax.set_axis_off();
            ax.text(0.5, 0.5, "foo").set_color("r").set_alpha(Some(0.0));
            let eps = empty.to_eps().unwrap();
            assert!(!eps.contains(" Tf\n"), "no text is shown");
            let dir = std::env::temp_dir().join(format!("pltrs-ps-{}/short-1", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            let options = SaveOptions {
                format: Some(crate::figure::SaveFormat::Ps),
                ..SaveOptions::default()
            };
            let path = dir.join("tex_demo.eps");
            fig.savefig_with(&path, options).unwrap();
            assert!(
                std::fs::read(&path)
                    .unwrap()
                    .starts_with(b"%!PS-Adobe-3.0\n")
            );
            std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
        });
    }

    /// matplotlib's `test_d_glyph`: no procedure is named `/d`, which the
    /// Type 3 glyph of "d" would replace, so Ghostscript renders the EPS of
    /// a "def" text.
    #[test]
    fn ghostscript_renders_a_d_glyph() {
        if check_distiller(Distiller::Ghostscript).is_err() {
            return;
        }
        let fig = Figure::new();
        fig.text(0.5, 0.5, "def");
        let file = WorkFile::new(fig.to_eps().unwrap().as_bytes(), 0).unwrap();
        let png = file.workspace().path("out.png");
        let args: Vec<std::ffi::OsString> = [
            "-dBATCH",
            "-dNOPAUSE",
            "-dSAFER",
            "-dEPSCrop",
            "-sDEVICE=png16m",
            "-r30",
        ]
        .map(Into::into)
        .into_iter()
        .chain([
            programs::path_argument("-sOutputFile=", &png),
            file.path().into(),
        ])
        .collect();
        let gs = Executable::Ghostscript.check().unwrap();
        file.workspace().run(gs, &args).unwrap();
        let image = pltrs_codec::png::decode(&std::fs::read(png).unwrap()).unwrap();
        assert_eq!((image.width(), image.height()), (192, 144));
    }
}
