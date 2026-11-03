//! The multi-symbol arithmetic coder of AV1 tile data.
//!
//! The specification describes the decoder (section 8.2, "Symbol
//! decoder"); [`SymbolEncoder`] is its inverse, the range encoder of
//! libaom's `entenc.c` with a carry buffer. A CDF of `N` symbols is an
//! array of `N` values: the first `N - 1` cumulative probabilities out of
//! 32768 (the last one, 32768, is implied) followed by the counter that
//! drives the adaptation of `read_symbol`. [`RateEstimate`] measures what
//! the same symbols would cost, for coding decisions.

/// Fifteen-bit probabilities: a CDF's total.
const PROB_TOP: u32 = 1 << 15;
/// `EC_PROB_SHIFT`: the probability bits the coder drops.
const PROB_SHIFT: u32 = 6;
/// `EC_MIN_PROB`: the least range each symbol keeps.
const MIN_PROB: u32 = 4;

/// Where coded symbols go: the arithmetic coder or a rate estimate.
pub(crate) trait SymbolSink {
    /// Codes `symbol` (below `N`) with `cdf`, adapting `cdf` when the
    /// sink writes the symbol.
    fn symbol<const N: usize>(&mut self, cdf: &mut [u16; N], symbol: usize);

    /// Codes one bit with probability `p0` (out of 32768) of being 0,
    /// without adaptation: the CDFs the specification builds on the fly.
    fn bool_with(&mut self, bit: bool, p0: u16);

    /// Codes the `bits` low bits of `value`, most significant first, each
    /// as an equiprobable bit (`read_literal`).
    fn literal(&mut self, value: u32, bits: u32) {
        for i in (0..bits).rev() {
            self.bool_with((value >> i) & 1 == 1, 1 << 14);
        }
    }
}

/// The interval of `symbol` in a CDF of `N` symbols, as libaom passes it
/// to the coder: 32768 minus the probabilities below and up to `symbol`.
fn interval<const N: usize>(cdf: &[u16; N], symbol: usize) -> (u32, u32) {
    let below = if symbol == 0 {
        0
    } else {
        u32::from(cdf[symbol - 1])
    };
    let upto = if symbol + 1 == N {
        PROB_TOP
    } else {
        u32::from(cdf[symbol])
    };
    (PROB_TOP - below, PROB_TOP - upto)
}

/// The adaptation of `read_symbol`: each probability moves towards the
/// coded symbol at a rate that slows over the first 32 symbols.
pub(crate) fn adapt<const N: usize>(cdf: &mut [u16; N], symbol: usize) {
    let count = cdf[N - 1];
    let rate = 3 + u32::from(count > 15) + u32::from(count > 31) + N.ilog2().min(2);
    for (i, p) in cdf[..N - 1].iter_mut().enumerate() {
        if i >= symbol {
            *p += (32768 - *p) >> rate;
        } else {
            *p -= *p >> rate;
        }
    }
    cdf[N - 1] += u16::from(count < 32);
}

/// The range encoder, writing a tile's symbols.
#[derive(Debug)]
pub(crate) struct SymbolEncoder {
    /// Output bytes before carry propagation: each entry holds a byte and
    /// the carry into the byte before it.
    precarry: Vec<u16>,
    /// The low end of the interval, with `count + 16` pending bits.
    low: u64,
    /// The width of the interval, kept in `[32768, 65535]`.
    range: u32,
    /// The pending bits of `low` beyond 16, offset by -9 so that it
    /// crosses zero once a byte and a carry bit have built up.
    count: i32,
}

impl SymbolEncoder {
    /// An encoder with nothing coded.
    pub(crate) fn new() -> Self {
        Self {
            precarry: Vec::new(),
            low: 0,
            range: 0x8000,
            count: -9,
        }
    }

    /// Narrows the interval to the one of `symbol` among `symbols`, given
    /// by `fl` and `fh` (see [`interval`]), as `od_ec_encode_q15` does.
    fn encode(&mut self, (fl, fh): (u32, u32), symbol: usize, symbols: usize) {
        let n = (symbols - 1) as u32;
        let s = symbol as u32;
        let r = self.range;
        let scaled = |f: u32| ((r >> 8) * (f >> PROB_SHIFT)) >> (7 - PROB_SHIFT);
        let v = scaled(fh) + MIN_PROB * (n - s);
        let (low, range) = if symbol > 0 {
            let u = scaled(fl) + MIN_PROB * (n + 1 - s);
            (self.low + u64::from(r - u), u - v)
        } else {
            (self.low, r - v)
        };
        self.normalize(low, range);
    }

    /// Renormalizes the interval to at least 32768 and moves whole bytes
    /// of `low` to the output.
    fn normalize(&mut self, mut low: u64, range: u32) {
        let d = 15 - range.ilog2() as i32;
        let mut c = self.count;
        let mut s = c + d;
        if s >= 0 {
            c += 16;
            let mut mask = (1u64 << c) - 1;
            if s >= 8 {
                self.precarry.push((low >> c) as u16);
                low &= mask;
                c -= 8;
                mask >>= 8;
            }
            self.precarry.push((low >> c) as u16);
            s = c + d - 24;
            low &= mask;
        }
        self.low = low << d;
        self.range = range << d;
        self.count = s;
    }

