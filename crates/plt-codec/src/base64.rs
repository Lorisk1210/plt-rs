//! Base64 (RFC 4648, standard alphabet with padding), as used to embed
//! images in HTML and SVG documents: encoding, and decoding.

/// The standard alphabet.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `data` in base64 on one line.
///
/// ```
/// assert_eq!(plt_codec::base64::encode(b"plt"), "cGx0");
/// assert_eq!(plt_codec::base64::encode(b"pl"), "cGw=");
/// ```
pub fn encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let bytes = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = u32::from(bytes[0]) << 16 | u32::from(bytes[1]) << 8 | u32::from(bytes[2]);
        let symbols =
            [18, 12, 6, 0].map(|shift| char::from(ALPHABET[((n >> shift) & 63) as usize]));
        let kept = chunk.len() + 1;
        out.extend(&symbols[..kept]);
        out.extend(std::iter::repeat_n('=', 4 - kept));
    }
    out
}

/// `data` in base64 in lines of 76 characters, each ending with a newline
/// (Python's `base64.encodebytes`, MIME style).
///
/// ```
/// let text = plt_codec::base64::encode_lines(&[0; 60]);
/// assert_eq!(text.lines().map(str::len).collect::<Vec<_>>(), [76, 4]);
/// assert!(text.ends_with('\n'));
/// ```
pub fn encode_lines(data: &[u8]) -> String {
    // 57 bytes make 76 characters.
    data.chunks(57).map(|chunk| encode(chunk) + "\n").collect()
}

/// Why text is not valid base64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base64Error {
    /// A character outside the alphabet, padding and line breaks, at this
    /// byte offset.
    InvalidCharacter(usize),
    /// The characters do not form whole groups of four, or padding is
    /// misplaced.
    InvalidLength,
}

impl std::fmt::Display for Base64Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCharacter(at) => write!(f, "invalid base64 character at offset {at}"),
            Self::InvalidLength => f.write_str("base64 text is not a whole number of groups"),
        }
    }
}

impl std::error::Error for Base64Error {}

/// The bytes of base64 `text` (standard alphabet, with padding). Line
/// breaks (`\n`, `\r`) are skipped, as in [`encode_lines`]'s output.
///
/// ```
/// assert_eq!(plt_codec::base64::decode("cGx0"), Ok(b"plt".to_vec()));
/// assert_eq!(plt_codec::base64::decode("cG\nw="), Ok(b"pl".to_vec()));
/// assert!(plt_codec::base64::decode("cGw").is_err());
/// ```
///
/// # Errors
///
/// When `text` holds other characters, or is not made of whole groups of
/// four characters with padding only at the end.
pub fn decode(text: &str) -> Result<Vec<u8>, Base64Error> {
    let mut symbols = Vec::with_capacity(text.len());
    for (at, byte) in text.bytes().enumerate() {
        match byte {
            b'\n' | b'\r' => {}
            b'=' => symbols.push(None),
            _ => {
                let value = ALPHABET
                    .iter()
                    .position(|&c| c == byte)
                    .ok_or(Base64Error::InvalidCharacter(at))?;
                symbols.push(Some(u32::try_from(value).expect("below 64")));
            }
        }
    }
    if symbols.len() % 4 != 0 {
        return Err(Base64Error::InvalidLength);
    }
    let groups = symbols.len() / 4;
    let mut out = Vec::with_capacity(groups * 3);
    for (index, group) in symbols.as_chunks::<4>().0.iter().enumerate() {
        let kept = group.iter().take_while(|s| s.is_some()).count();
        let padded_properly = group[kept..].iter().all(Option::is_none);
        if kept < 2 || !padded_properly || (kept < 4 && index + 1 != groups) {
            return Err(Base64Error::InvalidLength);
        }
        let n = group
            .iter()
            .fold(0, |n, symbol| n << 6 | symbol.unwrap_or(0));
        out.extend_from_slice(&n.to_be_bytes()[1..kept]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc_4648_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (plain, coded) in cases {
            assert_eq!(encode(plain.as_bytes()), coded);
        }
        assert_eq!(encode(&[0xff, 0xfe]), "//4=");
        assert_eq!(encode_lines(b""), "");
        for (plain, coded) in cases {
            assert_eq!(decode(coded), Ok(plain.as_bytes().to_vec()));
        }
    }

    #[test]
    fn decoding_reverses_encoding() {
        let data: Vec<u8> = (0..=255)
            .chain((0..100_u32).map(|i| (i * 7 % 256) as u8))
            .collect();
        for len in 0..data.len() {
            assert_eq!(decode(&encode(&data[..len])).as_deref(), Ok(&data[..len]));
        }
        assert_eq!(decode(&encode_lines(&data)), Ok(data));
    }

    #[test]
    fn bad_text_is_rejected() {
        assert_eq!(decode("ab$d"), Err(Base64Error::InvalidCharacter(2)));
        assert_eq!(decode("abc"), Err(Base64Error::InvalidLength));
        assert_eq!(decode("a==="), Err(Base64Error::InvalidLength));
        assert_eq!(decode("ab=c"), Err(Base64Error::InvalidLength));
        assert_eq!(decode("ab==cdef"), Err(Base64Error::InvalidLength));
    }
}
