//! The PFA and PFB containers and the split of a Type 1 font into its
//! cleartext part, its encrypted part and the closing zeros, after
//! matplotlib's `Type1Font._read` and `Type1Font._split`.

use super::error::Type1Error;
use super::python::{index, slice, slice_from};

/// The three parts of a Type 1 font as matplotlib's `Type1Font.parts`
/// holds them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Type1Parts {
    /// The cleartext part, up to and including `eexec` and the white
    /// space after it.
    pub cleartext: Vec<u8>,
    /// The encrypted part, in binary.
    pub encrypted: Vec<u8>,
    /// The closing 512 zeros, `cleartomark` and whatever follows.
    pub finale: Vec<u8>,
}

/// PFB segment types.
const ASCII_SEGMENT: u8 = 1;
const BINARY_SEGMENT: u8 = 2;
const END_SEGMENT: u8 = 3;

/// The font program of a PFA or PFB file: a PFA file as is, the
/// segments of a PFB file concatenated, binary ones in lowercase
/// hexadecimal.
pub(crate) fn decode(raw: &[u8]) -> Result<Vec<u8>, Type1Error> {
    if raw.first() != Some(&0x80) {
        return Ok(raw.to_vec());
    }
    let mut data = Vec::new();
    let mut rest = raw;
    while let Some(&marker) = rest.first() {
        if marker != 0x80 {
            return Err(Type1Error::BrokenPfb(marker));
        }
        let kind = *rest.get(1).ok_or(Type1Error::BadSegmentHeader)?;
        if kind == END_SEGMENT {
            break;
        }
        if kind != ASCII_SEGMENT && kind != BINARY_SEGMENT {
            return Err(Type1Error::UnknownSegment(kind));
        }
        let length = rest
            .get(2..6)
            .and_then(|bytes| bytes.try_into().ok())
            .map(i32::from_le_bytes)
            .ok_or(Type1Error::BadSegmentHeader)?;
        let length = usize::try_from(length).map_err(|_| Type1Error::BadSegmentHeader)?;
        let end = 6usize.saturating_add(length).min(rest.len());
        let segment = &rest[6.min(rest.len())..end];
        if kind == ASCII_SEGMENT {
            data.extend_from_slice(segment);
        } else {
            data.extend(segment.iter().flat_map(|&b| hex_pair(b)));
        }
        rest = &rest[end..];
    }
    Ok(data)
}

/// The lowercase hexadecimal digits of a byte.
fn hex_pair(byte: u8) -> [u8; 2] {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    [
        DIGITS[usize::from(byte >> 4)],
        DIGITS[usize::from(byte & 15)],
    ]
}

/// The value of a hexadecimal digit.
const fn hex_value(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

/// `binascii.unhexlify`: an even number of hexadecimal digits, without
/// white space.
pub(crate) fn unhexlify(text: &[u8]) -> Option<Vec<u8>> {
    let (pairs, rest) = text.as_chunks::<2>();
    if !rest.is_empty() {
        return None;
    }
    pairs
        .iter()
        .map(|&[high, low]| Some(hex_value(high)? << 4 | hex_value(low)?))
        .collect()
}

/// The offset of the first occurrence of `needle`.
fn find(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len())
        .position(|window| window == needle)
}

/// The offset of the last occurrence of `needle`.
fn rfind(data: &[u8], needle: &[u8]) -> Option<usize> {
    data.windows(needle.len())
        .rposition(|window| window == needle)
}

