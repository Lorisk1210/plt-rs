//! Several figures in one PDF document (matplotlib's
//! `backends.backend_pdf.PdfPages`).

use std::path::Path as FsPath;

use plt_render::{Metadata, PdfRenderer};

use super::Figure;
use super::print::SaveOptions;
use super::save::{SaveError, pdf_options};

/// Where a note goes when no position is given (matplotlib's default
/// `positionRect`): off the page.
const NOTE_RECT: [f64; 4] = [-100.0, -100.0, 0.0, 0.0];

/// A PDF document collecting figures, one page per saved figure, sharing
/// fonts and other resources between its pages.
///
/// The document is configured by the `pdf.*` rcParams in effect when it is
/// created; each page is drawn like [`Figure::savefig_with`] draws a PDF
/// file (the `metadata` of the save options is ignored: the document has
/// its own, see [`metadata_mut`](Self::metadata_mut)).
///
/// ```
/// use plt::figure::{Figure, PdfPages};
/// let mut pages = PdfPages::new()?;
/// for n in 1..=3 {
///     let fig = Figure::with_size(2.0, 1.5, 72.0);
///     fig.subplot().plot(vec![0.0, 1.0], vec![0.0, f64::from(n)])?;
///     pages.savefig(&fig)?;
/// }
/// assert_eq!(pages.page_count(), 3);
/// let pdf = pages.finish()?;
/// assert!(pdf.starts_with(b"%PDF-1.4"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct PdfPages {
    document: PdfRenderer,
}

impl PdfPages {
    /// An empty document with plt-rs as creator and producer, dated by
    /// `SOURCE_DATE_EPOCH` or the current time (matplotlib's defaults).
    ///
    /// # Errors
    ///
    /// When the document cannot be started (never in practice).
    pub fn new() -> Result<Self, SaveError> {
        Self::with_metadata(Metadata::default())
    }

    /// An empty document with the information `metadata` (matplotlib's
    /// `PdfPages(filename, metadata=...)`).
    ///
    /// # Errors
    ///
    /// When the document cannot be started (never in practice).
    pub fn with_metadata(metadata: Metadata) -> Result<Self, SaveError> {
        PdfRenderer::document(pdf_options(metadata))
            .map(|document| Self { document })
            .map_err(SaveError::Vector)
    }

    /// Adds `figure` as a new page.
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the page cannot be written.
    pub fn savefig(&mut self, figure: &Figure) -> Result<(), SaveError> {
        self.savefig_with(figure, &SaveOptions::default())
    }

    /// Adds `figure` as a new page saved with `options` (resolution,
    /// cropping, colours).
    ///
    /// # Errors
    ///
    /// When some artists cannot be drawn or the page cannot be written.
    pub fn savefig_with(
        &mut self,
        figure: &Figure,
        options: &SaveOptions,
    ) -> Result<(), SaveError> {
        figure.render_pdf_page(options, &mut self.document)
    }

    /// How many pages the document has (matplotlib's `get_pagecount`).
    pub fn page_count(&self) -> usize {
        self.document.page_count()
    }

    /// Attaches a text note to the next page saved, off the page
    /// (matplotlib's `attach_note`).
    pub fn attach_note(&mut self, text: &str) {
        self.attach_note_at(text, NOTE_RECT);
    }

    /// Attaches a text note shown at `rect` (`[x0, y0, x1, y1]` in points)
    /// to the next page saved.
    pub fn attach_note_at(&mut self, text: &str, rect: [f64; 4]) {
        self.document.attach_note(text, rect);
    }

    /// The document information, written when the document is finished
    /// (matplotlib's `infodict`).
    pub fn metadata_mut(&mut self) -> &mut Metadata {
        self.document.metadata_mut()
    }

    /// The finished document (matplotlib's `close`); a document without
    /// pages is still a valid PDF file.
    ///
    /// # Errors
    ///
    /// When a font, image or number met while drawing could not be
    /// written.
    pub fn finish(self) -> Result<Vec<u8>, SaveError> {
        self.document.finish().map_err(SaveError::Vector)
    }

    /// Finishes the document and writes it to `path`.
    ///
    /// # Errors
    ///
    /// As [`finish`](Self::finish), or when the file cannot be written.
    pub fn save(self, path: impl AsRef<FsPath>) -> Result<(), SaveError> {
        let document = self.finish()?;
        std::fs::write(path, document).map_err(SaveError::Io)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(haystack: &[u8], needle: &str) -> usize {
        String::from_utf8_lossy(haystack).matches(needle).count()
    }

    #[test]
    fn pages_share_one_document() {
        let mut pages = PdfPages::with_metadata(Metadata {
            title: Some("Two pages".to_owned()),
            ..Metadata::none()
        })
        .unwrap();
        pages.metadata_mut().author = Some("plt-rs".to_owned());
        let small = Figure::with_size(2.0, 1.0, 50.0);
        small.subplot().set_title("one");
        let large = Figure::with_size(3.0, 2.0, 50.0);
        large.subplot().set_title("two");
        pages.attach_note("about the second page");
        pages.savefig(&small).unwrap();
        pages.attach_note_at("on the second page", [0.0, 0.0, 10.0, 10.0]);
        pages.savefig(&large).unwrap();
        assert_eq!(pages.page_count(), 2);
        let pdf = pages.finish().unwrap();
        assert_eq!(count(&pdf, "/Type /Page "), 2);
        assert_eq!(count(&pdf, "/Count 2"), 1);
        assert_eq!(count(&pdf, "/MediaBox [ 0 0 144 72 ]"), 1);
        assert_eq!(count(&pdf, "/MediaBox [ 0 0 216 144 ]"), 1);
        assert_eq!(count(&pdf, "/Subtype /Text"), 2);
        assert_eq!(
            count(&pdf, "/Subtype /Type3"),
            1,
            "the pages share the font"
        );
        assert_eq!(count(&pdf, "/Title (Two pages) /Author (plt-rs)"), 1);
    }

    #[test]
    fn core_fonts_are_shared_by_the_pages() {
        let mut pages = crate::rc::context(|| {
            crate::rc::set("pdf.use14corefonts", "True").unwrap();
            crate::rc::set("pdf.compression", "0").unwrap();
            PdfPages::new().unwrap()
        });
        for title in ["one", "two"] {
            let fig = Figure::with_size(2.0, 1.0, 50.0);
            fig.text(0.5, 0.5, title)
                .set_halign(crate::text::HorizontalAlignment::Center);
            pages.savefig(&fig).unwrap();
        }
        let pdf = pages.finish().unwrap();
        // Centred by Helvetica's AFM widths.
        assert_eq!(count(&pdf, "63.66 36 Td\n(one) Tj"), 1);
        assert_eq!(count(&pdf, "64.27 36 Td\n(two) Tj"), 1);
        assert_eq!(count(&pdf, "/Subtype /Type1 /BaseFont /Helvetica"), 1);
        assert_eq!(count(&pdf, "/Subtype /Type3"), 0);
    }

    #[test]
    fn empty_documents_have_no_pages() {
        let pdf = PdfPages::new().unwrap().finish().unwrap();
        assert_eq!(count(&pdf, "/Kids [ ] /Count 0"), 1);
    }

    #[test]
    fn documents_are_saved_to_files() {
        let mut pages = PdfPages::new().unwrap();
        pages.savefig(&Figure::with_size(1.0, 1.0, 72.0)).unwrap();
        let path = std::env::temp_dir().join("plt-rs-pdf-pages.pdf");
        pages.save(&path).unwrap();
        assert!(std::fs::read(&path).unwrap().starts_with(b"%PDF-1.4"));
    }
}
