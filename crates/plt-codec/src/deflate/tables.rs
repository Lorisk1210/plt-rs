//! Constants and lookup tables of the DEFLATE format (RFC 1951 §3.2.5–3.2.7)
//! shared by the encoder and the decoder.

/// Shortest back-reference DEFLATE can express.
pub(crate) const MIN_MATCH: usize = 3;
/// Longest back-reference DEFLATE can express.
pub(crate) const MAX_MATCH: usize = 258;
/// Size of the sliding window: the farthest a back-reference may reach.
pub(crate) const WINDOW_SIZE: usize = 32_768;
/// Longest Huffman code in the literal/length and distance alphabets.
pub(crate) const MAX_CODE_BITS: usize = 15;
/// Longest Huffman code in the code-length alphabet.
pub(crate) const MAX_CODE_LENGTH_BITS: usize = 7;
/// The end-of-block symbol of the literal/length alphabet.
pub(crate) const END_OF_BLOCK: usize = 256;
/// Literal/length symbols a dynamic block may define (0–285).
pub(crate) const LITLEN_SYMBOLS: usize = 286;
/// Distance symbols a dynamic block may define (0–29).
pub(crate) const DIST_SYMBOLS: usize = 30;
/// Size of the code-length alphabet.
pub(crate) const CODE_LENGTH_SYMBOLS: usize = 19;
/// Literal/length symbols of the fixed code, including the two reserved ones.
pub(crate) const FIXED_LITLEN_SYMBOLS: usize = 288;
/// Distance symbols of the fixed code, including the two reserved ones.
pub(crate) const FIXED_DIST_SYMBOLS: usize = 32;

/// Smallest match length of each length symbol 257–285.
pub(crate) const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];

/// Extra bits following each length symbol 257–285.
pub(crate) const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// Smallest distance of each distance symbol 0–29.
pub(crate) const DIST_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];

/// Extra bits following each distance symbol 0–29.
pub(crate) const DIST_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// Order in which code-length code lengths are transmitted.
pub(crate) const CODE_LENGTH_ORDER: [usize; CODE_LENGTH_SYMBOLS] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

/// Length-symbol index (0–28) for each match length 3–258, indexed by
/// `length - 3`.
static LENGTH_INDEX: [u8; 256] = build_length_index();

/// Distance-symbol index for distances 1–256 (indexed by `distance - 1`)
/// followed by indices for larger distances (indexed by
/// `256 + ((distance - 1) >> 7)`), as in zlib's `_dist_code`.
static DIST_INDEX: [u8; 512] = build_dist_index();

const fn build_length_index() -> [u8; 256] {
    let mut table = [0; 256];
    let mut code = 0;
    while code < LENGTH_BASE.len() {
        let base = LENGTH_BASE[code] as usize;
        let mut offset = 0;
        while offset < 1 << LENGTH_EXTRA[code] {
            // Symbol 284 also spans length 258; symbol 285, written later,
            // takes it over because it encodes 258 without extra bits.
            table[base + offset - MIN_MATCH] = code as u8;
            offset += 1;
        }
        code += 1;
    }
    table
}

const fn build_dist_index() -> [u8; 512] {
    let mut table = [0; 512];
    let mut code = 0;
    while code < DIST_BASE.len() {
        let base = DIST_BASE[code] as usize;
        let mut offset = 0;
        while offset < 1 << DIST_EXTRA[code] {
            let distance = base + offset;
            if distance <= 256 {
                table[distance - 1] = code as u8;
            } else {
                table[256 + ((distance - 1) >> 7)] = code as u8;
            }
            offset += 1;
        }
        code += 1;
    }
    table
}

/// Index into [`LENGTH_BASE`] of the symbol encoding match `length` (3–258).
pub(crate) fn length_index(length: usize) -> usize {
    usize::from(LENGTH_INDEX[length - MIN_MATCH])
}

/// Index into [`DIST_BASE`] of the symbol encoding `distance` (1–32768).
pub(crate) fn dist_index(distance: usize) -> usize {
    let zero_based = distance - 1;
    if zero_based < 256 {
        usize::from(DIST_INDEX[zero_based])
    } else {
        usize::from(DIST_INDEX[256 + (zero_based >> 7)])
    }
}

/// Code lengths of the fixed literal/length code (RFC 1951 §3.2.6).
pub(crate) fn fixed_litlen_lengths() -> [u8; FIXED_LITLEN_SYMBOLS] {
    std::array::from_fn(|symbol| match symbol {
        144..=255 => 9,
        256..=279 => 7,
        _ => 8,
    })
}

/// Code lengths of the fixed distance code.
pub(crate) const FIXED_DIST_LENGTHS: [u8; FIXED_DIST_SYMBOLS] = [5; FIXED_DIST_SYMBOLS];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_index_brackets_every_length() {
        for length in MIN_MATCH..=MAX_MATCH {
            let index = length_index(length);
            let base = usize::from(LENGTH_BASE[index]);
            let span = 1 << LENGTH_EXTRA[index];
            assert!(base <= length && length < base + span, "length {length}");
        }
        assert_eq!(length_index(258), 28);
        assert_eq!(length_index(257), 27);
    }

    #[test]
    fn dist_index_brackets_every_distance() {
        for distance in 1..=WINDOW_SIZE {
            let index = dist_index(distance);
            let base = usize::from(DIST_BASE[index]);
            let span = 1 << DIST_EXTRA[index];
            assert!(
                base <= distance && distance < base + span,
                "distance {distance}"
            );
        }
    }

    #[test]
    fn fixed_code_has_rfc_shape() {
        let lengths = fixed_litlen_lengths();
        let kraft: u32 = lengths.iter().map(|&l| 1 << (9 - l)).sum();
        assert_eq!(kraft, 1 << 9, "the fixed code is complete");
        assert_eq!(lengths[END_OF_BLOCK], 7);
    }
}
