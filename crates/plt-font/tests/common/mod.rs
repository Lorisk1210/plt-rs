//! Shared helpers for the integration tests: a minimal JSON reader (the
//! crate has no dependencies, not even for tests) and font loading.

#![allow(
    dead_code,
    reason = "each test binary uses a different subset of the helpers"
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

/// A parsed JSON value.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

impl Json {
    pub(crate) fn parse(text: &str) -> Json {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = parser.value();
        parser.skip_ws();
        assert_eq!(parser.pos, parser.bytes.len(), "trailing data in JSON");
        value
    }

    pub(crate) fn get(&self, key: &str) -> &Json {
        match self {
            Json::Object(map) => map.get(key).unwrap_or_else(|| panic!("missing key {key}")),
            other => panic!("not an object: {other:?}"),
        }
    }

    pub(crate) fn opt(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(map) => map.get(key).filter(|value| **value != Json::Null),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> &[Json] {
        match self {
            Json::Array(items) => items,
            other => panic!("not an array: {other:?}"),
        }
    }

    pub(crate) fn as_object(&self) -> &BTreeMap<String, Json> {
        match self {
            Json::Object(map) => map,
            other => panic!("not an object: {other:?}"),
        }
    }

    pub(crate) fn as_f64(&self) -> f64 {
        match self {
            Json::Number(value) => *value,
            other => panic!("not a number: {other:?}"),
        }
    }

    pub(crate) fn as_i64(&self) -> i64 {
        self.as_f64() as i64
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            Json::String(text) => text,
            other => panic!("not a string: {other:?}"),
        }
    }

    pub(crate) fn as_bool(&self) -> bool {
        match self {
            Json::Bool(value) => *value,
            other => panic!("not a bool: {other:?}"),
        }
    }

    pub(crate) fn i64s(&self) -> Vec<i64> {
        self.as_array().iter().map(Json::as_i64).collect()
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_ws();
        self.bytes[self.pos]
    }

    fn expect(&mut self, byte: u8) {
        assert_eq!(self.peek(), byte, "unexpected JSON at {}", self.pos);
        self.pos += 1;
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => {
                self.pos += 1;
                let mut map = BTreeMap::new();
                if self.peek() == b'}' {
                    self.pos += 1;
                    return Json::Object(map);
                }
                loop {
                    let key = match self.value() {
                        Json::String(key) => key,
                        other => panic!("object key {other:?}"),
                    };
                    self.expect(b':');
                    map.insert(key, self.value());
                    if self.peek() != b',' {
                        self.expect(b'}');
                        return Json::Object(map);
                    }
                    self.pos += 1;
                }
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                if self.peek() == b']' {
                    self.pos += 1;
                    return Json::Array(items);
                }
                loop {
                    items.push(self.value());
                    if self.peek() != b',' {
                        self.expect(b']');
                        return Json::Array(items);
                    }
                    self.pos += 1;
                }
            }
            b'"' => Json::String(self.string()),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            _ => self.number(),
        }
    }

    fn literal(&mut self, word: &str, value: Json) -> Json {
        assert!(self.bytes[self.pos..].starts_with(word.as_bytes()));
        self.pos += word.len();
        value
    }

    fn number(&mut self) -> Json {
        let start = self.pos;
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| b"+-.eE0123456789".contains(b))
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).expect("ASCII number");
        Json::Number(text.parse().unwrap_or_else(|_| panic!("bad number {text}")))
    }

    fn hex4(&mut self) -> u32 {
        let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).expect("ASCII escape");
        self.pos += 4;
        u32::from_str_radix(text, 16).expect("hex escape")
    }

    fn string(&mut self) -> String {
        self.expect(b'"');
        let mut out = String::new();
        loop {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            match byte {
                b'"' => return out,
                b'\\' => {
                    let escape = self.bytes[self.pos];
                    self.pos += 1;
                    match escape {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let unit = self.hex4();
                            let code = if (0xD800..0xDC00).contains(&unit) {
                                self.pos += 2;
                                let low = self.hex4();
                                0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)
                            } else {
                                unit
                            };
                            out.push(char::from_u32(code).expect("valid escaped code point"));
                        }
                        other => out.push(char::from(other)),
                    }
                }
                _ => {
                    let start = self.pos - 1;
                    let len = match byte {
                        0x00..=0x7F => 1,
                        0xC0..=0xDF => 2,
                        0xE0..=0xEF => 3,
                        _ => 4,
                    };
                    out.push_str(
                        std::str::from_utf8(&self.bytes[start..start + len]).expect("UTF-8 JSON"),
                    );
                    self.pos = start + len;
                }
            }
        }
    }
}

/// Reads and parses `tests/fixtures/<name>`.
pub(crate) fn fixture(name: &str) -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    Json::parse(
        &std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())),
    )
}

/// Bytes of a bundled font file.
pub(crate) fn font_bytes(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fonts")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Converts recorded outline commands to matplotlib path codes with
/// coordinates multiplied by `scale` and rounded: `[code, x, y, ...]`.
pub(crate) fn path_codes(commands: &[plt_font::PathCommand], scale: f64) -> Vec<i64> {
    use plt_font::PathCommand;
    let s = |v: f32| (f64::from(v) * scale).round() as i64;
    let mut out = Vec::new();
    for command in commands {
        match *command {
            PathCommand::MoveTo(x, y) => out.extend([1, s(x), s(y)]),
            PathCommand::LineTo(x, y) => out.extend([2, s(x), s(y)]),
            PathCommand::QuadTo(x1, y1, x, y) => out.extend([3, s(x1), s(y1), 3, s(x), s(y)]),
            PathCommand::CurveTo(x1, y1, x2, y2, x, y) => {
                out.extend([4, s(x1), s(y1), 4, s(x2), s(y2), 4, s(x), s(y)]);
            }
            PathCommand::Close => out.extend([79, 0, 0]),
        }
    }
    out
}
