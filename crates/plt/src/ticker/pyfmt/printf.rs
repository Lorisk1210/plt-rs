//! printf-style formatting: Python's `fmt % value` with one number.

use super::FormatError;
use super::dtoa::{FloatStyle, Presentation, format_float};

/// The `%` conversion flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Flags {
    /// `-`: left-justify within the width.
    left: bool,
    /// `+`: always print a sign.
    sign: bool,
    /// ` `: a space in place of a plus sign.
    space: bool,
    /// `#`: alternate form.
    alternate: bool,
    /// `0`: pad numbers with zeros.
    zero: bool,
}

/// What a conversion does with its (floating-point) argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// `d`, `i`, `u`: truncate to an integer.
    Integer,
    /// `e`, `f`, `g` and their upper-case forms.
    Float(Presentation, bool),
    /// `s`, `r`, `a`: the value's `repr`.
    Text,
}

/// One `%` conversion specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Conversion {
    flags: Flags,
    width: Option<usize>,
    precision: Option<usize>,
    kind: Kind,
}

/// A parsed printf-style template with exactly one conversion, applied to
/// a float like Python's `template % value`.
///
/// ```
/// use plt::ticker::pyfmt::PrintfFormat;
/// let fmt = PrintfFormat::parse("%5.1f%%").unwrap();
/// assert_eq!(fmt.format(-1.0).unwrap(), " -1.0%");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintfFormat {
    template: String,
    prefix: String,
    conversion: Conversion,
    suffix: String,
}

impl PrintfFormat {
    /// Parses `template`, which must contain exactly one conversion that
    /// accepts a float (`%%` escapes do not count).
    pub fn parse(template: &str) -> Result<Self, FormatError> {
        let mut parser = Parser {
            chars: template.char_indices().peekable(),
        };
        let mut pieces = vec![String::new()];
        let mut conversions = Vec::new();
        while let Some((index, c)) = parser.chars.next() {
            if c != '%' {
                pieces.last_mut().expect("pieces is never empty").push(c);
                continue;
            }
            if parser.chars.next_if(|&(_, c)| c == '%').is_some() {
                pieces.last_mut().expect("pieces is never empty").push('%');
                continue;
            }
            conversions.push(parser.conversion(index)?);
            pieces.push(String::new());
        }
        match (conversions.as_slice(), pieces.as_mut_slice()) {
            ([conversion], [prefix, suffix]) => Ok(Self {
                template: template.to_owned(),
                prefix: std::mem::take(prefix),
                conversion: *conversion,
                suffix: std::mem::take(suffix),
            }),
            ([], _) => Err(FormatError::Type(
                "not all arguments converted during string formatting".into(),
            )),
            _ => Err(FormatError::Type(
                "not enough arguments for format string".into(),
            )),
        }
    }

    /// The template this format was parsed from.
    pub fn template(&self) -> &str {
        &self.template
    }

    /// Formats `value`. Integer conversions of NaN or infinity fail, as
    /// they do in Python.
    pub fn format(&self, value: f64) -> Result<String, FormatError> {
        let conv = &self.conversion;
        let body = match conv.kind {
            Kind::Integer => integer_body(value, conv.precision)?,
            Kind::Float(presentation, upper) => format_float(
                value,
                FloatStyle {
                    upper,
                    alternate: conv.flags.alternate,
                    ..FloatStyle::new(presentation, conv.precision.unwrap_or(6))
                },
            ),
            Kind::Text => {
                let text = format_float(value, FloatStyle::REPR);
                match conv.precision {
                    Some(p) => text.chars().take(p).collect(),
                    None => text,
                }
            }
        };
        let numeric = conv.kind != Kind::Text;
        Ok(format!(
            "{}{}{}",
            self.prefix,
            pad(&body, conv, numeric),
            self.suffix
        ))
    }
}

/// Python's `'%d' % value` digits: `int(value)` with `precision` as the
/// minimum digit count.
fn integer_body(value: f64, precision: Option<usize>) -> Result<String, FormatError> {
    if value.is_nan() {
        return Err(FormatError::Value(
            "cannot convert float NaN to integer".into(),
        ));
    }
    if value.is_infinite() {
        return Err(FormatError::Value(
            "cannot convert float infinity to integer".into(),
        ));
    }
    let truncated = value.trunc();
    let digits = format!("{:.0}", truncated.abs());
    let width = precision.unwrap_or(0);
    let sign = if truncated < 0.0 { "-" } else { "" };
    Ok(format!("{sign}{digits:0>width$}"))
}

