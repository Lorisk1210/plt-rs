//! The JSON the browser client exchanges with the server (RFC 8259): a
//! strict reader for untrusted messages, with a nesting limit, and the
//! escaping used to write replies.

use std::fmt::{self, Write as _};

/// How deeply arrays and objects may nest in a message.
const MAX_DEPTH: usize = 32;

/// A parsed JSON value; object members keep their order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Json {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A finite number.
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Json>),
    /// An object.
    Object(Vec<(String, Json)>),
}

/// Why a message is not valid JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JsonError {
    /// What is wrong.
    reason: &'static str,
    /// The byte offset where parsing stopped.
    offset: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.reason)
    }
}

impl std::error::Error for JsonError {}

impl Json {
    /// Parses a complete document.
    pub(crate) fn parse(text: &str) -> Result<Self, JsonError> {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = parser.value(0)?;
        parser.skip_whitespace();
        if parser.pos == parser.bytes.len() {
            Ok(value)
        } else {
            Err(parser.error("trailing characters"))
        }
    }

    /// The member `key` of an object (the first one, if repeated).
    pub(crate) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .find_map(|(name, value)| (name == key).then_some(value)),
            _ => None,
        }
    }

    /// The number, if this is one.
    pub(crate) fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// The string, if this is one.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The boolean, if this is one.
    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The elements, if this is an array.
    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn error(&self, reason: &'static str) -> JsonError {
        JsonError {
            reason,
            offset: self.pos,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect_literal(&mut self, literal: &[u8], value: Json) -> Result<Json, JsonError> {
        if self.bytes[self.pos..].starts_with(literal) {
            self.pos += literal.len();
            Ok(value)
        } else {
            Err(self.error("unknown literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            None => Err(self.error("unexpected end")),
            Some(b'n') => self.expect_literal(b"null", Json::Null),
            Some(b't') => self.expect_literal(b"true", Json::Bool(true)),
            Some(b'f') => self.expect_literal(b"false", Json::Bool(false)),
            Some(b'"') => self.string().map(Json::String),
            Some(b'[') => self.array(depth + 1),
            Some(b'{') => self.object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.pos += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(items));
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.error("nested too deeply"));
        }
        self.pos += 1;
        let mut members = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Json::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("expected a member name"));
            }
            let name = self.string()?;
            self.skip_whitespace();
            if self.peek() != Some(b':') {
                return Err(self.error("expected ':'"));
            }
            self.pos += 1;
            members.push((name, self.value(depth)?));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(members));
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.error("expected a digit")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.error("expected a fraction digit"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.error("expected an exponent digit"));
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.error("invalid number"))?;
        text.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(Json::Number)
            .ok_or_else(|| self.error("number out of range"))
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let digits = self
            .bytes
            .get(self.pos..self.pos + 4)
            .ok_or_else(|| self.error("truncated escape"))?;
        let text = std::str::from_utf8(digits).map_err(|_| self.error("invalid escape"))?;
        if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(self.error("invalid escape"));
        }
        let value = u32::from_str_radix(text, 16).map_err(|_| self.error("invalid escape"))?;
        self.pos += 4;
        Ok(value)
    }

    /// A `\u` escape (the `\u` already consumed), joining surrogate pairs;
    /// a lone surrogate becomes U+FFFD, as browsers may send them.
    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let high = self.hex4()?;
        if !(0xD800..0xDC00).contains(&high) {
            return Ok(char::from_u32(high).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        if !self.bytes[self.pos..].starts_with(b"\\u") {
            return Ok(char::REPLACEMENT_CHARACTER);
        }
        let saved = self.pos;
        self.pos += 2;
        let low = self.hex4()?;
        if (0xDC00..0xE000).contains(&low) {
            let code = 0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            Ok(char::from_u32(code).unwrap_or(char::REPLACEMENT_CHARACTER))
        } else {
            // Not a pair: the second escape stands on its own.
            self.pos = saved;
            Ok(char::REPLACEMENT_CHARACTER)
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.pos += 1;
        let mut out = String::new();
        loop {
            let run_start = self.pos;
            while matches!(self.peek(), Some(b) if b != b'"' && b != b'\\' && b >= 0x20) {
                self.pos += 1;
            }
            // The input is a `&str` and runs stop at ASCII bytes, so every
            // run is whole UTF-8.
            out.push_str(
                std::str::from_utf8(&self.bytes[run_start..self.pos])
                    .map_err(|_| self.error("invalid UTF-8"))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let escape = self.peek().ok_or_else(|| self.error("truncated escape"))?;
                    self.pos += 1;
                    out.push(match escape {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => self.unicode_escape()?,
                        _ => return Err(self.error("invalid escape")),
                    });
                }
                Some(_) => return Err(self.error("control character in string")),
                None => return Err(self.error("unterminated string")),
            }
        }
    }
}

