//! Behavioural tests of the DEFLATE encoder and decoder.

use super::bit_writer::BitWriter;
use super::huffman::HuffmanCode;
use super::*;
use crate::test_util::{Rng, varied_inputs};

fn all_levels() -> impl Iterator<Item = CompressionLevel> {
    (0..=9).map(|level| CompressionLevel::new(level).unwrap())
}

#[test]
fn round_trips_varied_inputs_at_every_level() {
    let mut rng = Rng::new(1);
    let inputs = varied_inputs(&mut rng);
    for level in all_levels() {
        for (index, input) in inputs.iter().enumerate() {
            let packed = compress(input, level).unwrap();
            let unpacked = decompress(&packed)
                .unwrap_or_else(|e| panic!("level {level:?}, input {index}: {e}"));
            assert!(unpacked == *input, "level {level:?}, input {index} differs");
        }
    }
}

#[test]
fn round_trips_many_small_random_inputs() {
    let mut rng = Rng::new(2);
    for case in 0..400 {
        let len = rng.below(600);
        let alphabet = 1 + rng.below(255);
        let input: Vec<u8> = (0..len).map(|_| rng.below(alphabet) as u8).collect();
        let level = CompressionLevel::new((case % 10) as u8).unwrap();
        assert_eq!(
            decompress(&compress(&input, level).unwrap()).unwrap(),
            input,
            "case {case}"
        );
    }
}

/// The window's span, the longest distance a match may reach.
const WINDOW: usize = 32_768;

/// Inputs whose compressed bytes are pinned: numbers as vector documents
/// write them, data repeating at the window's full distance, and runs
/// between random bursts.
fn pinned_inputs() -> [Vec<u8>; 3] {
    let mut state = 0x2545_F491_4F6C_DD1D_u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut numbers = Vec::new();
    for _ in 0..12_000 {
        let (x, y) = (next() % 64_000, next() % 48_000);
        let line = format!("{}.{:02} {}.{:02} l\n", x / 100, x % 100, y / 100, y % 100);
        numbers.extend_from_slice(line.as_bytes());
    }
    let mut period: Vec<u8> = (0..WINDOW).map(|_| (next() % 7) as u8).collect();
    for i in WINDOW..2 * WINDOW + 5_000 {
        let byte = if next() % 50 == 0 {
            (next() % 7) as u8
        } else {
            period[i - WINDOW]
        };
        period.push(byte);
    }
    let mut runs = Vec::new();
    while runs.len() < 100_000 {
        let byte = (next() % 4) as u8;
        runs.extend(std::iter::repeat_n(byte, (next() % 600) as usize));
        runs.extend((0..next() % 40).map(|_| next() as u8));
    }
    [numbers, period, runs]
}

#[test]
fn compressed_bytes_are_zlib_ngs() {
    // FNV-1a over the raw streams of levels 1 to 9 as zlib-ng 2.3.3's
    // `compress2` writes them (crates/pltrs-codec/tools/zlib_ng).
    let expected = [
        0xd080_ca6c_2146_a6d4_u64,
        0x042a_d978_1e47_6775,
        0x4944_9258_6559_68b7,
    ];
    for (input, expected) in pinned_inputs().iter().zip(expected) {
        let fingerprint = (1..=9)
            .flat_map(|level| compress(input, CompressionLevel::new(level).unwrap()).unwrap())
            .fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
                (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
            });
        assert_eq!(fingerprint, expected, "input of {} bytes", input.len());
    }
}

