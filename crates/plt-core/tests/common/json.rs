//! A minimal JSON reader for the fixture files (written by Python's `json`
//! module, so `NaN`, `Infinity` and `-Infinity` are accepted).

use std::collections::BTreeMap;

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
    /// Parses a complete document, panicking on malformed fixtures.
    pub(crate) fn parse(text: &str) -> Self {
        let mut parser = Parser {
            bytes: text.as_bytes(),
            pos: 0,
        };
        let value = parser.value();
        parser.skip_whitespace();
        assert_eq!(parser.pos, parser.bytes.len(), "trailing data in fixture");
        value
    }

    /// Object member `key`.
    pub(crate) fn get(&self, key: &str) -> &Self {
        match self {
            Self::Object(map) => map
                .get(key)
                .unwrap_or_else(|| panic!("missing key {key:?}")),
            _ => panic!("not an object when looking up {key:?}"),
        }
    }

    /// Whether this is an object with member `key`.
    pub(crate) fn has(&self, key: &str) -> bool {
        matches!(self, Self::Object(map) if map.contains_key(key))
    }

    pub(crate) fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub(crate) fn f64(&self) -> f64 {
        match self {
            Self::Number(n) => *n,
            other => panic!("expected a number, got {other:?}"),
        }
    }

    pub(crate) fn usize(&self) -> usize {
        self.f64() as usize
    }

    pub(crate) fn bool(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            other => panic!("expected a bool, got {other:?}"),
        }
    }

    pub(crate) fn str(&self) -> &str {
        match self {
            Self::String(s) => s,
            other => panic!("expected a string, got {other:?}"),
        }
    }

    pub(crate) fn array(&self) -> &[Self] {
        match self {
            Self::Array(items) => items,
            other => panic!("expected an array, got {other:?}"),
        }
    }

    pub(crate) fn f64_vec(&self) -> Vec<f64> {
        self.array().iter().map(Self::f64).collect()
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> u8 {
        self.skip_whitespace();
        *self.bytes.get(self.pos).expect("unexpected end of fixture")
    }

    fn eat(&mut self, literal: &str) -> bool {
        self.skip_whitespace();
        let found = self.bytes[self.pos..].starts_with(literal.as_bytes());
        if found {
            self.pos += literal.len();
        }
        found
    }

    fn expect(&mut self, literal: &str) {
        assert!(
            self.eat(literal),
            "expected {literal:?} at byte {}",
            self.pos
        );
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Json::String(self.string()),
            b't' => {
                self.expect("true");
                Json::Bool(true)
            }
            b'f' => {
                self.expect("false");
                Json::Bool(false)
            }
            b'n' => {
                self.expect("null");
                Json::Null
            }
            _ => Json::Number(self.number()),
        }
    }

    fn object(&mut self) -> Json {
        self.expect("{");
        let mut map = BTreeMap::new();
        if !self.eat("}") {
            loop {
                let key = self.string();
                self.expect(":");
                map.insert(key, self.value());
                if self.eat("}") {
                    break;
                }
                self.expect(",");
            }
        }
        Json::Object(map)
    }

    fn array(&mut self) -> Json {
        self.expect("[");
        let mut items = Vec::new();
        if !self.eat("]") {
            loop {
                items.push(self.value());
                if self.eat("]") {
                    break;
                }
                self.expect(",");
            }
        }
        Json::Array(items)
    }

    fn string(&mut self) -> String {
        self.expect("\"");
        let mut out = String::new();
        loop {
            let c = self.bytes[self.pos];
            self.pos += 1;
            match c {
                b'"' => return out,
                b'\\' => {
                    let escaped = self.bytes[self.pos];
                    self.pos += 1;
                    out.push(match escaped {
                        b'n' => '\n',
                        b't' => '\t',
                        other => char::from(other),
                    });
                }
                other => out.push(char::from(other)),
            }
        }
    }

    fn number(&mut self) -> f64 {
        for (literal, value) in [
            ("NaN", f64::NAN),
            ("Infinity", f64::INFINITY),
            ("-Infinity", f64::NEG_INFINITY),
        ] {
            if self.eat(literal) {
                return value;
            }
        }
        let start = self.pos;
        while self.pos < self.bytes.len()
            && matches!(
                self.bytes[self.pos],
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
            )
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).expect("ascii number");
        text.parse()
            .unwrap_or_else(|_| panic!("bad number {text:?} at byte {start}"))
    }
}
