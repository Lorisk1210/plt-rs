//! Python's format-specification mini-language (`format(value, spec)`).

use super::dtoa::{FloatStyle, Presentation, format_float};
use super::{FormatError, bounded_field};

/// Field alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Align {
    /// `<`
    Left,
    /// `>`
    Right,
    /// `^`
    Center,
    /// `=`: padding goes between the sign and the digits.
    AfterSign,
}

impl Align {
    fn from_char(c: char) -> Option<Self> {
        match c {
            '<' => Some(Self::Left),
            '>' => Some(Self::Right),
            '^' => Some(Self::Center),
            '=' => Some(Self::AfterSign),
            _ => None,
        }
    }
}

/// Sign option.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SignMode {
    /// `-`: only negative numbers get a sign.
    #[default]
    Negative,
    /// `+`: always print a sign.
    Always,
    /// ` `: a space in place of a plus sign.
    Space,
}

/// A parsed format specification such as `>8.3f`, `+.1e` or `,.0f`.
///
/// ```
/// use pltrs_core::pyfmt::FormatSpec;
/// let spec = FormatSpec::parse(",.0f").unwrap();
/// assert_eq!(spec.format_float(-1500.25).unwrap(), "-1,500");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatSpec {
    fill: Option<char>,
    align: Option<Align>,
    sign: SignMode,
    no_negative_zero: bool,
    alternate: bool,
    zero_pad: bool,
    width: Option<usize>,
    grouping: Option<char>,
    precision: Option<usize>,
    ty: Option<char>,
}

impl FormatSpec {
    /// Parses a format specification (the part after `:` in a
    /// replacement field).
    pub fn parse(spec: &str) -> Result<Self, FormatError> {
        let chars: Vec<char> = spec.chars().collect();
        let mut pos = 0;
        let mut fill = None;
        let mut align = None;
        if let Some(a) = chars.get(1).copied().and_then(Align::from_char) {
            fill = Some(chars[0]);
            align = Some(a);
            pos = 2;
        } else if let Some(a) = chars.first().copied().and_then(Align::from_char) {
            align = Some(a);
            pos = 1;
        }
        let mut take = |wanted: char| {
            let found = chars.get(pos) == Some(&wanted);
            if found {
                pos += 1;
            }
            found
        };
        let sign = if take('+') {
            SignMode::Always
        } else if take(' ') {
            SignMode::Space
        } else {
            take('-');
            SignMode::Negative
        };
        let no_negative_zero = take('z');
        let alternate = take('#');
        let zero_pad = fill.is_none() && take('0');
        let width = integer(&chars, &mut pos);
        let mut grouping = None;
        for sep in [',', '_'] {
            if chars.get(pos) == Some(&sep) {
                if grouping.is_some() {
                    return Err(FormatError::Value(
                        "Cannot specify both ',' and '_'.".into(),
                    ));
                }
                grouping = Some(sep);
                pos += 1;
            }
        }
        if grouping == Some('_') && chars.get(pos) == Some(&',') {
            return Err(FormatError::Value(
                "Cannot specify both ',' and '_'.".into(),
            ));
        }
        let precision =
            if chars.get(pos) == Some(&'.') {
                pos += 1;
                Some(integer(&chars, &mut pos).ok_or_else(|| {
                    FormatError::Value("Format specifier missing precision".into())
                })?)
            } else {
                None
            };
        let ty = match &chars[pos..] {
            [] => None,
            [c] => Some(*c),
            _ => {
                return Err(FormatError::Value(format!(
                    "Invalid format specifier '{spec}'"
                )));
            }
        };
        if width.max(precision) > Some(isize::MAX as usize) {
            return Err(FormatError::Value(
                "Too many decimal digits in format string".into(),
            ));
        }
        let width = bounded_field(width, "width")?;
        let precision = bounded_field(precision, "precision")?;
        if let Some(sep) = grouping {
            let allowed = match ty {
                None | Some('d' | 'e' | 'f' | 'g' | 'E' | 'G' | '%' | 'F') => true,
                Some('b' | 'o' | 'x' | 'X') => sep == '_',
                Some(_) => false,
            };
            if !allowed {
                let ty = ty.map_or_else(String::new, String::from);
                return Err(FormatError::Value(format!(
                    "Cannot specify '{sep}' with '{ty}'."
                )));
            }
        }
        Ok(Self {
            fill,
            align,
            sign,
            no_negative_zero,
            alternate,
            zero_pad,
            width,
            grouping,
            precision,
            ty,
        })
    }