#[test]
fn matches_reference_encodings_of_tiny_inputs() {
    assert_eq!(
        compress(b"", CompressionLevel::DEFAULT).unwrap(),
        [0x03, 0x00]
    );
    assert_eq!(
        compress(b"", CompressionLevel::NONE).unwrap(),
        [0x01, 0x00, 0x00, 0xFF, 0xFF]
    );
    assert_eq!(
        compress(b"abc", CompressionLevel::NONE).unwrap(),
        [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c']
    );
    assert_eq!(
        compress(b"hello", CompressionLevel::DEFAULT).unwrap(),
        [0xCB, 0x48, 0xCD, 0xC9, 0xC9, 0x07, 0x00]
    );
}

#[test]
fn decodes_reference_streams() {
    // Produced by zlib (raw deflate, fixed Huffman with a back-reference).
    let fixed = [0x4B, 0x4C, 0x4A, 0x4E, 0x84, 0x21, 0x00];
    assert_eq!(decompress(&fixed).unwrap(), b"abcabcabcabc");
    let stored = [0x01, 0x03, 0x00, 0xFC, 0xFF, b'a', b'b', b'c'];
    assert_eq!(decompress(&stored).unwrap(), b"abc");
}

#[test]
fn compresses_redundant_data_and_bounds_expansion() {
    let zeros = vec![0u8; 1 << 20];
    for level in all_levels().skip(1) {
        let packed = compress(&zeros, level).unwrap();
        // Level 1 writes fixed codes only, and its matches reach back to
        // the previous one: 20 bits per 258 bytes.
        let bound = if level == CompressionLevel::FASTEST {
            10_500
        } else {
            1100
        };
        assert!(
            packed.len() < bound,
            "level {level:?}: {} bytes",
            packed.len()
        );
    }
    let noise = Rng::new(3).bytes(200_000);
    for level in all_levels() {
        let packed = compress(&noise, level).unwrap();
        // Blocks fall back to stored blocks of up to 64 KiB, except at
        // level 1, whose one fixed-code block takes nine bits for the
        // upper half of the bytes (zlib-ng's `DEFLATE_QUICK_OVERHEAD`).
        let bound = if level == CompressionLevel::FASTEST {
            noise.len() + noise.len().div_ceil(8) + 8
        } else {
            noise.len() + 5 * noise.len().div_ceil(1 << 14) + 5
        };
        assert!(packed.len() <= bound, "level {level:?}: {}", packed.len());
    }
}

#[test]
fn higher_levels_do_not_compress_worse_on_text() {
    let mut rng = Rng::new(4);
    let text = varied_inputs(&mut rng).swap_remove(19);
    let sizes: Vec<usize> = all_levels()
        .map(|level| compress(&text, level).unwrap().len())
        .collect();
    assert!(sizes[1] < sizes[0] / 2);
    assert!(sizes[9] <= sizes[1]);
    assert!(sizes[6] <= sizes[4] + sizes[4] / 50);
}

#[test]
fn output_limit_is_enforced_exactly() {
    let input = Rng::new(5).bytes(5000);
    let packed = compress(&input, CompressionLevel::DEFAULT).unwrap();
    assert_eq!(decompress_with_limit(&packed, 5000).unwrap(), input);
    assert_eq!(
        decompress_with_limit(&packed, 4999),
        Err(InflateError::OutputLimitExceeded { limit: 4999 })
    );
    let zeros = compress(&[0; 10_000], CompressionLevel::DEFAULT).unwrap();
    assert_eq!(
        decompress_with_limit(&zeros, 100),
        Err(InflateError::OutputLimitExceeded { limit: 100 })
    );
}

#[test]
fn hitting_the_limit_leaves_exactly_limit_bytes() {
    let input: Vec<u8> = (0..3000u32).map(|i| (i % 7 + i / 300) as u8).collect();
    for level in all_levels() {
        let packed = compress(&input, level).unwrap();
        for limit in [0, 1, 2, 3, 100, 257, 1001, 2999] {
            let mut out = vec![9];
            assert_eq!(
                decompress_into(&packed, &mut out, limit),
                Err(InflateError::OutputLimitExceeded { limit })
            );
            assert_eq!(out[1..], input[..limit], "level {level:?}, limit {limit}");
        }
    }
}

#[test]
fn reports_consumed_bytes_and_ignores_trailing_data() {
    let input = b"some data some data some data";
    for level in all_levels() {
        let mut packed = compress(input, level).unwrap();
        let stream_len = packed.len();
        packed.extend_from_slice(b"trailing garbage");
        let mut out = b"prefix".to_vec();
        assert_eq!(
            decompress_into(&packed, &mut out, usize::MAX),
            Ok(stream_len)
        );
        assert_eq!(&out[..6], b"prefix");
        assert_eq!(&out[6..], input);
    }
}

#[test]
fn back_references_cannot_reach_existing_output() {
    // Fixed block: length 3 at distance 1 with nothing decoded yet.
    let stream = fixed_block(|w, lit| {
        write_code(w, lit, 257);
        w.write_bits(0, 5);
    });
    let mut out = vec![1, 2, 3];
    assert_eq!(
        decompress_into(&stream, &mut out, usize::MAX),
        Err(InflateError::DistanceTooFarBack {
            distance: 1,
            available: 0
        })
    );
}

/// Writes symbol `symbol` of `code` (MSB-first semantics, LSB-first bits).
fn write_code(writer: &mut BitWriter<'_>, code: &HuffmanCode, symbol: usize) {
    writer.write_bits(
        u32::from(code.codes[symbol]),
        u32::from(code.lengths[symbol]),
    );
}

/// Builds a final fixed-Huffman block whose body is written by `body`.
fn fixed_block(body: impl FnOnce(&mut BitWriter<'_>, &HuffmanCode)) -> Vec<u8> {
    let litlen = HuffmanCode::from_lengths(super::tables::fixed_litlen_lengths().to_vec());
    let mut out = Vec::new();
    let mut writer = BitWriter::new(&mut out);
    writer.write_bits(0b011, 3);
    body(&mut writer, &litlen);
    writer.align_to_byte();
    out
}

/// Builds a final dynamic block header with code-length code lengths
/// `cl_lengths` (indexed by symbol), followed by `ops` as
/// `(symbol, extra value, extra bits)`.
fn dynamic_header(
    hlit: u32,
    hdist: u32,
    cl_lengths: &[u8; 19],
    ops: &[(usize, u32, u32)],
) -> Vec<u8> {
    let cl_code = HuffmanCode::from_lengths(cl_lengths.to_vec());
    let mut out = Vec::new();
    let mut writer = BitWriter::new(&mut out);
    writer.write_bits(0b101, 3);
    writer.write_bits(hlit - 257, 5);
    writer.write_bits(hdist - 1, 5);
    writer.write_bits(19 - 4, 4);
    for &symbol in &super::tables::CODE_LENGTH_ORDER {
        writer.write_bits(u32::from(cl_lengths[symbol]), 3);
    }
    for &(symbol, extra, bits) in ops {
        write_code(&mut writer, &cl_code, symbol);
        writer.write_bits(extra, bits);
    }
    writer.write_bits(0, 32);
    writer.align_to_byte();
    out
}

fn cl_lengths(assignments: &[(usize, u8)]) -> [u8; 19] {
    let mut lengths = [0; 19];
    for &(symbol, length) in assignments {
        lengths[symbol] = length;
    }
    lengths
}

#[test]
fn rejects_malformed_streams_with_specific_errors() {
    use HuffmanTable::{CodeLength, Distance, LiteralLength};
    assert_eq!(decompress(&[]), Err(InflateError::UnexpectedEof));
    assert_eq!(decompress(&[0x07]), Err(InflateError::InvalidBlockType));
    assert_eq!(
        decompress(&[0x01, 0x05, 0x00, 0x00, 0x00]),
        Err(InflateError::StoredLengthMismatch {
            length: 5,
            complement: 0
        })
    );
    assert_eq!(
        decompress(&[0x01, 0x05, 0x00, 0xFA, 0xFF, b'a']),
        Err(InflateError::UnexpectedEof)
    );
    // A non-final empty block with nothing after it.
    assert_eq!(decompress(&[0x02, 0x00]), Err(InflateError::UnexpectedEof));

    let reserved_distance = fixed_block(|w, lit| {
        write_code(w, lit, usize::from(b'a'));
        write_code(w, lit, 257);
        w.write_bits(0b01111, 5);
    });
    assert_eq!(
        decompress(&reserved_distance),
        Err(InflateError::InvalidCode(Distance))
    );
    let reserved_length = fixed_block(|w, lit| write_code(w, lit, 286));
    assert_eq!(
        decompress(&reserved_length),
        Err(InflateError::InvalidCode(LiteralLength))
    );

    let mut too_many = Vec::new();
    let mut writer = BitWriter::new(&mut too_many);
    writer.write_bits(0b101, 3);
    writer.write_bits(30, 5);
    writer.write_bits(0, 5);
    writer.write_bits(0, 4);
    writer.align_to_byte();
    assert_eq!(
        decompress(&too_many),
        Err(InflateError::TooManyCodes {
            table: LiteralLength,
            count: 287
        })
    );

    let all_one_bit = cl_lengths(&[(16, 1), (17, 1), (18, 1), (0, 1)]);
    assert_eq!(
        decompress(&dynamic_header(257, 1, &all_one_bit, &[])),
        Err(InflateError::OversubscribedCode(CodeLength))
    );
    let single = cl_lengths(&[(18, 1)]);
    assert_eq!(
        decompress(&dynamic_header(257, 1, &single, &[])),
        Err(InflateError::IncompleteCode(CodeLength))
    );
    let repeat_first = cl_lengths(&[(16, 1), (17, 1)]);
    assert_eq!(
        decompress(&dynamic_header(257, 1, &repeat_first, &[(16, 0, 2)])),
        Err(InflateError::RepeatWithoutPrevious)
    );
    let zeros = cl_lengths(&[(0, 1), (18, 1)]);
    assert_eq!(
        decompress(&dynamic_header(
            257,
            1,
            &zeros,
            &[(18, 127, 7), (18, 127, 7)]
        )),
        Err(InflateError::CodeLengthOverflow)
    );
    assert_eq!(
        decompress(&dynamic_header(
            257,
            1,
            &zeros,
            &[(18, 127, 7), (18, 109, 7)]
        )),
        Err(InflateError::MissingEndOfBlock)
    );
    let three_ones = cl_lengths(&[(1, 1), (0, 2), (18, 2)]);
    let ops = [
        (1, 0, 0),
        (1, 0, 0),
        (1, 0, 0),
        (18, 127, 7),
        (18, 104, 7),
        (1, 0, 0),
        (1, 0, 0),
    ];
    assert_eq!(
        decompress(&dynamic_header(257, 1, &three_ones, &ops)),
        Err(InflateError::OversubscribedCode(LiteralLength))
    );
}

#[test]
fn every_truncation_is_an_error() {
    let mut rng = Rng::new(6);
    let input: Vec<u8> = (0..3000).map(|_| b'a' + rng.below(6) as u8).collect();
    for level in [0, 1, 6, 9] {
        let packed = compress(&input, CompressionLevel::new(level).unwrap()).unwrap();
        for cut in 0..packed.len() {
            assert!(
                decompress(&packed[..cut]).is_err(),
                "level {level}: prefix of {cut}/{} bytes decoded",
                packed.len()
            );
        }
    }
}

#[test]
fn random_and_corrupted_inputs_never_panic() {
    let mut rng = Rng::new(7);
    for _ in 0..3000 {
        let len = rng.below(200);
        let garbage = rng.bytes(len);
        let _ = decompress_with_limit(&garbage, 1 << 20);
    }
    let inputs = varied_inputs(&mut rng);
    for (index, input) in inputs.iter().enumerate().filter(|(_, i)| i.len() < 70_000) {
        let packed = compress(input, CompressionLevel::new((index % 10) as u8).unwrap()).unwrap();
        for _ in 0..40 {
            let mut corrupted = packed.clone();
            if corrupted.is_empty() {
                break;
            }
            for _ in 0..=rng.below(4) {
                let at = rng.below(corrupted.len());
                corrupted[at] ^= 1 << rng.below(8);
            }
            if let Ok(out) = decompress_with_limit(&corrupted, 1 << 21) {
                assert!(out.len() <= 1 << 21);
            }
        }
    }
}
