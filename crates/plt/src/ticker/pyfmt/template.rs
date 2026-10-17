//! `str.format` templates with the fields `x` (a float) and `pos` (an
//! optional integer), as used by matplotlib's `StrMethodFormatter`, or
//! with one positional float.

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

/// How replacement fields name their argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Naming {
    /// The keywords `x` and `pos`.
    Keywords,
    /// One positional argument (`{}`, `{0}`).
    Positional(Numbering),
}

/// Python's field numbering state: unknown until the first positional
/// field, then automatic (`{}`, counting the fields so far) or manual
/// (`{0}`) for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Numbering {
    Unknown,
    Automatic(usize),
    Manual,
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
    /// Parses and validates `template`, whose fields may name `x` and
    /// `pos`.
    pub fn parse(template: &str) -> Result<Self, FormatError> {
        Self::parse_with(template, Naming::Keywords)
    }

    /// Parses and validates a template for `template.format(value)`: its
    /// fields refer to one positional float (`{}`, `{0}`, `{:.0%}`), the
    /// `x` of [`StrTemplate::render`].
    ///
    /// ```
    /// use plt::ticker::pyfmt::StrTemplate;
    /// let t = StrTemplate::parse_positional("{0:.0%} of {0}").unwrap();
    /// assert_eq!(t.render(0.25, None, |s| s).unwrap(), "25% of 0.25");
    /// assert!(StrTemplate::parse_positional("{} {}").is_err());
    /// ```
    pub fn parse_positional(template: &str) -> Result<Self, FormatError> {
        Self::parse_with(template, Naming::Positional(Numbering::Unknown))
    }

    fn parse_with(template: &str, mut naming: Naming) -> Result<Self, FormatError> {
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
                    parts.push(Part::Field(parse_field(&body, &mut naming)?));
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
            (Arg::Pos, None) if self.as_text || self.spec_is_empty => self.spec.format_str("None"),
            (Arg::Pos, None) => Err(FormatError::Type(
                "unsupported format string passed to NoneType.__format__".into(),
            )),
        }
    }
}

/// Parses `name[!conv][:spec]` and validates it against the argument type.
fn parse_field(body: &str, naming: &mut Naming) -> Result<Field, FormatError> {
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
    let arg = match naming {
        Naming::Keywords => keyword_arg(name)?,
        Naming::Positional(numbering) => positional_arg(name, numbering)?,
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

/// The argument a field names under the `x`/`pos` convention.
fn keyword_arg(name: &str) -> Result<Arg, FormatError> {
    match name {
        "x" => Ok(Arg::X),
        "pos" => Ok(Arg::Pos),
        "" => Err(out_of_range("0")),
        n if n.bytes().all(|b| b.is_ascii_digit()) => Err(out_of_range(n)),
        n if n.starts_with("x.") || n.starts_with("x[") || n.starts_with("pos") => Err(
            FormatError::Unsupported(format!("attribute or index access in field {n:?}")),
        ),
        n => Err(FormatError::Key(n.to_owned())),
    }
}

/// The argument a field names when the only argument is positional,
/// following Python's automatic or manual numbering.
fn positional_arg(name: &str, numbering: &mut Numbering) -> Result<Arg, FormatError> {
    let mixed =
        |from: &str, to: &str| FormatError::Value(format!("cannot switch from {from} to {to}"));
    let index = match (name, *numbering) {
        ("", Numbering::Manual) => {
            return Err(mixed(
                "manual field specification",
                "automatic field numbering",
            ));
        }
        ("", Numbering::Unknown) => {
            *numbering = Numbering::Automatic(1);
            0
        }
        ("", Numbering::Automatic(next)) => {
            *numbering = Numbering::Automatic(next + 1);
            next
        }
        (n, Numbering::Automatic(_)) if n.bytes().all(|b| b.is_ascii_digit()) => {
            return Err(mixed(
                "automatic field numbering",
                "manual field specification",
            ));
        }
        (n, _) if n.bytes().all(|b| b.is_ascii_digit()) => {
            *numbering = Numbering::Manual;
            n.parse().unwrap_or(usize::MAX)
        }
        (n, _) if n.starts_with(|c: char| c.is_ascii_digit()) => {
            return Err(FormatError::Unsupported(format!(
                "attribute or index access in field {n:?}"
            )));
        }
        (n, _) => return Err(FormatError::Key(n.to_owned())),
    };
    if index == 0 {
        Ok(Arg::X)
    } else {
        Err(out_of_range(&index.to_string()))
    }
}

/// Python's `IndexError` for a positional field beyond the arguments.
fn out_of_range(index: &str) -> FormatError {
    FormatError::Index(format!(
        "Replacement index {index} out of range for positional args tuple"
    ))
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
        let out = t
            .render(-2.0, None, |s| s.replace('-', "\u{2212}"))
            .unwrap();
        assert_eq!(out, "a-\u{2212}2.0");
    }

    #[test]
    fn rejects_invalid_templates() {
        assert!(matches!(
            StrTemplate::parse("{y}"),
            Err(FormatError::Key(_))
        ));
        assert!(matches!(
            StrTemplate::parse("{}"),
            Err(FormatError::Index(_))
        ));
        assert!(matches!(
            StrTemplate::parse("{x:#x}"),
            Err(FormatError::Value(_))
        ));
        assert!(matches!(
            StrTemplate::parse("x}"),
            Err(FormatError::Value(_))
        ));
        assert!(matches!(
            StrTemplate::parse("{x"),
            Err(FormatError::Value(_))
        ));
        assert!(matches!(
            StrTemplate::parse("{x!q}"),
            Err(FormatError::Value(_))
        ));
        assert!(
            StrTemplate::parse("{pos:.2f}")
                .unwrap()
                .render(0.0, None, |s| s)
                .is_err()
        );
    }

    #[test]
    fn positional_templates_take_one_value() {
        let render = |template: &str, x: f64| {
            StrTemplate::parse_positional(template).and_then(|t| t.render(x, None, |s| s))
        };
        assert_eq!(render("{:.0%}", 0.428), Ok("43%".to_owned()));
        assert_eq!(render("{} m", 0.2), Ok("0.2 m".to_owned()));
        assert_eq!(render("{0}/{0:+.1f}", 2.0), Ok("2.0/+2.0".to_owned()));
        assert_eq!(render("const", 1.0), Ok("const".to_owned()));
        assert!(matches!(render("{} {}", 1.0), Err(FormatError::Index(_))));
        assert!(matches!(render("{1}", 1.0), Err(FormatError::Index(_))));
        assert!(matches!(render("{}{0}", 1.0), Err(FormatError::Value(_))));
        assert!(matches!(render("{0}{}", 1.0), Err(FormatError::Value(_))));
        assert!(matches!(render("{x}", 1.0), Err(FormatError::Key(_))));
    }
}
