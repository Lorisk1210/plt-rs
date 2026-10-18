//! PDF documents as drawing items.
//!
//! Every page's content stream is interpreted ([`interp`]): every painted
//! path, glyph, image and shading becomes one item carrying the
//! graphics state that affects it, so redundant or differently ordered
//! state operators do not matter. The embedded fonts follow as items of
//! their own: metrics, encoding and glyph procedures.

mod fonts;
mod interp;
mod samples;
mod syntax;

use self::fonts::font_item;
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

/// The items of one page: its size and annotations, then every painted
/// element.
fn page_items(doc: &Document, page: &Value, items: &mut Vec<Item>) -> Result<(), String> {
    let mut header = Item::new("page");
    if let Some(media) = page.get("MediaBox") {
        value_tokens(media, &mut header.tokens);
    }
    let annotations = page
        .get("Annots")
        .map(|a| doc.resolve(a))
        .and_then(Value::as_array)
        .unwrap_or_default();
    for annotation in annotations {
        value_tokens(doc.resolve(annotation), &mut header.tokens);
    }
    items.push(header);
    let resources = doc.resolve(page.get("Resources").unwrap_or(&Value::Null));
    let contents = doc
        .stream(page.get("Contents").ok_or("no contents")?)
        .ok_or("unreadable contents")?;
    let mut interpreter = Interpreter::new(doc);
    interpreter.run(resources, &contents);
    items.extend(interpreter.items);
    Ok(())
}

/// The drawing items of a PDF document: every page (its size,
/// annotations and painted elements), then the embedded fonts by name.
pub(super) fn items(document: &[u8]) -> Result<Vec<Item>, String> {
    let doc = Document::parse(document)?;
    let root = doc.resolve(doc.trailer.get("Root").ok_or("no root")?);
    let tree = doc.resolve(root.get("Pages").ok_or("no page tree")?);
    let pages: Vec<&Value> = tree
        .get("Kids")
        .and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .map(|kid| doc.resolve(kid))
        .collect();
    let mut items = Vec::new();
    for page in &pages {
        page_items(&doc, page, &mut items)?;
    }
    // The pages share one resource dictionary.
    let resources = pages.first().map_or(&Value::Null, |page| {
        doc.resolve(page.get("Resources").unwrap_or(&Value::Null))
    });
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
