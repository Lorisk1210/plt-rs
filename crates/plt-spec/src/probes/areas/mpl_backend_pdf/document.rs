//! What the probes read in PDF documents: the information dictionary,
//! the link annotations of the pages, the images and the decoded samples
//! of an indexed image.

use crate::figures::vector::pdf::syntax::{Document, Value};
use crate::json::Json;
use crate::probes::ProbeError;

/// The parsed objects of a document.
pub(super) fn parse(bytes: &[u8]) -> Result<Document, ProbeError> {
    Document::parse(bytes).map_err(ProbeError::rejected)
}

/// The text of a PDF string: UTF-16 after a byte order mark, else one
/// byte per character.
fn text(bytes: &[u8]) -> String {
    match bytes.strip_prefix(&[0xfe, 0xff]) {
        Some(utf16) => String::from_utf16_lossy(
            &utf16
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&pair| u16::from_be_bytes(pair))
                .collect::<Vec<_>>(),
        ),
        None => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// The document information dictionary as `{key: value}`: strings as
/// text, names as `/Name`, the `defaults` texts as `"<default>"`.
pub(super) fn info(doc: &Document, defaults: &[&str]) -> Result<Json, ProbeError> {
    let dict = doc.trailer.get("Info").map(|info| doc.resolve(info));
    let Some(Value::Dict(entries)) = dict else {
        return Err(ProbeError::rejected("no information dictionary"));
    };
    Ok(Json::Object(
        entries
            .iter()
            .map(|(key, value)| {
                let value = match value {
                    Value::Str(bytes) => {
                        let text = text(bytes);
                        Json::from(if defaults.contains(&text.as_str()) {
                            "<default>".to_owned()
                        } else {
                            text
                        })
                    }
                    Value::Name(name) => Json::from(format!("/{name}")),
                    other => Json::from(format!("{other:?}")),
                };
                (key.clone(), value)
            })
            .collect(),
    ))
}

/// The numbers of an array.
fn numbers(value: &Value) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(Value::as_num).collect()
}

/// The link annotations of every page as `[uri, rect, quadpoints]`.
pub(super) fn links(doc: &Document) -> Json {
    let mut found = Vec::new();
    let mut pages: Vec<_> = doc
        .objects
        .iter()
        .filter(|(_, stored)| stored.value.get("Type").and_then(Value::as_name) == Some("Page"))
        .collect();
    pages.sort_by_key(|(id, _)| **id);
    for (_, page) in pages {
        let annotations = page.value.get("Annots").map(|a| doc.resolve(a));
        for annotation in annotations.and_then(Value::as_array).unwrap_or_default() {
            let annotation = doc.resolve(annotation);
            if annotation.get("Subtype").and_then(Value::as_name) != Some("Link") {
                continue;
            }
            let uri = match annotation.get("A").and_then(|a| a.get("URI")) {
                Some(Value::Str(bytes)) => Json::from(text(bytes)),
                _ => Json::Null,
            };
            let array = |key: &str| {
                annotation
                    .get(key)
                    .and_then(numbers)
                    .map_or(Json::Null, Json::from)
            };
            found.push(Json::Array(vec![uri, array("Rect"), array("QuadPoints")]));
        }
    }
    Json::Array(found)
}

/// How many images the document holds, soft masks aside.
pub(super) fn image_count(doc: &Document) -> usize {
    let images: Vec<u32> = doc
        .objects
        .iter()
        .filter(|(_, stored)| stored.value.get("Subtype").and_then(Value::as_name) == Some("Image"))
        .map(|(&id, _)| id)
        .collect();
    let masks: Vec<u32> = doc
        .objects
        .values()
        .filter_map(|stored| match stored.value.get("SMask") {
            Some(Value::Ref(mask)) => Some(*mask),
            _ => None,
        })
        .collect();
    images.iter().filter(|id| !masks.contains(id)).count()
}

/// An image written with an indexed colour space: its size, palette and
/// one index per pixel.
pub(super) struct IndexedImage {
    /// Width in pixels.
    pub(super) width: usize,
    /// Height in pixels.
    pub(super) height: usize,
    /// RGB entries.
    pub(super) palette: Vec<[u8; 3]>,
    /// Row-major palette indices.
    pub(super) indices: Vec<u8>,
}

/// The first image of the document if it is indexed, `None` if its
/// colour space is not.
pub(super) fn indexed_image(doc: &Document) -> Result<Option<IndexedImage>, ProbeError> {
    let (&id, stored) = doc
        .objects
        .iter()
        .find(|(_, stored)| stored.value.get("Subtype").and_then(Value::as_name) == Some("Image"))
        .ok_or_else(|| ProbeError::rejected("no image"))?;
    let dict = &stored.value;
    let size = |key| {
        dict.get(key)
            .and_then(Value::as_num)
            .map(|n| n as usize)
            .ok_or_else(|| ProbeError::rejected(format!("no image {key}")))
    };
    let (width, height) = (size("Width")?, size("Height")?);
    let space = dict
        .get("ColorSpace")
        .and_then(Value::as_array)
        .unwrap_or_default();
    let [Value::Name(kind), _, _, Value::Str(palette)] = space else {
        return Ok(None);
    };
    if kind != "Indexed" {
        return Ok(None);
    }
    let data = doc
        .stream(&Value::Ref(id))
        .ok_or_else(|| ProbeError::rejected("undecodable image data"))?;
    let predicted = dict
        .get("DecodeParms")
        .and_then(|parms| parms.get("Predictor"))
        .and_then(Value::as_num)
        .is_some_and(|predictor| predictor >= 10.0);
    let indices = if predicted {
        unfilter(&data, width)
    } else {
        data
    };
    Ok(Some(IndexedImage {
        width,
        height,
        palette: palette.as_chunks::<3>().0.to_vec(),
        indices,
    }))
}

/// Undoes the PNG predictors of one-byte samples, `columns` a row.
fn unfilter(data: &[u8], columns: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    let mut previous = vec![0_u8; columns];
    for row in data.chunks(columns + 1) {
        let Some((&kind, raw)) = row.split_first() else {
            continue;
        };
        let mut current = raw.to_vec();
        for k in 0..current.len() {
            let left = if k > 0 { current[k - 1] } else { 0 };
            let (up, corner) = (previous[k], if k > 0 { previous[k - 1] } else { 0 });
            let prediction = match kind {
                1 => left,
                2 => up,
                3 => left.midpoint(up),
                4 => paeth(left, up, corner),
                _ => 0,
            };
            current[k] = current[k].wrapping_add(prediction);
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    out
}

/// The PNG Paeth predictor.
fn paeth(left: u8, up: u8, corner: u8) -> u8 {
    let p = i16::from(left) + i16::from(up) - i16::from(corner);
    let (pa, pb, pc) = (
        (p - i16::from(left)).abs(),
        (p - i16::from(up)).abs(),
        (p - i16::from(corner)).abs(),
    );
    if pa <= pb && pa <= pc {
        left
    } else if pb <= pc {
        up
    } else {
        corner
    }
}
