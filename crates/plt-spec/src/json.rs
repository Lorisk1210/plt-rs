//! A small, strict JSON reader/writer for golden files.
//!
//! Goldens are produced by Python's `json` module with `allow_nan=True`, so
//! the reader additionally accepts the `NaN`, `Infinity` and `-Infinity`
//! tokens. Object key order is preserved.

use std::fmt::{self, Write as _};

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// `true` / `false`.
    Bool(bool),
    /// Any number (integers are represented exactly up to 2^53).
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object with keys in document order.
    Object(Vec<(String, Json)>),
}

impl Json {
    /// Parses a complete JSON document.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = parser.value()?;
        parser.skip_whitespace();
        if parser.pos != parser.bytes.len() {
            return Err(parser.error("trailing characters after document"));
        }
        Ok(value)
    }

    /// Looks up `key` in an object.
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The number, if this is one.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// The number as an integer, if it is integral and in range.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_f64()
            .filter(|n| n.fract() == 0.0 && n.abs() < 9.0e15)
            .map(|n| n as i64)
    }

    /// The string, if this is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The entries, if this is an object.
    pub fn as_object(&self) -> Option<&[(String, Self)]> {
        match self {
            Self::Object(entries) => Some(entries),
            _ => None,
        }
    }

    /// Returns `true` for `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// An array of numbers, if every element is a number.
    pub fn as_f64_vec(&self) -> Option<Vec<f64>> {
        self.as_array()?.iter().map(Self::as_f64).collect()
    }
}

impl From<f64> for Json {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Self::String(s.to_owned())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl<T: Into<Self>> From<Vec<T>> for Json {
    fn from(items: Vec<T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Self>> From<Option<T>> for Json {
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
    }
}

impl fmt::Display for Json {
    /// Compact serialisation using the same non-finite tokens as Python.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) if n.is_nan() => f.write_str("NaN"),
            Self::Number(n) if n.is_infinite() => {
                f.write_str(if *n > 0.0 { "Infinity" } else { "-Infinity" })
            }
            Self::Number(n) => write!(f, "{n:?}"),
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

fn write_escaped(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if u32::from(c) < 0x20 => write!(f, "\\u{:04x}", u32::from(c))?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

/// A syntax error with its byte offset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset where parsing failed.
    pub offset: usize,
    /// What was wrong.
    pub message: &'static str,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, message: &'static str) -> ParseError {
        ParseError {
            offset: self.pos,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }

    fn eat_literal(&mut self, literal: &str) -> bool {
        let matches = self.bytes[self.pos..].starts_with(literal.as_bytes());
        if matches {
            self.pos += literal.len();
        }
        matches
    }

    fn expect(&mut self, byte: u8, message: &'static str) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn value(&mut self) -> Result<Json, ParseError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(Json::String),
            Some(b'-' | b'0'..=b'9' | b'N' | b'I') => self.number(),
            _ if self.eat_literal("true") => Ok(Json::Bool(true)),
            _ if self.eat_literal("false") => Ok(Json::Bool(false)),
            _ if self.eat_literal("null") => Ok(Json::Null),
            _ => Err(self.error("expected a value")),
        }
    }

    fn object(&mut self) -> Result<Json, ParseError> {
        self.pos += 1;
        let mut entries = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(entries));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected an object key"));
            }
            let key = self.string()?;
            self.expect(b':', "expected ':' after object key")?;
            entries.push((key, self.value()?));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(entries));
                }
                _ => return Err(self.error("expected ',' or '}' in object")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, ParseError> {
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']' in array")),
            }
        }
    }

    fn number(&mut self) -> Result<Json, ParseError> {
        if self.eat_literal("NaN") {
            return Ok(Json::Number(f64::NAN));
        }
        if self.eat_literal("Infinity") {
            return Ok(Json::Number(f64::INFINITY));
        }
        if self.eat_literal("-Infinity") {
            return Ok(Json::Number(f64::NEG_INFINITY));
        }
        let start = self.pos;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.pos += 1;
        }
        std::str::from_utf8(&self.bytes[start..self.pos])
            .ok()
            .and_then(|s| s.parse().ok())
            .map(Json::Number)
            .ok_or_else(|| self.error("malformed number"))
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let start = self.pos;
            while !matches!(self.peek(), Some(b'"' | b'\\') | None) {
                self.pos += 1;
            }
            out.push_str(
                std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|_| self.error("invalid UTF-8 in string"))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    out.push(self.escape()?);
                }
                _ => return Err(self.error("unterminated string")),
            }
        }
    }

    fn escape(&mut self) -> Result<char, ParseError> {
        let byte = self
            .peek()
            .ok_or_else(|| self.error("unterminated escape"))?;
        self.pos += 1;
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode_escape(),
            _ => return Err(self.error("unknown escape sequence")),
        })
    }

    fn unicode_escape(&mut self) -> Result<char, ParseError> {
        let high = self.hex4()?;
        if !(0xD800..0xDC00).contains(&high) {
            return char::from_u32(high).ok_or_else(|| self.error("invalid code point"));
        }
        if !self.eat_literal("\\u") {
            return Err(self.error("unpaired surrogate"));
        }
        let low = self.hex4()?;
        if !(0xDC00..0xE000).contains(&low) {
            return Err(self.error("invalid low surrogate"));
        }
        char::from_u32(0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00))
            .ok_or_else(|| self.error("invalid code point"))
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let digits = self
            .bytes
            .get(self.pos..self.pos + 4)
            .and_then(|d| std::str::from_utf8(d).ok())
            .and_then(|d| u32::from_str_radix(d, 16).ok())
            .ok_or_else(|| self.error("expected four hex digits"))?;
        self.pos += 4;
        Ok(digits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_documents() {
        let doc = Json::parse(r#"{"a": [1, 2.5, -3e2], "b": {"c": null, "d": true}}"#).unwrap();
        assert_eq!(
            doc.get("a").unwrap().as_f64_vec().unwrap(),
            vec![1.0, 2.5, -300.0]
        );
        assert!(doc.get("b").unwrap().get("c").unwrap().is_null());
        assert_eq!(
            doc.get("b").unwrap().get("d").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn accepts_python_non_finite_tokens() {
        let doc = Json::parse("[NaN, Infinity, -Infinity]").unwrap();
        let values = doc.as_f64_vec().unwrap();
        assert!(values[0].is_nan());
        assert_eq!(values[1], f64::INFINITY);
        assert_eq!(values[2], f64::NEG_INFINITY);
    }

    #[test]
    fn decodes_escapes_and_surrogates() {
        let doc = Json::parse(r#""a\n−😀\"""#).unwrap();
        assert_eq!(doc.as_str(), Some("a\n\u{2212}😀\""));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in ["", "[1,", "{\"a\" 1}", "[1] x", "\"abc", "tru", "[1 2]"] {
            assert!(Json::parse(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn display_round_trips() {
        let text = r#"{"k": [1.5, "x\"y", null, false, NaN]}"#;
        let doc = Json::parse(text).unwrap();
        assert_eq!(doc.to_string(), text);
        assert_eq!(Json::parse(&doc.to_string()).unwrap().to_string(), text);
    }
}
