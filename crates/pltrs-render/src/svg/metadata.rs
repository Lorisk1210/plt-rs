//! The document's Dublin Core metadata block (`_write_metadata`).

use super::xml::XmlWriter;
use crate::vector::{Metadata, Timestamp};

/// The `<metadata>` block being written, opened with the first entry.
struct Work<'a> {
    writer: &'a mut XmlWriter,
    start: Option<usize>,
}

impl Work<'_> {
    /// Opens `<metadata><rdf:RDF><cc:Work>` unless already open.
    fn open(&mut self) -> &mut XmlWriter {
        if self.start.is_none() {
            let start = self.writer.start("metadata", &[]);
            self.writer.start(
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
            self.writer.start("cc:Work", &[]);
            self.start = Some(start);
        }
        self.writer
    }

    /// A `dc:<key>` element holding `text`, if any.
    fn single(&mut self, key: &str, text: Option<&str>) {
        if let Some(text) = text {
            self.open().element(&format!("dc:{key}"), text, &[]);
        }
    }

    /// A `dc:<key>` element listing `agents` as `cc:Agent` titles, unless
    /// there are none.
    fn agents<'s>(&mut self, key: &str, agents: impl IntoIterator<Item = &'s str>) {
        let mut agents = agents.into_iter().peekable();
        if agents.peek().is_none() {
            return;
        }
        let writer = self.open();
        writer.start(&format!("dc:{key}"), &[]);
        for agent in agents {
            writer.start("cc:Agent", &[]);
            writer.element("dc:title", agent, &[]);
            writer.end(true);
        }
        writer.end(true);
    }

    /// The `dc:subject` bag of `keywords`, unless there are none.
    fn keywords<'s>(&mut self, keywords: impl IntoIterator<Item = &'s str>) {
        let mut keywords = keywords.into_iter().peekable();
        if keywords.peek().is_none() {
            return;
        }
        let writer = self.open();
        writer.start("dc:subject", &[]);
        writer.start("rdf:Bag", &[]);
        for keyword in keywords {
            writer.element("rdf:li", keyword, &[]);
        }
        writer.end(true);
        writer.end(true);
    }
}

/// The SVG entries of `metadata` that hold several values, else the
/// shared single one.
fn several<'a>(many: &'a [String], one: Option<&'a String>) -> impl Iterator<Item = &'a str> {
    let values = if many.is_empty() {
        one.map(std::slice::from_ref).unwrap_or_default()
    } else {
        many
    };
    values.iter().map(String::as_str)
}

/// Writes the optional `<title>` and the `<metadata>` block in
/// matplotlib's order: the resource type, the single-valued entries, the
/// agents, then the keywords; without any entry there is no block.
pub(super) fn write(writer: &mut XmlWriter, metadata: &Metadata) {
    if let Some(title) = &metadata.title {
        writer.element("title", title, &[]);
    }
    let svg = &metadata.svg;
    let date = if svg.dates.is_empty() {
        metadata.date.map(Timestamp::iso8601)
    } else {
        Some(svg.dates.join("/"))
    };
    let mut work = Work {
        writer,
        start: None,
    };
    if let Some(uri) = &svg.resource_type {
        work.open()
            .element("dc:type", "", &[("rdf:resource", uri.clone())]);
    }
    for (key, text) in [
        ("title", metadata.title.as_deref()),
        ("coverage", svg.coverage.as_deref()),
        ("date", date.as_deref()),
        ("description", svg.description.as_deref()),
        ("format", svg.format.as_deref()),
        ("identifier", svg.identifier.as_deref()),
        ("language", svg.language.as_deref()),
        ("relation", svg.relation.as_deref()),
        ("source", svg.source.as_deref()),
    ] {
        work.single(key, text);
    }
    work.agents("creator", several(&svg.creators, metadata.creator.as_ref()));
    work.agents("contributor", svg.contributors.iter().map(String::as_str));
    work.agents("publisher", svg.publishers.iter().map(String::as_str));
    work.agents("rights", svg.rights.iter().map(String::as_str));
    work.keywords(several(&svg.keywords, metadata.keywords.as_ref()));
    if let Some(start) = work.start {
        work.writer.close(start);
    }
}
