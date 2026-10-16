//! Robustness: truncated and corrupted fonts must produce errors (or
//! degraded results), never panics.

mod common;

use std::path::PathBuf;

use common::font_bytes;
use plt_font::{Face, FontChain, GlyphId, OutlineRecorder, ShapeOptions, Size, face_count, shape};

/// `SplitMix64`, for deterministic corruption positions.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// Runs every public entry point that reads font data.
fn exercise(data: &[u8]) {
    let Ok(count) = face_count(data) else { return };
    for index in 0..count.min(2) {
        let Ok(face) = Face::parse(data, index) else {
            continue;
        };
        let _ = (
            face.family_name(),
            face.style_name(),
            face.full_name(),
            face.postscript_name(),
        );
        let _ = (
            face.ascender(),
            face.descender(),
            face.height(),
            face.style_flags(),
            face.mappings().len(),
        );
        let size = Size::new(&face, 12.0, 72.0);
        let glyphs = u32::from(face.num_glyphs());
        for glyph in (0..glyphs.min(40)).chain((0..glyphs).step_by(97)) {
            let glyph = GlyphId(glyph as u16);
            let mut sink = OutlineRecorder::default();
            let _ = face.outline(glyph, &mut sink);
            let _ = size.glyph_cbox(&face, glyph);
            let _ = (
                face.glyph_name(glyph),
                face.glyph_advance(glyph),
                face.kerning(glyph, GlyphId(36)),
            );
        }
        let _ = face.glyph_index_by_name("A");
        let text = "AVATAR office fi éx\u{301} αβγ \u{2212}1";
        let _ = shape(&face, text);
        let _ = FontChain::new(vec![face]).layout(text, 10.0, 100.0, &ShapeOptions::default());
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut fonts: Vec<(String, Vec<u8>)> = [
        "DejaVuSans.ttf",
        "STIXGeneral.ttf",
        "cmr10.ttf",
        "LastResortHE-Regular.ttf",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), font_bytes(name)))
    .collect();
    for name in ["synthetic-cff.otf", "synthetic.ttc"] {
        fonts.push((
            name.to_owned(),
            std::fs::read(fixture_path(name)).expect("fixture font"),
        ));
    }
    fonts
}

#[test]
fn truncated_fonts_do_not_panic() {
    for (name, data) in corpus() {
        let step = (data.len() / 60).max(1);
        for len in (0..data.len())
            .step_by(step)
            .chain([1, 4, 11, 12, 13, 27, 28])
        {
            exercise(&data[..len.min(data.len())]);
        }
        assert!(
            Face::parse(&data[..12.min(data.len())], 0).is_err(),
            "{name}"
        );
    }
}

#[test]
fn corrupted_fonts_do_not_panic() {
    let mut rng = Rng(0x5EED);
    for (_, data) in corpus() {
        for _ in 0..60 {
            let mut copy = data.clone();
            for _ in 0..=rng.below(8) {
                let at = rng.below(copy.len());
                copy[at] = rng.next() as u8;
            }
            exercise(&copy);
        }
    }
}

#[test]
fn table_directory_corruption_does_not_panic() {
    let mut rng = Rng(42);
    for (_, data) in corpus() {
        // The first kilobyte holds the directory and small header tables,
        // where corruption reaches the most code.
        for _ in 0..120 {
            let mut copy = data.clone();
            let at = rng.below(copy.len().min(1024));
            copy[at] = rng.next() as u8;
            exercise(&copy);
        }
    }
}

/// Byte range of table `tag` according to the table directory.
fn table_range(data: &[u8], tag: [u8; 4]) -> Option<(usize, usize)> {
    let count = usize::from(u16::from_be_bytes([data[4], data[5]]));
    (0..count).find_map(|i| {
        let record = &data[12 + 16 * i..28 + 16 * i];
        let word = |at: usize| {
            u32::from_be_bytes([record[at], record[at + 1], record[at + 2], record[at + 3]])
                as usize
        };
        (record[..4] == tag).then_some((word(8), word(12)))
    })
}

#[test]
fn layout_and_outline_table_corruption_does_not_panic() {
    let mut rng = Rng(7);
    let fonts = [
        font_bytes("DejaVuSans.ttf"),
        font_bytes("STIXGeneral.ttf"),
        std::fs::read(fixture_path("synthetic-cff.otf")).unwrap(),
    ];
    for data in &fonts {
        for tag in [
            *b"GSUB", *b"GPOS", *b"GDEF", *b"cmap", *b"kern", *b"CFF ", *b"glyf", *b"post",
            *b"name",
        ] {
            let Some((offset, length)) = table_range(data, tag) else {
                continue;
            };
            for _ in 0..25 {
                let mut copy = data.clone();
                for _ in 0..=rng.below(6) {
                    // Headers and offset arrays sit near the table start.
                    let window = if rng.below(2) == 0 {
                        length.min(256)
                    } else {
                        length
                    };
                    let at = offset + rng.below(window.max(1));
                    copy[at] = rng.next() as u8;
                }
                exercise(&copy);
            }
        }
    }
}