    /// Checks that the spec can format floats (`{x:#x}` cannot).
    pub fn check_float(&self) -> Result<(), FormatError> {
        self.float_style().map(|_| ())
    }

    /// Checks that the spec can format integers.
    pub fn check_int(&self) -> Result<(), FormatError> {
        match self.ty {
            Some('e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%') => self.check_float(),
            None | Some('d' | 'n' | 'b' | 'o' | 'x' | 'X') => self.int_checks(),
            Some(c) => Err(unknown_code(c, "int")),
        }
    }

    /// Checks that the spec can format strings.
    pub fn check_str(&self) -> Result<(), FormatError> {
        if self.sign != SignMode::Negative {
            return Err(FormatError::Value(
                "Sign not allowed in string format specifier".into(),
            ));
        }
        if self.alternate {
            return Err(FormatError::Value(
                "Alternate form (#) not allowed in string format specifier".into(),
            ));
        }
        if self.align == Some(Align::AfterSign) {
            return Err(FormatError::Value(
                "'=' alignment not allowed in string format specifier".into(),
            ));
        }
        if self.grouping.is_some() {
            return Err(FormatError::Value("Cannot specify ',' with 's'.".into()));
        }
        match self.ty {
            None | Some('s') => Ok(()),
            Some(c) => Err(unknown_code(c, "str")),
        }
    }

    fn int_checks(&self) -> Result<(), FormatError> {
        if self.precision.is_some() {
            return Err(FormatError::Value(
                "Precision not allowed in integer format specifier".into(),
            ));
        }
        if self.no_negative_zero {
            return Err(FormatError::Value(
                "Negative zero coercion (z) not allowed in integer format specifier".into(),
            ));
        }
        Ok(())
    }

    /// The float style and whether the result is a percentage.
    fn float_style(&self) -> Result<(FloatStyle, bool), FormatError> {
        let (presentation, upper, percent) = match self.ty {
            None => (Presentation::Repr, false, false),
            Some('e') => (Presentation::Exponent, false, false),
            Some('E') => (Presentation::Exponent, true, false),
            Some('f') => (Presentation::Fixed, false, false),
            Some('F') => (Presentation::Fixed, true, false),
            Some('g' | 'n') => (Presentation::General, false, false),
            Some('G') => (Presentation::General, true, false),
            Some('%') => (Presentation::Fixed, false, true),
            Some(c) => return Err(unknown_code(c, "float")),
        };
        // An omitted type behaves like `repr` without a precision and like
        // `g` with at least one decimal digit otherwise.
        let (presentation, precision) = match (presentation, self.precision) {
            (Presentation::Repr, None) => (Presentation::Repr, 0),
            (Presentation::Repr, Some(p)) => (Presentation::General, p),
            (p, precision) => (p, precision.unwrap_or(6)),
        };
        Ok((
            FloatStyle {
                upper,
                add_dot_0: self.ty.is_none(),
                alternate: self.alternate,
                no_negative_zero: self.no_negative_zero,
                ..FloatStyle::new(presentation, precision)
            },
            percent,
        ))
    }

