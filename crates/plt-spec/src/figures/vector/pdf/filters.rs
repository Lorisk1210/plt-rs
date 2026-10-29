//! The stream filters of the documents compared: `FlateDecode`, and the
//! `ASCII85Decode` and `LZWDecode` that Ghostscript's `ps2write` device
//! applies to the PDF streams it writes into PostScript.

/// `ASCII85Decode`: groups of five characters `!`–`u` are four bytes, `z`
/// four zeros; whitespace is skipped and `~>` ends the data.
pub(super) fn ascii85(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len() * 4 / 5);
    let mut group = [0u8; 5];
    let mut len = 0;
    for &byte in data {
        match byte {
            b'~' => break,
            b'z' if len == 0 => out.extend_from_slice(&[0; 4]),
            b'!'..=b'u' => {
                group[len] = byte - b'!';
                len += 1;
                if len == 5 {
                    out.extend_from_slice(&group_value(group)?.to_be_bytes());
                    len = 0;
                }
            }
            byte if byte.is_ascii_whitespace() => {}
            _ => return None,
        }
    }
    if len == 1 {
        return None;
    }
    if len > 1 {
        // A final partial group is padded with `u` and cut back.
        group[len..].fill(b'u' - b'!');
        out.extend_from_slice(&group_value(group)?.to_be_bytes()[..len - 1]);
    }
    Some(out)
}

/// The value of five base-85 digits, `None` beyond 32 bits.
fn group_value(group: [u8; 5]) -> Option<u32> {
    let value = group
        .iter()
        .fold(0u64, |value, &digit| value * 85 + u64::from(digit));
    u32::try_from(value).ok()
}

/// The code clearing the LZW table.
const CLEAR: usize = 256;
/// The code ending LZW data.
const END: usize = 257;
/// The widest LZW code.
const MAX_BITS: u32 = 12;

/// `LZWDecode` with its default `EarlyChange` of 1: codes of 9 to 12 bits,
/// most significant bit first, a code wider as soon as the next table
/// entry needs it.
pub(super) fn lzw(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut table: Vec<Vec<u8>> = Vec::new();
    let reset = |table: &mut Vec<Vec<u8>>| {
        table.clear();
        table.extend((0..=255u8).map(|byte| vec![byte]));
        table.extend([Vec::new(), Vec::new()]);
    };
    reset(&mut table);
    let mut width = 9;
    let (mut buffer, mut bits) = (0u32, 0u32);
    let mut previous: Option<Vec<u8>> = None;
    for &byte in data {
        buffer = buffer << 8 | u32::from(byte);
        bits += 8;
        while bits >= width {
            bits -= width;
            let code = (buffer >> bits) as usize & ((1 << width) - 1);
            buffer &= (1 << bits) - 1;
            match code {
                CLEAR => {
                    reset(&mut table);
                    width = 9;
                    previous = None;
                    continue;
                }
                END => return Some(out),
                _ => {}
            }
            let entry = match (table.get(code), &previous) {
                (Some(entry), _) => entry.clone(),
                (None, Some(last)) if code == table.len() => {
                    let mut entry = last.clone();
                    entry.push(last[0]);
                    entry
                }
                _ => return None,
            };
            out.extend_from_slice(&entry);
            if let Some(mut last) = previous.take() {
                last.push(entry[0]);
                table.push(last);
            }
            if table.len() + 1 >= 1 << width && width < MAX_BITS {
                width += 1;
            }
            previous = Some(entry);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii85_decodes_groups_zeros_and_partial_groups() {
        assert_eq!(ascii85(b"87cURD]i,\"Ebo80~>").unwrap(), b"Hello World!");
        assert_eq!(ascii85(b"z\n!!~>").unwrap(), [0, 0, 0, 0, 0]);
        assert_eq!(ascii85(b"!~>"), None, "a lone digit");
        assert_eq!(ascii85(b"uuuuu~>"), None, "beyond 32 bits");
        assert_eq!(ascii85(b"v"), None);
    }

    #[test]
    fn lzw_decodes_the_pdf_reference_example() {
        // PDF 32000-1, 7.4.4.2: "-----A---B" encoded with EarlyChange 1.
        let encoded = [0x80, 0x0B, 0x60, 0x50, 0x22, 0x0C, 0x0C, 0x85, 0x01];
        assert_eq!(lzw(&encoded).unwrap(), b"-----A---B");
        assert_eq!(lzw(&[0xFF, 0xFF]), None, "a code beyond the table");
        assert_eq!(lzw(&[]).unwrap(), b"");
    }
}
