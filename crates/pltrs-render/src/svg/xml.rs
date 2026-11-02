//! matplotlib's `XMLWriter`: a streaming XML writer with one space of
//! indentation per level, empty elements collapsed to `<tag/>` and
//! attributes without a value left out.

/// An element attribute: name and value.
pub type Attribute<'a> = (&'a str, String);

/// `_escape_cdata`.
fn escape_cdata(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `_escape_comment`: character data escaping plus a space between
/// consecutive hyphens.
fn escape_comment(text: &str) -> String {
    let escaped = escape_cdata(text);
    let mut out = String::with_capacity(escaped.len());
    let mut chars = escaped.chars().peekable();
    while let Some(c) = chars.next() {
        out.push(c);
        if c == '-' && chars.peek() == Some(&'-') {
            out.push(' ');
        }
    }
    out
}

/// `_quote_escape_attrib`: double quotes unless the value contains one,
/// then single quotes, then full escaping.
fn quote_attribute(value: &str) -> String {
    if !value.contains('"') {
        format!("\"{}\"", escape_cdata(value))
    } else if !value.contains('\'') {
        format!("'{}'", escape_cdata(value))
    } else {
        let escaped = value
            .replace('&', "&amp;")
            .replace('\'', "&apos;")
            .replace('"', "&quot;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        format!("\"{escaped}\"")
    }
}

/// The deepest indentation written (matplotlib's 64-space buffer).
const MAX_INDENT: usize = 64;

/// Writes an XML document into a string, which [`take`](Self::take)
/// hands out as it grows (matplotlib's `backend_svg.XMLWriter`).
///
/// ```
/// use pltrs_render::XmlWriter;
/// let mut xml = XmlWriter::new("");
/// let svg = xml.start("svg", &[("width", "10pt".into())]);
/// xml.element("title", "a < b", &[]);
/// xml.close(svg);
/// assert_eq!(xml.take(), "<svg width=\"10pt\">\n <title>a &lt; b</title>\n</svg>\n");
/// assert_eq!(xml.take(), "");
/// ```
#[derive(Debug, Default)]
pub struct XmlWriter {
    out: String,
    tags: Vec<String>,
    /// Whether the last start tag still lacks its closing `>`.
    open: bool,
    data: String,
}

impl XmlWriter {
    /// A writer appending to `prolog`.
    pub fn new(prolog: &str) -> Self {
        Self {
            out: prolog.to_owned(),
            ..Self::default()
        }
    }

    fn indent(&mut self, depth: usize) {
        self.out
            .extend(std::iter::repeat_n(' ', depth.min(MAX_INDENT)));
    }

    /// Finishes a pending start tag and writes buffered character data.
    fn flush(&mut self, indent: bool) {
        if self.open {
            self.out.push_str(if indent { ">\n" } else { ">" });
            self.open = false;
        }
        if !self.data.is_empty() {
            let data = std::mem::take(&mut self.data);
            self.out.push_str(&escape_cdata(&data));
        }
    }

    /// Opens an element; returns its depth for [`close`](Self::close).
    pub fn start(&mut self, tag: &str, attributes: &[Attribute<'_>]) -> usize {
        self.flush(true);
        self.data.clear();
        let tag = escape_cdata(tag);
        self.indent(self.tags.len());
        self.out.push('<');
        self.out.push_str(&tag);
        self.tags.push(tag);
        for (name, value) in attributes {
            if !value.is_empty() {
                self.out.push(' ');
                self.out.push_str(&escape_cdata(name));
                self.out.push('=');
                self.out.push_str(&quote_attribute(value));
            }
        }
        self.open = true;
        self.tags.len() - 1
    }

    /// Writes a comment.
    pub fn comment(&mut self, text: &str) {
        self.flush(true);
        self.indent(self.tags.len());
        self.out.push_str("<!-- ");
        self.out.push_str(&escape_comment(text));
        self.out.push_str(" -->\n");
    }

    /// Adds character data to the current element.
    pub fn data(&mut self, text: &str) {
        self.data.push_str(text);
    }

    /// Closes the current element; `indent` indents the end tag.
    pub fn end(&mut self, indent: bool) {
        let Some(tag) = self.tags.pop() else {
            return;
        };
        if !self.data.is_empty() {
            self.flush(indent);
        } else if self.open {
            self.open = false;
            self.out.push_str("/>\n");
            return;
        }
        if indent {
            self.indent(self.tags.len());
        }
        self.out.push_str("</");
        self.out.push_str(&tag);
        self.out.push_str(">\n");
    }

    /// Closes every element down to and including depth `id`.
    pub fn close(&mut self, id: usize) {
        while self.tags.len() > id {
            self.end(true);
        }
    }

    /// Writes a whole element with optional character data.
    pub fn element(&mut self, tag: &str, text: &str, attributes: &[Attribute<'_>]) {
        self.start(tag, attributes);
        if !text.is_empty() {
            self.data(text);
        }
        self.end(false);
    }

    /// The open elements, outermost first, as their tags were escaped.
    pub fn open_tags(&self) -> &[String] {
        &self.tags
    }

    /// Whether `tag` names the innermost open element (matplotlib's check
    /// of `end(tag)`).
    pub fn closes(&self, tag: &str) -> bool {
        self.tags.last() == Some(&escape_cdata(tag))
    }

    /// The text written since the last call, which the writer forgets.
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    /// The document written so far.
    pub(crate) fn into_string(self) -> String {
        self.out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_like_matplotlib() {
        let mut w = XmlWriter::new("");
        let svg = w.start("svg", &[("width", "10pt".into()), ("id", String::new())]);
        w.start("defs", &[]);
        w.element("style", "*{a: b}", &[("type", "text/css".into())]);
        w.end(true);
        w.start("g", &[]);
        w.comment("a--b <c>");
        w.element("path", "", &[("d", "M 0 0 \nz\n".into())]);
        w.close(svg);
        assert_eq!(
            w.into_string(),
            "<svg width=\"10pt\">\n <defs>\n  <style type=\"text/css\">*{a: b}</style>\n \
             </defs>\n <g>\n  <!-- a- -b &lt;c&gt; -->\n  <path d=\"M 0 0 \nz\n\"/>\n </g>\n</svg>\n"
        );
    }

    #[test]
    fn end_tags_are_checked_against_the_open_element() {
        let mut w = XmlWriter::new("");
        w.start("a&b", &[]);
        assert!(w.closes("a&b"));
        assert!(!w.closes("a"));
        w.end(true);
        assert!(!w.closes("a&b"));
        assert_eq!(w.take(), "<a&amp;b/>\n");
    }

    #[test]
    fn quotes_attributes() {
        assert_eq!(quote_attribute("a\"b"), "'a\"b'");
        assert_eq!(quote_attribute("a\"b'c"), "\"a&quot;b&apos;c\"");
        assert_eq!(quote_attribute("x<y"), "\"x&lt;y\"");
    }
}
