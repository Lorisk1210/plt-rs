//! Block assembly: buffering LZ77 tokens and emitting each block as stored,
//! fixed-Huffman or dynamic-Huffman, whichever is smallest.

use super::bit_writer::BitWriter;
use super::code_lengths::{CodeLengthOp, run_length_ops};
use super::huffman::HuffmanCode;
use super::tables::{
    CODE_LENGTH_ORDER, CODE_LENGTH_SYMBOLS, DIST_BASE, DIST_EXTRA, DIST_SYMBOLS, END_OF_BLOCK,
    FIXED_DIST_LENGTHS, LENGTH_BASE, LENGTH_EXTRA, LITLEN_SYMBOLS, MAX_CODE_BITS,
    MAX_CODE_LENGTH_BITS, dist_index, fixed_litlen_lengths, length_index,
};

/// Number of tokens after which a block is flushed (zlib's default with
/// `memLevel` 8).
const BLOCK_TOKENS: usize = 1 << 14;

/// Largest payload of one stored block.
const MAX_STORED: usize = 65_535;

/// One LZ77 decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Token {
    Literal(u8),
    Match { length: u16, distance: u16 },
}

/// Collects tokens from a parser and writes them out block by block.
#[derive(Debug)]
pub(crate) struct BlockSink<'d, 'w, 'o> {
    data: &'d [u8],
    writer: &'w mut BitWriter<'o>,
    tokens: Vec<Token>,
    /// First input byte covered by the buffered tokens.
    block_start: usize,
    /// End of the input bytes covered by the buffered tokens.
    covered: usize,
}

impl<'d, 'w, 'o> BlockSink<'d, 'w, 'o> {
    pub(crate) fn new(data: &'d [u8], writer: &'w mut BitWriter<'o>) -> Self {
        Self {
            data,
            writer,
            tokens: Vec::with_capacity(BLOCK_TOKENS),
            block_start: 0,
            covered: 0,
        }
    }

    /// Emits the literal byte at the current position.
    pub(crate) fn literal(&mut self) {
        self.make_room();
        self.tokens.push(Token::Literal(self.data[self.covered]));
        self.covered += 1;
    }

    /// Emits a back-reference covering the next `length` bytes.
    pub(crate) fn matched(&mut self, length: usize, distance: usize) {
        self.make_room();
        self.tokens.push(Token::Match {
            length: length as u16,
            distance: distance as u16,
        });
        self.covered += length;
    }

    /// Writes the remaining tokens as the final block.
    pub(crate) fn finish(mut self) {
        debug_assert_eq!(self.covered, self.data.len(), "parser left input uncovered");
        self.flush(true);
    }

    /// Flushes a full buffer as a non-final block. Flushing lazily, only
    /// when another token arrives, guarantees the final block is never empty
    /// unless the whole input is.
    fn make_room(&mut self) {
        if self.tokens.len() == BLOCK_TOKENS {
            self.flush(false);
        }
    }

    fn flush(&mut self, is_final: bool) {
        let raw = &self.data[self.block_start..self.covered];
        write_block(self.writer, &self.tokens, raw, is_final);
        self.tokens.clear();
        self.block_start = self.covered;
    }
}

/// Writes `tokens` (which encode `raw`) as one block of the cheapest type.
fn write_block(writer: &mut BitWriter<'_>, tokens: &[Token], raw: &[u8], is_final: bool) {
    let freqs = Frequencies::of(tokens);
    let extra = freqs.extra_bits();
    let fixed = BlockCodes::fixed();
    let fixed_bits = 3 + fixed.cost(&freqs) + extra;
    let dynamic = BlockCodes::optimal(&freqs);
    let header = DynamicHeader::new(&dynamic);
    let dynamic_bits = 3 + header.bits() + dynamic.cost(&freqs) + extra;
    if stored_bits(raw.len(), writer.bit_offset()) <= fixed_bits.min(dynamic_bits) {
        write_stored(writer, raw, is_final);
    } else if fixed_bits <= dynamic_bits {
        writer.write_bits(u32::from(is_final) | 0b010, 3);
        write_tokens(writer, tokens, &fixed);
    } else {
        writer.write_bits(u32::from(is_final) | 0b100, 3);
        header.write(writer);
        write_tokens(writer, tokens, &dynamic);
    }
}

