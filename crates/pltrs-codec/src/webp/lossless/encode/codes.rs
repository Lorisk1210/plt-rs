//! Prefix codes for histograms, and their descriptions in the bitstream:
//! "simple" codes of one or two symbols, or run-length coded code lengths
//! under a code-length code.

use crate::deflate::bit_writer::BitWriter;
use crate::deflate::code_lengths::run_length_ops;
use crate::deflate::huffman::HuffmanCode;
use crate::webp::lossless::format::{CODE_LENGTH_CODES, CODE_LENGTH_ORDER};

/// The longest code of an image alphabet.
const MAX_BITS: usize = 15;
/// The longest code of the code-length alphabet.
const MAX_CODE_LENGTH_BITS: usize = 7;

/// The code of one alphabet.
#[derive(Clone, Debug)]
pub(super) enum PrefixCode {
    /// A single symbol (or none at all), coded with no bits.
    Single(usize),
    /// A Huffman code of two or more symbols.
    Huffman(HuffmanCode),
}

impl PrefixCode {
    /// The optimal code for `histogram`.
    pub(super) fn for_histogram(histogram: &[u32]) -> Self {
        let mut used = histogram
            .iter()
            .enumerate()
            .filter(|&(_, &count)| count > 0);
        match (used.next(), used.next()) {
            (None, _) => Self::Single(0),
            (Some((symbol, _)), None) => Self::Single(symbol),
            _ => Self::Huffman(HuffmanCode::optimal(histogram, MAX_BITS)),
        }
    }

    /// Writes `symbol`'s code.
    pub(super) fn write(&self, writer: &mut BitWriter<'_>, symbol: usize) {
        if let Self::Huffman(code) = self {
            writer.write_bits(
                u32::from(code.codes[symbol]),
                u32::from(code.lengths[symbol]),
            );
        }
    }

    /// Writes the description of the code over `alphabet` symbols.
    pub(super) fn write_description(&self, writer: &mut BitWriter<'_>, alphabet: usize) {
        match self {
            Self::Single(symbol) if *symbol < 256 => write_simple(writer, &[*symbol]),
            Self::Single(symbol) => {
                let mut lengths = vec![0u8; alphabet];
                lengths[*symbol] = 1;
                write_lengths(writer, &lengths);
            }
            Self::Huffman(code) => {
                let used: Vec<usize> = (0..alphabet).filter(|&s| code.lengths[s] > 0).collect();
                if used.len() == 2 && used[1] < 256 {
                    write_simple(writer, &used);
                } else {
                    write_lengths(writer, &code.lengths[..alphabet]);
                }
            }
        }
    }
}

/// A simple code of one or two symbols below 256 (the first symbol in
/// one bit when it is 0 or 1).
fn write_simple(writer: &mut BitWriter<'_>, symbols: &[usize]) {
    writer.write_bits(1, 1);
    writer.write_bits(symbols.len() as u32 - 1, 1);
    let first = symbols[0] as u32;
    if first < 2 {
        writer.write_bits(0, 1);
        writer.write_bits(first, 1);
    } else {
        writer.write_bits(1, 1);
        writer.write_bits(first, 8);
    }
    if let Some(&second) = symbols.get(1) {
        writer.write_bits(second as u32, 8);
    }
}

/// Code lengths, run-length coded under an optimal code-length code.
fn write_lengths(writer: &mut BitWriter<'_>, lengths: &[u8]) {
    let ops = run_length_ops(lengths);
    let mut frequencies = [0u32; CODE_LENGTH_CODES];
    for op in &ops {
        frequencies[usize::from(op.symbol)] += 1;
    }
    let code = HuffmanCode::optimal(&frequencies, MAX_CODE_LENGTH_BITS);
    let stored = CODE_LENGTH_ORDER
        .iter()
        .rposition(|&symbol| code.lengths[symbol] != 0)
        .map_or(4, |last| (last + 1).max(4));
    writer.write_bits(0, 1);
    writer.write_bits(stored as u32 - 4, 4);
    for &symbol in &CODE_LENGTH_ORDER[..stored] {
        writer.write_bits(u32::from(code.lengths[symbol]), 3);
    }
    // Every length is listed: no early stop.
    writer.write_bits(0, 1);
    for op in ops {
        let symbol = usize::from(op.symbol);
        writer.write_bits(
            u32::from(code.codes[symbol]),
            u32::from(code.lengths[symbol]),
        );
        writer.write_bits(u32::from(op.extra), op.extra_bits());
    }
}
