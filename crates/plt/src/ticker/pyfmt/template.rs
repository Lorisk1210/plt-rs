//! `str.format` templates with the fields `x` (a float) and `pos` (an
//! optional integer), as used by matplotlib's `StrMethodFormatter`.

use super::FormatError;
use super::dtoa::{FloatStyle, format_float};
use super::spec::FormatSpec;

/// The argument a replacement field refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arg {
    /// `{x}`: the tick value.
    X,
    /// `{pos}`: the tick index.
    Pos,
}

/// A replacement field such as `{x:.2f}` or `{pos!r:>3}`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Field {
    arg: Arg,
    /// `!r`, `!s` or `!a`: format the argument's string form instead.
    as_text: bool,
    spec: FormatSpec,
    /// The spec text, needed to reject non-empty specs for `pos = None`.
    spec_is_empty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Literal(String),
    Field(Field),
}

/// A parsed `str.format` template whose fields may only name `x` and
/// `pos` (matplotlib's `StrMethodFormatter` convention).
///
/// ```
/// use plt::ticker::pyfmt::StrTemplate;
/// let t = StrTemplate::parse("{pos}: {x:.1f} km").unwrap();
/// assert_eq!(t.render(2.25, Some(3), |s| s).unwrap(), "3: 2.2 km");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StrTemplate {
    template: String,
    parts: Vec<Part>,
}

impl StrTemplate {
    /// Parses and validates `template`.
    pub fn parse(template: &str) -> Result<Self, FormatError> {
        let mut parts = Vec::new();
        let mut literal = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' if chars.next_if_eq(&'{').is_some() => literal.push('{'),
                '}' if chars.next_if_eq(&'}').is_some() => literal.push('}'),
                '}' => {
                    return Err(FormatError::Value(
                        "Single '}' encountered in format string".into(),
                    ));
                }
                '{' => {
                    if chars.peek().is_none() {
                        return Err(FormatError::Value(
                            "Single '{' encountered in format string".into(),
                        ));
                    }
                    let mut depth = 1;
                    let mut body = String::new();
                    for c in chars.by_ref() {
                        match c {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        if depth == 0 {
                            break;
                        }
                        body.push(c);
                    }
                    if depth != 0 {
                        return Err(FormatError::Value(
                            "expected '}' before end of string".into(),
                        ));
                    }
                    if !literal.is_empty() {
                        parts.push(Part::Literal(std::mem::take(&mut literal)));
                    }
                    parts.push(Part::Field(parse_field(&body)?));
                }
                c => literal.push(c),
            }
        }
        if !literal.is_empty() {
            parts.push(Part::Literal(literal));
        }
        Ok(Self {
            template: template.to_owned(),
            parts,
        })
    }

    /// The template text.
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Renders the template for tick value `x` at index `pos`; `post` is
    /// applied to each formatted field (matplotlib passes `fix_minus`).
    ///
    /// Fails only when a field formats `pos = None` with a non-empty spec
    /// (Python's `TypeError`).
    pub fn render(
        &self,
        x: f64,
        pos: Option<usize>,
        mut post: impl FnMut(String) -> String,
    ) -> Result<String, FormatError> {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(text) => out.push_str(text),
                Part::Field(field) => out.push_str(&post(field.render(x, pos)?)),
            }
        }
        Ok(out)
    }
}

impl Field {
    fn render(&self, x: f64, pos: Option<usize>) -> Result<String, FormatError> {
        match (self.arg, pos) {
            (Arg::X, _) if self.as_text => self.spec.format_str(&format_float(x, FloatStyle::REPR)),
            (Arg::X, _) => self.spec.format_float(x),
            (Arg::Pos, Some(pos)) if self.as_text => self.spec.format_str(&pos.to_string()),
            (Arg::Pos, Some(pos)) => {
                let pos = i64::try_from(pos).map_err(|_| {
                    FormatError::Value("tick position does not fit an integer".into())
                })?;
                self.spec.format_int(pos)
            }
            (Arg::Pos, None) if self.as_text || self.spec_is_empty => {
                self.spec.format_str("None")
            }
            (Arg::Pos, None) => Err(FormatError::Type(
                "unsupported format string passed to NoneType.__format__".into(),
            )),
        }
    }
}

