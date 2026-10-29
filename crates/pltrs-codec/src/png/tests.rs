//! Behavioural tests of the PNG encoder and decoder.

use super::chunk::write_chunk;
use super::header::Header;
use super::types::PixelFormat;
use super::*;
use crate::test_util::Rng;
use crate::{CompressionLevel, zlib};

const ENCODABLE: [(ColorType, BitDepth); 11] = [
    (ColorType::Gray, BitDepth::One),
    (ColorType::Gray, BitDepth::Two),
    (ColorType::Gray, BitDepth::Four),
    (ColorType::Gray, BitDepth::Eight),
    (ColorType::Gray, BitDepth::Sixteen),
    (ColorType::GrayAlpha, BitDepth::Eight),
    (ColorType::GrayAlpha, BitDepth::Sixteen),
    (ColorType::Rgb, BitDepth::Eight),
    (ColorType::Rgb, BitDepth::Sixteen),
    (ColorType::Rgba, BitDepth::Eight),
    (ColorType::Rgba, BitDepth::Sixteen),
];

const STRATEGIES: [FilterStrategy; 6] = [
    FilterStrategy::Adaptive,
    FilterStrategy::Fixed(FilterType::None),
    FilterStrategy::Fixed(FilterType::Sub),
    FilterStrategy::Fixed(FilterType::Up),
    FilterStrategy::Fixed(FilterType::Average),
    FilterStrategy::Fixed(FilterType::Paeth),
];

/// Random pixels with some smooth structure, so filters have work to do.
fn pixels(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut value = 0u8;
    (0..len)
        .map(|_| {
            value = value.wrapping_add((rng.below(9) as u8).wrapping_sub(4));
            if rng.below(10) == 0 {
                rng.next_u64() as u8
            } else {
                value
            }
        })
        .collect()
}

fn header_bytes(
    width: u32,
    height: u32,
    color: ColorType,
    depth: BitDepth,
    interlaced: bool,
) -> Vec<u8> {
    Header {
        width,
        height,
        format: PixelFormat::new(color, depth).unwrap(),
        interlaced,
    }
    .to_bytes()
    .to_vec()
}

/// Assembles a PNG file from raw chunks.
fn assemble(chunks: &[(ChunkType, &[u8])]) -> Vec<u8> {
    let mut png = SIGNATURE.to_vec();
    for &(kind, data) in chunks {
        write_chunk(&mut png, kind, data);
    }
    png
}

/// Recomputes every chunk CRC so corrupted files get past CRC checks.
fn fix_crcs(png: &mut [u8]) {
    let mut at = SIGNATURE.len();
    while at + 8 <= png.len() {
        let length = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        let end = at + 8 + length;
        if end + 4 > png.len() {
            return;
        }
        let crc = crate::checksum::crc32(&png[at + 4..end]);
        png[end..end + 4].copy_from_slice(&crc.to_be_bytes());
        at = end + 4;
    }
}

/// A 3×2 grey 8-bit image's IHDR and compressed data, for error tests.
fn gray_parts() -> (Vec<u8>, Vec<u8>) {
    let ihdr = header_bytes(3, 2, ColorType::Gray, BitDepth::Eight, false);
    let idat = zlib::compress(&[0, 1, 2, 3, 0, 4, 5, 6], CompressionLevel::DEFAULT);
    (ihdr, idat)
}

