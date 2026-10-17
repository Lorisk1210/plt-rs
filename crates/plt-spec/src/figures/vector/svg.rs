//! SVG documents as drawing items: one item per drawn element, carrying
//! the significant attributes of its enclosing groups, with `<use>`
//! targets, clip paths, patterns and gradients resolved and images
//! reduced to their size and mean colour.

use std::collections::HashMap;

use super::diff::{Item, Token};
use super::xml::{self, Element};

/// Elements that are never drawn.
const SKIPPED: &[&str] = &["defs", "metadata", "title", "style", "filter"];

/// Every element with an `id`, by id.
type Defs<'a> = HashMap<&'a str, &'a Element>;

fn collect_defs<'a>(element: &'a Element, defs: &mut Defs<'a>) {
    if let Some(id) = element.attr("id") {
        defs.insert(id, element);
    }
    for child in &element.children {
        collect_defs(child, defs);
    }
}

/// Splits `text` into words and numbers at whitespace, commas and
/// parentheses.
fn split_values(text: &str, out: &mut Vec<Token>) {
    for part in text.split(|c: char| c.is_whitespace() || matches!(c, ',' | '(' | ')')) {
        if part.is_empty() {
            continue;
        }
        let number = part.strip_suffix("pt").unwrap_or(part);
        match number.parse::<f64>() {
            Ok(value) => out.push(Token::Num(value)),
            Err(_) => out.push(Token::word(part)),
        }
    }
}

/// Whether a `transform` ends with `scale(s -s)`, how text groups map
/// glyph units to the page.
#[expect(clippy::float_cmp, reason = "text groups scale by exactly (s, -s)")]
fn flips_y(transform: &str) -> bool {
    transform
        .rsplit("scale(")
        .next()
        .filter(|_| transform.contains("scale("))
        .and_then(|args| args.strip_suffix(')'))
        .and_then(|args| args.split_once(' '))
        .and_then(|(sx, sy)| Some((sx.parse::<f64>().ok()?, sy.parse::<f64>().ok()?)))
        .is_some_and(|(sx, sy)| sx > 0.0 && sy == -sx)
}

/// The id inside `url(#id)`, if `value` is one.
fn url_target(value: &str) -> Option<&str> {
    value.strip_prefix("url(#")?.strip_suffix(')')
}

/// Resolves references while normalizing elements.
struct Normalizer<'a> {
    defs: Defs<'a>,
}

impl Normalizer<'_> {
    /// One attribute value: references resolved, styles sorted by
    /// property, everything else split into words and numbers.
    fn value(&self, name: &str, value: &str, out: &mut Vec<Token>) {
        if let Some(target) = url_target(value) {
            self.reference(target, out);
        } else if name == "xlink:href" {
            if let Some(target) = value.strip_prefix('#') {
                self.reference(target, out);
            } else if let Some(data) = value.strip_prefix("data:image/png;base64,") {
                image_tokens(data, out);
            } else {
                out.push(Token::word(value));
            }
        } else if name == "style" {
            let mut properties: Vec<(&str, &str)> = value
                .split(';')
                .filter_map(|p| p.split_once(':'))
                .map(|(k, v)| (k.trim(), v.trim()))
                .collect();
            properties.sort_by_key(|(k, _)| *k);
            for (key, value) in properties {
                out.push(Token::word(format!("{key}:")));
                self.value(key, value, out);
            }
        } else {
            split_values(value, out);
        }
    }

    /// The content of the element with id `target` (fixed filter ids are
    /// kept as names).
    fn reference(&self, target: &str, out: &mut Vec<Token>) {
        match self.defs.get(target) {
            Some(element) if element.name != "filter" => self.element(element, out),
            _ => out.push(Token::word(format!("#{target}"))),
        }
    }

    /// An element with its attributes (sorted, ids dropped), children and
    /// text.
    fn element(&self, element: &Element, out: &mut Vec<Token>) {
        out.push(Token::word(format!("<{}", element.name)));
        let mut attributes: Vec<&(String, String)> = element
            .attributes
            .iter()
            .filter(|(name, _)| name != "id")
            .collect();
        attributes.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, value) in attributes {
            out.push(Token::word(format!("{name}=")));
            self.value(name, value, out);
        }
        for child in &element.children {
            self.element(child, out);
        }
        if !element.text.trim().is_empty() {
            out.push(Token::word(element.text.trim()));
        }
        out.push(Token::word(">"));
    }

    /// The tokens a group passes on to what it contains: its attributes
    /// other than `id`.
    fn group_context(&self, group: &Element) -> Vec<Token> {
        let mut context = Vec::new();
        let mut attributes: Vec<&(String, String)> = group
            .attributes
            .iter()
            .filter(|(name, _)| name != "id")
            .collect();
        attributes.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, value) in attributes {
            context.push(Token::word(format!("{}={name}=", group.name)));
            self.value(name, value, &mut context);
        }
        context
    }

    /// Appends the drawn elements under `element` to `items`; elements of
    /// a text group (one flipping y to glyph coordinates) are labelled
    /// `text:`.
    fn walk(&self, element: &Element, context: &[Token], text: bool, items: &mut Vec<Item>) {
        if SKIPPED.contains(&element.name.as_str()) {
            return;
        }
        if element.name == "g" || element.name == "a" {
            let mut inner = context.to_vec();
            inner.extend(self.group_context(element));
            let text = text || element.attr("transform").is_some_and(flips_y);
            for child in &element.children {
                self.walk(child, &inner, text, items);
            }
            return;
        }
        let kind = if text {
            format!("text:{}", element.name)
        } else {
            element.name.clone()
        };
        let mut item = Item::new(kind);
        item.tokens.extend_from_slice(context);
        self.element(element, &mut item.tokens);
        items.push(item);
    }
}

