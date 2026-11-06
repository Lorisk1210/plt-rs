//! PGF pictures typeset by LaTeX (matplotlib's PGF backend `print_pdf`,
//! `print_png` and its `PdfPages`): the picture is `\input` into a
//! document of the figure's size, which `pgf.texsystem` turns into a PDF
//! whose text LaTeX typeset with the document's fonts; a PNG is that PDF
//! rasterised by `pdftocairo` or Ghostscript. LaTeX needs the `pgf`
//! package.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use pltrs_render::{Metadata, Timestamp};

use super::print::SaveOptions;
use super::save::{SaveError, SaveFormat};
use super::{Executable, Figure, ProgramError, Workspace, pgf_preamble};

/// The class of the documents PGF pictures are typeset in (matplotlib's
/// `_DOCUMENTCLASS`).
pub(super) const DOCUMENTCLASS: &str = r"\documentclass{article}";

/// The `pdfinfo` option of `hyperref` that gives a PDF `metadata`
/// (matplotlib's `_metadata_to_str` of each entry of its info dict):
/// `Creator`, `Producer` and `CreationDate`, then the entries given.
fn pdfinfo(metadata: &Metadata) -> String {
    let texts = [
        ("Creator", metadata.creator.clone()),
        ("Producer", metadata.producer.clone()),
        ("CreationDate", metadata.date.map(Timestamp::pdf)),
        ("Title", metadata.title.clone()),
        ("Author", metadata.author.clone()),
        ("Subject", metadata.subject.clone()),
        ("Keywords", metadata.keywords.clone()),
        ("ModDate", metadata.modified.map(Timestamp::pdf)),
        ("Trapped", metadata.trapped.map(|t| t.name().to_owned())),
    ];
    texts
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| format!("{key}={{{value}}}")))
        .collect::<Vec<_>>()
        .join(",")
}

/// The header of a document of pages of `width × height` inches, as far
/// as `\begin{document}`.
fn document_header(metadata: &Metadata, (width, height): (f64, f64)) -> String {
    [
        DOCUMENTCLASS.to_owned(),
        format!(
            r"\usepackage[pdfinfo={{{}}}]{{hyperref}}",
            pdfinfo(metadata)
        ),
        format!(r"\usepackage[papersize={{{width:.6}in,{height:.6}in}}, margin=0in]{{geometry}}"),
        r"\usepackage{pgf}".to_owned(),
        pgf_preamble(&crate::rc::params()),
    ]
    .join("\n")
}

/// The document typesetting the picture `figure.pgf` of a figure of
/// `size` inches (matplotlib's `figure.tex` of `print_pdf`).
fn figure_document(metadata: &Metadata, size: (f64, f64)) -> String {
    [
        document_header(metadata, size),
        r"\begin{document}".to_owned(),
        r"\centering".to_owned(),
        r"\input{figure.pgf}".to_owned(),
        r"\end{document}".to_owned(),
    ]
    .join("\n")
}

/// A document LaTeX typesets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Job {
    /// One figure (`print_pdf`).
    Figure,
    /// The pages of a [`PgfPages`] (its `_run_latex`).
    Pages,
}

impl Job {
    /// The name of the document (its `.tex` and `.pdf` files).
    const fn name(self) -> &'static str {
        match self {
            Self::Figure => "figure",
            Self::Pages => "pdf_pages",
        }
    }

    /// The arguments of `pgf.texsystem` typesetting `source`, without
    /// stopping for input or running shell commands.
    fn arguments(source: &Path) -> Vec<OsString> {
        vec![
            "-interaction=nonstopmode".into(),
            "-halt-on-error".into(),
            "-no-shell-escape".into(),
            source.into(),
        ]
    }

    /// Runs `pgf.texsystem` on `document` in `workspace` and returns the
    /// PDF it wrote.
    fn typeset(self, workspace: &Workspace, document: &str) -> Result<Vec<u8>, SaveError> {
        let source = workspace.path(&format!("{}.tex", self.name()));
        std::fs::write(&source, document).map_err(SaveError::Io)?;
        let texsystem = crate::rc::params().pgf_texsystem.name();
        workspace
            .run(texsystem, &Self::arguments(&source))
            .map_err(SaveError::Program)?;
        std::fs::read(workspace.path(&format!("{}.pdf", self.name()))).map_err(SaveError::Io)
    }
}

/// The metadata of a typeset document: what `options` give, else pltrs'
/// defaults.
fn metadata_of(options: &SaveOptions) -> Metadata {
    options.metadata.clone().unwrap_or_default()
}

/// The seconds since the epoch programs date what they write.
fn source_date(metadata: &Metadata) -> i64 {
    metadata
        .date
        .unwrap_or_else(Timestamp::source_date_or_now)
        .0
}

