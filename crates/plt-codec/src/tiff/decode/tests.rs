use super::*;
use crate::base64;
use crate::checksum::crc32;
use crate::test_util::{Rng, mutate, pil_to_array_bytes};
use crate::tiff::encode_rgba8;

/// Small files Pillow reads (made by the spec's TIFF writer): a label, the
/// size, the CRC-32 of the array matplotlib's `imread` returns for them,
/// and the file in base64.
const PILLOW: [(&str, u32, u32, u32, &str); 5] = [
    (
        "RGB LZW predictor BE",
        9,
        7,
        0x7E12_8248,
        "TU0AKgAAAOaAGkFsgkg8ShsCvt/u9rv4UukHOgOOwGumHPN5u5tncANkYCIGhh3vAEBcBBgFPB5u1+v4Fu54ilwsFxBNNAlkiIAhwMAJ0PoTiETvBvu8EhV2Ax8uN8txvCt4QEAAgCPDTAEr7dwVC4hFgSeTqegYAYSdrYCglcDjdb1EDbToFRwVFwwFzsA4ODYHcDqDoKDzhcQNEzSCbnAD3fadCKHF4oC4BdAeEz7BDtG7raoFCwVfonZoXcLpe4KgIIAuAiUxUFx+BwWAQQ8wQ3hcEgM9g60H4D3GHAcy3kA4CAAACgEAAAQAAAABAAAACQEBAAQAAAABAAAABwECAAMAAAADAAABZAEDAAMAAAABAAUAAAEGAAMAAAABAAIAAAERAAQAAAADAAABagEVAAMAAAABAAMAAAEWAAQAAAABAAAAAwEXAAQAAAADAAABdgE9AAMAAAABAAIAAAAAAAAACAAIAAgAAAAIAAAAZgAAAMQAAABdAAAAXgAAACE=",
    ),
    (
        "grey old-style LZW",
        9,
        7,
        0x8F4A_BAC6,
        "SUkqAFIAAAAA08hqxkwZNmLCat059evYN2nQggnSxIuas3rkoM2K9IhWsnrfxOnS9KiTLHDunGHTxYhRJ2bN5oGrpewSIFzi0sV7Vq0UoikBAQkAAAEEAAEAAAAJAAAAAQEEAAEAAAAHAAAAAgEDAAEAAAAIAAAAAwEDAAEAAAAFAAAABgEDAAEAAAABAAAAEQEEAAEAAAAIAAAAFQEDAAEAAAABAAAAFgEEAAEAAAAHAAAAFwEEAAEAAABKAAAAAAAAAA==",
    ),
    (
        "RGBA 16-bit associated tiles deflate",
        9,
        7,
        0xC15D_8FD1,
        "SUkqAB4CAAB42u3SO0vrcBjH8S4OR0XI4FKE4gUpkkWDeAMHFQcXh7RQ0alOBfcOLqWLHYqhKhQiEZcSlyqmDZYQjC2pbUL+SQi9JDg0hHqJl2Ip2c/pW8h2oJ838OX58UQiIyPVaiRCUV7vzw9FAeDzfXwAIEljYxwnSaK4tHRzI4q67vPxvK7zPAxnszxfKgWD19elEk1vbh4f07THpWjU4zGMaJQkp6be3kiS4yCo0eC4SsXv1/VKpdVaWKhWWy1N8/nKZU1T1bm5XE5Vi0UUvbgoFuPxxUWSjMfd9nH8zx9BwHGWHR5+fWXZWm10FIBaTZYRpNORZceZngbAcdptGLasdltVl5dfXlQ1l5uff3jI5VKpgwOSTKXc9jHM6316wrB8fnz89jafFwQYVhRBcJz19WLRcfr3G0arZVlbW8/PlsUwodDXF8Pg+Npao4HjGLa/f3+PYW77BDE0dHpKEBSFII+PFGWa/Z1Ns9NZXZWk/gIzM7Yty7q+sdH/AobZ3f39ZZhkMhxuNpP/7O1pWjLpvg9BJycEIUkrK1dXkgTA5CTLAtDrTUzwfK9nmih6d2eaNB0Klcs0LYrB4Pe3KKbTR0cApNOx2OFhsxmLue0XChCEooWCZfn9Z2eWZdsIcn5u293u7OzlZberKIFANqso9frOjijW65nM9rZhZDKJRDj8/p5IoGgg8PmJop6BgYGB/8xf/WgyaAALAAABBAABAAAACQAAAAEBBAABAAAABwAAAAIBAwAEAAAAqAIAAAMBAwABAAAACAAAAAYBAwABAAAAAgAAABUBAwABAAAABAAAAEIBBAABAAAAEAAAAEMBBAABAAAAEAAAAEQBBAABAAAACAAAAEUBBAABAAAAFQIAAFIBAwABAAAAAQAAAAAAAAAQABAAEAAQAA==",
    ),
    (
        "palette 4-bit packbits fill order 2",
        9,
        7,
        0xE973_1A4E,
        "SUkqACwAAABEgAiMpAJASIykDADARMwKiMQsygrAyCRiCsiEqmoGSEwqYgoLAAABBAABAAAACQAAAAEBBAABAAAABwAAAAIBAwABAAAABAAAAAMBAwABAAAABYAAAAYBAwABAAAAAwAAAAoBAwABAAAAAgAAABEBBAABAAAACAAAABUBAwABAAAAAQAAABYBBAABAAAABwAAABcBBAABAAAAJAAAAEABAwAwAAAAtgAAAAAAAAD1UCpDVqMugUoqS427GazHKVcM9fApFEFl6Zv4s/PePimcJenmt8bJbi1X0FzCq3Snp0vuGdx8v2xjy+gApNMz3ZBXoA5tDuyIMjdFrYjCF7RxVwK19FbMkqEzXAvcbas=",
    ),
    (
        "BigTIFF orientation 6",
        7,
        9,
        0xA5C0_1205,
        "SUkrAAgAAADOAAAAAAAAAGkLyLIa7c0f6MwOv8o3qNgfxMQsrcJVoLVDe3cA2aci5r8R1sco2N8yyNIfxdAqs8FTlII1p5oJybwK5dQMzc4z7vUjzeQs4tA427Mxt5Fcp48awLQVrsksz/U+wd8y2eJExrpY65o71o9bsZ0FjrIzvuAfxe46zM4k6dhCyrpP8Ixi14xZ0p0Rh8w5ns0hvPMcxOBTr7VYxcpV7JdszYBj17gRU+Iokukzk/Emm89UrdVKyqVG2Yhi51NU6gAKAAAAAAAAAAABBAABAAAAAAAAAAkAAAAAAAAAAQEEAAEAAAAAAAAABwAAAAAAAAACAQMAAwAAAAAAAAAIAAgACAAAAAMBAwABAAAAAAAAAAEAAAAAAAAABgEDAAEAAAAAAAAAAgAAAAAAAAARARAAAQAAAAAAAAAQAAAAAAAAABIBAwABAAAAAAAAAAYAAAAAAAAAFQEDAAEAAAAAAAAAAwAAAAAAAAAWAQQAAQAAAAAAAAAHAAAAAAAAABcBEAABAAAAAAAAAL0AAAAAAAAAAAAAAAAAAAA=",
    ),
];

