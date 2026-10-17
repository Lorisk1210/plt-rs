//! A small XML reader for SVG documents: elements, attributes and text;
//! the declaration, doctype and comments are skipped.

/// An element with its attributes (in document order), children and text.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Element {
    pub(super) name: String,
    pub(super) attributes: Vec<(String, String)>,
    pub(super) children: Vec<Element>,
    pub(super) text: String,
}

impl Element {
    /// The value of attribute `name`.
    pub(super) fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Replaces the predefined and numeric entities.
fn unescape(text: &str) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let end = rest[start..]
            .find(';')
            .ok_or_else(|| format!("unterminated entity in {text:?}"))?;
        let entity = &rest[start + 1..start + end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let code = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16)
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse()
                } else {
                    return Err(format!("unknown entity &{entity};"));
                };
                code.ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| format!("bad character reference &{entity};"))?
            }
        };
        out.push(c);
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// A cursor over the document text.
struct Reader<'a> {
    text: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn rest(&self) -> &'a str {
        &self.text[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        let rest = self.rest();
        self.pos += rest.len() - rest.trim_start().len();
    }

    /// Skips past the next `end`.
    fn skip_past(&mut self, end: &str) -> Result<(), String> {
        let at = self
            .rest()
            .find(end)
            .ok_or_else(|| format!("missing {end:?}"))?;
        self.pos += at + end.len();
        Ok(())
    }

    /// Skips comments, processing instructions and declarations.
    fn skip_misc(&mut self) -> Result<(), String> {
        loop {
            self.skip_whitespace();
            let rest = self.rest();
            if rest.starts_with("<!--") {
                self.skip_past("-->")?;
            } else if rest.starts_with("<?") {
                self.skip_past("?>")?;
            } else if rest.starts_with("<!") {
                self.skip_past(">")?;
            } else {
                return Ok(());
            }
        }
    }

    fn name(&mut self) -> String {
        let rest = self.rest();
        let len = rest
            .find(|c: char| c.is_whitespace() || matches!(c, '>' | '/' | '='))
            .unwrap_or(rest.len());
        self.pos += len;
        rest[..len].to_owned()
    }

    /// Reads an element starting at `<`.
    fn element(&mut self) -> Result<Element, String> {
        self.pos += 1;
        let mut element = Element {
            name: self.name(),
            ..Element::default()
        };
        loop {
            self.skip_whitespace();
            let rest = self.rest();
            if rest.starts_with("/>") {
                self.pos += 2;
                return Ok(element);
            }
            if rest.starts_with('>') {
                self.pos += 1;
                break;
            }
            let name = self.name();
            self.skip_whitespace();
            if !self.rest().starts_with('=') {
                return Err(format!("attribute {name} without value"));
            }
            self.pos += 1;
            self.skip_whitespace();
            let quote = self.rest().chars().next().ok_or("truncated attribute")?;
            if quote != '"' && quote != '\'' {
                return Err(format!("unquoted attribute {name}"));
            }
            self.pos += 1;
            let end = self
                .rest()
                .find(quote)
                .ok_or_else(|| format!("unterminated attribute {name}"))?;
            let value = unescape(&self.rest()[..end])?;
            self.pos += end + 1;
            element.attributes.push((name, value));
        }
        loop {
            let rest = self.rest();
            let next = rest.find('<').ok_or("unterminated element")?;
            element.text.push_str(&unescape(&rest[..next])?);
            self.pos += next;
            let rest = self.rest();
            if rest.starts_with("</") {
                self.skip_past(">")?;
                return Ok(element);
            }
            if rest.starts_with("<!--") {
                self.skip_past("-->")?;
            } else {
                element.children.push(self.element()?);
            }
        }
    }
}

/// Parses a document into its root element.
pub(super) fn parse(text: &str) -> Result<Element, String> {
    let mut reader = Reader { text, pos: 0 };
    reader.skip_misc()?;
    if !reader.rest().starts_with('<') {
        return Err("no root element".to_owned());
    }
    reader.element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_svg_like_documents() {
        let doc = "<?xml version=\"1.0\"?>\n<!DOCTYPE svg PUBLIC \"x\"\n \"y\">\n\
                   <svg a=\"1\" b='x&lt;y'>\n <!-- c -- d -->\n <g id=\"g\">\
                   <path d=\"M 0 0 \nz\"/></g>\n <style>*{a: b}</style>\n</svg>\n";
        let root = parse(doc).unwrap();
        assert_eq!(root.name, "svg");
        assert_eq!(root.attr("b"), Some("x<y"));
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].children[0].attr("d"), Some("M 0 0 \nz"));
        assert_eq!(root.children[1].text, "*{a: b}");
        assert!(parse("<a><b></a>").is_err());
        assert_eq!(unescape("&#65;&#x42;&amp;").unwrap(), "AB&");
    }
}