impl Figure {
    /// The figure as a PDF document that LaTeX typeset from its PGF
    /// picture (matplotlib's `savefig(format="pdf", backend="pgf")`): the
    /// text is typeset by `pgf.texsystem` in the fonts of the document
    /// (`pgf.rcfonts`, `pgf.preamble`), with the PDF information of
    /// [`SaveOptions::metadata`] through `hyperref`.
    ///
    /// ```no_run
    /// use pltrs::prelude::*;
    /// let fig = Figure::new();
    /// fig.subplot().set_title(r"Typeset by \LaTeX");
    /// // Needs LaTeX with the `pgf` package.
    /// let pdf = fig.to_pgf_pdf()?;
    /// assert!(pdf.starts_with(b"%PDF"));
    /// # Ok::<(), pltrs::figure::SaveError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::to_pgf`] (images are written beside the picture
    /// here), and when LaTeX is missing or fails, quoting its output.
    pub fn to_pgf_pdf(&self) -> Result<Vec<u8>, SaveError> {
        self.to_pgf_pdf_with(SaveOptions::default())
    }

    /// [`Figure::to_pgf_pdf`] with save options.
    ///
    /// # Errors
    ///
    /// As [`Figure::to_pgf_pdf`].
    pub fn to_pgf_pdf_with(&self, options: SaveOptions) -> Result<Vec<u8>, SaveError> {
        let metadata = metadata_of(&options);
        let workspace = Workspace::new(source_date(&metadata)).map_err(SaveError::Program)?;
        let picture = SaveOptions {
            format: Some(SaveFormat::Pgf),
            metadata: None,
            ..options
        };
        self.savefig_with(workspace.path("figure.pgf"), picture)?;
        Job::Figure.typeset(&workspace, &figure_document(&metadata, self.size_inches()))
    }

    /// The figure as a PNG image of the PDF of [`Figure::to_pgf_pdf`],
    /// rasterised at the save resolution ([`SaveOptions::dpi`], else
    /// `savefig.dpi`) by [`PdfToPng::find`] (matplotlib's
    /// `savefig(format="png", backend="pgf")`).
    ///
    /// # Errors
    ///
    /// As [`Figure::to_pgf_pdf`], and when neither `pdftocairo` nor
    /// Ghostscript is installed or the conversion fails.
    pub fn to_pgf_png(&self) -> Result<Vec<u8>, SaveError> {
        self.to_pgf_png_with(SaveOptions::default())
    }

    /// [`Figure::to_pgf_png`] with save options.
    ///
    /// # Errors
    ///
    /// As [`Figure::to_pgf_png`].
    pub fn to_pgf_png_with(&self, options: SaveOptions) -> Result<Vec<u8>, SaveError> {
        let converter = PdfToPng::find().map_err(SaveError::Program)?;
        let dpi = self.save_dpi(&options);
        let pdf = self.to_pgf_pdf_with(options)?;
        let workspace = Workspace::new(0).map_err(SaveError::Program)?;
        let (pdf_path, png_path) = (workspace.path("figure.pdf"), workspace.path("figure.png"));
        std::fs::write(&pdf_path, pdf).map_err(SaveError::Io)?;
        converter
            .convert(&workspace, &pdf_path, &png_path, dpi)
            .map_err(SaveError::Program)?;
        std::fs::read(png_path).map_err(SaveError::Io)
    }
}

/// A program turning a PDF page into a PNG image (matplotlib's
/// `make_pdf_to_png_converter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PdfToPng {
    /// poppler's `pdftocairo`, preferred.
    Pdftocairo,
    /// Ghostscript's `pngalpha` device.
    Ghostscript,
}

impl PdfToPng {
    /// The converter installed here: `pdftocairo`, else Ghostscript.
    ///
    /// # Errors
    ///
    /// matplotlib's "No suitable pdf to png renderer found." when neither
    /// is installed.
    pub fn find() -> Result<Self, ProgramError> {
        if crate::process::output(
            std::process::Command::new("pdftocairo")
                .arg("-v")
                .stdin(std::process::Stdio::null()),
        )
        .is_ok()
        {
            Ok(Self::Pdftocairo)
        } else if Executable::Ghostscript.check().is_ok() {
            Ok(Self::Ghostscript)
        } else {
            Err(ProgramError::NotFound(
                "No suitable pdf to png renderer found.".to_owned(),
            ))
        }
    }