fn pillow(index: usize) -> Vec<u8> {
    base64::decode(PILLOW[index].4).unwrap()
}

/// A little-endian file of one strip, `data`, with the given `(tag, type,
/// values)` entries (sorted) followed by `StripOffsets` and
/// `StripByteCounts`.
fn file(entries: &[(u16, u16, &[u32])], data: &[u8]) -> Vec<u8> {
    file_with(entries, data, (STRIP_OFFSETS, STRIP_BYTE_COUNTS), 8)
}

/// A little-endian file of one strip or tile, `data`, with the given
/// entries followed by the offset and byte count tags `chunk_tags`, the
/// offset recorded being `offset` (`data` is at 8).
fn file_with(
    entries: &[(u16, u16, &[u32])],
    data: &[u8],
    chunk_tags: (u16, u16),
    offset: u32,
) -> Vec<u8> {
    let directory = 8 + data.len().next_multiple_of(2);
    let count = entries.len() + 2;
    let mut values_at = directory + 2 + 12 * count + 4;
    let mut out = b"II*\0".to_vec();
    out.extend_from_slice(&u32::try_from(directory).unwrap().to_le_bytes());
    out.extend_from_slice(data);
    out.resize(directory, 0);
    out.extend_from_slice(&u16::try_from(count).unwrap().to_le_bytes());
    let length = u32::try_from(data.len()).unwrap();
    let strip = [
        (chunk_tags.0, 4, vec![offset]),
        (chunk_tags.1, 4, vec![length]),
    ];
    let mut values = Vec::new();
    for (tag, kind, items) in entries
        .iter()
        .map(|&(tag, kind, items)| (tag, kind, items.to_vec()))
        .chain(strip)
    {
        let packed: Vec<u8> = items
            .iter()
            .flat_map(|&v| match kind {
                3 => u16::try_from(v).unwrap().to_le_bytes().to_vec(),
                _ => v.to_le_bytes().to_vec(),
            })
            .collect();
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&u32::try_from(items.len()).unwrap().to_le_bytes());
        if packed.len() <= 4 {
            out.extend_from_slice(&packed);
            out.resize(out.len() + 4 - packed.len(), 0);
        } else {
            out.extend_from_slice(&u32::try_from(values_at).unwrap().to_le_bytes());
            values_at += packed.len();
            values.extend_from_slice(&packed);
        }
    }
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&values);
    out
}