    /// `format(value, spec)` for a float.
    pub fn format_float(&self, value: f64) -> Result<String, FormatError> {
        let (style, percent) = self.float_style()?;
        let mut body = format_float(if percent { value * 100.0 } else { value }, style);
        if percent {
            body.push('%');
        }
        let (negative, unsigned) = match body.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, body.as_str()),
        };
        let int_len = unsigned.bytes().take_while(u8::is_ascii_digit).count();
        let (digits, remainder) = unsigned.split_at(int_len);
        let grouping = if self.ty == Some('n') {
            None
        } else {
            self.grouping
        };
        Ok(self.layout_number(negative, "", digits, remainder, grouping, 3))
    }

    /// `format(value, spec)` for an integer.
    pub fn format_int(&self, value: i64) -> Result<String, FormatError> {
        self.check_int()?;
        let (radix_digits, prefix, group_len) = match self.ty {
            Some('e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%') => {
                return self.format_float(value as f64);
            }
            Some('b') => (format!("{:b}", value.unsigned_abs()), "0b", 4),
            Some('o') => (format!("{:o}", value.unsigned_abs()), "0o", 4),
            Some('x') => (format!("{:x}", value.unsigned_abs()), "0x", 4),
            Some('X') => (format!("{:X}", value.unsigned_abs()), "0X", 4),
            _ => (value.unsigned_abs().to_string(), "", 3),
        };
        let prefix = if self.alternate { prefix } else { "" };
        let grouping = if self.ty == Some('n') {
            None
        } else {
            self.grouping
        };
        Ok(self.layout_number(value < 0, prefix, &radix_digits, "", grouping, group_len))
    }

    /// `format(value, spec)` for a string.
    pub fn format_str(&self, value: &str) -> Result<String, FormatError> {
        self.check_str()?;
        let text: String = match self.precision {
            Some(p) => value.chars().take(p).collect(),
            None => value.to_owned(),
        };
        let fill = self.fill.unwrap_or(if self.zero_pad { '0' } else { ' ' });
        let padding = self.width.unwrap_or(0).saturating_sub(text.chars().count());
        let (left, right) = split_padding(self.align.unwrap_or(Align::Left), padding);
        Ok(pad_around(&text, fill, left, right))
    }

    /// `CPython`'s `calc_number_widths` + `fill_number`.
    fn layout_number(
        &self,
        negative: bool,
        prefix: &str,
        digits: &str,
        remainder: &str,
        grouping: Option<char>,
        group_len: usize,
    ) -> String {
        let (fill, align) = match (self.fill, self.zero_pad) {
            (Some(fill), _) => (fill, self.align.unwrap_or(Align::Right)),
            (None, true) => ('0', self.align.unwrap_or(Align::AfterSign)),
            (None, false) => (' ', self.align.unwrap_or(Align::Right)),
        };
        let sign = match (negative, self.sign) {
            (true, _) => Some('-'),
            (false, SignMode::Always) => Some('+'),
            (false, SignMode::Space) => Some(' '),
            (false, SignMode::Negative) => None,
        };
        let non_digits = usize::from(sign.is_some()) + prefix.len() + remainder.chars().count();
        let width = self.width.unwrap_or(0);
        let min_width = if fill == '0' && align == Align::AfterSign {
            width.saturating_sub(non_digits)
        } else {
            0
        };
        let grouped = if digits.is_empty() {
            String::new()
        } else {
            group_digits(digits, min_width, grouping, group_len)
        };
        let padding = width.saturating_sub(non_digits + grouped.chars().count());
        let mut out = String::new();
        let (left, right) = split_padding(align, padding);
        let middle = if align == Align::AfterSign {
            padding
        } else {
            0
        };
        out.extend(std::iter::repeat_n(fill, left));
        out.extend(sign);
        out.push_str(prefix);
        out.extend(std::iter::repeat_n(fill, middle));
        out.push_str(&grouped);
        out.push_str(remainder);
        out.extend(std::iter::repeat_n(fill, right));
        out
    }
}

fn unknown_code(code: char, type_name: &str) -> FormatError {
    FormatError::Value(format!(
        "Unknown format code '{code}' for object of type '{type_name}'"
    ))
}

/// Parses a decimal integer at `pos`, if any.
fn integer(chars: &[char], pos: &mut usize) -> Option<usize> {
    let start = *pos;
    let mut value: usize = 0;
    while let Some(digit) = chars.get(*pos).and_then(|c| c.to_digit(10)) {
        value = value.saturating_mul(10).saturating_add(digit as usize);
        *pos += 1;
    }
    (*pos > start).then_some(value)
}

/// Left and right padding for an alignment (`=` pads in the middle).
fn split_padding(align: Align, padding: usize) -> (usize, usize) {
    match align {
        Align::Left => (0, padding),
        Align::Right => (padding, 0),
        Align::Center => (padding / 2, padding - padding / 2),
        Align::AfterSign => (0, 0),
    }
}

fn pad_around(text: &str, fill: char, left: usize, right: usize) -> String {
    let mut out: String = std::iter::repeat_n(fill, left).collect();
    out.push_str(text);
    out.extend(std::iter::repeat_n(fill, right));
    out
}

