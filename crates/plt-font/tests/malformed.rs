//! Robustness: truncated and corrupted fonts must produce errors (or
//! degraded results), never panics.

mod common;

use std::path::PathBuf;

use common::font_bytes;
use plt_font::{
    Face, FontChain, GlyphId, OutlineRecorder, ShapeOptions, Size, face_count, shape, subset,
};

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

/// Directory records `(record offset, table offset, table length)`.
fn records(data: &[u8]) -> Vec<(usize, usize, usize)> {
    let count = usize::from(u16::from_be_bytes([data[4], data[5]]));
    let word = |k: usize| u32::from_be_bytes([data[k], data[k + 1], data[k + 2], data[k + 3]]);
    (0..count)
        .map(|i| {
            let at = 12 + 16 * i;
            (at, word(at + 8) as usize, word(at + 12) as usize)
        })
        .collect()
}

fn small_corpus() -> Vec<Vec<u8>> {
    vec![
        font_bytes("cmr10.ttf"),
        std::fs::read(fixture_path("synthetic-cff.otf")).expect("fixture font"),
    ]
}

#[test]
fn random_tables_do_not_panic() {
    let mut rng = Rng(0xF017);
    let mut fonts = small_corpus();
    fonts.push(font_bytes("DejaVuSans.ttf"));
    for data in &fonts {
        for (_, offset, length) in records(data) {
            let mut copy = data.clone();
            let end = (offset + length).min(copy.len());
            for byte in &mut copy[offset.min(end)..end] {
                *byte = rng.next() as u8;
            }
            exercise(&copy);
        }
    }
}

#[test]
fn table_offsets_past_the_end_do_not_panic() {
    for data in small_corpus() {
        let len = data.len() as u32;
        for (at, offset, _) in records(&data) {
            for (new_offset, new_length) in [
                (len, 16),
                (len - 2, 64),
                (u32::MAX - 4, 8),
                (offset as u32, u32::MAX),
                (offset as u32, 0),
            ] {
                let mut copy = data.clone();
                copy[at + 8..at + 12].copy_from_slice(&new_offset.to_be_bytes());
                copy[at + 12..at + 16].copy_from_slice(&new_length.to_be_bytes());
                exercise(&copy);
            }
        }
    }
}

/// The `glyf` offset, every glyph's record offset (from `loca`) and whether
/// `loca` uses the long format.
fn glyph_offsets(data: &[u8]) -> (usize, Vec<usize>, bool) {
    let (head, _) = table_range(data, *b"head").expect("head");
    let (loca, loca_len) = table_range(data, *b"loca").expect("loca");
    let (glyf, _) = table_range(data, *b"glyf").expect("glyf");
    let long = data[head + 50..head + 52] == [0, 1];
    let table = &data[loca..loca + loca_len];
    let offsets = if long {
        table
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&c| u32::from_be_bytes(c) as usize)
            .collect()
    } else {
        table
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&c| usize::from(u16::from_be_bytes(c)) * 2)
            .collect()
    };
    (glyf, offsets, long)
}

fn outline_all(data: &[u8], glyphs: &[u16]) {
    let face = Face::parse(data, 0).expect("directory intact");
    for &glyph in glyphs {
        let _ = face.outline(GlyphId(glyph), &mut OutlineRecorder::default());
        let _ = face.glyph_bbox(GlyphId(glyph));
    }
}

#[test]
fn composite_cycles_do_not_hang() {
    let data = font_bytes("DejaVuSans.ttf");
    let (glyf, offsets, _) = glyph_offsets(&data);
    // Composite glyphs (negative contour count); the first component's
    // glyph index sits at record offset 12.
    let composites: Vec<u16> = (0..offsets.len() - 1)
        .filter(|&g| offsets[g + 1] > offsets[g] + 14 && data[glyf + offsets[g]] & 0x80 != 0)
        .map(|g| g as u16)
        .take(40)
        .collect();
    assert!(composites.len() > 10);
    let mut copy = data.clone();
    for pair in composites.chunks(2) {
        // Two glyphs pointing at each other, or a lone glyph at itself.
        for (i, &glyph) in pair.iter().enumerate() {
            let partner = pair[(i + 1) % pair.len()];
            let at = glyf + offsets[usize::from(glyph)] + 12;
            copy[at..at + 2].copy_from_slice(&partner.to_be_bytes());
        }
    }
    outline_all(&copy, &composites);
}

