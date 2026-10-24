//! PDF syntax: objects, the cross-reference table and content-stream
//! operators.

use std::collections::HashMap;

/// A PDF object.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Name(String),
    Str(Vec<u8>),
    Array(Vec<Value>),
    Dict(Vec<(String, Value)>),
    Ref(u32),
    /// An operator in a content stream.
    Op(String),
}

impl Value {
    /// The entry `key` of a dictionary.
    pub(crate) fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Dict(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub(crate) fn as_num(&self) -> Option<f64> {
        match self {
            Self::Num(n) => Some(*n),
            _ => None,
        }
    }

    pub(crate) fn as_name(&self) -> Option<&str> {
        match self {
            Self::Name(n) => Some(n),
            _ => None,
        }
    }

    pub(crate) fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// Whether `b` ends a token.
fn is_delimiter(b: u8) -> bool {
    b.is_ascii_whitespace() || b"()<>[]{}/%".contains(&b)
}

/// A cursor over PDF bytes.
pub(crate) struct Lexer<'a> {
    data: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) const fn new(data: &'a [u8], pos: usize) -> Self {
        Self { data, pos }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn skip_space(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.pos += 1;
            } else if b == b'%' {
                while self.peek().is_some_and(|b| b != b'\n' && b != b'\r') {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Whether only whitespace is left.
    pub(crate) fn at_end(&mut self) -> bool {
        self.skip_space();
        self.pos >= self.data.len()
    }

    fn word(&mut self) -> &'a str {
        let start = self.pos;
        while self.peek().is_some_and(|b| !is_delimiter(b)) {
            self.pos += 1;
        }
        std::str::from_utf8(&self.data[start..self.pos]).unwrap_or("")
    }

    fn literal_string(&mut self) -> Result<Vec<u8>, String> {
        self.pos += 1;
        let mut out = Vec::new();
        let mut depth = 1;
        loop {
            let b = self.peek().ok_or("unterminated string")?;
            self.pos += 1;
            match b {
                b'\\' => {
                    let e = self.peek().ok_or("unterminated escape")?;
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'0'..=b'7' => {
                            let mut code = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        code = code * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push(code as u8);
                        }
                        b'\n' => {}
                        other => out.push(other),
                    }
                }
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                    out.push(b);
                }
                _ => out.push(b),
            }
        }
    }

    fn hex_string(&mut self) -> Result<Vec<u8>, String> {
        self.pos += 1;
        let start = self.pos;
        let end = self.data[start..]
            .iter()
            .position(|&b| b == b'>')
            .ok_or("unterminated hex string")?;
        self.pos = start + end + 1;
        let digits: Vec<u8> = self.data[start..start + end]
            .iter()
            .filter(|b| b.is_ascii_hexdigit())
            .copied()
            .collect();
        Ok(digits
            .chunks(2)
            .map(|pair| {
                let text = std::str::from_utf8(pair).unwrap_or("0");
                let value = u8::from_str_radix(text, 16).unwrap_or(0);
                if pair.len() == 1 { value << 4 } else { value }
            })
            .collect())
    }

    /// The next object or operator; `N G R` becomes a reference.
    pub(crate) fn value(&mut self) -> Result<Value, String> {
        self.skip_space();
        let b = self.peek().ok_or("unexpected end of data")?;
        match b {
            b'/' => {
                self.pos += 1;
                let raw = self.word();
                Ok(Value::Name(decode_name(raw)))
            }
            b'(' => self.literal_string().map(Value::Str),
            b'<' if self.data.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                let mut entries = Vec::new();
                loop {
                    self.skip_space();
                    if self.data[self.pos..].starts_with(b">>") {
                        self.pos += 2;
                        return Ok(Value::Dict(entries));
                    }
                    let Value::Name(key) = self.value()? else {
                        return Err("dictionary key is not a name".to_owned());
                    };
                    entries.push((key, self.value()?));
                }
            }
            b'<' => self.hex_string().map(Value::Str),
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    if self.peek() == Some(b']') {
                        self.pos += 1;
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value()?);
                }
            }
            _ => {
                let word = self.word();
                if word.is_empty() {
                    self.pos += 1;
                    return Err(format!("unexpected byte {:?}", char::from(b)));
                }
                if let Ok(number) = word.parse::<f64>() {
                    return Ok(self.maybe_reference(number));
                }
                Ok(match word {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    "null" => Value::Null,
                    _ => Value::Op(word.to_owned()),
                })
            }
        }
    }

    /// `number` alone, or the start of `N G R`.
    fn maybe_reference(&mut self, number: f64) -> Value {
        let save = self.pos;
        self.skip_space();
        let generation = self.word();
        if generation.parse::<u32>().is_ok() {
            self.skip_space();
            if self.word() == "R" {
                return Value::Ref(number as u32);
            }
        }
        self.pos = save;
        Value::Num(number)
    }
}

