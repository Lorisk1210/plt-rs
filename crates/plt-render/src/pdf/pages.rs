//! The pages of a document and its information dictionary (`newPage`,
//! `newTextnote`, the link annotations of texts, `_write_annotations`, the
//! page tree and `writeInfoDict`).

use plt_core::Point;

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

/// A link from a drawn text to a URI (`_get_link_annotation`).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Link {
    uri: String,
    /// The box covering the link, `[x0, y0, x1, y1]` in points.
    rect: [f64; 4],
    /// The corners of a block not aligned with the page, which viewers
    /// read inside `rect` (`QuadPoints`).
    quad_points: Option<[f64; 8]>,
}

impl Link {
    /// The link over the `width × height` block whose bottom left corner
    /// is `origin`, turned by `angle` degrees about it
    /// (`_get_coordinates_of_block`): a block not aligned with the page
    /// gives its corners too, in a box padded to hold them.
    pub(super) fn over_block(
        uri: &str,
        origin: Point,
        width: f64,
        height: f64,
        angle: f64,
    ) -> Self {
        let (sin, cos) = (-angle).to_radians().sin_cos();
        let Point { x, y } = origin;
        let corners = [
            [x, y],
            [x + width * cos, y - width * sin],
            [
                x + width * cos + height * sin,
                y - width * sin + height * cos,
            ],
            [x + height * sin, y + height * cos],
        ];
        let aligned = angle.rem_euclid(90.0) == 0.0;
        let pad = if aligned { 0.0 } else { 0.00001 };
        let extreme = |axis: usize, pick: fn(f64, f64) -> f64| {
            corners
                .iter()
                .map(|corner| corner[axis])
                .reduce(pick)
                .unwrap_or(0.0)
        };
        Self {
            uri: uri.to_owned(),
            rect: [
                extreme(0, f64::min) - pad,
                extreme(1, f64::min) - pad,
                extreme(0, f64::max) + pad,
                extreme(1, f64::max) + pad,
            ],
            quad_points: (!aligned)
                .then_some(corners.as_flattened())
                .and_then(|flat| flat.try_into().ok()),
        }
    }

    /// The annotation dictionary.
    fn annotation(&self) -> Obj {
        let link = Obj::dict([
            ("Type", Obj::name("Annot")),
            ("Subtype", Obj::name("Link")),
            ("Rect", Obj::reals(self.rect)),
            ("Border", Obj::Array(vec![Obj::Int(0); 3])),
            (
                "A",
                Obj::dict([("S", Obj::name("URI")), ("URI", text_string(&self.uri))]),
            ),
        ]);
        match (link, self.quad_points) {
            (Obj::Dict(mut entries), Some(points)) => {
                entries.push(("QuadPoints".to_owned(), Obj::reals(points)));
                Obj::Dict(entries)
            }
            (link, _) => link,
        }
    }
}

/// One page: its object, its annotation array and what goes in it.
#[derive(Clone, Debug, PartialEq)]
struct Page {
    object: u32,
    annotations: u32,
    notes: Vec<Note>,
    links: Vec<Link>,
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

    /// Adds a link to the current page, after its notes.
    pub(super) fn add_link(&mut self, link: Link) {
        if let Some(page) = self.pages.last_mut() {
            page.links.push(link);
        }
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
            links: Vec::new(),
        });
        Ok(contents)
    }

    /// Writes every page's annotation array (`_write_annotations`).
    pub(super) fn write_annotations(&self, writer: &mut PdfWriter) -> Result<(), VectorError> {
        for page in &self.pages {
            let annotations = (page.notes.iter().map(Note::annotation))
                .chain(page.links.iter().map(Link::annotation))
                .collect();
            writer.write_object(page.annotations, &Obj::Array(annotations))?;
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
    fn links_of_turned_blocks_record_their_corners() {
        let flat = Link::over_block("u", Point::new(14.4, 7.2), 83.0, 13.0, 0.0);
        assert_eq!(flat.rect, [14.4, 7.2, 97.4, 20.2]);
        assert_eq!(flat.quad_points, None);
        let annotation = text(&flat.annotation());
        assert!(
            annotation.starts_with("<< /Type /Annot /Subtype /Link /Rect [ 14.4 7.2 97.4 20.2 ]")
        );
        assert!(annotation.ends_with("/A << /S /URI /URI (u) >> >>"));
        assert!(!annotation.contains("QuadPoints"));
        let turned = Link::over_block("u", Point::new(0.0, 0.0), 10.0, 5.0, 45.0);
        let corners = turned.quad_points.expect("corners of a turned block");
        // matplotlib's test_text_rotated_urls: the box's left edge is the
        // fourth corner's x, padded.
        assert_eq!(turned.rect[0], corners[6] - 0.00001);
        assert!((corners[2] - 10.0 / 2_f64.sqrt()).abs() < 1e-12);
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
