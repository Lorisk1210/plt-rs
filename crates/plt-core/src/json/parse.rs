//! The reader: one recursive-descent parser for both dialects.

use std::fmt;

use super::Json;

/// What a document may contain beyond RFC 8259.
#[derive(Clone, Copy, Debug)]
pub(super) struct Dialect {
    /// Whether `NaN`, `Infinity` and `-Infinity` are values, and numbers
    /// too large for `f64` read as infinite instead of failing.
    pub(super) non_finite: bool,
    /// How deeply arrays and objects may nest.
    pub(super) max_depth: usize,
}

impl Dialect {
    /// Python's `json` module with `allow_nan=True`, nested at most as
    /// deeply as untrusted input can be read without exhausting the stack.
    pub(super) const PYTHON: Self = Self {
        non_finite: true,
        max_depth: 128,
    };

    /// RFC 8259 exactly, for untrusted messages.
    pub(super) const STRICT: Self = Self {
        non_finite: false,
        max_depth: 32,
    };
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

impl Json {
    /// Parses a complete document as Python's `json` module writes it with
    /// `allow_nan=True`: RFC 8259 plus the `NaN`, `Infinity` and
    /// `-Infinity` tokens, with integer literals that fit an `i64` read as
    /// [`Json::Integer`]s (all but `-0`, which an integer cannot hold) and
    /// numbers beyond `f64` as infinite. Nesting is limited to 128 arrays
    /// and objects.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        parse(text, Dialect::PYTHON)
    }

    /// Parses a complete, untrusted document strictly by RFC 8259: only
    /// finite numbers, and arrays and objects nested at most 32 deep.
    /// Integer literals are read as by [`Json::parse`].
    pub fn parse_strict(text: &str) -> Result<Self, ParseError> {
        parse(text, Dialect::STRICT)
    }
}

/// Parses `text` as one document of `dialect`.
fn parse(text: &str, dialect: Dialect) -> Result<Json, ParseError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        dialect,
    };
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.pos == parser.bytes.len() {
        Ok(value)
    } else {
        Err(parser.error("trailing characters"))
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    dialect: Dialect,
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
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    /// Consumes `literal` if the input continues with it.
    fn eat(&mut self, literal: &[u8]) -> bool {
        let matches = self.bytes[self.pos..].starts_with(literal);
        if matches {
            self.pos += literal.len();
        }
        matches
    }

    fn literal(&mut self, literal: &[u8], value: Json) -> Result<Json, ParseError> {
        if self.eat(literal) {
            Ok(value)
        } else {
            Err(self.error("unknown literal"))
        }
    }

    /// A value whose arrays and objects open nesting level `depth + 1`.
    fn value(&mut self, depth: usize) -> Result<Json, ParseError> {
        self.skip_whitespace();
        let non_finite = self.dialect.non_finite;
        match self.peek() {
            None => Err(self.error("unexpected end")),
            Some(b'n') => self.literal(b"null", Json::Null),
            Some(b't') => self.literal(b"true", Json::Bool(true)),
            Some(b'f') => self.literal(b"false", Json::Bool(false)),
            Some(b'N') if non_finite => self.literal(b"NaN", Json::Number(f64::NAN)),
            Some(b'I') if non_finite => self.literal(b"Infinity", Json::Number(f64::INFINITY)),
            Some(b'-') if non_finite && self.eat(b"-Infinity") => {
                Ok(Json::Number(f64::NEG_INFINITY))
            }
            Some(b'"') => self.string().map(Json::String),
            Some(b'[') => self.array(depth + 1),
            Some(b'{') => self.object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth > self.dialect.max_depth {
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

    fn object(&mut self, depth: usize) -> Result<Json, ParseError> {
        if depth > self.dialect.max_depth {
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

    /// A number by RFC 8259's grammar: an integer literal that fits an
    /// `i64` (other than `-0`) is an [`Json::Integer`], anything else a
    /// [`Json::Number`].
    fn number(&mut self) -> Result<Json, ParseError> {
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
        let mut integral = true;
        if self.peek() == Some(b'.') {
            integral = false;
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.error("expected a fraction digit"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integral = false;
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
        // An `i64` has no negative zero, so `-0` (the only such literal the
        // grammar allows) stays the float it denotes.
        if integral
            && text != "-0"
            && let Ok(integer) = text.parse()
        {
            return Ok(Json::Integer(integer));
        }
        text.parse::<f64>()
            .ok()
            .filter(|n| n.is_finite() || self.dialect.non_finite)
            .map(Json::Number)
            .ok_or_else(|| self.error("number out of range"))
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
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
    fn unicode_escape(&mut self) -> Result<char, ParseError> {
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

    fn string(&mut self) -> Result<String, ParseError> {
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
