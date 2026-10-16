//! Decodes the PNG fixtures written by `fixtures/generate.py` and checks the
//! raw samples and RGBA conversions against the expectations stored with
//! them.

use std::fs;
use std::path::{Path, PathBuf};

use plt_codec::checksum::crc32;
use plt_codec::png::{
    self, BitDepth, ColorType, DecodedImage, ImageRef, PhysicalDimensions, PhysicalUnit, TextEntry,
};

struct Fixture {
    name: String,
    png: Vec<u8>,
    expect: Vec<u8>,
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/png")
}

fn fixtures() -> Vec<Fixture> {
    let mut fixtures: Vec<Fixture> = fs::read_dir(fixture_dir())
        .expect("fixture directory exists")
        .map(|entry| entry.expect("readable directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .map(|path| Fixture {
            name: path
                .file_stem()
                .expect("fixture files have names")
                .to_string_lossy()
                .into_owned(),
            png: fs::read(&path).expect("readable fixture"),
            expect: fs::read(path.with_extension("expect"))
                .expect("every fixture has expectations"),
        })
        .collect();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    fixtures
}

fn fixture(name: &str) -> DecodedImage {
    let png = fs::read(fixture_dir().join(format!("{name}.png"))).expect("readable fixture");
    png::decode(&png).expect("fixture decodes")
}

#[test]
fn every_fixture_decodes_to_the_expected_pixels() {
    let fixtures = fixtures();
    assert!(
        fixtures.len() >= 47,
        "only {} fixtures found",
        fixtures.len()
    );
    for Fixture { name, png, expect } in &fixtures {
        let image = png::decode(png).unwrap_or_else(|e| panic!("{name}: {e}"));
        let raw = image.data().len();
        let pixels = image.width() as usize * image.height() as usize;
        assert_eq!(
            expect.len(),
            raw + 4 * pixels + 8 * pixels,
            "{name}: expectation size"
        );
        assert_eq!(image.data(), &expect[..raw], "{name}: raw samples");
        assert_eq!(
            image.to_rgba8(),
            &expect[raw..raw + 4 * pixels],
            "{name}: RGBA8"
        );
        let (pairs, _) = expect[raw + 4 * pixels..].as_chunks::<2>();
        let rgba16: Vec<u16> = pairs.iter().map(|&pair| u16::from_le_bytes(pair)).collect();
        assert_eq!(image.to_rgba16(), rgba16, "{name}: RGBA16");
    }
}

/// Parses names like `grayalpha16_5x5_interlaced` into the format they
/// promise.
fn promised_format(name: &str) -> Option<(ColorType, BitDepth, bool)> {
    let interlaced = name.ends_with("_interlaced");
    let digits = name.find(|c: char| c.is_ascii_digit())?;
    let color = match &name[..digits] {
        "gray" => ColorType::Gray,
        "grayalpha" => ColorType::GrayAlpha,
        "rgb" => ColorType::Rgb,
        "rgba" => ColorType::Rgba,
        "indexed" => ColorType::Indexed,
        _ => return None,
    };
    let bits: String = name[digits..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    Some((color, BitDepth::from_bits(bits.parse().ok()?)?, interlaced))
}

#[test]
fn fixtures_cover_every_format_and_interlace_mode() {
    let mut covered = Vec::new();
    for Fixture { name, png, .. } in &fixtures() {
        let Some(promised) = promised_format(name) else {
            continue;
        };
        let image = png::decode(png).unwrap();
        let actual = (image.color_type(), image.bit_depth(), image.is_interlaced());
        assert_eq!(actual, promised, "{name}");
        covered.push(actual);
    }
    covered.sort_by_key(|&(color, depth, interlaced)| (format!("{color:?}"), depth, interlaced));
    covered.dedup();
    assert_eq!(covered.len(), 30, "15 formats, each plain and interlaced");
}

#[test]
fn metadata_is_exposed() {
    let text = fixture("text_metadata");
    assert_eq!(
        text.metadata().text,
        [
            TextEntry::new(
                "Software",
                "Matplotlib version3.11.0, https://matplotlib.org/"
            ),
            TextEntry::new("Comment", "café crème"),
            TextEntry::new("Title", "σ² plot"),
            TextEntry::new("Description", "日本語"),
        ]
    );
    assert_eq!(
        text.metadata().physical,
        PhysicalDimensions::from_dpi(100.0)
    );

    let soup = fixture("chunk_soup");
    assert_eq!(
        soup.metadata().physical,
        Some(PhysicalDimensions {
            x: 1000,
            y: 2000,
            unit: PhysicalUnit::Unknown
        })
    );
    assert_eq!(
        soup.metadata().text,
        [TextEntry::new("Comment", "after the data")]
    );

    let pillow = fixture("pillow_rgba");
    assert_eq!(
        pillow.metadata().physical,
        PhysicalDimensions::from_dpi(72.0)
    );
    assert_eq!(pillow.metadata().text[0].keyword, "Software");
    assert!(
        fixture("pillow_palette")
            .palette()
            .is_some_and(|p| p[3][3] == 0)
    );
}

#[test]
fn decoded_fixtures_re_encode_losslessly() {
    for Fixture { name, png, .. } in &fixtures() {
        let image = png::decode(png).unwrap();
        if image.color_type() == ColorType::Indexed {
            continue;
        }
        let view = ImageRef::new(
            image.width(),
            image.height(),
            image.color_type(),
            image.bit_depth(),
            image.data(),
        )
        .unwrap();
        let again = png::decode(&png::encode(view).unwrap()).unwrap();
        assert_eq!(again.data(), image.data(), "{name}");
    }
}

/// Recomputes every chunk CRC so corrupted files reach the deeper checks.
fn fix_crcs(png: &mut [u8]) {
    let mut at = png::SIGNATURE.len();
    while at + 8 <= png.len() {
        let length = u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]) as usize;
        let Some(end) = (at + 8)
            .checked_add(length)
            .filter(|end| end + 4 <= png.len())
        else {
            return;
        };
        let crc = crc32(&png[at + 4..end]);
        png[end..end + 4].copy_from_slice(&crc.to_be_bytes());
        at = end + 4;
    }
}

#[test]
fn corrupted_fixtures_never_panic() {
    let mut state = 0x1234_5678_9ABC_DEF0_u64;
    let mut next = move |bound: usize| {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % bound as u64) as usize
    };
    for Fixture { png, .. } in &fixtures() {
        for _ in 0..60 {
            let mut corrupted = png.clone();
            for _ in 0..=next(3) {
                let at = next(corrupted.len());
                corrupted[at] = next(256) as u8;
            }
            fix_crcs(&mut corrupted);
            if let Ok(image) = png::Decoder::new()
                .max_image_bytes(1 << 24)
                .decode(&corrupted)
            {
                let pixels = image.width() as usize * image.height() as usize;
                assert_eq!(image.to_rgba8().len(), 4 * pixels);
            }
        }
    }
}
