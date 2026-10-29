//! The PostScript tokenizer of matplotlib's `_type1font._tokenize`.
//!
//! Positions are byte offsets: matplotlib decodes the font as ASCII with
//! replacement characters, one character per byte, so its character
//! positions are byte positions. The consumer may ask for the next token
//! to be a binary run of a given length ([`Tokenizer::binary`]), as
//! matplotlib's generator accepts through `send`.

use super::error::Type1Error;
use super::python::{ascii_lossy, parse_float, parse_int};
use super::value::{PsNumber, PsValue};

/// What a PostScript token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PsTokenKind {
    /// `/name` or `//name`.
    Name,
    /// `true` or `false`.
    Boolean,
    /// Any other regular token that is not a number.
    Keyword,
    /// `[`, `]`, `{`, `}`, `<<`, `>>` or a stray `)`, `>` or `/`.
    Delimiter,
    /// White space or a comment.
    Whitespace,
    /// A `(...)` or `<...>` string.
    String,
    /// Raw bytes requested by the consumer.
    Binary,
    /// A regular token Python's `float()` accepts.
    Number,
}

impl PsTokenKind {
    /// matplotlib's name for the kind (`_Token.kind`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Boolean => "boolean",
            Self::Keyword => "keyword",
            Self::Delimiter => "delimiter",
            Self::Whitespace => "whitespace",
            Self::String => "string",
            Self::Binary => "binary",
            Self::Number => "number",
        }
    }
}

/// A PostScript token and its value, as matplotlib's `_tokenize` yields
/// them.
#[derive(Clone, Debug, PartialEq)]
pub struct PsToken {
    /// What the token is.
    pub kind: PsTokenKind,
    /// The offset matplotlib records (`_Token.pos`): where the token
    /// starts, except for `<...>` strings, which it records at their end.
    pub pos: usize,
    /// The source bytes (`_Token.raw`).
    pub raw: Vec<u8>,
    /// Its value: numbers, booleans and strings decoded, names without the
    /// slash, other tokens as their source text.
    pub value: PsValue,
}

/// The tokens of PostScript `data` as matplotlib's Type 1 parser reads
/// them (`_type1font._tokenize`), white space and comments included
/// unless `skip_whitespace` is set.
pub fn ps_tokenize(data: &[u8], skip_whitespace: bool) -> Result<Vec<PsToken>, Type1Error> {
    let mut tokenizer = Tokenizer::new(data, skip_whitespace);
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token()? {
        tokens.push(token.to_ps_token()?);
    }
    Ok(tokens)
}

/// One token: its kind, its position and its source bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Token<'a> {
    pub(crate) kind: PsTokenKind,
    /// The offset matplotlib records: the start of the token, except for
    /// `<...>` strings, which matplotlib records at their end.
    pub(crate) pos: usize,
    pub(crate) raw: &'a [u8],
}