/// Splits a font program into its three parts: the cleartext up to the
/// white space after `eexec`, the encrypted part converted from
/// hexadecimal, and everything from the zeros before `cleartomark` on.
pub(crate) fn split(data: &[u8]) -> Result<Type1Parts, Type1Error> {
    let byte = |at: i64| {
        index(data.len(), at)
            .map(|i| data[i])
            .ok_or(Type1Error::TruncatedParts)
    };
    let mut at = find(data, b"eexec").ok_or(Type1Error::MissingEexec)? as i64 + 5;
    while matches!(byte(at)?, b' ' | b'\t' | b'\r' | b'\n') {
        at += 1;
    }
    let len1 = at;
    let mut at = rfind(data, b"cleartomark").ok_or(Type1Error::MissingCleartomark)? as i64 - 1;
    let mut zeros = 512;
    loop {
        let current = byte(at)?;
        let zero = current == b'0';
        if !((zeros > 0 && zero) || matches!(current, b'\r' | b'\n')) {
            break;
        }
        if zero {
            zeros -= 1;
        }
        at -= 1;
    }
    // An even number of digits, as matplotlib computes it.
    let end1 = len1 + ((at - len1 + 2) & !1);
    let encrypted =
        unhexlify(&data[slice(data.len(), len1, end1)]).ok_or(Type1Error::InvalidHex)?;
    Ok(Type1Parts {
        cleartext: data[slice(data.len(), 0, len1)].to_vec(),
        encrypted,
        finale: data[slice_from(data.len(), at + 1)..].to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PFB segment of `kind` holding `body`.
    fn segment(kind: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![0x80, kind];
        out.extend((body.len() as i32).to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn pfb_segments_are_concatenated() {
        let mut pfb = segment(1, b"head eexec\n");
        pfb.extend(segment(2, &[0xAB, 0x01]));
        pfb.extend(segment(1, b"\n0000cleartomark\n"));
        pfb.extend([0x80, 3]);
        assert_eq!(
            decode(&pfb).unwrap(),
            b"head eexec\nab01\n0000cleartomark\n".to_vec()
        );
        assert_eq!(decode(b"%!PS").unwrap(), b"%!PS".to_vec());
        assert_eq!(decode(&[0x80, 1, 9, 0, 0, 0, b'a']).unwrap(), b"a".to_vec());
    }

    #[test]
    fn broken_pfb_files() {
        assert_eq!(decode(&[0x80]), Err(Type1Error::BadSegmentHeader));
        assert_eq!(decode(&[0x80, 7]), Err(Type1Error::UnknownSegment(7)));
        assert_eq!(decode(&[0x80, 1, 1, 0]), Err(Type1Error::BadSegmentHeader));
        assert_eq!(
            decode(&[0x80, 1, 0xFF, 0xFF, 0xFF, 0xFF]),
            Err(Type1Error::BadSegmentHeader)
        );
        let mut two = segment(1, b"x");
        two.push(0x41);
        assert_eq!(decode(&two), Err(Type1Error::BrokenPfb(0x41)));
    }

    #[test]
    fn parts_are_split_around_the_encrypted_hex() {
        let font = format!("cleartext eexec \r\nabcd\n{}cleartomark\n", "0".repeat(512));
        let parts = split(font.as_bytes()).unwrap();
        assert_eq!(parts.cleartext, b"cleartext eexec \r\n".to_vec());
        assert_eq!(parts.encrypted, vec![0xAB, 0xCD]);
        assert!(parts.finale.starts_with(b"\n000"));
        // Too few zeros: the walk stops at the hex digits regardless.
        let short = "x eexec ab\n00cleartomark";
        assert_eq!(split(short.as_bytes()).unwrap().encrypted, vec![0xAB]);
    }

    #[test]
    fn missing_markers_and_bad_hex() {
        assert_eq!(split(b"no markers"), Err(Type1Error::MissingEexec));
        assert_eq!(split(b"eexec"), Err(Type1Error::TruncatedParts));
        assert_eq!(split(b"eexec x"), Err(Type1Error::MissingCleartomark));
        assert_eq!(split(b"eexec zz cleartomark"), Err(Type1Error::InvalidHex));
        assert_eq!(unhexlify(b"a"), None);
        assert_eq!(unhexlify(b"0aFf"), Some(vec![10, 255]));
    }
}