/// CPython's `unicode_format_arg_output`: sign, width and fill.
fn pad(body: &str, conv: &Conversion, numeric: bool) -> String {
    let flags = conv.flags;
    let fill = if numeric && flags.zero { '0' } else { ' ' };
    let (mut sign, rest) = match body.strip_prefix(['-', '+']) {
        Some(rest) if numeric => (body.chars().next(), rest),
        _ => (None, body),
    };
    if numeric && sign.is_none() {
        if flags.sign {
            sign = Some('+');
        } else if flags.space {
            sign = Some(' ');
        }
    }
    let len = rest.chars().count();
    let mut width = conv.width.unwrap_or(0).max(len);
    let mut out = String::new();
    if let Some(s) = sign {
        if fill != ' ' {
            out.push(s);
        }
        if width > len {
            width -= 1;
        }
    }
    if width > len && !flags.left {
        out.extend(std::iter::repeat_n(fill, width - len));
        width = len;
    }
    if fill == ' ' {
        out.extend(sign);
    }
    out.push_str(rest);
    out.extend(std::iter::repeat_n(' ', width - len));
    out
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl Parser<'_> {
    fn digits(&mut self) -> Option<usize> {
        let mut value: Option<usize> = None;
        while let Some((_, c)) = self.chars.next_if(|(_, c)| c.is_ascii_digit()) {
            let digit = c.to_digit(10).expect("checked ASCII digit") as usize;
            value = Some(value.unwrap_or(0).saturating_mul(10).saturating_add(digit));
        }
        value
    }

    /// Parses the conversion that starts with the `%` at `start`.
    fn conversion(&mut self, start: usize) -> Result<Conversion, FormatError> {
        let mut flags = Flags::default();
        while let Some((_, c)) = self.chars.next_if(|(_, c)| "-+ #0".contains(*c)) {
            match c {
                '-' => flags.left = true,
                '+' => flags.sign = true,
                ' ' => flags.space = true,
                '#' => flags.alternate = true,
                _ => flags.zero = true,
            }
        }
        let unsupported = |what: &str| FormatError::Unsupported(format!("{what} in '%' format"));
        if self.chars.next_if(|&(_, c)| c == '(').is_some() {
            return Err(FormatError::Type("format requires a mapping".into()));
        }
        if self.chars.next_if(|&(_, c)| c == '*').is_some() {
            return Err(unsupported("'*' width"));
        }
        let width = self.digits();
        let precision = if self.chars.next_if(|&(_, c)| c == '.').is_some() {
            if self.chars.next_if(|&(_, c)| c == '*').is_some() {
                return Err(unsupported("'*' precision"));
            }
            Some(self.digits().unwrap_or(0))
        } else {
            None
        };
        while self.chars.next_if(|(_, c)| "hlL".contains(*c)).is_some() {}
        let Some((index, c)) = self.chars.next() else {
            return Err(FormatError::Value("incomplete format".into()));
        };
        let kind = match c {
            'd' | 'i' | 'u' => Kind::Integer,
            'e' | 'E' => Kind::Float(Presentation::Exponent, c == 'E'),
            'f' | 'F' => Kind::Float(Presentation::Fixed, c == 'F'),
            'g' | 'G' => Kind::Float(Presentation::General, c == 'G'),
            's' | 'r' | 'a' => Kind::Text,
            'o' | 'x' | 'X' | 'c' => {
                return Err(FormatError::Type(format!(
                    "%{c} format: an integer is required, not float"
                )));
            }
            other => {
                return Err(FormatError::Value(format!(
                    "unsupported format character '{other}' at index {index} (conversion at {start})"
                )));
            }
        };
        Ok(Conversion {
            flags,
            width,
            precision,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn printf(template: &str, value: f64) -> String {
        PrintfFormat::parse(template).unwrap().format(value).unwrap()
    }

    #[test]
    fn float_conversions() {
        assert_eq!(printf("%1.2f", -1500.25), "-1500.25");
        assert_eq!(printf("%.3e", -1e-5), "-1.000e-05");
        assert_eq!(printf("%g", 1e7), "1e+07");
        assert_eq!(printf("%+.1f", 0.0), "+0.0");
        assert_eq!(printf("%+.1f", -1e-5), "-0.0");
        assert_eq!(printf("%5.1f%%", 1.0), "  1.0%");
        assert_eq!(printf("%-#.4g", 2.0), "2.000");
        assert_eq!(printf("%-#.0g", 2.0), "2.");
        assert_eq!(printf("%08.2f", -3.5), "-0003.50");
        assert_eq!(printf("% f", 1.0), " 1.000000");
        assert_eq!(printf("%-8.2fx", 1.0), "1.00    x");
        assert_eq!(printf(r"$\mathdefault{%1.1f}$", 0.5), r"$\mathdefault{0.5}$");
    }

    #[test]
    fn integer_and_text_conversions() {
        assert_eq!(printf("%d", -1500.25), "-1500");
        assert_eq!(printf("%d", -1e-5), "0");
        assert_eq!(printf("%.3d", 5.0), "005");
        assert_eq!(printf("%05d", -42.0), "-0042");
        assert_eq!(printf("%s", 1e7), "10000000.0");
        assert_eq!(printf("%s", -1e-5), "-1e-05");
        assert_eq!(printf("%.3s", 1.2345), "1.2");
        assert_eq!(printf("%5s|", 1.0), "  1.0|");
        assert!(PrintfFormat::parse("%d").unwrap().format(f64::NAN).is_err());
    }

    #[test]
    fn invalid_templates() {
        assert!(matches!(
            PrintfFormat::parse("%x"),
            Err(FormatError::Type(_))
        ));
        assert!(matches!(
            PrintfFormat::parse("no conversion"),
            Err(FormatError::Type(_))
        ));
        assert!(matches!(
            PrintfFormat::parse("%d %d"),
            Err(FormatError::Type(_))
        ));
        assert!(matches!(PrintfFormat::parse("%"), Err(FormatError::Value(_))));
        assert!(matches!(
            PrintfFormat::parse("%y"),
            Err(FormatError::Value(_))
        ));
        assert_eq!(PrintfFormat::parse("100%%").map(|_| ()).ok(), None);
    }
}