impl Token<'_> {
    /// One past the end of the token as matplotlib computes it
    /// (`pos + len(raw)`).
    pub(crate) const fn end(&self) -> usize {
        self.pos + self.raw.len()
    }

    /// Whether this is a keyword spelled as one of `names`.
    pub(crate) fn is_keyword(&self, names: &[&str]) -> bool {
        self.kind == PsTokenKind::Keyword && names.iter().any(|name| name.as_bytes() == self.raw)
    }

    /// Whether this is a `/name`.
    pub(crate) fn is_slash_name(&self) -> bool {
        self.kind == PsTokenKind::Name && self.raw.starts_with(b"/")
    }

    /// Whether this is a delimiter.
    pub(crate) fn is_delimiter(&self) -> bool {
        self.kind == PsTokenKind::Delimiter
    }

    /// Whether this is a number.
    pub(crate) fn is_number(&self) -> bool {
        self.kind == PsTokenKind::Number
    }

    /// The source text as matplotlib holds it.
    pub(crate) fn text(&self) -> String {
        ascii_lossy(self.raw)
    }

    /// The delimiter closing (or opening) this one, `None` for the
    /// delimiters matplotlib has no opposite for.
    pub(crate) fn opposite(&self) -> Option<&'static [u8]> {
        Some(match self.raw {
            b"[" => b"]",
            b"]" => b"[",
            b"{" => b"}",
            b"}" => b"{",
            b"<<" => b">>",
            b">>" => b"<<",
            _ => return None,
        })
    }

    /// The number this token reads as: an integer without a decimal point,
    /// else a real (matplotlib's `_NumberToken.value`).
    pub(crate) fn number(&self) -> Result<PsNumber, Type1Error> {
        let text = self.text();
        let number = if text.contains('.') {
            parse_float(&text).map(PsNumber::Real)
        } else {
            parse_int(&text).map(PsNumber::Int)
        };
        number.ok_or(Type1Error::InvalidNumber(text))
    }

    /// The integer this number token reads as, `None` for reals.
    pub(crate) fn integer(&self) -> Result<Option<i64>, Type1Error> {
        Ok(match self.number()? {
            PsNumber::Int(value) => Some(value),
            PsNumber::Real(_) => None,
        })
    }

    /// The value of the token (matplotlib's `_Token.value`).
    pub(crate) fn value(&self) -> Result<PsValue, Type1Error> {
        Ok(match self.kind {
            PsTokenKind::Name => PsValue::Name(ascii_lossy(&self.raw[1..])),
            PsTokenKind::Boolean => PsValue::Bool(self.raw == b"true"),
            PsTokenKind::Number => PsValue::Number(self.number()?),
            PsTokenKind::String if self.raw.starts_with(b"(") => {
                PsValue::Text(unescape(&self.text()))
            }
            PsTokenKind::String => PsValue::Bytes(hex_string(self.raw)),
            PsTokenKind::Binary => PsValue::Bytes(self.raw.get(1..).unwrap_or_default().to_vec()),
            PsTokenKind::Keyword | PsTokenKind::Delimiter | PsTokenKind::Whitespace => {
                PsValue::Expression(self.text())
            }
        })
    }

    /// The token with its value, as the public API gives it.
    pub(crate) fn to_ps_token(self) -> Result<PsToken, Type1Error> {
        Ok(PsToken {
            kind: self.kind,
            pos: self.pos,
            raw: self.raw.to_vec(),
            value: self.value()?,
        })
    }

    /// The bytes of a binary token after the separating space.
    pub(crate) fn binary_value(&self) -> Vec<u8> {
        self.raw.get(1..).unwrap_or_default().to_vec()
    }
}

/// The text of a `(...)` string with its escapes resolved: `\\`, `\(`,
/// `\)`, `\n`, `\r`, `\t`, `\b`, `\f` and one to three octal digits;
/// other backslashes stay.
fn unescape(raw: &str) -> String {
    let inner = raw
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(raw);
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let replacement = match chars.peek() {
            Some(&e @ ('\\' | '(' | ')')) => Some(e),
            Some('n') => Some('\n'),
            Some('r') => Some('\r'),
            Some('t') => Some('\t'),
            Some('b') => Some('\u{8}'),
            Some('f') => Some('\u{c}'),
            _ => None,
        };
        if let Some(replacement) = replacement {
            chars.next();
            out.push(replacement);
            continue;
        }
        let mut code = 0u32;
        let mut digits = 0;
        while digits < 3 {
            match chars.peek().and_then(|d| d.to_digit(8)) {
                Some(digit) => {
                    code = code * 8 + digit;
                    digits += 1;
                    chars.next();
                }
                None => break,
            }
        }
        if digits == 0 {
            out.push('\\');
        } else {
            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
        }
    }
    out
}

/// The bytes of a `<...>` string: white space dropped, an odd final digit
/// completed with `0`.
fn hex_string(raw: &[u8]) -> Vec<u8> {
    let inner = raw.get(1..raw.len().saturating_sub(1)).unwrap_or_default();
    let mut digits: Vec<u8> = inner
        .iter()
        .copied()
        .filter(u8::is_ascii_hexdigit)
        .collect();
    if digits.len() % 2 == 1 {
        digits.push(b'0');
    }
    super::container::unhexlify(&digits).unwrap_or_default()
}

