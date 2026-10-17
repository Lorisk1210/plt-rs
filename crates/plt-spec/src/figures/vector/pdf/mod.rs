//! PDF documents as drawing items.
//!
//! The page's content stream is interpreted ([`interp`]): every painted
//! path, glyph, image and shading becomes one item carrying the
//! graphics state that affects it, so redundant or differently ordered
//! state operators do not matter. The embedded fonts follow as items of
//! their own: metrics, encoding and glyph procedures.

mod interp;
mod samples;
mod syntax;

use self::interp::Interpreter;
use self::syntax::{Document, Value};
use super::diff::{Item, Token};

/// A font's name without its subset tag (`ABCDEF+DejaVuSans`).
fn base_name(name: &str) -> &str {
    match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    }
}

/// Plain tokens for a value: numbers, names and strings as words, arrays
/// and dictionaries flattened (dictionary keys sorted).
fn value_tokens(value: &Value, out: &mut Vec<Token>) {
    match value {
        Value::Num(n) => out.push(Token::Num(*n)),
        Value::Name(n) => out.push(Token::word(format!("/{}", base_name(n)))),
        Value::Str(s) => out.push(Token::word(format!("({})", String::from_utf8_lossy(s)))),
        Value::Bool(b) => out.push(Token::word(b.to_string())),
        Value::Null => out.push(Token::word("null")),
        Value::Ref(_) => out.push(Token::word("ref")),
        Value::Op(op) => out.push(Token::word(op.clone())),
        Value::Array(items) => {
            out.push(Token::word("["));
            for item in items {
                value_tokens(item, out);
            }
            out.push(Token::word("]"));
        }
        Value::Dict(entries) => {
            let mut sorted: Vec<&(String, Value)> = entries.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            out.push(Token::word("<<"));
            for (key, value) in sorted {
                out.push(Token::word(format!("/{key}")));
                value_tokens(value, out);
            }
            out.push(Token::word(">>"));
        }
    }
}

/// An embedded font: its metrics, encoding, glyph procedures and text
/// mapping.
fn font_item(doc: &Document, font: &Value) -> Item {
    let mut item = Item::new("font");
    let base = font.get("BaseFont").and_then(Value::as_name).unwrap_or("");
    item.tokens.push(Token::word(base_name(base)));
    for key in [
        "Subtype",
        "FirstChar",
        "LastChar",
        "FontBBox",
        "FontMatrix",
        "Encoding",
        "Widths",
    ] {
        if let Some(value) = font.get(key) {
            value_tokens(doc.resolve(value), &mut item.tokens);
        }
    }
    let descriptor = doc.resolve(font.get("FontDescriptor").unwrap_or(&Value::Null));
    for key in [
        "Flags",
        "Ascent",
        "Descent",
        "CapHeight",
        "XHeight",
        "ItalicAngle",
        "MaxWidth",
    ] {
        if let Some(value) = descriptor.get(key) {
            value_tokens(value, &mut item.tokens);
        }
    }
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
    if let Some(cmap) = font.get("ToUnicode").and_then(|c| doc.stream(c)) {
        let text = String::from_utf8_lossy(&cmap).into_owned();
        if let Some(ranges) = text
            .split("beginbfrange")
            .nth(1)
            .and_then(|r| r.split("endbfrange").next())
        {
            item.tokens
                .extend(ranges.split_whitespace().map(Token::word));
        }
    }
    item
}

/// The drawing items of a PDF document: the page size, every painted
/// element, then the embedded fonts by name.
pub(super) fn items(document: &[u8]) -> Result<Vec<Item>, String> {
    let doc = Document::parse(document)?;
    let root = doc.resolve(doc.trailer.get("Root").ok_or("no root")?);
    let pages = doc.resolve(root.get("Pages").ok_or("no page tree")?);
    let page = pages
        .get("Kids")
        .and_then(Value::as_array)
        .and_then(|kids| kids.first())
        .map(|kid| doc.resolve(kid))
        .ok_or("no page")?;
    let mut size = Item::new("page");
    if let Some(media) = page.get("MediaBox") {
        value_tokens(media, &mut size.tokens);
    }
    let resources = doc.resolve(page.get("Resources").unwrap_or(&Value::Null));
    let contents = doc
        .stream(page.get("Contents").ok_or("no contents")?)
        .ok_or("unreadable contents")?;
    let mut interpreter = Interpreter::new(&doc);
    interpreter.run(resources, &contents);
    let mut items = vec![size];
    items.extend(interpreter.items);
    if let Value::Dict(fonts) = doc.resolve(resources.get("Font").unwrap_or(&Value::Null)) {
        let mut fonts: Vec<Item> = fonts
            .iter()
            .map(|(_, font)| font_item(&doc, doc.resolve(font)))
            .collect();
        fonts.sort_by_key(ToString::to_string);
        items.extend(fonts);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset_tags_are_dropped() {
        assert_eq!(base_name("CIVIRS+DejaVuSans"), "DejaVuSans");
        assert_eq!(base_name("DejaVuSans"), "DejaVuSans");
        assert_eq!(base_name("ab+c"), "ab+c");
    }
}
