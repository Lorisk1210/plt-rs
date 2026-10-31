//! Multi-page PDF documents (matplotlib's `backend_pdf.PdfPages`).

use pltrs::figure::{Figure, PdfPages};

use super::print::PrintSettings;
use crate::binding::prelude::*;

/// A PDF document collecting pages, until it is finished.
struct PdfDocument(Option<PdfPages>);

impl PdfDocument {
    /// The document, if it is not finished yet.
    fn pages(&mut self) -> PyResult<&mut PdfPages> {
        self.0
            .as_mut()
            .ok_or_else(|| PyErr::value_error("I/O operation on closed file."))
    }
}

impl IntoPy for PdfDocument {
    /// A native object holding the document.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// An empty document.
fn pdf_pages_new() -> PyResult<PdfDocument> {
    Ok(PdfDocument(Some(PdfPages::new()?)))
}

/// Adds `figure` as a new page, saved with `settings` (those of
/// `print_figure`).
fn pdf_pages_savefig(
    document: Native<PdfDocument>,
    figure: Figure,
    settings: PrintSettings,
) -> PyResult<()> {
    Ok(document
        .borrow_mut()
        .pages()?
        .savefig_with(&figure, settings.0)?)
}

/// How many pages the document has.
fn pdf_pages_count(document: Native<PdfDocument>) -> PyResult<usize> {
    Ok(document.borrow_mut().pages()?.page_count())
}

/// The names of the images the document holds so far
/// (`PdfFile._images`).
fn pdf_pages_images(document: Native<PdfDocument>) -> PyResult<Vec<String>> {
    Ok(document.borrow_mut().pages()?.image_names())
}

/// Attaches a text note to the next page saved.
fn pdf_pages_attach_note(document: Native<PdfDocument>, text: String) -> PyResult<()> {
    document.borrow_mut().pages()?.attach_note(&text);
    Ok(())
}

/// Finishes the document: its bytes (a document without pages is still
/// a valid PDF file).
fn pdf_pages_finish(document: Native<PdfDocument>) -> PyResult<Bytes> {
    let pages = document
        .borrow_mut()
        .0
        .take()
        .ok_or_else(|| PyErr::value_error("I/O operation on closed file."))?;
    Ok(Bytes(pages.finish()?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("pdf_pages_new", pdf_pages_new)?;
    module.add("pdf_pages_savefig", pdf_pages_savefig)?;
    module.add("pdf_pages_count", pdf_pages_count)?;
    module.add("pdf_pages_images", pdf_pages_images)?;
    module.add("pdf_pages_attach_note", pdf_pages_attach_note)?;
    module.add("pdf_pages_finish", pdf_pages_finish)
}
