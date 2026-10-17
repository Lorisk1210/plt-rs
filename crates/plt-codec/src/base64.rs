//! Base64 encoding (RFC 4648, standard alphabet with padding), as used to
//! embed images in HTML and SVG documents.

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
    }
}