#[test]
fn pillow_files_decode_exactly() {
    for (index, &(label, width, height, checksum, _)) in PILLOW.iter().enumerate() {
        let image = decode(&pillow(index)).unwrap();
        assert_eq!((image.width(), image.height()), (width, height), "{label}");
        assert_eq!(crc32(&pil_to_array_bytes(&image)), checksum, "{label}");
    }
}

#[test]
fn encoder_output_round_trips() {
    let pixels: Vec<u8> = (0..5 * 3 * 4).map(|i| (i * 11 % 256) as u8).collect();
    let image = decode(&encode_rgba8(5, 3, &pixels, 96.0).unwrap()).unwrap();
    assert_eq!((image.width(), image.height()), (5, 3));
    assert_eq!(
        image.pixels(),
        &Pixels::Rgba(pixels.as_chunks::<4>().0.to_vec())
    );
}

#[test]
fn hand_built_layouts() {
    let gray16 = file(
        &[
            (256, 4, &[2]),
            (257, 4, &[1]),
            (258, 3, &[16]),
            (262, 3, &[1]),
        ],
        &[0x34, 0x12, 0xFF, 0x00],
    );
    assert_eq!(
        decode(&gray16).unwrap().pixels(),
        &Pixels::Gray16(vec![0x1234, 0xFF])
    );
    // White is zero, bits reversed: 0b0000_0101 reads 0b1010_0000.
    let bilevel = file(
        &[
            (256, 4, &[3]),
            (257, 4, &[1]),
            (258, 3, &[1]),
            (262, 3, &[0]),
            (266, 3, &[2]),
        ],
        &[0b0000_0101],
    );
    assert_eq!(
        decode(&bilevel).unwrap().pixels(),
        &Pixels::Bilevel(vec![0, 255, 0])
    );
    let palette = file(
        &[
            (256, 4, &[2]),
            (257, 4, &[1]),
            (258, 3, &[1]),
            (262, 3, &[3]),
            (320, 3, &[0x1000, 0xFFFF, 0x2000, 0, 0x3000, 0x0100]),
        ],
        &[0b0100_0000],
    );
    let Pixels::Indexed { palette, indices } = decode(&palette).unwrap().into_pixels() else {
        panic!("a palette image");
    };
    assert_eq!(indices, [0, 1]);
    assert_eq!(palette.colors(), [[0x10, 0x20, 0x30], [0xFF, 0, 1]]);
}