    /// The coded bytes: the fewest that decode to the symbols coded,
    /// whatever follows, ending in the trailing bit the specification's
    /// exit process expects (`od_ec_enc_done`).
    pub(crate) fn finish(mut self) -> Vec<u8> {
        let mut c = self.count;
        let mut s = c + 10;
        let mask = 0x3FFF;
        let mut e = ((self.low + mask) & !mask) | (mask + 1);
        if s > 0 {
            let mut n = (1u64 << (c + 16)) - 1;
            while s > 0 {
                self.precarry.push((e >> (c + 16)) as u16);
                e &= n;
                s -= 8;
                c -= 8;
                n >>= 8;
            }
        }
        let mut out = vec![0; self.precarry.len()];
        let mut carry = 0u32;
        for (byte, &entry) in out.iter_mut().zip(&self.precarry).rev() {
            carry += u32::from(entry);
            *byte = carry as u8;
            carry >>= 8;
        }
        out
    }
}

impl SymbolSink for SymbolEncoder {
    fn symbol<const N: usize>(&mut self, cdf: &mut [u16; N], symbol: usize) {
        self.encode(interval(cdf, symbol), symbol, N);
        adapt(cdf, symbol);
    }

    fn bool_with(&mut self, bit: bool, p0: u16) {
        let cdf = [p0, 0];
        let symbol = usize::from(bit);
        self.encode(interval(&cdf, symbol), symbol, 2);
    }
}

/// The cost of coded symbols in bits, from the probabilities of the CDFs
/// as they stand (they are not adapted).
#[derive(Debug, Default)]
pub(crate) struct RateEstimate {
    /// The bits counted so far.
    pub(crate) bits: f64,
}

impl RateEstimate {
    /// Counts the cost of a symbol with interval `(fl, fh)`: minus the
    /// base-2 logarithm of its probability.
    fn add(&mut self, (fl, fh): (u32, u32)) {
        let width = (fl - fh).max(1);
        self.bits += (f64::from(PROB_TOP) / f64::from(width)).log2();
    }
}

impl SymbolSink for RateEstimate {
    fn symbol<const N: usize>(&mut self, cdf: &mut [u16; N], symbol: usize) {
        self.add(interval(cdf, symbol));
    }

    fn bool_with(&mut self, bit: bool, p0: u16) {
        self.add(interval(&[p0, 0], usize::from(bit)));
    }

    fn literal(&mut self, _value: u32, bits: u32) {
        self.bits += f64::from(bits);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The symbol decoder of the specification (section 8.2.2 to 8.2.6),
    /// adapting its CDFs as `read_symbol` does.
    struct SpecDecoder<'a> {
        data: &'a [u8],
        bit_pos: usize,
        value: u32,
        range: u32,
        max_bits: i64,
    }

    impl<'a> SpecDecoder<'a> {
        fn new(data: &'a [u8]) -> Self {
            let mut decoder = Self {
                data,
                bit_pos: 0,
                value: 0,
                range: 1 << 15,
                max_bits: 8 * data.len() as i64 - 15,
            };
            let num_bits = (data.len() * 8).min(15);
            let buf = decoder.read_bits(num_bits);
            let padded = buf << (15 - num_bits);
            decoder.value = ((1 << 15) - 1) ^ padded;
            decoder
        }

        fn read_bits(&mut self, n: usize) -> u32 {
            let mut x = 0;
            for _ in 0..n {
                let byte = self.data.get(self.bit_pos / 8).copied().unwrap_or(0);
                x = (x << 1) | u32::from((byte >> (7 - self.bit_pos % 8)) & 1);
                self.bit_pos += 1;
            }
            x
        }

        /// `read_symbol` over a CDF in the encoder's layout.
        fn symbol<const N: usize>(&mut self, cdf: &mut [u16; N], adapt_cdf: bool) -> usize {
            let full = |i: usize| if i + 1 == N { 32768 } else { u32::from(cdf[i]) };
            let mut symbol = 0;
            let mut prev = self.range;
            let cur = loop {
                let f = (1 << 15) - full(symbol);
                let cur = (((self.range >> 8) * (f >> PROB_SHIFT)) >> (7 - PROB_SHIFT))
                    + MIN_PROB * (N - symbol - 1) as u32;
                if self.value >= cur {
                    break cur;
                }
                prev = cur;
                symbol += 1;
            };
            self.range = prev - cur;
            self.value -= cur;
            let bits = 15 - self.range.ilog2() as usize;
            self.range <<= bits;
            let num_bits = bits.min(self.max_bits.max(0) as usize);
            let new_data = self.read_bits(num_bits);
            let padded = new_data << (bits - num_bits);
            self.value = padded ^ (((self.value + 1) << bits) - 1);
            self.max_bits -= bits as i64;
            if adapt_cdf {
                adapt(cdf, symbol);
            }
            symbol
        }