/// `text` as a JSON string literal.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                // Writing to a String cannot fail.
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `value` as a JSON number (`null` when it is not finite, which JSON
/// cannot express).
pub(crate) fn number(value: f64) -> String {
    if value.is_finite() {
        format!("{value}")
    } else {
        "null".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_client_messages() {
        let message = Json::parse(
            r#"{"type": "motion_notify", "x": 12.5, "y": -3e2, "button": 0,
                "modifiers": ["ctrl", "shift"], "guiEvent": {"isTrusted": true, "key": null}}"#,
        )
        .unwrap();
        assert_eq!(
            message.get("type").and_then(Json::as_str),
            Some("motion_notify")
        );
        assert_eq!(message.get("x").and_then(Json::as_f64), Some(12.5));
        assert_eq!(message.get("y").and_then(Json::as_f64), Some(-300.0));
        let modifiers = message.get("modifiers").and_then(Json::as_array).unwrap();
        assert_eq!(modifiers.len(), 2);
        let gui = message.get("guiEvent").unwrap();
        assert_eq!(gui.get("isTrusted").and_then(Json::as_bool), Some(true));
        assert_eq!(gui.get("key"), Some(&Json::Null));
        assert_eq!(message.get("missing"), None);
    }

    #[test]
    fn strings_unescape() {
        let parsed = Json::parse(r#""a\"b\\c\/\n\u00e9\ud83d\ude00\ud800x""#).unwrap();
        assert_eq!(parsed.as_str(), Some("a\"b\\c/\né\u{1f600}\u{fffd}x"));
        assert_eq!(
            Json::parse(r#""\ud800\u0041""#).unwrap().as_str(),
            Some("\u{fffd}A")
        );
        assert_eq!(Json::parse("\"日本\"").unwrap().as_str(), Some("日本"));
    }

    #[test]
    fn malformed_documents_are_rejected() {
        for text in [
            "",
            "{",
            "[1,]",
            "{\"a\" 1}",
            "{\"a\":1,}",
            "01",
            "1.",
            "1e",
            "-",
            "1e999",
            "NaN",
            "\"abc",
            "\"\u{1}\"",
            "\"\\x\"",
            "\"\\u12\"",
            "\"\\u12g4\"",
            "tru",
            "{} {}",
            "{1: 2}",
        ] {
            assert!(Json::parse(text).is_err(), "{text:?} should be rejected");
        }
    }

    #[test]
    fn nesting_is_limited() {
        let deep = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(Json::parse(&deep).is_ok());
        let too_deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert!(Json::parse(&too_deep).is_err());
        // Hostile input fails fast instead of overflowing the stack.
        assert!(Json::parse(&"{\"a\":".repeat(100_000)).is_err());
    }

    #[test]
    fn writes_literals() {
        assert_eq!(string("a\"b\\\n\u{1}é"), r#""a\"b\\\n\u0001é""#);
        assert_eq!(number(640.0), "640");
        assert_eq!(number(-0.5), "-0.5");
        assert_eq!(number(f64::NAN), "null");
        let round_trip = Json::parse(&string("tab\there")).unwrap();
        assert_eq!(round_trip.as_str(), Some("tab\there"));
    }
}