/// Writes `raw` as stored blocks of at most 65 535 bytes (a single empty one
/// if `raw` is empty); only the last can be final.
pub(crate) fn write_stored(writer: &mut BitWriter<'_>, raw: &[u8], is_final: bool) {
    let mut rest = raw;
    loop {
        let (chunk, tail) = rest.split_at(rest.len().min(MAX_STORED));
        let last = tail.is_empty();
        writer.write_bits(u32::from(is_final && last), 3);
        writer.align_to_byte();
        let length = chunk.len() as u16;
        writer.write_aligned_bytes(&length.to_le_bytes());
        writer.write_aligned_bytes(&(!length).to_le_bytes());
        writer.write_aligned_bytes(chunk);
        if last {
            return;
        }
        rest = tail;
    }
}

/// Size in bits of `len` bytes written as stored blocks when the output is
/// `bit_offset` bits into a byte.
fn stored_bits(len: usize, bit_offset: u32) -> u64 {
    let blocks = len.div_ceil(MAX_STORED).max(1) as u64;
    let first_header = u64::from((bit_offset + 3).next_multiple_of(8) - bit_offset);
    first_header + (blocks - 1) * 8 + blocks * 32 + 8 * len as u64
}

fn write_tokens(writer: &mut BitWriter<'_>, tokens: &[Token], codes: &BlockCodes) {
    let litlen = &codes.litlen;
    let dist = &codes.dist;
    for &token in tokens {
        match token {
            Token::Literal(byte) => {
                let symbol = usize::from(byte);
                writer.write_bits(
                    u32::from(litlen.codes[symbol]),
                    u32::from(litlen.lengths[symbol]),
                );
            }
            Token::Match { length, distance } => {
                let (length, distance) = (usize::from(length), usize::from(distance));
                let index = length_index(length);
                let symbol = 257 + index;
                let code_length = u32::from(litlen.lengths[symbol]);
                let extra = (length - usize::from(LENGTH_BASE[index])) as u32;
                writer.write_bits(
                    u32::from(litlen.codes[symbol]) | extra << code_length,
                    code_length + u32::from(LENGTH_EXTRA[index]),
                );
                let index = dist_index(distance);
                let code_length = u32::from(dist.lengths[index]);
                let extra = (distance - usize::from(DIST_BASE[index])) as u32;
                writer.write_bits(
                    u32::from(dist.codes[index]) | extra << code_length,
                    code_length + u32::from(DIST_EXTRA[index]),
                );
            }
        }
    }
    writer.write_bits(
        u32::from(litlen.codes[END_OF_BLOCK]),
        u32::from(litlen.lengths[END_OF_BLOCK]),
    );
}

/// Symbol frequencies of one block.
struct Frequencies {
    litlen: [u32; LITLEN_SYMBOLS],
    dist: [u32; DIST_SYMBOLS],
}

impl Frequencies {
    fn of(tokens: &[Token]) -> Self {
        let mut freqs = Self {
            litlen: [0; LITLEN_SYMBOLS],
            dist: [0; DIST_SYMBOLS],
        };
        for &token in tokens {
            match token {
                Token::Literal(byte) => freqs.litlen[usize::from(byte)] += 1,
                Token::Match { length, distance } => {
                    freqs.litlen[257 + length_index(usize::from(length))] += 1;
                    freqs.dist[dist_index(usize::from(distance))] += 1;
                }
            }
        }
        freqs.litlen[END_OF_BLOCK] = 1;
        freqs
    }

    /// Bits taken by the extra fields of lengths and distances, which do not
    /// depend on the Huffman codes.
    fn extra_bits(&self) -> u64 {
        let length_extra = self.litlen[257..]
            .iter()
            .zip(LENGTH_EXTRA)
            .map(|(&freq, extra)| u64::from(freq) * u64::from(extra));
        let dist_extra = self
            .dist
            .iter()
            .zip(DIST_EXTRA)
            .map(|(&freq, extra)| u64::from(freq) * u64::from(extra));
        length_extra.chain(dist_extra).sum()
    }
}

