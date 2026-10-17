//! Decoding of image and shading data into comparable numbers: images as
//! their size and mean channel values, shadings as their vertices.

use super::syntax::{Document, Value};
use crate::figures::vector::diff::Token;

/// PNG's Paeth predictor.
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Undoes PNG row filtering (`/Predictor 10` and up).
fn unpredict(data: &[u8], row_bytes: usize, bpp: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    let mut previous = vec![0u8; row_bytes];
    for row in data.chunks(row_bytes + 1) {
        let (&filter, raw) = row.split_first().unwrap_or((&0, &[]));
        let mut current = vec![0u8; row_bytes];
        for (i, &byte) in raw.iter().enumerate().take(row_bytes) {
            let left = if i >= bpp { current[i - bpp] } else { 0 };
            let up = previous[i];
            let corner = if i >= bpp { previous[i - bpp] } else { 0 };
            current[i] = byte.wrapping_add(match filter {
                1 => left,
                2 => up,
                3 => left.midpoint(up),
                4 => paeth(left, up, corner),
                _ => 0,
            });
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    out
}

/// Unpacks `bits`-bit samples, rows starting on byte boundaries.
fn unpack(data: &[u8], width: usize, height: usize, channels: usize, bits: usize) -> Vec<u8> {
    if bits == 8 {
        return data.to_vec();
    }
    let row_bytes = (width * channels * bits).div_ceil(8);
    let mut out = Vec::with_capacity(width * height * channels);
    for row in data.chunks(row_bytes.max(1)).take(height) {
        for i in 0..width * channels {
            let bit = i * bits;
            let byte = row.get(bit / 8).copied().unwrap_or(0);
            let shift = 8 - bits - bit % 8;
            out.push((byte >> shift) & ((1 << bits) - 1) as u8);
        }
    }
    out
}

/// Size and mean value of every channel of decoded samples.
pub(super) fn sample_tokens(
    samples: &[u8],
    width: usize,
    height: usize,
    channels: usize,
) -> Vec<Token> {
    let mut tokens = vec![
        Token::Num(width as f64),
        Token::Num(height as f64),
        Token::Num(channels as f64),
    ];
    let pixels = (width * height).max(1) as f64;
    for channel in 0..channels {
        let sum: f64 = samples
            .iter()
            .skip(channel)
            .step_by(channels)
            .map(|&v| f64::from(v))
            .sum();
        tokens.push(Token::Num(sum / pixels));
    }
    tokens
}

/// An image `XObject` as its size and mean colour (indexed images through
/// their palette), plus its soft mask.
pub(super) fn image_tokens(doc: &Document, reference: &Value) -> Vec<Token> {
    let dict = doc.resolve(reference);
    let num = |key: &str| dict.get(key).and_then(Value::as_num).unwrap_or(0.0) as usize;
    let (width, height, bits) = (num("Width"), num("Height"), num("BitsPerComponent"));
    let Some(data) = doc.stream(reference) else {
        return vec![Token::word("unreadable-image")];
    };
    let space = doc.resolve(dict.get("ColorSpace").unwrap_or(&Value::Null));
    let (channels, palette) = match space {
        Value::Name(name) if name == "DeviceGray" => (1, None),
        Value::Array(items) => match items.get(3) {
            Some(Value::Str(table)) => (1, Some(table.clone())),
            _ => (3, None),
        },
        _ => (3, None),
    };
    let predicted = dict
        .get("DecodeParms")
        .and_then(|p| p.get("Predictor"))
        .and_then(Value::as_num)
        .is_some_and(|p| p >= 10.0);
    let data = if predicted {
        let row_bytes = (width * channels * bits).div_ceil(8);
        unpredict(&data, row_bytes, (channels * bits).div_ceil(8))
    } else {
        data
    };
    let samples = unpack(&data, width, height, channels, bits);
    let mut tokens = vec![Token::word("image")];
    match palette {
        Some(table) => {
            let rgb: Vec<u8> = samples
                .iter()
                .flat_map(|&i| {
                    let at = usize::from(i) * 3;
                    table
                        .get(at..at + 3)
                        .map_or([0, 0, 0], |c| [c[0], c[1], c[2]])
                })
                .collect();
            tokens.extend(sample_tokens(&rgb, width, height, 3));
        }
        None => tokens.extend(sample_tokens(&samples, width, height, channels)),
    }
    if let Some(mask) = dict.get("SMask") {
        tokens.push(Token::word("smask"));
        tokens.extend(image_tokens(doc, mask));
    }
    tokens
}

/// A free-form triangle shading as its colour space and decoded vertices
/// mapped through `ctm` (`[a b c d e f]`).
pub(super) fn shading_tokens(doc: &Document, reference: &Value, ctm: &[f64; 6]) -> Vec<Token> {
    let dict = doc.resolve(reference);
    let mut tokens = vec![Token::word("shading")];
    for key in ["ShadingType", "ColorSpace"] {
        if let Some(value) = dict.get(key) {
            tokens.push(match value {
                Value::Num(n) => Token::Num(*n),
                Value::Name(n) => Token::word(n.clone()),
                _ => Token::word("?"),
            });
        }
    }
    let decode: Vec<f64> = dict
        .get("Decode")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_num).collect())
        .unwrap_or_default();
    let channels = decode.len().saturating_sub(4) / 2;
    let Some(data) = doc.stream(reference) else {
        return tokens;
    };
    let span = |i: usize| (decode[2 * i], decode[2 * i + 1]);
    for vertex in data.chunks_exact(9 + channels) {
        let coord = |at: usize, axis: usize| {
            let raw =
                u32::from_be_bytes([vertex[at], vertex[at + 1], vertex[at + 2], vertex[at + 3]]);
            let (lo, hi) = span(axis);
            lo + f64::from(raw) * (hi - lo) / f64::from(u32::MAX)
        };
        if decode.len() >= 4 {
            let (x, y) = (coord(1, 0), coord(5, 1));
            tokens.push(Token::Num(ctm[0] * x + ctm[2] * y + ctm[4]));
            tokens.push(Token::Num(ctm[1] * x + ctm[3] * y + ctm[5]));
        }
        tokens.extend(vertex[9..].iter().map(|&c| Token::Num(f64::from(c))));
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_predictors_and_bit_unpacking() {
        // Two rows of two RGB pixels: Sub then Up filtered.
        let data = [1, 10, 20, 30, 5, 5, 5, 2, 1, 1, 1, 0, 0, 0];
        assert_eq!(
            unpredict(&data, 6, 3),
            [10, 20, 30, 15, 25, 35, 11, 21, 31, 15, 25, 35]
        );
        assert_eq!(unpack(&[0b1011_0000], 3, 1, 1, 2), [2, 3, 0]);
        assert_eq!(paeth(10, 20, 15), 15);
        let tokens = sample_tokens(&[0, 10, 20, 30], 2, 1, 2);
        assert_eq!(tokens[3], Token::Num(10.0));
        assert_eq!(tokens[4], Token::Num(20.0));
    }
}
