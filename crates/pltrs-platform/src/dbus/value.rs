//! D-Bus values as the wire carries them: signatures, and values read by
//! their signature, with the alignment and limits of the D-Bus
//! specification ("Marshaling").

use super::BusError as Error;

/// How deep containers (arrays, structures, variants) may nest.
const MAX_DEPTH: usize = 64;
/// The longest signature.
const MAX_SIGNATURE: usize = 255;

/// A value read off the wire.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    /// `y`.
    Byte(u8),
    /// `b`.
    Bool(bool),
    /// `n`, `i`, `x`.
    Int(i64),
    /// `q`, `u`, `t`, and `h` (a file descriptor's index).
    UInt(u64),
    /// `d`.
    Double(f64),
    /// `s`, `o`, `g`.
    Str(String),
    /// `v`.
    Variant(Box<Value>),
    /// `a...`.
    Array(Vec<Value>),
    /// `(...)`.
    Struct(Vec<Value>),
    /// `{kv}`, inside arrays.
    Entry(Box<Value>, Box<Value>),
}

impl Value {
    /// The string a value holds, through any variants around it.
    pub(crate) fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(text) => Some(text),
            Self::Variant(inner) => inner.as_str(),
            _ => None,
        }
    }

    /// The value of key `key` of a dictionary (`a{s...}`).
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        let Self::Array(entries) = self else {
            return None;
        };
        entries.iter().find_map(|entry| match entry {
            Self::Entry(k, v) if k.as_str() == Some(key) => Some(&**v),
            _ => None,
        })
    }
}

fn malformed() -> Error {
    Error::Malformed
}

/// The alignment of values of the type starting `signature`.
fn alignment(code: u8) -> usize {
    match code {
        b'n' | b'q' => 2,
        b'b' | b'i' | b'u' | b'h' | b's' | b'o' | b'a' => 4,
        b'x' | b't' | b'd' | b'(' | b'{' => 8,
        _ => 1,
    }
}

/// The length of the single complete type at the start of `signature`.
pub(crate) fn complete_type(signature: &[u8]) -> Result<usize, Error> {
    complete_type_at(signature, 0)
}

fn complete_type_at(signature: &[u8], depth: usize) -> Result<usize, Error> {
    if depth > MAX_DEPTH {
        return Err(malformed());
    }
    match signature.first() {
        Some(
            b'y' | b'b' | b'n' | b'q' | b'i' | b'u' | b'x' | b't' | b'd' | b'h' | b's' | b'o'
            | b'g' | b'v',
        ) => Ok(1),
        Some(b'a') => {
            let element = &signature[1..];
            if element.first() == Some(&b'{') {
                let key = complete_type_at(&element[1..], depth + 1)?;
                if !b"ybnqiuxtdhsog".contains(&element[1]) {
                    return Err(malformed());
                }
                let value = complete_type_at(&element[1 + key..], depth + 1)?;
                if element.get(1 + key + value) != Some(&b'}') {
                    return Err(malformed());
                }
                return Ok(1 + key + value + 2);
            }
            Ok(1 + complete_type_at(element, depth + 1)?)
        }
        Some(b'(') => {
            let mut length = 1;
            while signature.get(length) != Some(&b')') {
                if length >= signature.len() {
                    return Err(malformed());
                }
                length += complete_type_at(&signature[length..], depth + 1)?;
            }
            if length == 1 {
                return Err(malformed());
            }
            Ok(length + 1)
        }
        _ => Err(malformed()),
    }
}

/// Splits a signature into its complete types.
pub(crate) fn types(signature: &[u8]) -> Result<Vec<&[u8]>, Error> {
    if signature.len() > MAX_SIGNATURE {
        return Err(malformed());
    }
    let mut types = Vec::new();
    let mut rest = signature;
    while !rest.is_empty() {
        let length = complete_type(rest)?;
        types.push(&rest[..length]);
        rest = &rest[length..];
    }
    Ok(types)
}

/// Reads values off a message's bytes, aligned from its start.
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    big_endian: bool,
}

impl<'a> Reader<'a> {
    /// A reader of `bytes` from `pos`, in the message's byte order.
    pub(crate) fn new(bytes: &'a [u8], pos: usize, big_endian: bool) -> Self {
        Self {
            bytes,
            pos,
            big_endian,
        }
    }

