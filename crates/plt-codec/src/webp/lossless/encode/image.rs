//! Entropy-coded images: the pixel stream as literals, colour-cache hits
//! and backward references, under one group of five prefix codes.

use super::backward::{Token, tokens};
use super::codes::PrefixCode;
use crate::deflate::bit_writer::BitWriter;
use crate::webp::lossless::format::{DISTANCE_CODES, LENGTH_CODES, LITERALS, cache_index};

/// The colour-cache sizes tried, in bits (0: no cache).
const CACHE_BITS: [u32; 6] = [0, 3, 5, 7, 9, 10];

/// A symbol of the green alphabet and what follows it.
#[derive(Clone, Copy, Debug)]
enum Symbol {
    /// A pixel's channels.
    Literal(u32),
    /// A colour-cache index.
    Cache(usize),
    /// A copy: its length and distance codes.
    Copy { length: usize, distance: usize },
}

/// The prefix coding of a length or distance value of at least 1: its
/// symbol, then `bits` extra bits of `extra`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Prefixed {
    symbol: usize,
    bits: u32,
    extra: u32,
}

/// The inverse of the decoder's `copy_amount`.
fn prefixed(value: usize) -> Prefixed {
    if value <= 4 {
        return Prefixed {
            symbol: value - 1,
            bits: 0,
            extra: 0,
        };
    }
    let offset = value - 1;
    let high = offset.ilog2();
    let bits = high - 1;
    Prefixed {
        symbol: 2 * high as usize + ((offset >> bits) & 1),
        bits,
        extra: (offset & ((1 << bits) - 1)) as u32,
    }
}

/// The symbols of `tokens` with a colour cache of `1 << cache_bits`
/// entries (none for 0): literal pixels found in the cache become cache
/// hits; every pixel passes through the cache.
fn symbols(tokens: &[Token], pixels: &[u32], cache_bits: u32) -> Vec<Symbol> {
    let mut cache = vec![None; if cache_bits > 0 { 1 << cache_bits } else { 0 }];
    let mut position = 0;
    let mut out = Vec::with_capacity(tokens.len());
    for &token in tokens {
        out.push(match token {
            Token::Literal(argb)
                if cache_bits > 0 && cache[cache_index(argb, cache_bits)] == Some(argb) =>
            {
                Symbol::Cache(cache_index(argb, cache_bits))
            }
            Token::Literal(argb) => Symbol::Literal(argb),
            Token::Copy {
                length,
                distance_code,
            } => Symbol::Copy {
                length,
                distance: distance_code,
            },
        });
        if cache_bits > 0 {
            for &argb in &pixels[position..position + token.len()] {
                cache[cache_index(argb, cache_bits)] = Some(argb);
            }
        }
        position += token.len();
    }
    out
}

/// The histograms of the five alphabets, and the extra bits of copies.
#[derive(Debug)]
struct Histograms {
    green: Vec<u32>,
    red: Vec<u32>,
    blue: Vec<u32>,
    alpha: Vec<u32>,
    distance: Vec<u32>,
    extra_bits: u64,
}

impl Histograms {
    fn of(symbols: &[Symbol], cache_size: usize) -> Self {
        let mut histograms = Self {
            green: vec![0; LITERALS + LENGTH_CODES + cache_size],
            red: vec![0; LITERALS],
            blue: vec![0; LITERALS],
            alpha: vec![0; LITERALS],
            distance: vec![0; DISTANCE_CODES],
            extra_bits: 0,
        };
        for &symbol in symbols {
            match symbol {
                Symbol::Literal(argb) => {
                    let [a, r, g, b] = argb.to_be_bytes();
                    histograms.green[usize::from(g)] += 1;
                    histograms.red[usize::from(r)] += 1;
                    histograms.blue[usize::from(b)] += 1;
                    histograms.alpha[usize::from(a)] += 1;
                }
                Symbol::Cache(index) => histograms.green[LITERALS + LENGTH_CODES + index] += 1,
                Symbol::Copy { length, distance } => {
                    let (length, distance) = (prefixed(length), prefixed(distance));
                    histograms.green[LITERALS + length.symbol] += 1;
                    histograms.distance[distance.symbol] += 1;
                    histograms.extra_bits += u64::from(length.bits + distance.bits);
                }
            }
        }
        histograms
    }

