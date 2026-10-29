//! The Dublin Core metadata of SVG documents: the defaults, clearing them
//! and every entry matplotlib's SVG writer takes.

use pltrs::figure::{Figure, Metadata, SaveOptions};

use super::super::mpl_tests::in_test_style;
use super::{EPOCH, root};
use crate::figures::vector::xml::Element;
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};

/// A date of the probe's metadata: an ISO text, or `{"date": iso}` /
/// `{"datetime": iso}` (which matplotlib writes as the same ISO text).
fn date_text(value: &Json) -> Result<String, ProbeError> {
    let text = match value {
        Json::Object(entries) => entries.first().map(|(_, iso)| iso),
        other => Some(other),
    };
    text.and_then(Json::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ProbeError::BadArgs(format!("bad date {value}")))
}

/// A string or a list of strings as a list.
fn texts(value: &Json) -> Result<Vec<String>, ProbeError> {
    match value {
        Json::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad text {item}")))
            })
            .collect(),
        other => other
            .as_str()
            .map(|text| vec![text.to_owned()])
            .ok_or_else(|| ProbeError::BadArgs(format!("bad text {other}"))),
    }
}

/// The pltrs metadata of the probe's matplotlib `metadata` dictionary,
/// on top of the defaults dated by the tests' `SOURCE_DATE_EPOCH`.
fn decode(entries: &[(String, Json)]) -> Result<Metadata, ProbeError> {
    let mut metadata = Metadata {
        date: Some(EPOCH),
        ..Metadata::default()
    };
    for (key, value) in entries {
        let single = || -> Result<Option<String>, ProbeError> {
            match value {
                Json::Null => Ok(None),
                other => other
                    .as_str()
                    .map(|text| Some(text.to_owned()))
                    .ok_or_else(|| ProbeError::BadArgs(format!("bad {key} {other}"))),
            }
        };
        let several = || -> Result<Vec<String>, ProbeError> {
            match value {
                Json::Null => Ok(Vec::new()),
                other => texts(other),
            }
        };
        let svg = &mut metadata.svg;
        match key.as_str() {
            "Title" => metadata.title = single()?,
            "Type" => svg.resource_type = single()?,
            "Format" => svg.format = single()?,
            "Coverage" => svg.coverage = single()?,
            "Description" => svg.description = single()?,
            "Identifier" => svg.identifier = single()?,
            "Language" => svg.language = single()?,
            "Relation" => svg.relation = single()?,
            "Source" => svg.source = single()?,
            "Contributor" => svg.contributors = several()?,
            "Publisher" => svg.publishers = several()?,
            "Rights" => svg.rights = several()?,
            "Creator" => {
                metadata.creator = None;
                svg.creators = several()?;
            }
            "Keywords" => svg.keywords = several()?,
            "Date" => {
                metadata.date = None;
                svg.dates = match value {
                    Json::Null => Vec::new(),
                    Json::Array(items) => items.iter().map(date_text).collect::<Result<_, _>>()?,
                    other => vec![date_text(other)?],
                };
            }
            other => return Err(ProbeError::BadArgs(format!("unknown key {other}"))),
        }
    }
    Ok(metadata)
}

/// The probe's `[tag, value]` of a `cc:Work` entry.
fn entry(element: &Element, default_creator: Option<&str>) -> Json {
    let child_texts = |path: &[&str]| -> Vec<Json> {
        let mut level: Vec<&Element> = vec![element];
        for name in path {
            level = level
                .iter()
                .flat_map(|e| e.children.iter().filter(|c| c.name == *name))
                .collect();
        }
        level
            .iter()
            .map(|e| {
                if Some(e.text.as_str()) == default_creator {
                    Json::from("<default>")
                } else {
                    Json::from(e.text.as_str())
                }
            })
            .collect()
    };
    let value = match element.name.as_str() {
        "dc:type" => element.attr("rdf:resource").map_or(Json::Null, Json::from),
        "dc:subject" => Json::Array(child_texts(&["rdf:Bag", "rdf:li"])),
        _ => {
            let agents = child_texts(&["cc:Agent", "dc:title"]);
            if agents.is_empty() {
                Json::from(element.text.as_str())
            } else {
                Json::Array(agents)
            }
        }
    };
    Json::Array(vec![Json::from(element.name.as_str()), value])
}

/// The titles and Dublin Core entries of a figure saved with the probe's
/// metadata.
pub(super) fn metadata(args: Args<'_>) -> Outcome {
    in_test_style(args, || {
        let entries = args.json("metadata")?.as_object().unwrap_or(&[]);
        let fig = Figure::new();
        fig.subplot();
        let options = SaveOptions {
            metadata: Some(decode(entries)?),
            ..SaveOptions::default()
        };
        let document = fig.to_svg_with(options).map_err(ProbeError::rejected)?;
        let svg = root(&document)?;
        let named = |element: &Element, name: &str| -> Vec<Element> {
            element
                .children
                .iter()
                .filter(|child| child.name == name)
                .cloned()
                .collect()
        };
        let titles = named(&svg, "title")
            .iter()
            .map(|title| Json::from(title.text.as_str()))
            .collect();
        let default_creator = Metadata::default().creator;
        let work = named(&svg, "metadata")
            .iter()
            .flat_map(|m| named(m, "rdf:RDF"))
            .flat_map(|rdf| named(&rdf, "cc:Work"))
            .next()
            .map_or(Json::Null, |work| {
                Json::Array(
                    work.children
                        .iter()
                        .map(|child| entry(child, default_creator.as_deref()))
                        .collect(),
                )
            });
        Ok(Json::Object(vec![
            ("titles".to_owned(), Json::Array(titles)),
            ("work".to_owned(), work),
        ]))
    })
}