/// The literal/length and distance codes of a Huffman block.
struct BlockCodes {
    litlen: HuffmanCode,
    dist: HuffmanCode,
}

impl BlockCodes {
    fn fixed() -> Self {
        Self {
            litlen: HuffmanCode::from_lengths(fixed_litlen_lengths().to_vec()),
            dist: HuffmanCode::from_lengths(FIXED_DIST_LENGTHS.to_vec()),
        }
    }

    fn optimal(freqs: &Frequencies) -> Self {
        Self {
            litlen: HuffmanCode::optimal(&freqs.litlen, MAX_CODE_BITS),
            dist: HuffmanCode::optimal(&freqs.dist, MAX_CODE_BITS),
        }
    }

    /// Bits for the Huffman codes of all symbols, excluding extra fields.
    fn cost(&self, freqs: &Frequencies) -> u64 {
        self.litlen.cost(&freqs.litlen) + self.dist.cost(&freqs.dist)
    }
}

/// The header of a dynamic block: code counts and the run-length encoded,
/// Huffman-coded code lengths (RFC 1951 §3.2.7).
struct DynamicHeader {
    litlen_count: usize,
    dist_count: usize,
    code_length_count: usize,
    ops: Vec<CodeLengthOp>,
    code: HuffmanCode,
}

impl DynamicHeader {
    fn new(codes: &BlockCodes) -> Self {
        let litlen_count = used_prefix(&codes.litlen.lengths, 257);
        let dist_count = used_prefix(&codes.dist.lengths, 1);
        let lengths: Vec<u8> = codes.litlen.lengths[..litlen_count]
            .iter()
            .chain(&codes.dist.lengths[..dist_count])
            .copied()
            .collect();
        let ops = run_length_ops(&lengths);
        let mut freqs = [0u32; CODE_LENGTH_SYMBOLS];
        for op in &ops {
            freqs[usize::from(op.symbol)] += 1;
        }
        let code = HuffmanCode::optimal(&freqs, MAX_CODE_LENGTH_BITS);
        let code_length_count = CODE_LENGTH_ORDER
            .iter()
            .rposition(|&symbol| code.lengths[symbol] != 0)
            .map_or(4, |last| (last + 1).max(4));
        Self {
            litlen_count,
            dist_count,
            code_length_count,
            ops,
            code,
        }
    }

    fn bits(&self) -> u64 {
        let ops: u64 = self
            .ops
            .iter()
            .map(|&op| {
                u64::from(self.code.lengths[usize::from(op.symbol)]) + u64::from(op.extra_bits())
            })
            .sum();
        5 + 5 + 4 + 3 * self.code_length_count as u64 + ops
    }

    fn write(&self, writer: &mut BitWriter<'_>) {
        writer.write_bits((self.litlen_count - 257) as u32, 5);
        writer.write_bits((self.dist_count - 1) as u32, 5);
        writer.write_bits((self.code_length_count - 4) as u32, 4);
        for &symbol in &CODE_LENGTH_ORDER[..self.code_length_count] {
            writer.write_bits(u32::from(self.code.lengths[symbol]), 3);
        }
        for &op in &self.ops {
            let symbol = usize::from(op.symbol);
            writer.write_bits(
                u32::from(self.code.codes[symbol]),
                u32::from(self.code.lengths[symbol]),
            );
            writer.write_bits(u32::from(op.extra), op.extra_bits());
        }
    }
}

/// Number of leading code lengths to transmit: up to the last non-zero one,
/// but at least `minimum`.
fn used_prefix(lengths: &[u8], minimum: usize) -> usize {
    lengths
        .iter()
        .rposition(|&length| length != 0)
        .map_or(minimum, |last| (last + 1).max(minimum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_cost_counts_headers_and_alignment() {
        assert_eq!(stored_bits(0, 0), 8 + 32);
        assert_eq!(stored_bits(0, 5), 3 + 32);
        assert_eq!(stored_bits(0, 6), 10 + 32);
        assert_eq!(stored_bits(10, 0), 8 + 32 + 80);
        assert_eq!(
            stored_bits(MAX_STORED + 1, 0),
            2 * (8 + 32) + 8 * (MAX_STORED as u64 + 1)
        );
    }
}
