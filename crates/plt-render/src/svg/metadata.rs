//! The document's Dublin Core metadata block (`_write_metadata`).

use super::xml::XmlWriter;
use crate::vector::Metadata;

/// Writes the optional `<title>` and the `<metadata>` block: the resource
/// type and format matplotlib always records, then title, date and
/// creator when set.
pub(super) fn write(writer: &mut XmlWriter, metadata: &Metadata) {
    if let Some(title) = &metadata.title {
        writer.element("title", title, &[]);
    }
    let start = writer.start("metadata", &[]);
    writer.start(
        "rdf:RDF",
        &[
            ("xmlns:dc", "http://purl.org/dc/elements/1.1/".to_owned()),
            ("xmlns:cc", "http://creativecommons.org/ns#".to_owned()),
            (
                "xmlns:rdf",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_owned(),
            ),
        ],
    );
    writer.start("cc:Work", &[]);
    writer.element(
        "dc:type",
        "",
        &[(
            "rdf:resource",
            "http://purl.org/dc/dcmitype/StillImage".to_owned(),
        )],
    );
    if let Some(title) = &metadata.title {
        writer.element("dc:title", title, &[]);
    }
    if let Some(date) = metadata.date {
        writer.element("dc:date", &date.iso8601(), &[]);
    }
    writer.element("dc:format", "image/svg+xml", &[]);
    if let Some(creator) = &metadata.creator {
        writer.start("dc:creator", &[]);
        writer.start("cc:Agent", &[]);
        writer.element("dc:title", creator, &[]);
        writer.end(true);
        writer.end(true);
    }
    writer.close(start);
}
