//! The DEFLATE decoder (RFC 1951 §3.2).

use super::bit_reader::BitReader;
use super::decode_table::{DecodeTable, Entry, Symbol};
use super::error::{HuffmanTable, InflateError};
use super::tables::{
    CODE_LENGTH_ORDER, CODE_LENGTH_SYMBOLS, DIST_SYMBOLS, END_OF_BLOCK, FIXED_DIST_LENGTHS,
    LITLEN_SYMBOLS, fixed_litlen_lengths,
};

/// Decompresses the DEFLATE stream at the start of `input`, appending at most
/// `limit` bytes to `out`. Returns the number of input bytes consumed.
pub(crate) fn inflate(
    input: &[u8],
    out: &mut Vec<u8>,
    limit: usize,
) -> Result<usize, InflateError> {
    Inflater::new(input, out, limit).run()
}

/// Which literal/length and distance codes the tables currently hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LoadedCodes {
    None,
    Fixed,
    Dynamic,
}

struct Inflater<'a, 'o> {
    bits: BitReader<'a>,
    out: &'o mut Vec<u8>,
    /// Length of `out` on entry: back-references may not reach before it.
    start: usize,
    /// The caller's limit on appended bytes.
    limit: usize,
    /// `out` may not grow beyond this length.
    end: usize,
    litlen: DecodeTable,
    dist: DecodeTable,
    code_lengths: DecodeTable,
    loaded: LoadedCodes,
}

impl<'a, 'o> Inflater<'a, 'o> {
    fn new(input: &'a [u8], out: &'o mut Vec<u8>, limit: usize) -> Self {
        let start = out.len();
        Self {
            bits: BitReader::new(input),
            out,
            start,
            limit,
            end: start.saturating_add(limit),
            litlen: DecodeTable::new(HuffmanTable::LiteralLength),
            dist: DecodeTable::new(HuffmanTable::Distance),
            code_lengths: DecodeTable::new(HuffmanTable::CodeLength),
            loaded: LoadedCodes::None,
        }
    }

    fn run(mut self) -> Result<usize, InflateError> {
        loop {
            let header = self.bits.read(3)?;
            match header >> 1 {
                0 => self.stored_block()?,
                1 => {
                    self.load_fixed_codes()?;
                    self.huffman_block()?;
                }
                2 => {
                    self.load_dynamic_codes()?;
                    self.huffman_block()?;
                }
                _ => return Err(InflateError::InvalidBlockType),
            }
            if header & 1 == 1 {
                return Ok(self.bits.consumed());
            }
        }
    }

    fn stored_block(&mut self) -> Result<(), InflateError> {
        self.bits.align_to_byte();
        let header = self.bits.read_bytes(4)?;
        let length = u16::from_le_bytes([header[0], header[1]]);
        let complement = u16::from_le_bytes([header[2], header[3]]);
        if length != !complement {
            return Err(InflateError::StoredLengthMismatch { length, complement });
        }
        let data = self.bits.read_bytes(usize::from(length))?;
        let fitting = self.fit_output(data.len());
        self.out.extend_from_slice(&data[..fitting]);
        self.check_fit(fitting, data.len())
    }

    fn load_fixed_codes(&mut self) -> Result<(), InflateError> {
        if self.loaded != LoadedCodes::Fixed {
            self.litlen.build(&fixed_litlen_lengths())?;
            self.dist.build(&FIXED_DIST_LENGTHS)?;
            self.loaded = LoadedCodes::Fixed;
        }
        Ok(())
    }

    /// Reads the code descriptions at the start of a dynamic block
    /// (RFC 1951 §3.2.7) and builds the decoding tables.
    fn load_dynamic_codes(&mut self) -> Result<(), InflateError> {
        self.loaded = LoadedCodes::Dynamic;
        let litlen_count = self.bits.read(5)? as usize + 257;
        let dist_count = self.bits.read(5)? as usize + 1;
        let code_length_count = self.bits.read(4)? as usize + 4;
        if litlen_count > LITLEN_SYMBOLS {
            return Err(InflateError::TooManyCodes {
                table: HuffmanTable::LiteralLength,
                count: litlen_count,
            });
        }
        if dist_count > DIST_SYMBOLS {
            return Err(InflateError::TooManyCodes {
                table: HuffmanTable::Distance,
                count: dist_count,
            });
        }

        let mut code_length_lengths = [0u8; CODE_LENGTH_SYMBOLS];
        for &symbol in &CODE_LENGTH_ORDER[..code_length_count] {
            code_length_lengths[symbol] = self.bits.read(3)? as u8;
        }
        self.code_lengths.build(&code_length_lengths)?;

        let total = litlen_count + dist_count;
        let mut lengths = [0u8; LITLEN_SYMBOLS + DIST_SYMBOLS];
        let mut filled = 0;
        while filled < total {
            let (value, run) = self.read_code_length_instruction(&lengths[..filled])?;
            let run_end = filled + run;
            if run_end > total {
                return Err(InflateError::CodeLengthOverflow);
            }
            lengths[filled..run_end].fill(value);
            filled = run_end;
        }
        if lengths[END_OF_BLOCK] == 0 {
            return Err(InflateError::MissingEndOfBlock);
        }
        self.litlen.build(&lengths[..litlen_count])?;
        self.dist.build(&lengths[litlen_count..total])
    }

