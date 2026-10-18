//! The pages of a document and its information dictionary (`newPage`,
//! `newTextnote`, `_write_annotations`, the page tree and
//! `writeInfoDict`).

use super::object::{Obj, text_string};
use super::writer::PdfWriter;
use crate::vector::{Metadata, VectorError};

/// A text note shown on a page (`newTextnote`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Note {
    /// The note's text.
    pub(super) text: String,
    /// Where it is shown, `[x0, y0, x1, y1]` in points.
    pub(super) rect: [f64; 4],
}

impl Note {
    /// The annotation dictionary.
    fn annotation(&self) -> Obj {
        Obj::dict([
            ("Type", Obj::name("Annot")),
            ("Subtype", Obj::name("Text")),
            ("Contents", text_string(&self.text)),
            ("Rect", Obj::reals(self.rect)),
        ])
    }
}

/// One page: its object and its annotation array.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    object: u32,
    annotations: u32,
    notes: Vec<Note>,
}

/// The pages written so far (`pageList` and `_annotations`) and the notes
/// waiting for the next page (`pageAnnotations`).
#[derive(Debug, Default)]
pub(super) struct Pages {
    pages: Vec<Page>,
    pending: Vec<Note>,
}

/// The object numbers a new page refers to.
#[derive(Clone, Copy, Debug)]
pub(super) struct PageParents {
    /// The page tree.
    pub(super) pages: u32,
    /// The shared resource dictionary.
    pub(super) resources: u32,
}

impl Pages {
    /// How many pages were started.
    pub(super) fn len(&self) -> usize {
        self.pages.len()
    }

    /// Queues a note for the next page started.
    pub(super) fn add_note(&mut self, note: Note) {
        self.pending.push(note);
    }

    /// Writes the page object of a `width × height` inch page whose
    /// content stream is object `contents` and returns that object number
    /// (the page itself takes the pending notes).
    pub(super) fn start(
        &mut self,
        writer: &mut PdfWriter,
        parents: PageParents,
        width: f64,
        height: f64,
    ) -> Result<u32, VectorError> {
        let contents = writer.reserve();
        let annotations = writer.reserve();
        let object = writer.reserve();
        writer.write_object(
            object,
            &Obj::dict([
                ("Type", Obj::name("Page")),
                ("Parent", Obj::Ref(parents.pages)),
                ("Resources", Obj::Ref(parents.resources)),
                (
                    "MediaBox",
                    Obj::Array(vec![
                        Obj::Int(0),
                        Obj::Int(0),
                        Obj::Real(72.0 * width),
                        Obj::Real(72.0 * height),
                    ]),
                ),
                ("Contents", Obj::Ref(contents)),
                ("Annots", Obj::Ref(annotations)),
            ]),
        )?;
        self.pages.push(Page {
            object,
            annotations,
            notes: std::mem::take(&mut self.pending),
        });
        Ok(contents)
    }

    /// Writes every page's annotation array (`_write_annotations`).
    pub(super) fn write_annotations(&self, writer: &mut PdfWriter) -> Result<(), VectorError> {
        for page in &self.pages {
            let notes = page.notes.iter().map(Note::annotation).collect();
            writer.write_object(page.annotations, &Obj::Array(notes))?;
        }
        Ok(())
    }

    /// Writes the page tree as object `tree`.
    pub(super) fn write_tree(&self, writer: &mut PdfWriter, tree: u32) -> Result<(), VectorError> {
        let kids = self.pages.iter().map(|p| Obj::Ref(p.object)).collect();
        writer.write_object(
            tree,
            &Obj::dict([
                ("Type", Obj::name("Pages")),
                ("Kids", Obj::Array(kids)),
                ("Count", Obj::Int(self.pages.len() as i64)),
            ]),
        )
    }
}

/// The document information dictionary (`_create_pdf_info_dict`): the
/// creator, producer and creation date first, then the other entries.
pub(super) fn info_dict(metadata: &Metadata) -> Obj {
    let text = |key: &str, value: &Option<String>| {
        value
            .as_ref()
            .map(|value| (key.to_owned(), text_string(value)))
    };
    let date = |key: &str, value: Option<crate::vector::Timestamp>| {
        value.map(|date| (key.to_owned(), text_string(&date.pdf())))
    };
    let entries = [
        text("Creator", &metadata.creator),
        text("Producer", &metadata.producer),
        date("CreationDate", metadata.date),
        text("Title", &metadata.title),
        text("Author", &metadata.author),
        text("Subject", &metadata.subject),
        text("Keywords", &metadata.keywords),
        date("ModDate", metadata.modified),
        metadata
            .trapped
            .map(|trapped| ("Trapped".to_owned(), Obj::name(trapped.name()))),
    ];
    Obj::Dict(entries.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{Timestamp, Trapped};

    fn text(obj: &Obj) -> String {
        String::from_utf8(obj.repr().expect("finite")).expect("ASCII")
    }

    #[test]
    fn info_lists_the_defaults_first() {
        let metadata = Metadata {
            title: Some("T".to_owned()),
            author: Some("A".to_owned()),
            date: Some(Timestamp(0)),
            trapped: Some(Trapped::Unknown),
            ..Metadata::none()
        };
        assert_eq!(
            text(&info_dict(&metadata)),
            "<< /CreationDate (D:19700101000000Z) /Title (T) /Author (A)\n/Trapped /Unknown >>"
        );
        assert_eq!(text(&info_dict(&Metadata::none())), "<< >>");
    }

    #[test]
    fn notes_go_on_the_next_page() {
        let mut writer = PdfWriter::new(0);
        let parents = PageParents {
            pages: writer.reserve(),
            resources: writer.reserve(),
        };
        let mut pages = Pages::default();
        pages.add_note(Note {
            text: "hello".to_owned(),
            rect: [-100.0, -100.0, 0.0, 0.0],
        });
        pages.start(&mut writer, parents, 1.0, 1.0).unwrap();
        pages.start(&mut writer, parents, 2.0, 1.0).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages.pages[0].notes.len(), 1);
        assert_eq!(pages.pages[1].notes, []);
        assert!(text(&pages.pages[0].notes[0].annotation()).contains("/Contents (hello)"));
    }
}