/// Parses `name[!conv][:spec]` and validates it against the argument type.
fn parse_field(body: &str) -> Result<Field, FormatError> {
    let name_end = body.find([':', '!']).unwrap_or(body.len());
    let (name, rest) = body.split_at(name_end);
    let (as_text, spec_text) = match rest.strip_prefix('!') {
        Some(conv) => {
            let mut chars = conv.chars();
            match (chars.next(), chars.as_str()) {
                (None, _) => {
                    return Err(FormatError::Value(
                        "end of string while looking for conversion specifier".into(),
                    ));
                }
                (Some('r' | 's' | 'a'), "") => (true, ""),
                (Some('r' | 's' | 'a'), tail) => match tail.strip_prefix(':') {
                    Some(spec) => (true, spec),
                    None => {
                        return Err(FormatError::Value(
                            "expected ':' after conversion specifier".into(),
                        ));
                    }
                },
                (Some(other), _) => {
                    return Err(FormatError::Value(format!(
                        "Unknown conversion specifier {other}"
                    )));
                }
            }
        }
        None => (false, rest.strip_prefix(':').unwrap_or("")),
    };
    let arg = match name {
        "x" => Arg::X,
        "pos" => Arg::Pos,
        "" => {
            return Err(FormatError::Index(
                "Replacement index 0 out of range for positional args tuple".into(),
            ));
        }
        n if n.bytes().all(|b| b.is_ascii_digit()) => {
            return Err(FormatError::Index(format!(
                "Replacement index {n} out of range for positional args tuple"
            )));
        }
        n if n.starts_with("x.") || n.starts_with("x[") || n.starts_with("pos") => {
            return Err(FormatError::Unsupported(format!(
                "attribute or index access in field {n:?}"
            )));
        }
        n => return Err(FormatError::Key(n.to_owned())),
    };
    if spec_text.contains('{') {
        return Err(FormatError::Unsupported(
            "nested replacement fields in a format spec".into(),
        ));
    }
    let spec = FormatSpec::parse(spec_text)?;
    match (arg, as_text) {
        (_, true) => spec.check_str()?,
        (Arg::X, false) => spec.check_float()?,
        (Arg::Pos, false) => spec.check_int()?,
    }
    Ok(Field {
        arg,
        as_text,
        spec,
        spec_is_empty: spec_text.is_empty(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(template: &str, x: f64, pos: Option<usize>) -> String {
        StrTemplate::parse(template)
            .unwrap()
            .render(x, pos, |s| s)
            .unwrap()
    }

    #[test]
    fn renders_fields_and_escapes() {
        assert_eq!(render("{x}", -1e-5, Some(0)), "-1e-05");
        assert_eq!(render("{x} km", 1e7, Some(0)), "10000000.0 km");
        assert_eq!(render("{pos}:{x:g}", 1e7, Some(9)), "9:1e+07");
        assert_eq!(render("{{x}}={x:.1f}", 0.25, None), "{x}=0.2");
        assert_eq!(render("{pos}", 0.0, None), "None");
        assert_eq!(render("{x!r:>8}", 1.5, None), "     1.5");
        assert_eq!(render("{pos:>3}", 0.0, Some(7)), "  7");
    }

    #[test]
    fn post_processing_applies_to_fields_only() {
        let t = StrTemplate::parse("a-{x}").unwrap();
        let out = t.render(-2.0, None, |s| s.replace('-', "\u{2212}")).unwrap();
        assert_eq!(out, "a-\u{2212}2.0");
    }

    #[test]
    fn rejects_invalid_templates() {
        assert!(matches!(StrTemplate::parse("{y}"), Err(FormatError::Key(_))));
        assert!(matches!(StrTemplate::parse("{}"), Err(FormatError::Index(_))));
        assert!(matches!(StrTemplate::parse("{x:#x}"), Err(FormatError::Value(_))));
        assert!(matches!(StrTemplate::parse("x}"), Err(FormatError::Value(_))));
        assert!(matches!(StrTemplate::parse("{x"), Err(FormatError::Value(_))));
        assert!(matches!(StrTemplate::parse("{x!q}"), Err(FormatError::Value(_))));
        assert!(
            StrTemplate::parse("{pos:.2f}")
                .unwrap()
                .render(0.0, None, |s| s)
                .is_err()
        );
    }
}