/// White space as the tokenizer skips it.
const fn is_space(byte: u8) -> bool {
    matches!(byte, b'\0' | b'\t' | b'\r' | b'\x0c' | b'\n' | b' ')
}

/// The bytes that may form a regular token.
const fn is_regular(byte: u8) -> bool {
    !is_space(byte)
        && !matches!(
            byte,
            b']' | b'(' | b')' | b'<' | b'>' | b'{' | b'}' | b'/' | b'%' | b'['
        )
}

/// Where a tokenizer stands in its data, kept by a
/// [`PsTokenStream`](super::PsTokenStream) between its calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cursor {
    pos: usize,
    /// Tokens left before the data is deemed to loop: negative binary
    /// lengths move backwards, and matplotlib would loop forever where
    /// they keep returning to the same place.
    tokens_left: usize,
}

impl Cursor {
    /// The cursor at the start of `data`.
    pub(crate) const fn start(data: &[u8]) -> Self {
        Self {
            pos: 0,
            tokens_left: data.len().saturating_mul(8).saturating_add(64),
        }
    }
}

/// A tokenizer over font data.
#[derive(Clone, Debug)]
pub(crate) struct Tokenizer<'a> {
    data: &'a [u8],
    pos: usize,
    skip_whitespace: bool,
    tokens_left: usize,
}

impl<'a> Tokenizer<'a> {
    /// A tokenizer from the start of `data`, dropping white space and
    /// comments when `skip_whitespace` is set.
    pub(crate) const fn new(data: &'a [u8], skip_whitespace: bool) -> Self {
        Self::resume(data, skip_whitespace, Cursor::start(data))
    }

    /// A tokenizer over `data` standing at `cursor`.
    pub(crate) const fn resume(data: &'a [u8], skip_whitespace: bool, cursor: Cursor) -> Self {
        Self {
            data,
            pos: cursor.pos,
            skip_whitespace,
            tokens_left: cursor.tokens_left,
        }
    }

    /// Where the tokenizer stands, to [`resume`](Self::resume) from.
    pub(crate) const fn cursor(&self) -> Cursor {
        Cursor {
            pos: self.pos,
            tokens_left: self.tokens_left,
        }
    }

