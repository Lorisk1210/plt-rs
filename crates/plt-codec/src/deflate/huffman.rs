//! Length-limited canonical Huffman codes for the encoder.
//!
//! Code lengths come from the package-merge algorithm (Larmore & Hirschberg),
//! which yields an optimal prefix code under a maximum-length constraint —
//! DEFLATE caps literal/length and distance codes at 15 bits and code-length
//! codes at 7.

/// A canonical prefix code.
#[derive(Clone, Debug)]
pub(crate) struct HuffmanCode {
    /// Code length per symbol; 0 for symbols without a code.
    pub(crate) lengths: Vec<u8>,
    /// Code per symbol, bit-reversed so it can be written LSB-first.
    pub(crate) codes: Vec<u16>,
}

impl HuffmanCode {
    /// The optimal code for `freqs` with no code longer than `max_bits`.
    ///
    /// The code is always complete and has at least two symbols: when fewer
    /// than two symbols occur, one-bit codes are handed to the occurring
    /// symbol (if any) and a neighbour, because decoders such as zlib reject
    /// degenerate codes.
    pub(crate) fn optimal(freqs: &[u32], max_bits: usize) -> Self {
        Self::from_lengths(limited_lengths(freqs, max_bits))
    }

    /// The canonical code with the given lengths (RFC 1951 §3.2.2).
    pub(crate) fn from_lengths(lengths: Vec<u8>) -> Self {
        let mut counts = [0u32; 16];
        for &length in &lengths {
            counts[usize::from(length)] += 1;
        }
        counts[0] = 0;
        let mut next_code = [0u32; 16];
        let mut code = 0;
        for (length, next) in next_code.iter_mut().enumerate().skip(1) {
            code = (code + counts[length - 1]) << 1;
            *next = code;
        }
        let codes = lengths
            .iter()
            .map(|&length| {
                if length == 0 {
                    return 0;
                }
                let next = &mut next_code[usize::from(length)];
                let reversed = next.reverse_bits() >> (32 - u32::from(length));
                *next += 1;
                reversed as u16
            })
            .collect();
        Self { lengths, codes }
    }

    /// Total bits needed to encode symbols with frequencies `freqs`.
    pub(crate) fn cost(&self, freqs: &[u32]) -> u64 {
        freqs
            .iter()
            .zip(&self.lengths)
            .map(|(&freq, &length)| u64::from(freq) * u64::from(length))
            .sum()
    }
}

/// Optimal code lengths no longer than `max_bits` (package-merge).
///
/// The symbols that occur are leaves sorted by weight. Each of the
/// `max_bits` levels holds the leaves merged with "packages" (adjacent pairs)
/// of the level below; the code length of a leaf is the number of times it
/// is used by the cheapest `2n - 2` items of the top level, which is found by
/// walking back down and counting.
fn limited_lengths(freqs: &[u32], max_bits: usize) -> Vec<u8> {
    let mut lengths = vec![0; freqs.len()];
    let mut leaves: Vec<(u32, usize)> = freqs
        .iter()
        .enumerate()
        .filter(|&(_, &freq)| freq > 0)
        .map(|(symbol, &freq)| (freq, symbol))
        .collect();
    if leaves.len() < 2 {
        let used = leaves.first().map_or(0, |&(_, symbol)| symbol);
        let partner = usize::from(used == 0);
        lengths[used] = 1;
        lengths[partner] = 1;
        return lengths;
    }
    leaves.sort_unstable();
    debug_assert!(
        leaves.len() <= 1 << max_bits,
        "too many symbols for max_bits"
    );

    let leaf_weights: Vec<u64> = leaves.iter().map(|&(freq, _)| u64::from(freq)).collect();
    // `levels[d]` flags which items of the merged list at depth
    // `max_bits - d` are packages rather than leaves.
    let mut levels = vec![vec![false; leaves.len()]];
    let mut weights = leaf_weights.clone();
    for _ in 1..max_bits {
        let (pairs, _) = weights.as_chunks::<2>();
        let packages: Vec<u64> = pairs.iter().map(|&[a, b]| a + b).collect();
        let (merged, is_package) = merge(&leaf_weights, &packages);
        weights = merged;
        levels.push(is_package);
    }

    let mut selected = 2 * leaves.len() - 2;
    for is_package in levels.iter().rev() {
        let packages = is_package[..selected].iter().filter(|&&p| p).count();
        for &(_, symbol) in &leaves[..selected - packages] {
            lengths[symbol] += 1;
        }
        selected = 2 * packages;
    }
    lengths
}