    /// An estimate of the coded size in bits: the Shannon entropy of each
    /// alphabet plus the extra bits.
    fn entropy(&self) -> f64 {
        let entropy = |histogram: &[u32]| {
            let total: f64 = histogram.iter().map(|&c| f64::from(c)).sum();
            histogram
                .iter()
                .filter(|&&count| count > 0)
                .map(|&count| {
                    let count = f64::from(count);
                    -count * (count / total).log2()
                })
                .sum::<f64>()
        };
        [
            &self.green,
            &self.red,
            &self.blue,
            &self.alpha,
            &self.distance,
        ]
        .into_iter()
        .map(|histogram| entropy(histogram))
        .sum::<f64>()
            + self.extra_bits as f64
    }
}

/// Writes an entropy-coded image of rows of `width` pixels. The main
/// image (`main`) says it uses a single group of codes; images inside
/// transforms cannot use groups and say nothing.
pub(super) fn write_image(writer: &mut BitWriter<'_>, pixels: &[u32], width: usize, main: bool) {
    let tokens = tokens(pixels, width);
    let (cache_bits, symbols, histograms) = CACHE_BITS
        .iter()
        .map(|&bits| {
            let symbols = symbols(&tokens, pixels, bits);
            let cache_size = if bits > 0 { 1 << bits } else { 0 };
            let histograms = Histograms::of(&symbols, cache_size);
            (bits, symbols, histograms)
        })
        .min_by(|a, b| a.2.entropy().total_cmp(&b.2.entropy()))
        .expect("several cache sizes are tried");
    if cache_bits > 0 {
        writer.write_bits(1, 1);
        writer.write_bits(cache_bits, 4);
    } else {
        writer.write_bits(0, 1);
    }
    if main {
        writer.write_bits(0, 1);
    }
    let codes = [
        &histograms.green,
        &histograms.red,
        &histograms.blue,
        &histograms.alpha,
        &histograms.distance,
    ]
    .map(|histogram| {
        let code = PrefixCode::for_histogram(histogram);
        code.write_description(writer, histogram.len());
        code
    });
    let [green, red, blue, alpha, distance] = &codes;
    for symbol in symbols {
        match symbol {
            Symbol::Literal(argb) => {
                let [a, r, g, b] = argb.to_be_bytes();
                green.write(writer, usize::from(g));
                red.write(writer, usize::from(r));
                blue.write(writer, usize::from(b));
                alpha.write(writer, usize::from(a));
            }
            Symbol::Cache(index) => green.write(writer, LITERALS + LENGTH_CODES + index),
            Symbol::Copy {
                length,
                distance: code,
            } => {
                let length = prefixed(length);
                green.write(writer, LITERALS + length.symbol);
                writer.write_bits(length.extra, length.bits);
                let code = prefixed(code);
                distance.write(writer, code.symbol);
                writer.write_bits(code.extra, code.bits);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes_invert_the_decoder() {
        // The decoder's reading of a symbol and its extra bits.
        let value = |p: Prefixed| {
            if p.symbol < 4 {
                p.symbol + 1
            } else {
                let bits = (p.symbol - 2) >> 1;
                ((2 + (p.symbol & 1)) << bits) + p.extra as usize + 1
            }
        };
        for v in (1..5000).chain([4096, 1 << 20]) {
            let p = prefixed(v);
            assert_eq!(value(p), v, "{p:?}");
            assert_eq!(
                p.bits as usize,
                if p.symbol < 4 { 0 } else { (p.symbol - 2) >> 1 }
            );
        }
        assert_eq!(prefixed(4096).symbol, LENGTH_CODES - 1, "the longest copy");
        assert_eq!(
            prefixed(1 << 20).symbol,
            DISTANCE_CODES - 1,
            "the farthest distance"
        );
    }

    #[test]
    fn repeated_literals_become_cache_hits() {
        let pixels = [7, 9, 7, 9];
        let tokens: Vec<Token> = pixels.iter().map(|&p| Token::Literal(p)).collect();
        let found = symbols(&tokens, &pixels, 4);
        assert!(matches!(
            found[..2],
            [Symbol::Literal(7), Symbol::Literal(9)]
        ));
        assert!(matches!(found[2..], [Symbol::Cache(_), Symbol::Cache(_)]));
        assert!(
            symbols(&tokens, &pixels, 0)
                .iter()
                .all(|s| matches!(s, Symbol::Literal(_)))
        );
    }
}