#[test]
fn round_trips_every_encodable_format() {
    let mut rng = Rng::new(31);
    for (color, depth) in ENCODABLE {
        for (width, height) in [(1, 1), (3, 2), (17, 5), (64, 3)] {
            let row_bytes = (width * color.channels() * usize::from(depth.bits())).div_ceil(8);
            let data = pixels(&mut rng, row_bytes * height);
            let image = ImageRef::new(width as u32, height as u32, color, depth, &data).unwrap();
            for (index, strategy) in STRATEGIES.into_iter().enumerate() {
                let level = CompressionLevel::new((index * 9 / 5) as u8).unwrap();
                let file = Encoder::new()
                    .compression(level)
                    .filter(strategy)
                    .encode(image)
                    .unwrap();
                let decoded = decode(&file).unwrap();
                let context = format!("{color} {depth} {width}×{height} {strategy:?}");
                assert_eq!(decoded.data(), data, "{context}");
                assert_eq!(
                    (decoded.width(), decoded.height()),
                    (width as u32, height as u32)
                );
                assert_eq!((decoded.color_type(), decoded.bit_depth()), (color, depth));
                assert_eq!(decoded.row_bytes(), row_bytes);
                assert!(!decoded.is_interlaced());
            }
        }
    }
}

#[test]
fn rgba_conversions_preserve_values() {
    let rgba = pixels(&mut Rng::new(32), 7 * 5 * 4);
    let decoded = decode(&encode(ImageRef::rgba8(7, 5, &rgba).unwrap()).unwrap()).unwrap();
    assert_eq!(decoded.to_rgba8(), rgba);
    let wide: Vec<u16> = rgba.iter().map(|&v| u16::from(v) * 257).collect();
    assert_eq!(decoded.to_rgba16(), wide);

    let rgb16 = [0x12, 0x34, 0xFF, 0xFF, 0x00, 0x80];
    let image = ImageRef::new(1, 1, ColorType::Rgb, BitDepth::Sixteen, &rgb16).unwrap();
    let decoded = decode(&encode(image).unwrap()).unwrap();
    assert_eq!(decoded.to_rgba16(), [0x1234, 0xFFFF, 0x0080, 0xFFFF]);
    assert_eq!(decoded.to_rgba8(), [0x12, 0xFF, 0x00, 0xFF]);

    let gray2 = [0b0001_1011];
    let image = ImageRef::new(4, 1, ColorType::Gray, BitDepth::Two, &gray2).unwrap();
    let decoded = decode(&encode(image).unwrap()).unwrap();
    let expected: Vec<u8> = [0, 85, 170, 255]
        .iter()
        .flat_map(|&g| [g, g, g, 255])
        .collect();
    assert_eq!(decoded.to_rgba8(), expected);
    let gray_alpha = [9, 200];
    let image = ImageRef::new(1, 1, ColorType::GrayAlpha, BitDepth::Eight, &gray_alpha).unwrap();
    assert_eq!(
        decode(&encode(image).unwrap()).unwrap().to_rgba8(),
        [9, 9, 9, 200]
    );
}

#[test]
fn metadata_round_trips_in_chunk_order() {
    let rgba = [0; 4 * 4];
    let file = Encoder::new()
        .dpi(100.0)
        .text("Software", "pltrs")
        .text("Title", "π ≈ 3.14")
        .encode(ImageRef::rgba8(2, 2, &rgba).unwrap())
        .unwrap();
    let order: Vec<&[u8]> = [&b"IHDR"[..], b"tEXt", b"iTXt", b"pHYs", b"IDAT", b"IEND"].to_vec();
    let mut positions = order
        .iter()
        .map(|kind| file.windows(4).position(|w| w == *kind));
    let mut last = 0;
    for kind in &order {
        let at = positions
            .next()
            .flatten()
            .unwrap_or_else(|| panic!("no {kind:?}"));
        assert!(at > last, "{kind:?} out of order");
        last = at;
    }
    let metadata = decode(&file).unwrap().metadata().clone();
    assert_eq!(metadata.physical, PhysicalDimensions::from_dpi(100.0));
    assert_eq!(
        metadata.text,
        [
            TextEntry::new("Software", "pltrs"),
            TextEntry::new("Title", "π ≈ 3.14")
        ]
    );

    let dims = PhysicalDimensions {
        x: 2,
        y: 3,
        unit: PhysicalUnit::Unknown,
    };
    let file = Encoder::new()
        .physical_dimensions(dims)
        .encode(ImageRef::rgba8(2, 2, &rgba).unwrap())
        .unwrap();
    assert_eq!(decode(&file).unwrap().metadata().physical, Some(dims));
}