/// Merges two ascending weight lists, preferring leaves on ties; returns the
/// merged weights and, per item, whether it is a package.
fn merge(leaves: &[u64], packages: &[u64]) -> (Vec<u64>, Vec<bool>) {
    let total = leaves.len() + packages.len();
    let mut weights = Vec::with_capacity(total);
    let mut is_package = Vec::with_capacity(total);
    let (mut leaves, mut packages) = (leaves.iter().peekable(), packages.iter().peekable());
    loop {
        let take_package = match (leaves.peek(), packages.peek()) {
            (Some(&&leaf), Some(&&package)) => package < leaf,
            (None, Some(_)) => true,
            (Some(_), None) => false,
            (None, None) => break,
        };
        let next = if take_package {
            packages.next()
        } else {
            leaves.next()
        };
        weights.extend(next);
        is_package.push(take_package);
    }
    (weights, is_package)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;

    /// Cost of an unconstrained Huffman code, by the classic heap method.
    fn huffman_cost(freqs: &[u32]) -> u64 {
        let mut heap: BinaryHeap<Reverse<u64>> = freqs
            .iter()
            .filter(|&&f| f > 0)
            .map(|&f| Reverse(u64::from(f)))
            .collect();
        let mut cost = 0;
        while heap.len() > 1 {
            let Reverse(a) = heap.pop().unwrap();
            let Reverse(b) = heap.pop().unwrap();
            cost += a + b;
            heap.push(Reverse(a + b));
        }
        cost
    }

    fn kraft_sum(lengths: &[u8], max_bits: usize) -> u64 {
        lengths
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 1u64 << (max_bits - usize::from(l)))
            .sum()
    }

    fn pseudo_random_freqs(seed: u64, count: usize, skew: u32) -> Vec<u32> {
        let mut state = seed;
        (0..count)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let r = (state >> 33) as u32;
                if r.is_multiple_of(5) {
                    0
                } else {
                    (r % 1000) >> (r % skew)
                }
            })
            .collect()
    }

    #[test]
    fn matches_unlimited_huffman_when_limit_is_loose() {
        for seed in 0..50 {
            let freqs = pseudo_random_freqs(seed, 40, 4);
            let code = HuffmanCode::optimal(&freqs, 15);
            if freqs.iter().filter(|&&f| f > 0).count() >= 2 {
                assert_eq!(code.cost(&freqs), huffman_cost(&freqs), "seed {seed}");
            }
        }
    }

    #[test]
    fn respects_length_limit_and_stays_complete() {
        // Fibonacci weights force a maximally skewed unconstrained tree.
        let mut freqs = vec![1u32, 1];
        while freqs.len() < 30 {
            let n = freqs.len();
            freqs.push(freqs[n - 1] + freqs[n - 2]);
        }
        for max_bits in [7, 9, 15] {
            let code = HuffmanCode::optimal(&freqs, max_bits);
            assert!(code.lengths.iter().all(|&l| usize::from(l) <= max_bits));
            assert_eq!(kraft_sum(&code.lengths, max_bits), 1 << max_bits);
            assert!(code.cost(&freqs) >= huffman_cost(&freqs));
        }
    }

    #[test]
    fn many_random_codes_are_complete_prefix_codes() {
        for seed in 0..200 {
            let freqs = pseudo_random_freqs(seed, 286, 10);
            let code = HuffmanCode::optimal(&freqs, 15);
            assert_eq!(kraft_sum(&code.lengths, 15), 1 << 15, "seed {seed}");
            for (&freq, &length) in freqs.iter().zip(&code.lengths) {
                assert_eq!(freq > 0, length > 0, "seed {seed}");
            }
        }
    }

    #[test]
    fn degenerate_inputs_get_two_one_bit_codes() {
        assert_eq!(HuffmanCode::optimal(&[0, 0, 0], 15).lengths, [1, 1, 0]);
        assert_eq!(HuffmanCode::optimal(&[0, 0, 9], 15).lengths, [1, 0, 1]);
        assert_eq!(HuffmanCode::optimal(&[4, 0, 0], 15).lengths, [1, 1, 0]);
    }

    #[test]
    fn canonical_codes_follow_rfc_example() {
        // RFC 1951 §3.2.2: lengths (3, 3, 3, 3, 3, 2, 4, 4) give codes
        // 010, 011, 100, 101, 110, 00, 1110, 1111.
        let code = HuffmanCode::from_lengths(vec![3, 3, 3, 3, 3, 2, 4, 4]);
        let expected: [(u16, u8); 8] = [
            (0b010, 3),
            (0b011, 3),
            (0b100, 3),
            (0b101, 3),
            (0b110, 3),
            (0b00, 2),
            (0b1110, 4),
            (0b1111, 4),
        ];
        for (symbol, &(msb_first, length)) in expected.iter().enumerate() {
            let reversed = msb_first.reverse_bits() >> (16 - length);
            assert_eq!(code.codes[symbol], reversed, "symbol {symbol}");
        }
    }
}
