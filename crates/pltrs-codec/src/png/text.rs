//! Text chunks: `tEXt` (Latin-1), `zTXt` (compressed Latin-1) and `iTXt`
//! (UTF-8, optionally compressed).

use super::chunk::{ChunkType, MAX_CHUNK_LEN};
use super::error::EncodeError;
use super::metadata::TextEntry;
use crate::zlib;

/// Longest keyword PNG allows, in bytes.
const MAX_KEYWORD_LEN: usize = 79;

/// Cap on decompressed text, against decompression bombs.
const MAX_TEXT_LEN: usize = 1 << 24;

/// Encodes `entry` as `tEXt` if its text is Latin-1, as `iTXt` otherwise
/// (the same choice Pillow makes).
pub(crate) fn encode(entry: &TextEntry) -> Result<(ChunkType, Vec<u8>), EncodeError> {
    let mut data = encode_keyword(&entry.keyword)
        .ok_or_else(|| EncodeError::InvalidTextKeyword(entry.keyword.clone()))?;
    let invalid_value = || EncodeError::InvalidTextValue {
        keyword: entry.keyword.clone(),
    };
    if entry.text.contains('\0') {
        return Err(invalid_value());
    }
    data.push(0);
    let kind = if let Some(latin1) = to_latin1(&entry.text) {
        data.extend(latin1);
        ChunkType::TEXT
    } else {
        // Uncompressed, no language tag, no translated keyword.
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(entry.text.as_bytes());
        ChunkType::ITXT
    };
    if data.len() > MAX_CHUNK_LEN as usize {
        return Err(invalid_value());
    }
    Ok((kind, data))
}

/// The Latin-1 bytes of `keyword` if it is a valid PNG keyword: 1–79
/// printable Latin-1 characters without leading, trailing or consecutive
/// spaces.
fn encode_keyword(keyword: &str) -> Option<Vec<u8>> {
    let bytes = to_latin1(keyword)?;
    let printable = bytes.iter().all(|&b| matches!(b, 32..=126 | 161..=255));
    let well_spaced = !bytes.starts_with(b" ")
        && !bytes.ends_with(b" ")
        && !bytes.windows(2).any(|pair| pair == b"  ");
    let sized = (1..=MAX_KEYWORD_LEN).contains(&bytes.len());
    (printable && well_spaced && sized).then_some(bytes)
}

fn to_latin1(text: &str) -> Option<Vec<u8>> {
    text.chars().map(|c| u8::try_from(c).ok()).collect()
}

fn from_latin1(bytes: &[u8]) -> String {
    bytes.iter().copied().map(char::from).collect()
}

/// Decodes a `tEXt`, `zTXt` or `iTXt` payload. Malformed chunks yield `None`
/// and are skipped, like other damaged ancillary data.
pub(crate) fn decode(kind: ChunkType, data: &[u8]) -> Option<TextEntry> {
    let (keyword, rest) = split_at_nul(data)?;
    if !(1..=MAX_KEYWORD_LEN).contains(&keyword.len()) {
        return None;
    }
    let text = match kind {
        ChunkType::TEXT => from_latin1(rest),
        ChunkType::ZTXT => {
            let (&method, compressed) = rest.split_first()?;
            if method != 0 {
                return None;
            }
            from_latin1(&inflate_text(compressed)?)
        }
        ChunkType::ITXT => decode_international(rest)?,
        _ => return None,
    };
    Some(TextEntry {
        keyword: from_latin1(keyword),
        text,
    })
}

/// Decodes the part of an `iTXt` payload after the keyword. The language tag
/// and translated keyword are not kept.
fn decode_international(data: &[u8]) -> Option<String> {
    let (&[compressed, method], rest) = data.split_first_chunk::<2>()?;
    let (_language, rest) = split_at_nul(rest)?;
    let (_translated_keyword, text) = split_at_nul(rest)?;
    let text = match (compressed, method) {
        (0, _) => text.to_vec(),
        (1, 0) => inflate_text(text)?,
        _ => return None,
    };
    String::from_utf8(text).ok()
}

fn inflate_text(data: &[u8]) -> Option<Vec<u8>> {
    zlib::decompress_with_limit(data, MAX_TEXT_LEN).ok()
}

fn split_at_nul(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let nul = data.iter().position(|&byte| byte == 0)?;
    Some((&data[..nul], &data[nul + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompressionLevel;

    #[test]
    fn latin1_text_becomes_text_chunk() {
        let entry = TextEntry::new(
            "Software",
            "Matplotlib version3.11, https://matplotlib.org/ é",
        );
        let (kind, data) = encode(&entry).unwrap();
        assert_eq!(kind, ChunkType::TEXT);
        assert!(data.starts_with(b"Software\0Matplotlib"));
        assert_eq!(*data.last().unwrap(), 0xE9);
        assert_eq!(decode(kind, &data), Some(entry));
    }

    #[test]
    fn other_text_becomes_international_chunk() {
        let entry = TextEntry::new("Title", "σ² ≈ 1 — 日本");
        let (kind, data) = encode(&entry).unwrap();
        assert_eq!(kind, ChunkType::ITXT);
        assert!(data.starts_with(b"Title\0\0\0\0\0"));
        assert_eq!(decode(kind, &data), Some(entry));
    }

    #[test]
    fn compressed_chunks_decode() {
        let mut ztxt = b"Comment\0\0".to_vec();
        ztxt.extend(zlib::compress(b"caf\xE9 au lait", CompressionLevel::DEFAULT).unwrap());
        assert_eq!(
            decode(ChunkType::ZTXT, &ztxt),
            Some(TextEntry::new("Comment", "café au lait"))
        );
        let mut itxt = b"Title\0\x01\x00en\0Titel\0".to_vec();
        itxt.extend(zlib::compress("Ünïcödé".as_bytes(), CompressionLevel::BEST).unwrap());
        assert_eq!(
            decode(ChunkType::ITXT, &itxt),
            Some(TextEntry::new("Title", "Ünïcödé"))
        );
    }

    #[test]
    fn invalid_keywords_and_values_are_rejected() {
        let long = "k".repeat(80);
        for keyword in [
            "",
            " lead",
            "trail ",
            "two  spaces",
            "tab\there",
            "€uro",
            long.as_str(),
        ] {
            assert_eq!(
                encode(&TextEntry::new(keyword, "x")),
                Err(EncodeError::InvalidTextKeyword(keyword.to_owned())),
                "{keyword:?}"
            );
        }
        assert!(encode(&TextEntry::new("k".repeat(79), "x")).is_ok());
        assert_eq!(
            encode(&TextEntry::new("Key", "a\0b")),
            Err(EncodeError::InvalidTextValue {
                keyword: "Key".to_owned()
            })
        );
    }

    #[test]
    fn malformed_chunks_are_skipped() {
        assert_eq!(decode(ChunkType::TEXT, b"no separator"), None);
        assert_eq!(decode(ChunkType::TEXT, b"\0empty keyword"), None);
        assert_eq!(decode(ChunkType::ZTXT, b"Key\0\x00not zlib"), None);
        assert_eq!(decode(ChunkType::ZTXT, b"Key\0\x07"), None);
        assert_eq!(decode(ChunkType::ITXT, b"Key\0\x00\x00en"), None);
        assert_eq!(decode(ChunkType::ITXT, b"Key\0\x00\x00\0\0\xFF\xFE"), None);
        assert_eq!(decode(ChunkType::ITXT, b"Key\0\x02\x00\0\0text"), None);
        assert_eq!(decode(ChunkType::IDAT, b"Key\0text"), None);
    }
}
