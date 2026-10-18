//! Embedded PDF fonts as items: Type 3 fonts with their metrics, encoding
//! and glyph procedures; Type 0 fonts with their descendant `CIDFontType2`,
//! widths, `CIDToGIDMap` and the outlines of the glyphs they map to.

use super::interp::Interpreter;
use super::syntax::{Document, Value};
use super::{base_name, value_tokens};
use crate::figures::vector::diff::{Item, Token};
use crate::figures::vector::truetype::glyph_tokens;

/// The descriptor entries compared (`FontFile2` and the name are not).
const DESCRIPTOR_KEYS: [&str; 7] = [
    "Flags",
    "Ascent",
    "Descent",
    "CapHeight",
    "XHeight",
    "ItalicAngle",
    "MaxWidth",
];

/// Appends the values of `keys` present in `dict`, resolved.
fn entries(doc: &Document, dict: &Value, keys: &[&str], out: &mut Vec<Token>) {
    for key in keys {
        if let Some(value) = dict.get(key) {
            value_tokens(doc.resolve(value), out);
        }
    }
}

/// The ranges of a `ToUnicode` `CMap`.
fn unicode_ranges(doc: &Document, font: &Value, out: &mut Vec<Token>) {
    if let Some(cmap) = font.get("ToUnicode").and_then(|c| doc.stream(c)) {
        let text = String::from_utf8_lossy(&cmap).into_owned();
        if let Some(ranges) = text
            .split("beginbfrange")
            .nth(1)
            .and_then(|r| r.split("endbfrange").next())
        {
            out.extend(ranges.split_whitespace().map(Token::word));
        }
    }
}

/// A Type 3 font: metrics, encoding, glyph procedures and text mapping.
fn type3(doc: &Document, font: &Value, item: &mut Item) {
    entries(
        doc,
        font,
        &[
            "Subtype",
            "FirstChar",
            "LastChar",
            "FontBBox",
            "FontMatrix",
            "Encoding",
            "Widths",
        ],
        &mut item.tokens,
    );
    let descriptor = doc.resolve(font.get("FontDescriptor").unwrap_or(&Value::Null));
    entries(doc, descriptor, &DESCRIPTOR_KEYS, &mut item.tokens);
    if let Value::Dict(procs) = doc.resolve(font.get("CharProcs").unwrap_or(&Value::Null)) {
        let mut sorted: Vec<&(String, Value)> = procs.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, proc) in sorted {
            item.tokens.push(Token::word(format!("/{name}")));
            let mut glyph = Interpreter::new(doc);
            glyph.run(&Value::Null, &doc.stream(proc).unwrap_or_default());
            for op in glyph.items {
                item.tokens.push(Token::word(op.kind));
                item.tokens.extend(op.tokens);
            }
        }
    }
    unicode_ranges(doc, font, &mut item.tokens);
}

/// A Type 0 font: encoding, the descendant font's widths and glyph
/// mapping, the outlines of the mapped glyphs and the text mapping.
fn type0(doc: &Document, font: &Value, item: &mut Item) {
    entries(doc, font, &["Subtype", "Encoding"], &mut item.tokens);
    let descendant = font
        .get("DescendantFonts")
        .map(|d| doc.resolve(d))
        .and_then(Value::as_array)
        .and_then(<[Value]>::first)
        .map_or(&Value::Null, |d| doc.resolve(d));
    entries(
        doc,
        descendant,
        &["Subtype", "CIDSystemInfo", "W"],
        &mut item.tokens,
    );
    let descriptor = doc.resolve(descendant.get("FontDescriptor").unwrap_or(&Value::Null));
    entries(doc, descriptor, &DESCRIPTOR_KEYS, &mut item.tokens);
    let map = descendant
        .get("CIDToGIDMap")
        .and_then(|m| doc.stream(m))
        .unwrap_or_default();
    let glyphs: Vec<u16> = map
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_be_bytes(pair))
        .collect();
    item.tokens.push(Token::word("cid-to-gid"));
    item.tokens
        .extend(glyphs.iter().map(|&g| Token::Num(f64::from(g))));
    let program = descriptor
        .get("FontFile2")
        .and_then(|f| doc.stream(f))
        .unwrap_or_default();
    let mut used: Vec<u16> = glyphs.into_iter().filter(|&g| g != 0).collect();
    used.sort_unstable();
    used.dedup();
    glyph_tokens(&program, used, &mut item.tokens);
    unicode_ranges(doc, font, &mut item.tokens);
}

/// An embedded font as one item, named by its base font without subset
/// tag.
pub(super) fn font_item(doc: &Document, font: &Value) -> Item {
    let mut item = Item::new("font");
    let base = font.get("BaseFont").and_then(Value::as_name).unwrap_or("");
    item.tokens.push(Token::word(base_name(base)));
    if font.get("Subtype").and_then(Value::as_name) == Some("Type0") {
        type0(doc, font, &mut item);
    } else {
        type3(doc, font, &mut item);
    }
    item
}