/// An embedded PNG as its size and mean channel values.
fn image_tokens(data: &str, out: &mut Vec<Token>) {
    let bytes = decode_base64(data);
    match plt_codec::png::decode(&bytes) {
        Ok(image) => {
            let rgba = image.to_rgba8();
            let pixels = (rgba.len() / 4).max(1) as f64;
            out.push(Token::word("png"));
            out.push(Token::Num(f64::from(image.width())));
            out.push(Token::Num(f64::from(image.height())));
            for channel in 0..4 {
                let sum: f64 = rgba
                    .iter()
                    .skip(channel)
                    .step_by(4)
                    .map(|&v| f64::from(v))
                    .sum();
                out.push(Token::Num(sum / pixels));
            }
        }
        Err(err) => out.push(Token::word(format!("bad-png:{err}"))),
    }
}

/// Decodes standard Base64, ignoring whitespace.
fn decode_base64(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let sextets: Vec<u8> = text.bytes().filter_map(value).collect();
    let mut out = Vec::with_capacity(sextets.len() * 3 / 4);
    for chunk in sextets.chunks(4) {
        let mut word = 0u32;
        for (i, &s) in chunk.iter().enumerate() {
            word |= u32::from(s) << (18 - 6 * i);
        }
        let bytes = word.to_be_bytes();
        out.extend_from_slice(&bytes[1..chunk.len()]);
    }
    out
}

/// The drawing items of an SVG document: the canvas size, then every
/// drawn element in order.
pub(super) fn items(document: &[u8]) -> Result<Vec<Item>, String> {
    let text = std::str::from_utf8(document).map_err(|e| e.to_string())?;
    let root = xml::parse(text)?;
    let mut defs = Defs::new();
    collect_defs(&root, &mut defs);
    let normalizer = Normalizer { defs };
    let mut canvas = Item::new("svg");
    for name in ["width", "height", "viewBox"] {
        if let Some(value) = root.attr(name) {
            split_values(value, &mut canvas.tokens);
        }
    }
    let mut items = vec![canvas];
    for child in &root.children {
        normalizer.walk(child, &[], false, &mut items);
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_references_and_flattens_groups() {
        let doc = br##"<svg width="10pt" height="5pt" viewBox="0 0 10 5">
 <defs><path id="m1" d="M 0 0 L 1 1" style="stroke: #000000"/>
 <clipPath id="p1"><rect x="0" y="0" width="4" height="4"/></clipPath></defs>
 <g id="figure_1"><g clip-path="url(#p1)">
  <use xlink:href="#m1" x="2" y="3" style="fill: none; stroke: #000000"/>
 </g></g></svg>"##;
        let items = items(doc).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].tokens,
            [
                Token::Num(10.0),
                Token::Num(5.0),
                Token::Num(0.0),
                Token::Num(0.0),
                Token::Num(10.0),
                Token::Num(5.0)
            ]
        );
        let text = items[1].to_string();
        assert!(
            text.starts_with("use g=clip-path= <clipPath <rect"),
            "{text}"
        );
        assert!(
            text.contains("<path d= M 0 0 L 1 1 style= stroke: #000000 >"),
            "{text}"
        );
        assert!(text.contains("style= fill: none stroke: #000000"), "{text}");
    }

    #[test]
    fn text_groups_are_recognised() {
        assert!(flips_y("translate(65.88 322.18) scale(0.1 -0.1)"));
        assert!(flips_y("translate(1 2) rotate(-90) scale(0.12 -0.12)"));
        assert!(!flips_y("scale(1 -1) translate(0 -5)"));
        assert!(!flips_y("translate(1 2)"));
        let doc = br##"<svg><g transform="translate(1 2) scale(0.1 -0.1)"><use xlink:href="#a"/></g><use xlink:href="#a"/></svg>"##;
        let items = items(doc).unwrap();
        assert_eq!(items[1].kind, "text:use");
        assert_eq!(items[2].kind, "use");
    }

    #[test]
    fn base64_round_trip() {
        assert_eq!(decode_base64("Zm9vYmE=\n"), b"fooba");
        assert_eq!(decode_base64("Zm9v"), b"foo");
    }
}
