//! Multi-page PDF documents (matplotlib's `backend_pdf.PdfPages`).

use pltrs::figure::{Figure, PdfPages};
use pltrs_render::PdfRenderer;

use super::pdf_file::PyPdfObject;
use super::print::{DocumentMetadata, PrintSettings, sending_draw_event};
use crate::binding::prelude::*;

/// A PDF document collecting pages, until it is closed.
pub(super) struct PdfDocument(Option<PdfPages>);

impl PdfDocument {
    /// The document, if it is not closed yet.
    fn pages(&mut self) -> PyResult<&mut PdfPages> {
        self.0.as_mut().ok_or_else(closed)
    }

    /// The PDF file of the document, if it is not closed yet.
    pub(super) fn file(&mut self) -> PyResult<&mut PdfRenderer> {
        Ok(self.pages()?.document_mut())
    }

    /// The PDF file of the document, closing the document.
    pub(super) fn take(&mut self) -> PyResult<PdfRenderer> {
        self.0
            .take()
            .map(PdfPages::into_document)
            .ok_or_else(closed)
    }
}

/// Python's error for a closed file.
fn closed() -> PyErr {
    PyErr::value_error("I/O operation on closed file.")
}

impl IntoPy for PdfDocument {
    /// A native object holding the document.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// An empty document with the information `metadata` (pltrs' defaults
/// for `None`).
fn pdf_pages_new(metadata: Option<DocumentMetadata>) -> PyResult<PdfDocument> {
    let metadata = metadata.map(|metadata| metadata.0).unwrap_or_default();
    Ok(PdfDocument(Some(PdfPages::with_metadata(metadata)?)))
}

/// Adds `figure` as a new page, saved with `settings` (those of
/// `print_figure`), calling `on_draw` once it is drawn.
fn pdf_pages_savefig(
    document: Native<PdfDocument>,
    figure: Figure,
    settings: PrintSettings,
    on_draw: Option<Callback>,
) -> PyResult<()> {
    let mut document = document.borrow_mut();
    let pages = document.pages()?;
    Ok(sending_draw_event(&figure, on_draw, || {
        pages.savefig_with(&figure, settings.0)
    })?)
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

/// Attaches a text note `text` shown at `rect` (points), as matplotlib's
/// `newTextnote` writes them (any objects), to the next page saved.
fn pdf_pages_attach_note(
    document: Native<PdfDocument>,
    text: PyPdfObject,
    rect: PyPdfObject,
) -> PyResult<()> {
    document
        .borrow_mut()
        .pages()?
        .document_mut()
        .attach_note_objects(text.0, rect.0);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("pdf_pages_new", pdf_pages_new)?;
    module.add("pdf_pages_savefig", pdf_pages_savefig)?;
    module.add("pdf_pages_count", pdf_pages_count)?;
    module.add("pdf_pages_images", pdf_pages_images)?;
    module.add("pdf_pages_attach_note", pdf_pages_attach_note)
}