    /// The position reached.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// Skips the padding before a value aligned to `alignment`.
    pub(crate) fn align(&mut self, alignment: usize) -> Result<(), Error> {
        let aligned = self.pos.next_multiple_of(alignment);
        if aligned > self.bytes.len() {
            return Err(malformed());
        }
        self.pos = aligned;
        Ok(())
    }

    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N).ok_or_else(malformed)?;
        let bytes = self.bytes.get(self.pos..end).ok_or_else(malformed)?;
        self.pos = end;
        let mut array: [u8; N] = bytes.try_into().map_err(|_| malformed())?;
        if !self.big_endian {
            array.reverse();
        }
        Ok(array)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take::<1>()?[0])
    }

    pub(crate) fn u32(&mut self) -> Result<u32, Error> {
        self.align(4)?;
        Ok(u32::from_be_bytes(self.take()?))
    }

    fn u16(&mut self) -> Result<u16, Error> {
        self.align(2)?;
        Ok(u16::from_be_bytes(self.take()?))
    }

    fn u64(&mut self) -> Result<u64, Error> {
        self.align(8)?;
        Ok(u64::from_be_bytes(self.take()?))
    }

    /// `length` bytes and the NUL after them, as UTF-8.
    fn text(&mut self, length: usize) -> Result<String, Error> {
        let end = self.pos.checked_add(length).ok_or_else(malformed)?;
        let bytes = self.bytes.get(self.pos..end).ok_or_else(malformed)?;
        if self.bytes.get(end) != Some(&0) {
            return Err(malformed());
        }
        self.pos = end + 1;
        String::from_utf8(bytes.to_vec()).map_err(|_| malformed())
    }

    /// A signature (`g`).
    pub(crate) fn signature(&mut self) -> Result<String, Error> {
        let length = usize::from(self.u8()?);
        self.text(length)
    }

    /// A value of the single complete type `signature`.
    pub(crate) fn value(&mut self, signature: &[u8]) -> Result<Value, Error> {
        self.value_at(signature, 0)
    }

    fn value_at(&mut self, signature: &[u8], depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(malformed());
        }
        let code = *signature.first().ok_or_else(malformed)?;
        let value = match code {
            b'y' => Value::Byte(self.u8()?),
            b'b' => match self.u32()? {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(malformed()),
            },
            b'n' => Value::Int(i64::from(self.u16()?.cast_signed())),
            b'q' => Value::UInt(u64::from(self.u16()?)),
            b'i' => Value::Int(i64::from(self.u32()?.cast_signed())),
            b'u' | b'h' => Value::UInt(u64::from(self.u32()?)),
            b'x' => Value::Int(self.u64()?.cast_signed()),
            b't' => Value::UInt(self.u64()?),
            b'd' => Value::Double(f64::from_bits(self.u64()?)),
            b's' | b'o' => {
                let length = self.u32()? as usize;
                Value::Str(self.text(length)?)
            }
            b'g' => Value::Str(self.signature()?),
            b'v' => {
                let inner = self.signature()?;
                let inner = inner.as_bytes();
                if complete_type(inner)? != inner.len() {
                    return Err(malformed());
                }
                Value::Variant(Box::new(self.value_at(inner, depth + 1)?))
            }
            b'a' => self.array(&signature[1..], depth)?,
            b'(' => {
                self.align(8)?;
                let inner = &signature[1..complete_type(signature)? - 1];
                let fields = types(inner)?
                    .into_iter()
                    .map(|field| self.value_at(field, depth + 1))
                    .collect::<Result<_, _>>()?;
                Value::Struct(fields)
            }
            _ => return Err(malformed()),
        };
        Ok(value)
    }

    /// An array of elements of type `element`.
    fn array(&mut self, element: &[u8], depth: usize) -> Result<Value, Error> {
        let length = self.u32()? as usize;
        let first = *element.first().ok_or_else(malformed)?;
        self.align(alignment(first))?;
        let end = self.pos.checked_add(length).ok_or_else(malformed)?;
        if end > self.bytes.len() {
            return Err(malformed());
        }
        let mut items = Vec::new();
        while self.pos < end {
            let start = self.pos;
            let item = if first == b'{' {
                self.align(8)?;
                let key_length = complete_type(&element[1..])?;
                let key = self.value_at(&element[1..=key_length], depth + 1)?;
                let value = self.value_at(&element[1 + key_length..], depth + 1)?;
                Value::Entry(Box::new(key), Box::new(value))
            } else {
                self.value_at(element, depth + 1)?
            };
            if self.pos == start || self.pos > end {
                return Err(malformed());
            }
            items.push(item);
        }
        Ok(Value::Array(items))
    }
}

/// Writes values in little-endian order, aligned from the message's
/// start.
#[derive(Debug, Default)]
pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
}

