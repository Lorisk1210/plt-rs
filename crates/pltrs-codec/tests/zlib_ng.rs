//! pltrs's DEFLATE writes zlib-ng 2.3's bytes: the streams of
//! `corpus.bin` at every level, strategy and some memory levels as
//! `compress2` writes them, and the `IDAT` data of images as Pillow writes
//! it with zlib-ng, its rows filtered by Pillow's heuristic and compressed
//! one at a time into 64 KiB buffers, one `IDAT` chunk each. The values
//! come from a zlib-ng 2.3.3 build (`tools/zlib_ng/expected.py`).

use std::fs;
use std::path::Path;

use pltrs_codec::checksum::crc32;
use pltrs_codec::deflate::{MemoryLevel, Options, Strategy};
use pltrs_codec::png::{BitDepth, ColorType, Encoder, ImageRef};
use pltrs_codec::{CompressionLevel, zlib};

/// The zlib streams of `corpus.bin`: level, strategy, memory level,
/// length and CRC-32.
const CORPUS: [(u8, Strategy, u8, usize, u32); 70] = [
    (0, Strategy::Default, 8, 26_433, 0xa5c2_ae19),
    (0, Strategy::Filtered, 8, 26_433, 0xa5c2_ae19),
    (0, Strategy::HuffmanOnly, 8, 26_433, 0xa5c2_ae19),
    (0, Strategy::Rle, 8, 26_433, 0xa5c2_ae19),
    (0, Strategy::Fixed, 8, 26_433, 0xa5c2_ae19),
    (1, Strategy::Default, 8, 6_895, 0x5fdb_8a11),
    (1, Strategy::Filtered, 8, 6_895, 0x5fdb_8a11),
    (1, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (1, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (1, Strategy::Fixed, 8, 6_895, 0x5fdb_8a11),
    (2, Strategy::Default, 8, 6_060, 0x3476_a236),
    (2, Strategy::Filtered, 8, 6_060, 0x3476_a236),
    (2, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (2, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (2, Strategy::Fixed, 8, 6_415, 0xa4a1_4e04),
    (3, Strategy::Default, 8, 5_850, 0xa3de_f0e2),
    (3, Strategy::Filtered, 8, 5_850, 0xa3de_f0e2),
    (3, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (3, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (3, Strategy::Fixed, 8, 6_168, 0x91a4_1f3d),
    (4, Strategy::Default, 8, 5_516, 0x6993_34a1),
    (4, Strategy::Filtered, 8, 5_516, 0x6993_34a1),
    (4, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (4, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (4, Strategy::Fixed, 8, 5_751, 0xd10e_edf3),
    (5, Strategy::Default, 8, 5_401, 0xde7f_21a8),
    (5, Strategy::Filtered, 8, 5_401, 0xde7f_21a8),
    (5, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (5, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (5, Strategy::Fixed, 8, 5_596, 0xc811_5841),
    (6, Strategy::Default, 8, 5_273, 0x41af_8bf2),
    (6, Strategy::Filtered, 8, 5_273, 0x41af_8bf2),
    (6, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (6, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (6, Strategy::Fixed, 8, 5_419, 0x61b5_47f6),
    (7, Strategy::Default, 8, 5_245, 0x6bb5_4be5),
    (7, Strategy::Filtered, 8, 5_270, 0x13c3_fa15),
    (7, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (7, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (7, Strategy::Fixed, 8, 5_385, 0x198e_0f3c),
    (8, Strategy::Default, 8, 5_217, 0xf0e2_0e1a),
    (8, Strategy::Filtered, 8, 5_242, 0x1b78_3439),
    (8, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (8, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (8, Strategy::Fixed, 8, 5_349, 0xab99_6d9f),
    (9, Strategy::Default, 8, 5_217, 0xb781_e14a),
    (9, Strategy::Filtered, 8, 5_242, 0x79a7_efc0),
    (9, Strategy::HuffmanOnly, 8, 17_471, 0x82a8_9289),
    (9, Strategy::Rle, 8, 14_541, 0x2afa_38fb),
    (9, Strategy::Fixed, 8, 5_348, 0x1131_a2b1),
    (0, Strategy::Default, 1, 26_433, 0xa5c2_ae19),
    (1, Strategy::Default, 1, 6_895, 0x5fdb_8a11),
    (2, Strategy::Default, 1, 5_813, 0xc4f3_c054),
    (3, Strategy::Default, 1, 5_590, 0xa566_cd1c),
    (4, Strategy::Default, 1, 5_345, 0xd941_5d34),
    (5, Strategy::Default, 1, 5_161, 0x51f5_6dec),
    (6, Strategy::Default, 1, 5_110, 0xb1e6_aa73),
    (7, Strategy::Default, 1, 5_094, 0x9213_5dbe),
    (8, Strategy::Default, 1, 5_060, 0xe30b_ea17),
    (9, Strategy::Default, 1, 5_058, 0xd01d_8827),
    (0, Strategy::Default, 9, 26_433, 0xa5c2_ae19),
    (1, Strategy::Default, 9, 6_895, 0x5fdb_8a11),
    (2, Strategy::Default, 9, 6_060, 0x3476_a236),
    (3, Strategy::Default, 9, 5_850, 0xa3de_f0e2),
    (4, Strategy::Default, 9, 5_516, 0x6993_34a1),
    (5, Strategy::Default, 9, 5_401, 0xde7f_21a8),
    (6, Strategy::Default, 9, 5_273, 0x41af_8bf2),
    (7, Strategy::Default, 9, 5_245, 0x6bb5_4be5),
    (8, Strategy::Default, 9, 5_217, 0xf0e2_0e1a),
    (9, Strategy::Default, 9, 5_217, 0xb781_e14a),
];

/// The IDAT streams of the images, as Pillow writes them with
/// zlib-ng: image, level, strategy, length, CRC-32 and chunks.
const PNGS: [(usize, u8, Strategy, usize, u32, usize); 16] = [
    (0, 1, Strategy::Filtered, 58_854, 0x298a_bee3, 1),
    (0, 6, Strategy::Filtered, 30_032, 0x7886_9c12, 1),
    (0, 9, Strategy::Filtered, 29_033, 0x7666_2add, 1),
    (0, 9, Strategy::Default, 28_280, 0x8624_3d23, 1),
    (1, 1, Strategy::Filtered, 1_024_488, 0x13bf_019f, 16),
    (1, 6, Strategy::Filtered, 474_083, 0x2923_b7f1, 8),
    (1, 9, Strategy::Filtered, 476_936, 0xa6ae_35e5, 8),
    (1, 9, Strategy::Default, 472_748, 0xddcd_8fc8, 8),
    (2, 1, Strategy::Filtered, 906, 0xcc4e_78d9, 1),
    (2, 6, Strategy::Filtered, 215, 0x74e9_bbeb, 1),
    (2, 9, Strategy::Filtered, 141, 0x9d22_dc75, 1),
    (2, 9, Strategy::Default, 141, 0x9d22_dc75, 1),
    (3, 1, Strategy::Filtered, 2_781, 0xff7e_6585, 1),
    (3, 6, Strategy::Filtered, 1_473, 0xb114_6d09, 1),
    (3, 9, Strategy::Filtered, 1_494, 0xb1d7_7833, 1),
    (3, 9, Strategy::Default, 1_475, 0xfda9_e3ad, 1),
];

/// The images of `PNGS`: colour type, channels, width, height and the
/// formula of their pixels.
const IMAGES: [(ColorType, usize, usize, usize, u8); 4] = [
    (ColorType::Rgba, 4, 300, 200, 0),
    (ColorType::Rgb, 3, 1000, 700, 2),
    (ColorType::Gray, 1, 17_000, 3, 1),
    (ColorType::GrayAlpha, 2, 64, 48, 2),
];

/// Sample `c` of pixel `(x, y)` of an image of `kind` (as `expected.py`
/// computes it).
fn pixel(kind: u8, x: u64, y: u64, c: u64) -> u8 {
    let value = match kind {
        0 => x * 3 + y * 5 + c * 50 + x * y / 7,
        1 => (x / 23 + y / 17) % 5 * 40 + c * 10,
        _ => {
            let noise = (((x * 7919 + y * 104_729 + c * 31) * 2_654_435_761) >> 13) & 7;
            (x * x + 3 * y * y) / 97 + 40 * c + noise
        }
    };
    (value & 255) as u8
}

/// The `IDAT` chunks of a PNG file.
fn idat_chunks(png: &[u8]) -> Vec<&[u8]> {
    let mut chunks = Vec::new();
    let mut at = 8;
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().expect("four bytes")) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            chunks.push(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    chunks
}

#[test]
fn one_shot_streams_are_zlib_ngs() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zlib/corpus.bin");
    let corpus = fs::read(path).unwrap();
    for (level, strategy, memory, len, crc) in CORPUS {
        let options = Options::new(CompressionLevel::new(level).unwrap())
            .strategy(strategy)
            .memory_level(MemoryLevel::new(memory).unwrap());
        let stream = zlib::compress_with(&corpus, options).unwrap();
        assert_eq!(
            (stream.len(), crc32(&stream)),
            (len, crc),
            "level {level}, {strategy:?}, memory level {memory}"
        );
        assert_eq!(zlib::decompress(&stream).unwrap(), corpus);
    }
}

#[test]
fn png_streams_are_pillows_with_zlib_ng() {
    for (image, level, strategy, len, crc, chunks) in PNGS {
        let (color, channels, width, height, kind) = IMAGES[image];
        let data: Vec<u8> = (0..height as u64)
            .flat_map(|y| {
                (0..width as u64)
                    .flat_map(move |x| (0..channels as u64).map(move |c| pixel(kind, x, y, c)))
            })
            .collect();
        let image_ref =
            ImageRef::new(width as u32, height as u32, color, BitDepth::Eight, &data).unwrap();
        let png = Encoder::new()
            .compression(CompressionLevel::new(level).unwrap())
            .strategy(strategy)
            .encode(image_ref)
            .unwrap();
        let idat = idat_chunks(&png);
        let stream = idat.concat();
        assert_eq!(
            (stream.len(), crc32(&stream), idat.len()),
            (len, crc, chunks),
            "image {image}, level {level}, {strategy:?}"
        );
        let buffer = 65_536.max(4 * width);
        assert!(
            idat[..idat.len() - 1]
                .iter()
                .all(|chunk| chunk.len() == buffer)
        );
    }
}