#[test]
fn unsupported_files_are_refused() {
    let base: [(u16, u16, &[u32]); 3] = [(256, 4, &[1]), (257, 4, &[1]), (262, 3, &[1])];
    let with = |extra: (u16, u16, &'static [u32])| {
        let mut entries = base.to_vec();
        entries.push(extra);
        entries.sort_by_key(|e| e.0);
        decode(&file(&entries, &[0]))
    };
    assert_eq!(
        with((259, 3, &[7])),
        Err(DecodeError::UnsupportedCompression(7))
    );
    assert_eq!(
        with((284, 3, &[2])),
        Err(DecodeError::UnsupportedPlanarConfiguration)
    );
    assert_eq!(with((258, 3, &[32])), Err(DecodeError::UnsupportedLayout));
    assert_eq!(with((277, 3, &[7])), Err(DecodeError::UnsupportedLayout));
    assert!(with((258, 3, &[8])).is_ok());
    let no_width = file(&[(257, 4, &[1]), (262, 3, &[1])], &[0]);
    assert_eq!(decode(&no_width), Err(DecodeError::MissingTag(256)));
    let lzw_4bit_predictor = file(
        &[
            (256, 4, &[2]),
            (257, 4, &[1]),
            (258, 3, &[4]),
            (259, 3, &[5]),
            (262, 3, &[1]),
            (317, 3, &[2]),
        ],
        &[0x80],
    );
    assert_eq!(
        decode(&lzw_4bit_predictor),
        Err(DecodeError::UnsupportedPredictor(2))
    );
    assert_eq!(decode(b"II*\0"), Err(DecodeError::NotTiff));
    assert_eq!(decode(b"MM\0*\0\0\0\0"), Err(DecodeError::NoImage));
}

#[test]
fn huge_declared_sizes_are_refused_before_allocating() {
    let huge = file(
        &[(256, 4, &[65_535]), (257, 4, &[65_535]), (262, 3, &[1])],
        &[0],
    );
    assert!(matches!(decode(&huge), Err(DecodeError::TooLarge { .. })));
    let packed = file(
        &[
            (256, 4, &[100_000]),
            (257, 4, &[1000]),
            (258, 3, &[8]),
            (259, 3, &[32_773]),
            (262, 3, &[1]),
        ],
        &[0x81, 0],
    );
    assert!(matches!(decode(&packed), Err(DecodeError::TooLarge { .. })));
    let tiles = file_with(
        &[
            (256, 4, &[1]),
            (257, 4, &[1]),
            (258, 3, &[8]),
            (259, 3, &[8]),
            (262, 3, &[1]),
            (322, 4, &[1 << 30]),
            (323, 4, &[1 << 30]),
        ],
        &[0],
        (TILE_OFFSETS, TILE_BYTE_COUNTS),
        8,
    );
    assert_eq!(
        decode(&tiles),
        Err(DecodeError::CorruptData),
        "a tile of 2^60 bytes"
    );
    let gray = [
        (256, 4, &[4][..]),
        (257, 4, &[4]),
        (258, 3, &[8]),
        (262, 3, &[1]),
    ];
    let far = file_with(&gray, &[0; 16], (STRIP_OFFSETS, STRIP_BYTE_COUNTS), 1 << 20);
    assert_eq!(decode(&far), Err(DecodeError::Truncated));
    let at_end = file_with(&gray, &[0; 16], (STRIP_OFFSETS, STRIP_BYTE_COUNTS), 120);
    assert_eq!(decode(&at_end), Err(DecodeError::Truncated));
}

#[test]
fn truncations_and_corruptions_never_panic() {
    let mut rng = Rng::new(23);
    for index in 0..PILLOW.len() {
        let data = pillow(index);
        for len in 0..data.len() {
            let _ = decode(&data[..len]);
        }
    }
    for round in 0..4000 {
        let data = pillow(round % PILLOW.len());
        let _ = decode(&mutate(&data, &mut rng));
    }
}