#[test]
fn encoder_rejects_invalid_settings() {
    let image = ImageRef::rgba8(1, 1, &[0; 4]).unwrap();
    assert_eq!(
        Encoder::new().dpi(-1.0).encode(image),
        Err(EncodeError::InvalidDpi(-1.0))
    );
    assert_eq!(
        Encoder::new().text(" bad", "x").encode(image),
        Err(EncodeError::InvalidTextKeyword(" bad".to_owned()))
    );
}

#[test]
fn large_images_span_several_idat_chunks() {
    let noise = Rng::new(33).bytes(1024 * 600 * 4);
    let file = encode(ImageRef::rgba8(1024, 600, &noise).unwrap()).unwrap();
    assert!(file.windows(4).filter(|w| w == b"IDAT").count() >= 3);
    assert_eq!(decode(&file).unwrap().into_data(), noise);
}

#[test]
fn size_limit_applies_before_decoding() {
    let file = encode(ImageRef::rgba8(10, 10, &[7; 400]).unwrap()).unwrap();
    assert!(Decoder::new().max_image_bytes(400).decode(&file).is_ok());
    assert_eq!(
        Decoder::new().max_image_bytes(399).decode(&file),
        Err(DecodeError::ImageTooLarge { limit: 399 })
    );
    // A tiny file claiming a huge image is refused without allocating it.
    let ihdr = header_bytes(1 << 30, 1 << 30, ColorType::Rgba, BitDepth::Sixteen, false);
    let idat = zlib::compress(&[0], CompressionLevel::DEFAULT);
    let bomb = assemble(&[
        (ChunkType::IHDR, &ihdr),
        (ChunkType::IDAT, &idat),
        (ChunkType::IEND, &[]),
    ]);
    assert!(matches!(
        decode(&bomb),
        Err(DecodeError::ImageTooLarge { .. })
    ));
    assert_eq!(Decoder::default(), Decoder::new());
}

#[test]
fn indexed_images_use_palette_and_transparency() {
    // 2-bit indices 0, 1, 2, 3 against a 3-entry palette with two alphas.
    let ihdr = header_bytes(4, 1, ColorType::Indexed, BitDepth::Two, false);
    let plte = [255, 0, 0, 0, 255, 0, 0, 0, 255];
    let trns = [10, 20];
    let idat = zlib::compress(&[0, 0b0001_1011], CompressionLevel::DEFAULT);
    let file = assemble(&[
        (ChunkType::IHDR, &ihdr),
        (ChunkType::PLTE, &plte),
        (ChunkType::TRNS, &trns),
        (ChunkType::IDAT, &idat),
        (ChunkType::IEND, &[]),
    ]);
    let image = decode(&file).unwrap();
    assert_eq!(image.data(), [0b0001_1011]);
    assert_eq!(
        image.palette(),
        Some(&[[255, 0, 0, 10], [0, 255, 0, 20], [0, 0, 255, 255]][..])
    );
    assert_eq!(
        image.to_rgba8(),
        [255, 0, 0, 10, 0, 255, 0, 20, 0, 0, 255, 255, 0, 0, 0, 255]
    );
}

#[test]
fn colour_keys_make_matching_pixels_transparent() {
    let ihdr = header_bytes(2, 1, ColorType::Rgb, BitDepth::Eight, false);
    let idat = zlib::compress(&[0, 1, 2, 3, 4, 5, 6], CompressionLevel::DEFAULT);
    let trns = [0, 4, 0, 5, 0, 6];
    let file = assemble(&[
        (ChunkType::IHDR, &ihdr),
        (ChunkType::TRNS, &trns),
        (ChunkType::IDAT, &idat),
        (ChunkType::IEND, &[]),
    ]);
    let image = decode(&file).unwrap();
    assert_eq!(image.color_key(), Some(ColorKey::Rgb([4, 5, 6])));
    assert_eq!(image.to_rgba8(), [1, 2, 3, 255, 4, 5, 6, 0]);
    assert_eq!(image.to_rgba16()[4..], [4 * 257, 5 * 257, 6 * 257, 0]);
}