/// `CPython`'s `_PyUnicode_InsertThousandsGrouping`: separators every
/// `group_len` digits, zero-padded to at least `min_width` characters.
fn group_digits(digits: &str, min_width: usize, sep: Option<char>, group_len: usize) -> String {
    let digits: Vec<char> = digits.chars().collect();
    let mut remaining = digits.len();
    let mut min_width = min_width;
    let Some(sep) = sep else {
        let zeros = min_width.max(remaining).max(1) - remaining;
        return std::iter::repeat_n('0', zeros).chain(digits).collect();
    };
    // Chunks are produced right to left.
    let mut chunks: Vec<String> = Vec::new();
    let mut end = digits.len();
    loop {
        let len = group_len.min(remaining.max(min_width).max(1));
        let n_chars = remaining.min(len);
        let n_zeros = len - n_chars;
        let mut chunk: String = std::iter::repeat_n('0', n_zeros).collect();
        chunk.extend(&digits[end - n_chars..end]);
        if !chunks.is_empty() {
            chunk.push(sep);
        }
        chunks.push(chunk);
        end -= n_chars;
        remaining -= n_chars;
        min_width = min_width.saturating_sub(len);
        if remaining == 0 && min_width == 0 {
            break;
        }
        min_width = min_width.saturating_sub(1);
    }
    chunks.iter().rev().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn float(spec: &str, value: f64) -> String {
        FormatSpec::parse(spec)
            .unwrap()
            .format_float(value)
            .unwrap()
    }

    #[test]
    fn float_specs_match_python() {
        assert_eq!(float("", 1500.25), "1500.25");
        assert_eq!(float("", 1.0), "1.0");
        assert_eq!(float(".2f", -1.0), "-1.00");
        assert_eq!(float(",.0f", 10_000_000.0), "10,000,000");
        assert_eq!(float(",.0f", -1500.25), "-1,500");
        assert_eq!(float("+.1e", 0.0), "+0.0e+00");
        assert_eq!(float("g", 1e7), "1e+07");
        assert_eq!(float(".0%", -1e-5), "-0%");
        assert_eq!(float(".0%", 0.5), "50%");
        assert_eq!(float(">8.3f", -1.0), "  -1.000");
        assert_eq!(float(">8.3f", 1e7), "10000000.000");
        assert_eq!(float(".3g", 1234.5), "1.23e+03");
        assert_eq!(float(".3", 1.0), "1.0");
        assert_eq!(float(".3", 1234.5), "1.23e+03");
        assert_eq!(float("1.0e", 12345.0), "1e+04");
        assert_eq!(float("0.2f", 1.234_56), "1.23");
        assert_eq!(float("08.2f", -3.5), "-0003.50");
        assert_eq!(float("010,.1f", 1234.5), "0,001,234.5");
        assert_eq!(float("*^9.1f", 2.0), "***2.0***");
        assert_eq!(float("_", 1e7), "10_000_000.0");
        assert_eq!(float("z.1f", -0.01), "0.0");
        assert_eq!(float("", f64::NAN), "nan");
    }

    #[test]
    fn huge_widths_and_precisions_are_refused() {
        assert_eq!(
            FormatSpec::parse(">99999999999999999999"),
            Err(FormatError::Value(
                "Too many decimal digits in format string".into()
            ))
        );
        // Found by fuzzing: the width used to be allocated in full.
        for spec in [">429496729616", ".3000000000f", "2000000"] {
            assert!(matches!(
                FormatSpec::parse(spec),
                Err(FormatError::Unsupported(_))
            ));
        }
        assert_eq!(float(">1048576", 1.0).len(), 1 << 20);
    }

    #[test]
    fn int_and_str_specs() {
        let spec = |s: &str| FormatSpec::parse(s).unwrap();
        assert_eq!(spec("").format_int(-42).unwrap(), "-42");
        assert_eq!(spec(">4").format_int(7).unwrap(), "   7");
        assert_eq!(spec("#x").format_int(255).unwrap(), "0xff");
        assert_eq!(spec(",").format_int(1_234_567).unwrap(), "1,234,567");
        assert_eq!(spec("08,d").format_int(1234).unwrap(), "0,001,234");
        assert_eq!(spec(".1f").format_int(3).unwrap(), "3.0");
        assert!(spec(".2").format_int(3).is_err());
        assert_eq!(spec(">6").format_str("None").unwrap(), "  None");
        assert_eq!(spec(".2").format_str("None").unwrap(), "No");
        assert!(spec("+").format_str("x").is_err());
    }

    #[test]
    fn invalid_specs() {
        assert!(FormatSpec::parse(".f").is_err());
        assert!(FormatSpec::parse("abc").is_err());
        assert!(FormatSpec::parse(",_d").is_err());
        assert!(FormatSpec::parse(",s").is_err());
        assert!(FormatSpec::parse("#x").unwrap().check_float().is_err());
        assert!(FormatSpec::parse("c").unwrap().check_float().is_err());
    }
}
