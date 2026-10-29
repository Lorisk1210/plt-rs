//! The values of Type 1 font dictionary entries, as matplotlib's
//! `Type1Font.prop` holds them, displayed the way Python's `str()` shows
//! them when matplotlib writes them back into a font.

use std::fmt::{self, Write};

use super::python::{parse_float, repr};

/// A number token's value: an integer when written without a decimal
/// point, else a real.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PsNumber {
    /// An integer.
    Int(i64),
    /// A real number.
    Real(f64),
}

impl PsNumber {
    /// The number as a float.
    pub fn to_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Real(value) => value,
        }
    }
}

impl fmt::Display for PsNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(value) => write!(f, "{value}"),
            Self::Real(value) => f.write_str(&repr(*value)),
        }
    }
}

/// The value of a font dictionary entry.
#[derive(Clone, Debug, PartialEq)]
pub enum PsValue {
    /// A number.
    Number(PsNumber),
    /// `true` or `false`.
    Bool(bool),
    /// A name (`/Name`), without its slash.
    Name(String),
    /// A `(...)` string with its escapes resolved.
    Text(String),
    /// A `<...>` hexadecimal string.
    Bytes(Vec<u8>),
    /// The source text of a balanced `[...]` or `{...}` expression.
    Expression(String),
}

impl PsValue {
    /// The text of a name, string or expression; `None` for other values.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Name(text) | Self::Text(text) | Self::Expression(text) => Some(text),
            Self::Number(_) | Self::Bool(_) | Self::Bytes(_) => None,
        }
    }

    /// Python's `float()` of the value, `None` where it raises.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(number.to_f64()),
            Self::Bool(value) => Some(f64::from(u8::from(*value))),
            Self::Name(text) | Self::Text(text) | Self::Expression(text) => parse_float(text),
            Self::Bytes(bytes) => std::str::from_utf8(bytes).ok().and_then(parse_float),
        }
    }

    /// Whether Python considers the value true (`bool(value)`).
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::Number(number) => number.to_f64() != 0.0,
            Self::Bool(value) => *value,
            Self::Name(text) | Self::Text(text) | Self::Expression(text) => !text.is_empty(),
            Self::Bytes(bytes) => !bytes.is_empty(),
        }
    }
}

impl From<&str> for PsValue {
    fn from(text: &str) -> Self {
        Self::Text(text.to_owned())
    }
}

/// Python's `repr` of a `bytes` object.
fn bytes_repr(bytes: &[u8]) -> String {
    let quote = if bytes.contains(&b'\'') && !bytes.contains(&b'"') {
        '"'
    } else {
        '\''
    };
    let mut out = format!("b{quote}");
    for &byte in bytes {
        match byte {
            b'\\' => out.push_str("\\\\"),
            b'\t' => out.push_str("\\t"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            _ if char::from(byte) == quote => {
                out.push('\\');
                out.push(quote);
            }
            0x20..=0x7E => out.push(char::from(byte)),
            _ => {
                write!(out, "\\x{byte:02x}").expect("writing to a String cannot fail");
            }
        }
    }
    out.push(quote);
    out
}

impl fmt::Display for PsValue {
    /// Python's `str()` of the value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(number) => write!(f, "{number}"),
            Self::Bool(true) => f.write_str("True"),
            Self::Bool(false) => f.write_str("False"),
            Self::Name(text) | Self::Text(text) | Self::Expression(text) => f.write_str(text),
            Self::Bytes(bytes) => f.write_str(&bytes_repr(bytes)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_str_of_values() {
        assert_eq!(PsValue::Number(PsNumber::Int(0)).to_string(), "0");
        assert_eq!(PsValue::Number(PsNumber::Real(-45.0)).to_string(), "-45.0");
        assert_eq!(PsValue::Bool(true).to_string(), "True");
        assert_eq!(PsValue::Name("CMR10".into()).to_string(), "CMR10");
        assert_eq!(
            PsValue::Bytes(vec![1, b'\'', b'a']).to_string(),
            "b\"\\x01'a\""
        );
        assert_eq!(PsValue::Bytes(b"\\\n".to_vec()).to_string(), "b'\\\\\\n'");
    }

    #[test]
    fn conversions() {
        assert_eq!(PsValue::from("12").to_f64(), Some(12.0));
        assert_eq!(PsValue::Bool(true).to_f64(), Some(1.0));
        assert_eq!(PsValue::Name("x".into()).to_f64(), None);
        assert_eq!(PsValue::Bytes(b"2.5".to_vec()).to_f64(), Some(2.5));
        assert!(!PsValue::Text(String::new()).is_truthy());
        assert!(PsValue::Number(PsNumber::Real(0.5)).is_truthy());
        assert_eq!(PsValue::Expression("[1]".into()).as_text(), Some("[1]"));
        assert_eq!(PsNumber::Int(3).to_f64(), 3.0);
    }
}