    /// The next token, `None` at the end of the data.
    pub(crate) fn next_token(&mut self) -> Result<Option<Token<'a>>, Type1Error> {
        while self.pos < self.data.len() {
            self.tokens_left = self
                .tokens_left
                .checked_sub(1)
                .ok_or(Type1Error::ParseLimit)?;
            if let Some(end) = self.space_or_comment() {
                let token = self.take(PsTokenKind::Whitespace, end);
                if self.skip_whitespace {
                    continue;
                }
                return Ok(Some(token));
            }
            return self.token().map(Some);
        }
        Ok(None)
    }

    /// The next token that passes `wanted`, `None` at the end of the data.
    pub(crate) fn next_matching(
        &mut self,
        wanted: impl Fn(&Token<'a>) -> bool,
    ) -> Result<Option<Token<'a>>, Type1Error> {
        while let Some(token) = self.next_token()? {
            if wanted(&token) {
                return Ok(Some(token));
            }
        }
        Ok(None)
    }

    /// The next `length` bytes (fewer at the end) as a binary token,
    /// `Ok(None)` at the end of the data. A negative length gives an empty
    /// token and moves the tokenizer back, as matplotlib's position
    /// arithmetic does, unless it would move before the start.
    pub(crate) fn binary(&mut self, length: i64) -> Result<Option<Token<'a>>, Type1Error> {
        if self.pos >= self.data.len() {
            return Ok(None);
        }
        let start = self.pos;
        let end = if length < 0 {
            usize::try_from(length.unsigned_abs())
                .ok()
                .and_then(|back| start.checked_sub(back))
                .ok_or(Type1Error::NegativeLength(length))?
        } else {
            start.saturating_add(usize::try_from(length).unwrap_or(usize::MAX))
        };
        self.pos = end;
        Ok(Some(Token {
            kind: PsTokenKind::Binary,
            pos: start,
            raw: &self.data[start..end.clamp(start, self.data.len())],
        }))
    }

    /// A token of `kind` from the current position to `end`.
    fn take(&mut self, kind: PsTokenKind, end: usize) -> Token<'a> {
        let token = Token {
            kind,
            pos: self.pos,
            raw: &self.data[self.pos..end],
        };
        self.pos = end;
        token
    }

    /// The end of the white space or comment at the current position.
    fn space_or_comment(&self) -> Option<usize> {
        let rest = &self.data[self.pos..];
        let spaces = rest.iter().take_while(|&&b| is_space(b)).count();
        if spaces > 0 {
            return Some(self.pos + spaces);
        }
        if rest.first() == Some(&b'%') {
            let comment = rest
                .iter()
                .position(|&b| b == b'\r' || b == b'\n')
                .unwrap_or(rest.len());
            return Some(self.pos + comment);
        }
        None
    }

    /// The token at the current position, which is not white space.
    fn token(&mut self) -> Result<Token<'a>, Type1Error> {
        let rest = &self.data[self.pos..];
        if rest[0] == b'(' {
            let end = self.string_end()?;
            return Ok(self.take(PsTokenKind::String, end));
        }
        if rest.starts_with(b"<<") || rest.starts_with(b">>") {
            return Ok(self.take(PsTokenKind::Delimiter, self.pos + 2));
        }
        if rest[0] == b'<' {
            let start = self.pos;
            let close = rest
                .iter()
                .position(|&b| b == b'>')
                .ok_or(Type1Error::UnterminatedHexString(start))?;
            let inner = &rest[1..close];
            if !inner.iter().all(|&b| b.is_ascii_hexdigit() || is_space(b)) {
                return Err(Type1Error::MalformedHexString(start));
            }
            // matplotlib records hex strings at their end.
            let end = start + close + 1;
            self.pos = end;
            return Ok(Token {
                kind: PsTokenKind::String,
                pos: end,
                raw: &self.data[start..end],
            });
        }
        let slashes = rest.iter().take(3).take_while(|&&b| b == b'/').count();
        if slashes <= 2 && rest.get(slashes).is_some_and(|&b| is_regular(b)) {
            let length = slashes
                + rest[slashes..]
                    .iter()
                    .take_while(|&&b| is_regular(b))
                    .count();
            let raw = &rest[..length];
            let kind = if raw[0] == b'/' {
                PsTokenKind::Name
            } else if raw == b"true" || raw == b"false" {
                PsTokenKind::Boolean
            } else if parse_float(&ascii_lossy(raw)).is_some() {
                PsTokenKind::Number
            } else {
                PsTokenKind::Keyword
            };
            return Ok(self.take(kind, self.pos + length));
        }
        Ok(self.take(PsTokenKind::Delimiter, self.pos + 1))
    }

    /// The end of the `(...)` string at the current position: parentheses
    /// balance, and backslashes escape `\`, `(`, `)`, `nrtbf` and up to
    /// three octal digits.
    fn string_end(&self) -> Result<usize, Type1Error> {
        let start = self.pos;
        let unterminated = Type1Error::UnterminatedString(start);
        let mut pos = start + 1;
        let mut depth = 1;
        while depth > 0 {
            let offset = self.data[pos..]
                .iter()
                .position(|&b| matches!(b, b'(' | b')' | b'\\'))
                .ok_or_else(|| unterminated.clone())?;
            let found = self.data[pos + offset];
            pos += offset + 1;
            match found {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {
                    let escaped = *self.data.get(pos).ok_or_else(|| unterminated.clone())?;
                    if b"\\()nrtbf".contains(&escaped) {
                        pos += 1;
                    } else {
                        pos += self.data[pos..]
                            .iter()
                            .take(3)
                            .take_while(|&&b| (b'0'..=b'7').contains(&b))
                            .count();
                    }
                }
            }
        }
        Ok(pos)
    }
}

#[cfg(test)]
mod tests;