    /// The program to run: `pdftocairo`, or the Ghostscript installed here
    /// (`gs`, or one of its Windows names).
    fn program(self) -> Result<&'static str, ProgramError> {
        match self {
            Self::Pdftocairo => Ok("pdftocairo"),
            Self::Ghostscript => Executable::Ghostscript.check(),
        }
    }

    /// The arguments rasterising `pdf` into `png` at `dpi` (pixels per
    /// inch, truncated as matplotlib's `%d` writes it).
    fn arguments(self, pdf: &Path, png: &Path, dpi: f64) -> Vec<OsString> {
        let dpi = format!("{}", dpi.trunc());
        let path = |prefix: &str, path: &Path| {
            let mut argument = OsString::from(prefix);
            argument.push(path);
            argument
        };
        match self {
            Self::Pdftocairo => vec![
                "-singlefile".into(),
                "-transp".into(),
                "-png".into(),
                "-r".into(),
                dpi.into(),
                pdf.into(),
                png.with_extension("").into(),
            ],
            Self::Ghostscript => vec![
                "-dQUIET".into(),
                "-dSAFER".into(),
                "-dBATCH".into(),
                "-dNOPAUSE".into(),
                "-dNOPROMPT".into(),
                "-dUseCIEColor".into(),
                "-dTextAlphaBits=4".into(),
                "-dGraphicsAlphaBits=4".into(),
                "-dDOINTERPOLATE".into(),
                "-sDEVICE=pngalpha".into(),
                path("-sOutputFile=", png),
                format!("-r{dpi}").into(),
                pdf.into(),
            ],
        }
    }

    /// Rasterises the first page of `pdf` into `png` at `dpi`, running the
    /// program in `workspace`.
    fn convert(
        self,
        workspace: &Workspace,
        pdf: &Path,
        png: &Path,
        dpi: f64,
    ) -> Result<(), ProgramError> {
        workspace
            .run(self.program()?, &self.arguments(pdf, png, dpi))
            .map(drop)
    }
}

/// A PDF document of several figures typeset by LaTeX from their PGF
/// pictures, a page each (matplotlib's `backend_pgf.PdfPages`): pages take
/// the size of their figure, the PDF information is the document's, and
/// [`PgfPages::finish`] runs `pgf.texsystem` once on the whole document.
///
/// ```no_run
/// use pltrs::figure::{Figure, PgfPages};
/// let mut pages = PgfPages::new("figures.pdf", None);
/// for title in ["first", "second"] {
///     let fig = Figure::new();
///     fig.subplot().set_title(title);
///     pages.savefig(&fig)?;
/// }
/// assert_eq!(pages.page_count(), 2);
/// // Needs LaTeX with the `pgf` package.
/// pages.finish()?;
/// # Ok::<(), pltrs::figure::SaveError>(())
/// ```
#[derive(Debug)]
pub struct PgfPages {
    path: PathBuf,
    metadata: Metadata,
    document: String,
    pages: usize,
}

impl PgfPages {
    /// An empty document to be written to `path`, with `metadata` (pltrs'
    /// defaults when `None`).
    pub fn new(path: impl Into<PathBuf>, metadata: Option<Metadata>) -> Self {
        Self {
            path: path.into(),
            metadata: metadata.unwrap_or_default(),
            document: String::new(),
            pages: 0,
        }
    }

    /// The number of pages so far (matplotlib's `get_pagecount`).
    pub fn page_count(&self) -> usize {
        self.pages
    }

    /// Adds `figure` as a new page (matplotlib's `savefig`).
    ///
    /// # Errors
    ///
    /// As [`Figure::to_pgf`]: a picture typeset in a document of several
    /// pages cannot have images.
    pub fn savefig(&mut self, figure: &Figure) -> Result<(), SaveError> {
        self.savefig_with(figure, SaveOptions::default())
    }

    /// Adds `figure` as a new page saved with `options`.
    ///
    /// # Errors
    ///
    /// As [`PgfPages::savefig`].
    pub fn savefig_with(&mut self, figure: &Figure, options: SaveOptions) -> Result<(), SaveError> {
        let picture = figure.to_pgf_with(options)?;
        let (width, height) = figure.size_inches();
        if self.pages == 0 {
            self.document = [
                document_header(&self.metadata, (width, height)),
                r"\setlength{\parindent}{0pt}".to_owned(),
                r"\begin{document}%".to_owned(),
            ]
            .join("\n");
        } else {
            // `\pdfpagewidth` and `\pdfpageheight` became `\pagewidth`
            // and `\pageheight` in LuaTeX 0.85.
            let newpage = format!(
                r"\newpage\ifdefined\pdfpagewidth\pdfpagewidth\else\pagewidth\fi={width:.6}in\ifdefined\pdfpageheight\pdfpageheight\else\pageheight\fi={height:.6}in%"
            );
            self.document.push_str(&newpage);
            self.document.push('\n');
        }
        self.document.push_str(&picture);
        self.pages += 1;
        Ok(())
    }

    /// Ends the document, typesets it and writes the PDF (matplotlib's
    /// `close`); a document without pages writes nothing.
    ///
    /// # Errors
    ///
    /// When LaTeX is missing or fails, or the file cannot be written.
    pub fn finish(mut self) -> Result<(), SaveError> {
        if self.pages == 0 {
            return Ok(());
        }
        self.document.push_str("\\end{document}\\n");
        let workspace = Workspace::new(source_date(&self.metadata)).map_err(SaveError::Program)?;
        let pdf = Job::Pages.typeset(&workspace, &self.document)?;
        std::fs::write(&self.path, pdf).map_err(SaveError::Io)
    }

    /// The LaTeX source of the document so far.
    #[cfg(test)]
    fn source(&self) -> &str {
        &self.document
    }
}

#[cfg(test)]
mod tests;