#[test]
fn tolerates_ancillary_noise_and_surplus_data() {
    let (ihdr, _) = gray_parts();
    let idat = zlib::compress(
        &[0, 1, 2, 3, 0, 4, 5, 6, 0, 9, 9, 9],
        CompressionLevel::DEFAULT,
    );
    let (first, second) = idat.split_at(5);
    let file = assemble(&[
        (ChunkType::IHDR, &ihdr),
        (ChunkType(*b"prVt"), b"private data"),
        (ChunkType::PHYS, b"short"),
        (ChunkType::TEXT, b"no separator"),
        (ChunkType::IDAT, first),
        (ChunkType::IDAT, &[]),
        (ChunkType::IDAT, second),
        (ChunkType::TRNS, &[0, 1]),
        (ChunkType::IEND, &[]),
    ]);
    let image = decode(&file).unwrap();
    assert_eq!(image.data(), [1, 2, 3, 4, 5, 6]);
    assert_eq!(image.color_key(), None);
    assert_eq!(*image.metadata(), Metadata::default());
}

/// Asserts that the file made of `chunks` fails to decode with `expected`.
fn assert_rejected(chunks: &[(ChunkType, &[u8])], expected: &DecodeError) {
    assert_eq!(
        decode(&assemble(chunks)).as_ref(),
        Err(expected),
        "{expected}"
    );
}

#[test]
fn rejects_missing_duplicate_and_misplaced_chunks() {
    use ChunkType as C;
    let (ihdr, idat) = gray_parts();
    let (ihdr, idat, iend) = (ihdr.as_slice(), idat.as_slice(), &[][..]);
    assert_eq!(decode(b"GIF89a"), Err(DecodeError::InvalidSignature));
    assert_rejected(
        &[(C::TEXT, b"a\0b"), (C::IHDR, ihdr)],
        &DecodeError::MissingHeader,
    );
    assert_rejected(
        &[(C::IHDR, ihdr), (C::IHDR, ihdr)],
        &DecodeError::DuplicateChunk(C::IHDR),
    );
    assert_rejected(
        &[(C::IHDR, ihdr), (C::IEND, iend)],
        &DecodeError::MissingImageData,
    );
    assert_rejected(
        &[(C::IHDR, ihdr), (C::IDAT, idat)],
        &DecodeError::UnexpectedEof,
    );
    let split_idat = [
        (C::IHDR, ihdr),
        (C::IDAT, idat),
        (C::TEXT, b"a\0b"),
        (C::IDAT, idat),
    ];
    assert_rejected(&split_idat, &DecodeError::ChunkOutOfOrder(C::IDAT));
    let unknown = C(*b"ABCD");
    assert_rejected(
        &[(C::IHDR, ihdr), (unknown, b"")],
        &DecodeError::UnknownCriticalChunk(unknown),
    );
}