impl Writer {
    /// Pads to a multiple of `alignment`.
    pub(crate) fn align(&mut self, alignment: usize) {
        let aligned = self.bytes.len().next_multiple_of(alignment);
        self.bytes.resize(aligned, 0);
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.align(4);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// A string or an object path (`s`, `o`).
    pub(crate) fn string(&mut self, text: &str) {
        self.u32(u32::try_from(text.len()).expect("strings fit D-Bus"));
        self.bytes.extend_from_slice(text.as_bytes());
        self.bytes.push(0);
    }

    /// A signature (`g`).
    pub(crate) fn signature(&mut self, signature: &str) {
        self.u8(u8::try_from(signature.len()).expect("signatures are short"));
        self.bytes.extend_from_slice(signature.as_bytes());
        self.bytes.push(0);
    }

    /// An array of strings (`as`).
    pub(crate) fn strings(&mut self, texts: &[&str]) {
        self.u32(0);
        let length_at = self.bytes.len() - 4;
        let start = self.bytes.len();
        for text in texts {
            self.string(text);
        }
        let length = u32::try_from(self.bytes.len() - start).expect("arrays fit D-Bus");
        self.bytes[length_at..start].copy_from_slice(&length.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_split_into_complete_types() {
        assert_eq!(
            types(b"sa{sv}(iu)v").unwrap(),
            [&b"s"[..], b"a{sv}", b"(iu)", b"v"]
        );
        assert_eq!(complete_type(b"a{sa{sv}}").unwrap(), 9);
        for bad in [
            &b"a"[..],
            b"(",
            b"()",
            b"a{vs}",
            b"{ss}",
            b"a{s}",
            b"z",
            b"(ii",
        ] {
            assert!(types(bad).is_err(), "{:?}", String::from_utf8_lossy(bad));
        }
        let deep = format!("{}y", "a".repeat(MAX_DEPTH + 2));
        assert!(types(deep.as_bytes()).is_err());
    }

    #[test]
    fn values_round_trip_through_their_wire_form() {
        let mut writer = Writer::default();
        writer.u8(7);
        writer.string("héllo");
        writer.signature("as");
        writer.strings(&["a", "bc"]);
        let mut reader = Reader::new(&writer.bytes, 0, false);
        assert_eq!(reader.value(b"y").unwrap(), Value::Byte(7));
        assert_eq!(reader.value(b"s").unwrap(), Value::Str("héllo".into()));
        assert_eq!(reader.value(b"g").unwrap(), Value::Str("as".into()));
        assert_eq!(
            reader.value(b"as").unwrap(),
            Value::Array(vec![Value::Str("a".into()), Value::Str("bc".into())])
        );
        assert_eq!(reader.pos(), writer.bytes.len());
    }

    #[test]
    fn dictionaries_of_variants_are_read_in_either_byte_order() {
        // a{sv} holding {"k": <"v">}, little- and big-endian.
        let little: &[u8] = &[
            18, 0, 0, 0, 0, 0, 0, 0, // length, padding to the entry
            1, 0, 0, 0, b'k', 0, // the key
            1, b's', 0, // the variant's signature
            0, 0, 0, // padding
            1, 0, 0, 0, b'v', 0, // the string
        ];
        let mut big = little.to_vec();
        big[..4].reverse();
        big[8..12].reverse();
        big[20..24].reverse();
        for (bytes, big_endian) in [(little, false), (&big[..], true)] {
            let value = Reader::new(bytes, 0, big_endian).value(b"a{sv}").unwrap();
            assert_eq!(value.get("k").and_then(Value::as_str), Some("v"));
            assert_eq!(value.get("x"), None);
        }
    }

    #[test]
    fn numbers_structures_and_nested_variants() {
        let bytes: &[u8] = &[
            1, 0, 0, 0, // b: true
            0xfe, 0xff, // n: -2
            0, 0, // padding
            42, 0, 0, 0, 0, 0, 0, 0, // t: 42
            1, b'v', 0, 1, b's', 0, 0, 0, 1, 0, 0, 0, b'x', 0, // v holding v holding "x"
        ];
        let mut reader = Reader::new(bytes, 0, false);
        assert_eq!(reader.value(b"b").unwrap(), Value::Bool(true));
        assert_eq!(reader.value(b"n").unwrap(), Value::Int(-2));
        assert_eq!(reader.value(b"t").unwrap(), Value::UInt(42));
        assert_eq!(reader.value(b"v").unwrap().as_str(), Some("x"));
        let pair: &[u8] = &[2, 0, 0, 0, 3, 0, 0, 0];
        assert_eq!(
            Reader::new(pair, 0, false).value(b"(iu)").unwrap(),
            Value::Struct(vec![Value::Int(2), Value::UInt(3)])
        );
    }

    #[test]
    fn broken_values_are_errors() {
        let cases: [(&[u8], &[u8]); 7] = [
            (&[5, 0, 0, 0, b'a', 0], b"s"),
            (&[1, 0, 0, 0, b'a', 1], b"s"),
            (&[2, 0, 0, 0], b"b"),
            (&[1, 0, 0, 0, 0xff, 0], b"s"),
            (&[0xff, 0xff, 0xff, 0xff], b"ay"),
            (&[1, b'z', 0], b"v"),
            (&[2, b's', b's', 0], b"v"),
        ];
        for (bytes, signature) in cases {
            assert!(
                Reader::new(bytes, 0, false).value(signature).is_err(),
                "{bytes:?}"
            );
        }
        // A variant nesting itself past the limit.
        let mut deep = Vec::new();
        for _ in 0..=MAX_DEPTH {
            deep.extend_from_slice(&[1, b'v', 0]);
        }
        assert!(Reader::new(&deep, 0, false).value(b"v").is_err());
    }
}