/// A `GSUB` table whose `ccmp` feature runs lookup 0, a chaining-context
/// lookup that applies itself eight times per nesting level, then lookups
/// 1 to 6, each a multiple substitution turning `glyph` into 64 copies of
/// itself.
fn hostile_gsub(glyph: u16) -> Vec<u8> {
    const LOOKUPS: u16 = 7;
    let mut t = Vec::new();
    let mut put = |values: &[u16]| t.extend(values.iter().flat_map(|v| v.to_be_bytes()));
    let tag = |b: &[u8; 4]| {
        [
            u16::from_be_bytes([b[0], b[1]]),
            u16::from_be_bytes([b[2], b[3]]),
        ]
    };
    let lookup_list = 42 + 2 * LOOKUPS;
    // Header, script list (DFLT → default LangSys → feature 0) and feature
    // list (ccmp → lookups 0..7).
    put(&[1, 0, 10, 30, lookup_list]);
    put(&[1]);
    put(&tag(b"DFLT"));
    put(&[8, 4, 0, 0, 0xFFFF, 1, 0]);
    put(&[1]);
    put(&tag(b"ccmp"));
    put(&[8, 0, LOOKUPS]);
    put(&(0..LOOKUPS).collect::<Vec<_>>());
    // Lookup list: lookup 0, then six offsets to one shared lookup.
    let header = 2 + 2 * LOOKUPS;
    put(&[LOOKUPS, header]);
    put(&[header + 58; LOOKUPS as usize - 1]);
    // Chaining context, format 3: input [glyph], eight records (0, lookup 0).
    put(&[6, 0, 1, 8, 3, 0, 1, 44, 0, 8]);
    put(&[0; 16]);
    put(&[1, 1, glyph]);
    // Multiple substitution: glyph → 64 × glyph.
    put(&[2, 0, 1, 8, 1, 138, 1, 8, 64]);
    put(&[glyph; 64]);
    put(&[1, 1, glyph]);
    t
}

#[test]
fn subsetting_corrupted_fonts_does_not_panic() {
    let mut rng = Rng(0x5B5E7);
    let fonts = [font_bytes("DejaVuSans.ttf"), font_bytes("cmr10.ttf")];
    for data in &fonts {
        for tag in [
            *b"GSUB", *b"glyf", *b"loca", *b"cmap", *b"hmtx", *b"head", *b"maxp",
        ] {
            let Some((offset, length)) = table_range(data, tag) else {
                continue;
            };
            for _ in 0..8 {
                let mut copy = data.clone();
                for _ in 0..=rng.below(4) {
                    let at = offset + rng.below(length.clamp(1, 512));
                    copy[at] = rng.next() as u8;
                }
                let Ok(face) = Face::parse(&copy, 0) else {
                    continue;
                };
                let glyphs = (0..48).chain([400, 1000, 0xFFFF]).map(GlyphId);
                if let Ok(kept) = subset(&face, glyphs) {
                    let _ = Face::parse(&kept.data, 0).map(|face| face.mappings().len());
                }
            }
        }
    }
}

#[test]
fn hostile_substitutions_stay_bounded() {
    let data = font_bytes("DejaVuSans.ttf");
    let (offset, length) = table_range(&data, *b"GSUB").expect("GSUB");
    let glyph = Face::parse(&data, 0)
        .expect("valid font")
        .glyph_index('A')
        .expect("A");
    let gsub = hostile_gsub(glyph.0);
    let mut copy = data.clone();
    copy[offset..offset + length].fill(0);
    copy[offset..offset + gsub.len()].copy_from_slice(&gsub);
    let face = Face::parse(&copy, 0).expect("valid font");
    // Unbounded, the buffer would hold 64^6 glyphs after 8^64 nested steps.
    let glyphs = shape(&face, "A");
    assert!(glyphs.len() > 64 && glyphs.len() <= 16_384);
}

#[test]
fn inconsistent_loca_does_not_panic() {
    let data = font_bytes("DejaVuSans.ttf");
    let (loca, _) = table_range(&data, *b"loca").expect("loca");
    let (_, offsets, long) = glyph_offsets(&data);
    let glyphs: Vec<u16> = (0..120).collect();
    let mut rng = Rng(0x10CA);
    for variant in 0..4 {
        let mut copy = data.clone();
        for slot in 0..121 {
            let value = match variant {
                // Decreasing offsets.
                0 => offsets[offsets.len() - 1 - slot],
                // Past the end of `glyf`.
                1 => 0x7FFF_FFF0,
                // Into the middle of other glyphs.
                2 => offsets[slot] + rng.below(64),
                // All equal: every glyph empty.
                _ => offsets[1],
            };
            if long {
                let at = loca + 4 * slot;
                copy[at..at + 4].copy_from_slice(&(value as u32).to_be_bytes());
            } else {
                let at = loca + 2 * slot;
                copy[at..at + 2].copy_from_slice(&((value / 2) as u16).to_be_bytes());
            }
        }
        outline_all(&copy, &glyphs);
    }
}