#[test]
fn rejects_invalid_palettes_and_transparency() {
    use ChunkType as C;
    let (gray, idat) = gray_parts();
    let (gray, idat, iend) = (gray.as_slice(), idat.as_slice(), &[][..]);
    assert_rejected(
        &[(C::IHDR, gray), (C::PLTE, &[0, 0, 0])],
        &DecodeError::UnexpectedPalette,
    );
    assert_rejected(
        &[(C::IHDR, gray), (C::TRNS, &[0])],
        &DecodeError::InvalidTransparency,
    );

    let rgb = header_bytes(1, 1, ColorType::Rgb, BitDepth::Eight, false);
    let palette = |length| DecodeError::InvalidPalette { length };
    assert_rejected(&[(C::IHDR, &rgb), (C::PLTE, &[0, 0])], &palette(2));
    assert_rejected(&[(C::IHDR, &rgb), (C::PLTE, &[0; 771])], &palette(771));
    let twice = [(C::IHDR, &rgb[..]), (C::PLTE, &[0; 3]), (C::PLTE, &[0; 3])];
    assert_rejected(&twice, &DecodeError::DuplicateChunk(C::PLTE));
    let late = [(C::IHDR, &rgb[..]), (C::IDAT, idat), (C::PLTE, &[0; 3])];
    assert_rejected(&late, &DecodeError::ChunkOutOfOrder(C::PLTE));

    let rgba = header_bytes(1, 1, ColorType::Rgba, BitDepth::Eight, false);
    assert_rejected(
        &[(C::IHDR, &rgba), (C::TRNS, &[0; 6])],
        &DecodeError::InvalidTransparency,
    );

    let indexed = header_bytes(1, 1, ColorType::Indexed, BitDepth::Eight, false);
    let no_palette = [(C::IHDR, &indexed[..]), (C::IDAT, idat), (C::IEND, iend)];
    assert_rejected(&no_palette, &DecodeError::MissingPalette);
    let long_trns = [
        (C::IHDR, &indexed[..]),
        (C::PLTE, &[0; 3]),
        (C::TRNS, &[0, 0]),
    ];
    assert_rejected(&long_trns, &DecodeError::InvalidTransparency);
}

#[test]
fn rejects_bad_image_data() {
    use ChunkType as C;
    let (ihdr, _) = gray_parts();
    let with_data = |data: &[u8]| assemble(&[(C::IHDR, &ihdr), (C::IDAT, data), (C::IEND, &[])]);
    let short = zlib::compress(&[0, 1, 2], CompressionLevel::DEFAULT);
    assert_eq!(
        decode(&with_data(&short)),
        Err(DecodeError::ImageDataTooShort {
            expected: 8,
            actual: 3
        })
    );
    let bad_filter = zlib::compress(&[7, 1, 2, 3, 0, 4, 5, 6], CompressionLevel::DEFAULT);
    assert_eq!(
        decode(&with_data(&bad_filter)),
        Err(DecodeError::InvalidFilterType(7))
    );
    assert_eq!(
        decode(&with_data(&[0x78, 0x9C, 0x07])),
        Err(DecodeError::Zlib(zlib::ZlibError::Inflate(
            crate::deflate::InflateError::InvalidBlockType
        )))
    );
}

#[test]
fn corrupted_files_never_panic() {
    let mut rng = Rng::new(34);
    let mut files = Vec::new();
    for (color, depth) in ENCODABLE {
        let row_bytes = (9 * color.channels() * usize::from(depth.bits())).div_ceil(8);
        let data = pixels(&mut rng, row_bytes * 7);
        files.push(encode(ImageRef::new(9, 7, color, depth, &data).unwrap()).unwrap());
    }
    for file in &files {
        for _ in 0..150 {
            let mut corrupted = file.clone();
            for _ in 0..=rng.below(3) {
                let at = rng.below(corrupted.len());
                corrupted[at] = rng.next_u64() as u8;
            }
            if rng.below(2) == 0 {
                fix_crcs(&mut corrupted);
            }
            if let Ok(image) = Decoder::new().max_image_bytes(1 << 24).decode(&corrupted) {
                let pixels = image.width() as usize * image.height() as usize;
                assert_eq!(image.to_rgba8().len(), pixels * 4);
                assert_eq!(image.to_rgba16().len(), pixels * 4);
            }
        }
        for cut in (0..file.len()).step_by(7) {
            assert!(decode(&file[..cut]).is_err());
        }
    }
}

#[test]
fn errors_display_and_chain() {
    let error = DecodeError::Zlib(zlib::ZlibError::ChecksumMismatch {
        expected: 1,
        actual: 2,
    });
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().contains("checksum"));
    assert_eq!(
        EncodeError::DataLengthMismatch {
            expected: 4,
            actual: 3
        }
        .to_string(),
        "pixel data has 3 bytes but the image needs 4"
    );
}