    /// Decodes one code-length symbol into a `(length, repeat count)` pair.
    fn read_code_length_instruction(
        &mut self,
        previous: &[u8],
    ) -> Result<(u8, usize), InflateError> {
        self.bits.refill();
        let entry = self.code_lengths.lookup(self.bits.peek());
        let Symbol::Literal(symbol) = entry.symbol else {
            return Err(self.invalid_code(HuffmanTable::CodeLength, entry));
        };
        self.bits.consume(u32::from(entry.length))?;
        let (value, base, extra_bits) = match symbol {
            0..=15 => return Ok((symbol, 1)),
            16 => {
                let &last = previous.last().ok_or(InflateError::RepeatWithoutPrevious)?;
                (last, 3, 2)
            }
            17 => (0, 3, 3),
            _ => (0, 11, 7),
        };
        Ok((value, base + self.bits.take(extra_bits)? as usize))
    }

    /// Decodes literals and matches until the end-of-block code.
    fn huffman_block(&mut self) -> Result<(), InflateError> {
        loop {
            // A refill yields at least 56 bits unless the input is nearly
            // exhausted, enough for the longest length/distance sequence
            // (15 + 5 + 15 + 13 bits).
            self.bits.refill();
            let entry = self.litlen.lookup(self.bits.peek());
            match entry.symbol {
                Symbol::Literal(byte) => {
                    self.bits.consume(u32::from(entry.length))?;
                    self.check_fit(self.fit_output(1), 1)?;
                    self.out.push(byte);
                }
                Symbol::Base { base, extra } => {
                    self.bits.consume(u32::from(entry.length))?;
                    let length = usize::from(base) + self.bits.take(u32::from(extra))? as usize;
                    let distance = self.read_distance()?;
                    self.copy_match(length, distance)?;
                }
                Symbol::EndOfBlock => {
                    self.bits.consume(u32::from(entry.length))?;
                    return Ok(());
                }
                Symbol::Subtable { .. } | Symbol::Invalid => {
                    return Err(self.invalid_code(HuffmanTable::LiteralLength, entry));
                }
            }
        }
    }

    fn read_distance(&mut self) -> Result<usize, InflateError> {
        let entry = self.dist.lookup(self.bits.peek());
        let Symbol::Base { base, extra } = entry.symbol else {
            return Err(self.invalid_code(HuffmanTable::Distance, entry));
        };
        self.bits.consume(u32::from(entry.length))?;
        Ok(usize::from(base) + self.bits.take(u32::from(extra))? as usize)
    }

    /// Appends `length` bytes copied from `distance` bytes back.
    fn copy_match(&mut self, wanted: usize, distance: usize) -> Result<(), InflateError> {
        let available = self.out.len() - self.start;
        if distance > available {
            return Err(InflateError::DistanceTooFarBack {
                distance,
                available,
            });
        }
        let length = self.fit_output(wanted);
        let from = self.out.len() - distance;
        if distance >= length {
            self.out.extend_from_within(from..from + length);
        } else if distance == 1 {
            let byte = self.out[from];
            self.out.resize(self.out.len() + length, byte);
        } else {
            // Overlapping copy: the output from `from` on is periodic with
            // period `distance`, and stays so while whole periods are
            // appended, so copy ever larger chunks starting at `from`.
            let mut remaining = length;
            while remaining > 0 {
                let chunk = remaining.min(self.out.len() - from);
                self.out.extend_from_within(from..from + chunk);
                remaining -= chunk;
            }
        }
        self.check_fit(length, wanted)
    }

    /// How many of `count` further output bytes fit within the limit.
    fn fit_output(&self, count: usize) -> usize {
        count.min(self.end - self.out.len())
    }

    /// Fails once output had to be cut short: the caller then holds exactly
    /// the first `limit` bytes of the decompressed data.
    fn check_fit(&self, written: usize, wanted: usize) -> Result<(), InflateError> {
        if written < wanted {
            return Err(InflateError::OutputLimitExceeded { limit: self.limit });
        }
        Ok(())
    }

    /// The error for a lookup that hit `entry`, which is no valid code. If the
    /// input ran out before the bits that make the code invalid, this is
    /// reported as truncation rather than as a bad code made of padding.
    fn invalid_code(&self, table: HuffmanTable, entry: Entry) -> InflateError {
        if self.bits.is_short_of(u32::from(entry.length.max(1))) {
            InflateError::UnexpectedEof
        } else {
            InflateError::InvalidCode(table)
        }
    }
}
