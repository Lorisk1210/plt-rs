//! `str.format` templates with the fields `x` (a float) and `pos` (an
//! optional integer), as used by matplotlib's `StrMethodFormatter`, with
//! one positional float, or with fields of other names.

use super::FormatError;
use super::dtoa::{FloatStyle, format_float};
use super::spec::FormatSpec;

/// A value a replacement field formats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TemplateValue {
    /// A Python `float`.
    Float(f64),
    /// A Python `int`.
    Int(i64),
    /// Python's `None`.
    None,
}

/// The type a field's argument is known to have when the template is
/// parsed, for early checks of its spec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Float,
    Int,
}

/// How replacement fields name their argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Naming<'a> {
    /// The keywords `x` (a float) and `pos` (an integer).
    Keywords,
    /// One positional argument of this type (`{}`, `{0}`).
    Positional(Numbering, Kind),
    /// These keywords, the arguments in order.
    Named(&'a [&'a str]),
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
    /// The index of the argument formatted.
    arg: usize,
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
/// use plt_core::pyfmt::StrTemplate;
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
    /// use plt_core::pyfmt::StrTemplate;
    /// let t = StrTemplate::parse_positional("{0:.0%} of {0}").unwrap();
    /// assert_eq!(t.render(0.25, None, |s| s).unwrap(), "25% of 0.25");
    /// assert!(StrTemplate::parse_positional("{} {}").is_err());
    /// ```
    pub fn parse_positional(template: &str) -> Result<Self, FormatError> {
        Self::parse_with(
            template,
            Naming::Positional(Numbering::Unknown, Kind::Float),
        )
    }

    /// Like [`StrTemplate::parse_positional`] for an integer, rendered with
    /// [`StrTemplate::render_values`] (`template.format(3)`).
    ///
    /// ```
    /// use plt_core::pyfmt::{StrTemplate, TemplateValue};
    /// let t = StrTemplate::parse_positional_int("{:d} items").unwrap();
    /// assert_eq!(t.render_values(&[TemplateValue::Int(5)], |s| s).unwrap(), "5 items");
    /// assert!(StrTemplate::parse_positional("{:d}").is_err());
    /// ```
    pub fn parse_positional_int(template: &str) -> Result<Self, FormatError> {
        Self::parse_with(template, Naming::Positional(Numbering::Unknown, Kind::Int))
    }

    /// Parses and validates a template whose fields name the keywords
    /// `names`, rendered with [`StrTemplate::render_values`] in that order.
    ///
    /// ```
    /// use plt_core::pyfmt::{StrTemplate, TemplateValue};
    /// let t = StrTemplate::parse_named("{absval:d} ({frac:.0%})", &["absval", "frac"]).unwrap();
    /// let values = [TemplateValue::Int(3), TemplateValue::Float(0.25)];
    /// assert_eq!(t.render_values(&values, |s| s).unwrap(), "3 (25%)");
    /// ```
    pub fn parse_named(template: &str, names: &[&str]) -> Result<Self, FormatError> {
        Self::parse_with(template, Naming::Named(names))
    }

    fn parse_with(template: &str, mut naming: Naming<'_>) -> Result<Self, FormatError> {
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
        post: impl FnMut(String) -> String,
    ) -> Result<String, FormatError> {
        let pos =
            match pos {
                Some(pos) => TemplateValue::Int(i64::try_from(pos).map_err(|_| {
                    FormatError::Value("tick position does not fit an integer".into())
                })?),
                None => TemplateValue::None,
            };
        self.render_values(&[TemplateValue::Float(x), pos], post)
    }

    /// Renders the template with `values`, the arguments its fields refer
    /// to in order; `post` is applied to each formatted field.
    ///
    /// Fails when a field's spec does not suit its value (Python's
    /// `ValueError` or `TypeError`) or names a missing argument.
    pub fn render_values(
        &self,
        values: &[TemplateValue],
        mut post: impl FnMut(String) -> String,
    ) -> Result<String, FormatError> {
        let mut out = String::new();
        for part in &self.parts {
            match part {
                Part::Literal(text) => out.push_str(text),
                Part::Field(field) => out.push_str(&post(field.render(values)?)),
            }
        }
        Ok(out)
    }
}

