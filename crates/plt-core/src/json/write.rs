//! The writers: [`Json`]'s `Display` in Python's style, and literals for
//! building strict JSON text piece by piece.

use std::fmt::{self, Write as _};

use super::Json;

impl fmt::Display for Json {
    /// Serialises on one line with Python's `json.dumps` separators (`", "`
    /// and `": "`) and its `NaN`, `Infinity` and `-Infinity` tokens;
    /// [`Json::Number`]s are written in Rust's shortest round-trip `Debug`
    /// form, which always shows a fraction or exponent (`1.0`, `1e20`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) if n.is_nan() => f.write_str("NaN"),
            Self::Number(n) if n.is_infinite() => {
                f.write_str(if *n > 0.0 { "Infinity" } else { "-Infinity" })
            }
            Self::Number(n) => write!(f, "{n:?}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::String(s) => write_escaped(f, s),
            Self::Array(items) => {
                f.write_char('[')?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_char(']')
            }
            Self::Object(entries) => {
                f.write_char('{')?;
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write_escaped(f, key)?;
                    write!(f, ": {value}")?;
                }
                f.write_char('}')
            }
        }
    }
}

/// Writes `s` as a JSON string literal: quotes, backslashes and control
/// characters escaped, everything else verbatim.
fn write_escaped(out: &mut impl fmt::Write, s: &str) -> fmt::Result {
    out.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => out.write_str("\\\"")?,
            '\\' => out.write_str("\\\\")?,
            '\n' => out.write_str("\\n")?,
            '\r' => out.write_str("\\r")?,
            '\t' => out.write_str("\\t")?,
            c if u32::from(c) < 0x20 => write!(out, "\\u{:04x}", u32::from(c))?,
            c => out.write_char(c)?,
        }
    }
    out.write_char('"')
}

/// `text` as a JSON string literal, escaped as [`Json`]'s `Display`
/// escapes strings.
pub fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    write_escaped(&mut out, text).expect("writing to a String cannot fail");
    out
}

/// `value` as a strict JSON number: its shortest round-trip decimal
/// without a forced fraction (`640`, `-0.5`), or `null` when it is not
/// finite, which RFC 8259 cannot express.
pub fn number(value: f64) -> String {
    if value.is_finite() {
        format!("{value}")
    } else {
        "null".to_owned()
    }
}