        /// The exit process: the trailing one bit and zero padding.
        fn trailing_ok(&self) -> bool {
            let padding_end = self.bit_pos + self.max_bits.max(0) as usize;
            let trailing = self.bit_pos - (self.max_bits + 15).min(15) as usize;
            let bit = |pos: usize| (self.data[pos / 8] >> (7 - pos % 8)) & 1;
            padding_end == self.data.len() * 8
                && bit(trailing) == 1
                && (trailing + 1..padding_end).all(|pos| bit(pos) == 0)
        }
    }

    /// `SplitMix64`, for reproducible symbol streams.
    fn next(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    #[test]
    fn adaptation_follows_the_specification() {
        // Symbol 1 of a fresh 4-symbol CDF: rate 3 + 0 + 0 + 2 = 5.
        let mut cdf = [8192, 16384, 24576, 0];
        adapt(&mut cdf, 1);
        assert_eq!(cdf, [8192 - 256, 16384 + 512, 24576 + 256, 1]);
        // The counter saturates at 32, where the rate is 3 + 2 + 1 for 2
        // symbols.
        let mut cdf = [16384, 32];
        adapt(&mut cdf, 0);
        assert_eq!(cdf, [16384 + 256, 32]);
    }

    #[test]
    fn a_single_equiprobable_bit() {
        // A 0: cur = ((32768 >> 8) * (16384 >> 6) >> 1) + 4 = 16388, so
        // the range becomes 32768 - 16388 = 16380 and low stays 0. The
        // range doubles twice (count -7); the flush rounds low up to a
        // multiple of 2^14, sets bit 14 and writes 0x4000 >> 9.
        let mut encoder = SymbolEncoder::new();
        encoder.bool_with(false, 1 << 14);
        assert_eq!(encoder.finish(), [0b0010_0000]);
        // A 1: low = 32768 - 16388 = 16380 with range 16388, doubled once
        // (count -8): (32760 + 0x3FFF) & !0x3FFF | 0x4000 = 0xC000, of
        // which the flush writes the top byte.
        let mut encoder = SymbolEncoder::new();
        encoder.bool_with(true, 1 << 14);
        assert_eq!(encoder.finish(), [0b1100_0000]);
        // Nothing coded: just the trailing bit.
        assert_eq!(SymbolEncoder::new().finish(), [0b1000_0000]);
    }

    #[test]
    fn the_specification_decoder_reads_what_is_written() {
        let mut state = 7;
        for length in [0, 1, 2, 5, 40, 1000] {
            let mut cdfs = ([4000, 9000, 20000, 31000, 0], [16384, 0], [100, 32000, 0]);
            let mut encoder = SymbolEncoder::new();
            let mut symbols = Vec::new();
            for _ in 0..length {
                let roll = next(&mut state);
                let kind = roll % 4;
                let symbol = (roll >> 8) as usize;
                let value = match kind {
                    0 => {
                        let s = symbol % 5;
                        encoder.symbol(&mut cdfs.0, s);
                        s
                    }
                    1 => {
                        let s = symbol % 2;
                        encoder.symbol(&mut cdfs.1, s);
                        s
                    }
                    2 => {
                        let s = symbol % 3;
                        encoder.symbol(&mut cdfs.2, s);
                        s
                    }
                    _ => {
                        let v = symbol % 64;
                        encoder.literal(v as u32, 6);
                        v
                    }
                };
                symbols.push((kind, value));
            }
            let data = encoder.finish();
            let mut cdfs = ([4000, 9000, 20000, 31000, 0], [16384, 0], [100, 32000, 0]);
            let mut decoder = SpecDecoder::new(&data);
            for &(kind, value) in &symbols {
                let decoded = match kind {
                    0 => decoder.symbol(&mut cdfs.0, true),
                    1 => decoder.symbol(&mut cdfs.1, true),
                    2 => decoder.symbol(&mut cdfs.2, true),
                    _ => (0..6).fold(0, |x, _| 2 * x + decoder.symbol(&mut [16384, 0], false)),
                };
                assert_eq!(decoded, value, "length {length}");
            }
            if !data.is_empty() {
                assert!(decoder.trailing_ok(), "length {length}");
            }
        }
    }

    #[test]
    fn rates_follow_probabilities() {
        let mut estimate = RateEstimate::default();
        estimate.bool_with(false, 1 << 14);
        assert!((estimate.bits - 1.0).abs() < 1e-9);
        let mut cdf = [8192, 0];
        estimate.symbol(&mut cdf, 1);
        assert!((estimate.bits - (1.0 + (4.0f64 / 3.0).log2())).abs() < 0.01);
        assert_eq!(cdf, [8192, 0]);
        estimate.literal(5, 3);
        assert!((estimate.bits - 4.415).abs() < 0.01);
    }
}