impl Field {
    fn render(&self, values: &[TemplateValue]) -> Result<String, FormatError> {
        let value = values
            .get(self.arg)
            .copied()
            .ok_or_else(|| out_of_range(&self.arg.to_string()))?;
        match value {
            TemplateValue::Float(x) if self.as_text => {
                self.spec.format_str(&format_float(x, FloatStyle::REPR))
            }
            TemplateValue::Float(x) => {
                self.spec.check_float()?;
                self.spec.format_float(x)
            }
            TemplateValue::Int(i) if self.as_text => self.spec.format_str(&i.to_string()),
            TemplateValue::Int(i) => {
                self.spec.check_int()?;
                self.spec.format_int(i)
            }
            TemplateValue::None if self.as_text || self.spec_is_empty => {
                self.spec.format_str("None")
            }
            TemplateValue::None => Err(FormatError::Type(
                "unsupported format string passed to NoneType.__format__".into(),
            )),
        }
    }
}

/// Parses `name[!conv][:spec]` and validates it against the argument type
/// when that is known.
fn parse_field(body: &str, naming: &mut Naming<'_>) -> Result<Field, FormatError> {
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
    let (arg, kind) = match naming {
        Naming::Keywords => keyword_arg(name)?,
        Naming::Positional(numbering, kind) => (positional_arg(name, numbering)?, Some(*kind)),
        Naming::Named(names) => (named_arg(name, names)?, None),
    };
    if spec_text.contains('{') {
        return Err(FormatError::Unsupported(
            "nested replacement fields in a format spec".into(),
        ));
    }
    let spec = FormatSpec::parse(spec_text)?;
    match (kind, as_text) {
        (_, true) => spec.check_str()?,
        (Some(Kind::Float), false) => spec.check_float()?,
        (Some(Kind::Int), false) => spec.check_int()?,
        (None, false) => {}
    }
    Ok(Field {
        arg,
        as_text,
        spec,
        spec_is_empty: spec_text.is_empty(),
    })
}

/// The argument a field names under the `x`/`pos` convention, and its
/// type.
fn keyword_arg(name: &str) -> Result<(usize, Option<Kind>), FormatError> {
    match name {
        "x" => Ok((0, Some(Kind::Float))),
        "pos" => Ok((1, Some(Kind::Int))),
        "" => Err(out_of_range("0")),
        n if n.bytes().all(|b| b.is_ascii_digit()) => Err(out_of_range(n)),
        n if n.starts_with("x.") || n.starts_with("x[") || n.starts_with("pos") => Err(
            FormatError::Unsupported(format!("attribute or index access in field {n:?}")),
        ),
        n => Err(FormatError::Key(n.to_owned())),
    }
}

/// The argument a field names among the keywords `names`.
fn named_arg(name: &str, names: &[&str]) -> Result<usize, FormatError> {
    if let Some(index) = names.iter().position(|&n| n == name) {
        return Ok(index);
    }
    match name {
        "" => Err(out_of_range("0")),
        n if n.bytes().all(|b| b.is_ascii_digit()) => Err(out_of_range(n)),
        n if n.contains(['.', '[']) => Err(FormatError::Unsupported(format!(
            "attribute or index access in field {n:?}"
        ))),
        n => Err(FormatError::Key(n.to_owned())),
    }
}

/// The argument a field names when the only argument is positional,
/// following Python's automatic or manual numbering.
fn positional_arg(name: &str, numbering: &mut Numbering) -> Result<usize, FormatError> {
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
        Ok(0)
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
    fn named_templates_check_types_when_rendering() {
        let t = StrTemplate::parse_named("{a:d}/{b:.1f}", &["a", "b"]).unwrap();
        let ok = [TemplateValue::Int(2), TemplateValue::Float(0.25)];
        assert_eq!(t.render_values(&ok, |s| s), Ok("2/0.2".to_owned()));
        let float_for_d = [TemplateValue::Float(2.0), TemplateValue::Float(0.25)];
        assert!(matches!(
            t.render_values(&float_for_d, |s| s),
            Err(FormatError::Value(_))
        ));
        assert!(matches!(
            StrTemplate::parse_named("{c}", &["a"]),
            Err(FormatError::Key(_))
        ));
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