/// Undoes `#xx` escapes in a name.
fn decode_name(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#'
            && let Some(code) = raw
                .get(i + 1..i + 3)
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(code);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// An object as stored in the file: its value and, for streams, the raw
/// (still encoded) data.
#[derive(Clone, Debug)]
pub(crate) struct Stored {
    pub(crate) value: Value,
    pub(crate) stream: Option<Vec<u8>>,
}

/// Every object of a file, by number.
pub(crate) struct Document {
    pub(crate) objects: HashMap<u32, Stored>,
    pub(crate) trailer: Value,
}

impl Document {
    /// Reads a file through its cross-reference table.
    pub(crate) fn parse(data: &[u8]) -> Result<Self, String> {
        let start = find_last(data, b"startxref").ok_or("no startxref")?;
        let mut lexer = Lexer::new(data, start + b"startxref".len());
        let xref = lexer.value()?.as_num().ok_or("bad startxref")? as usize;
        let mut lexer = Lexer::new(data, xref);
        if lexer.value()? != Value::Op("xref".to_owned()) {
            return Err("no xref table".to_owned());
        }
        let first = lexer.value()?.as_num().ok_or("bad xref")? as u32;
        let count = lexer.value()?.as_num().ok_or("bad xref")? as u32;
        let mut offsets = Vec::new();
        for i in 0..count {
            let offset = lexer.value()?.as_num().ok_or("bad xref entry")? as usize;
            lexer.value()?;
            if lexer.value()? == Value::Op("n".to_owned()) {
                offsets.push((first + i, offset));
            }
        }
        if lexer.value()? != Value::Op("trailer".to_owned()) {
            return Err("no trailer".to_owned());
        }
        let trailer = lexer.value()?;
        let mut objects = HashMap::new();
        let mut pending = Vec::new();
        for (id, offset) in offsets {
            let mut lexer = Lexer::new(data, offset);
            lexer.value()?;
            lexer.value()?;
            if lexer.value()? != Value::Op("obj".to_owned()) {
                return Err(format!("object {id} not at its offset"));
            }
            let value = lexer.value()?;
            lexer.skip_space();
            if data[lexer.pos..].starts_with(b"stream") {
                let mut body = lexer.pos + b"stream".len();
                if data.get(body) == Some(&b'\r') {
                    body += 1;
                }
                if data.get(body) == Some(&b'\n') {
                    body += 1;
                }
                pending.push((id, value, body));
            } else {
                objects.insert(
                    id,
                    Stored {
                        value,
                        stream: None,
                    },
                );
            }
        }
        for (id, value, body) in pending {
            let length = match value.get("Length") {
                Some(Value::Num(n)) => *n as usize,
                Some(Value::Ref(r)) => objects
                    .get(r)
                    .and_then(|s| s.value.as_num())
                    .ok_or("unresolved stream length")?
                    as usize,
                _ => return Err(format!("stream {id} without length")),
            };
            let data = data
                .get(body..body + length)
                .ok_or("stream beyond end of file")?
                .to_vec();
            objects.insert(
                id,
                Stored {
                    value,
                    stream: Some(data),
                },
            );
        }
        Ok(Self { objects, trailer })
    }

    /// Follows references.
    pub(crate) fn resolve<'a>(&'a self, value: &'a Value) -> &'a Value {
        match value {
            Value::Ref(id) => self
                .objects
                .get(id)
                .map_or(&Value::Null, |stored| &stored.value),
            other => other,
        }
    }

    /// The decoded data of the stream `value` refers to.
    pub(crate) fn stream(&self, value: &Value) -> Option<Vec<u8>> {
        let Value::Ref(id) = value else { return None };
        let stored = self.objects.get(id)?;
        let raw = stored.stream.as_ref()?;
        match stored.value.get("Filter").and_then(Value::as_name) {
            Some("FlateDecode") => plt_codec::zlib::decompress(raw).ok(),
            _ => Some(raw.clone()),
        }
    }
}

fn find_last(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len()).rposition(|w| w == needle)
}

/// Splits content-stream data into `(operands, operator)` pairs.
pub(crate) fn operations(data: &[u8]) -> Result<Vec<(Vec<Value>, String)>, String> {
    let mut lexer = Lexer::new(data, 0);
    let mut ops = Vec::new();
    let mut operands = Vec::new();
    while !lexer.at_end() {
        match lexer.value()? {
            Value::Op(op) => ops.push((std::mem::take(&mut operands), op)),
            value => operands.push(value),
        }
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_objects_and_operators() {
        let ops = operations(b"q 1 0 -0 1 2.5 3 cm /F1 10 Tf [ (a\\(b) -12 (c) ] TJ <48ff> Tj\n")
            .unwrap();
        assert_eq!(ops.len(), 5);
        assert_eq!(ops[0], (vec![], "q".to_owned()));
        assert_eq!(ops[1].0[4], Value::Num(2.5));
        assert_eq!(ops[2].0[0], Value::Name("F1".to_owned()));
        assert_eq!(
            ops[3].0[0],
            Value::Array(vec![
                Value::Str(b"a(b".to_vec()),
                Value::Num(-12.0),
                Value::Str(b"c".to_vec())
            ])
        );
        let mut lexer = Lexer::new(b"<< /A 1 0 R /B [ 2 0 R 3 ] /C#20d (x) >>", 0);
        let dict = lexer.value().unwrap();
        assert_eq!(dict.get("A"), Some(&Value::Ref(1)));
        assert_eq!(
            dict.get("B"),
            Some(&Value::Array(vec![Value::Ref(2), Value::Num(3.0)]))
        );
        assert_eq!(dict.get("C d"), Some(&Value::Str(b"x".to_vec())));
    }
}
