//! Robustness of the Type 1 font code: truncated and corrupted files,
//! corrupted private dictionaries and random charstrings must produce
//! errors (or degraded glyphs), never panics or endless loops.

mod common;

use common::fixture_path;
use plt_core::rng::SplitMix64;
use plt_font::{
    GlyphId, OutlineRecorder, Type1Cipher, Type1Effects, Type1Encoding, Type1Face, Type1Font,
    Type1Parts,
};

/// The Type 1 fixtures.
fn corpus() -> Vec<Vec<u8>> {
    ["cmr10.pfb", "Courier10PitchBT-Bold.pfb"]
        .map(|name| std::fs::read(fixture_path(&format!("type1/{name}"))).expect("a fixture"))
        .into()
}

/// Every reading and rewriting of a parsed font.
fn exercise_font(font: &Type1Font) {
    let _ = font.transform(Type1Effects {
        slant: Some(0.25),
        extend: Some(0.8),
    });
    let _ = font.with_encoding(&Type1Encoding::from_codes([(65, "A".to_owned())]));
    let _ = font.subset("Hello, World".chars().map(u32::from), "ABCDEF+");
    let Ok(face) = Type1Face::new(font) else {
        return;
    };
    let _ = (
        face.encoding_vector(),
        face.style_name(),
        face.glyph_index("A"),
    );
    let glyphs = face.num_glyphs();
    let size = face.size(13.0, 96.0);
    for glyph in (0..glyphs.min(24)).chain((0..glyphs).step_by(41)) {
        if let Ok(loaded) = face.load_glyph(GlyphId(glyph)) {
            let mut sink = OutlineRecorder::default();
            loaded.scaled(0x2_0000, 0x2_0000).outline(&mut sink);
            let _ = loaded.control_box();
        }
        if let Ok(hinted) = face.load_glyph_hinted(GlyphId(glyph), &size) {
            hinted.outline(&mut OutlineRecorder::default());
        }
    }
}

fn exercise(data: &[u8]) {
    if let Ok(font) = Type1Font::from_bytes(data) {
        exercise_font(&font);
    }
}

#[test]
fn truncated_files() {
    for data in corpus() {
        let step = (data.len() / 80).max(1);
        for len in (0..data.len()).step_by(step).chain([1, 2, 5, 6, 7]) {
            exercise(&data[..len.min(data.len())]);
        }
        assert!(Type1Font::from_bytes(&data[..data.len() / 2]).is_err());
    }
}

#[test]
fn corrupted_files() {
    let mut rng = SplitMix64::new(0x7E1);
    for data in corpus() {
        for _ in 0..40 {
            let mut copy = data.clone();
            for _ in 0..=rng.below(6) {
                let at = rng.below(copy.len());
                copy[at] = rng.next_u64() as u8;
            }
            exercise(&copy);
        }
    }
}

/// The font with its decrypted private part changed by `edit` and
/// encrypted again: corruption that reaches the dictionaries and the
/// charstrings in the clear.
fn with_private(font: &Type1Font, edit: impl FnOnce(&mut Vec<u8>)) -> Option<Type1Font> {
    let mut private = font.decrypted().to_vec();
    edit(&mut private);
    let parts = Type1Parts {
        cleartext: font.parts().cleartext.clone(),
        encrypted: Type1Font::encrypt(&private, Type1Cipher::Eexec),
        finale: font.parts().finale.clone(),
    };
    Type1Font::from_parts(parts).ok()
}

#[test]
fn corrupted_private_dictionaries() {
    let mut rng = SplitMix64::new(0xDEC);
    for data in corpus() {
        let font = Type1Font::from_bytes(&data).unwrap();
        for _ in 0..60 {
            let corrupted = with_private(&font, |private| {
                for _ in 0..=rng.below(4) {
                    let at = rng.below(private.len());
                    private[at] = rng.next_u64() as u8;
                }
            });
            if let Some(font) = corrupted {
                exercise_font(&font);
            }
        }
    }
}

/// A random charstring biased towards the operators and numbers that
/// reach the interpreter's corners: flex and hint replacement OtherSubrs,
/// `seac`, `div`, subroutine calls and large numbers.
fn random_charstring(rng: &mut SplitMix64) -> Vec<u8> {
    const PIECES: &[&[u8]] = &[
        &[139, 247, 136, 13],
        &[139, 139, 21],
        &[149, 6],
        &[149, 7],
        &[149, 149, 5],
        &[149, 149, 149, 149, 149, 149, 8],
        &[149, 149, 149, 149, 30],
        &[149, 149, 149, 149, 31],
        &[9],
        &[14],
        &[11],
        &[139, 10],
        &[140, 10],
        &[138, 10],
        &[139, 140, 12, 16],
        &[139, 141, 12, 16],
        &[139, 139, 139, 142, 139, 12, 16],
        &[142, 140, 12, 16, 12, 17],
        &[12, 17],
        &[12, 33],
        &[139, 139, 139, 204, 247, 86, 12, 6],
        &[149, 150, 12, 12],
        &[255, 0, 1, 0, 0, 142, 12, 12],
        &[139, 139, 139, 139, 12, 7],
        &[139, 149, 139, 149, 139, 149, 12, 1],
        &[139, 149, 139, 149, 139, 149, 12, 2],
        &[12, 0],
        &[149, 149, 19, 0],
        &[149, 149, 20, 0],
        &[28, 1, 0],
        &[139, 139, 147, 150, 147, 148, 12, 16],
    ];
    let mut code = Vec::new();
    for _ in 0..rng.below(24) {
        if rng.below(8) == 0 {
            code.push(rng.next_u64() as u8);
        } else {
            code.extend_from_slice(PIECES[rng.below(PIECES.len())]);
        }
    }
    code
}

#[test]
fn random_charstrings() {
    let mut rng = SplitMix64::new(0xC5);
    let data = corpus().swap_remove(0);
    let font = Type1Font::from_bytes(&data).unwrap();
    let face = Type1Face::new(&font).unwrap();
    for _ in 0..200 {
        let glyph = GlyphId(1 + rng.below(usize::from(face.num_glyphs()) - 1) as u16);
        let name = face.glyph_name(glyph).unwrap().to_owned();
        let program = font.prop().char_strings().program(&name).unwrap();
        let old = program.encrypted().to_vec();
        let new = Type1Font::encrypt(&random_charstring(&mut rng), Type1Cipher::Charstring);
        let replaced = with_private(&font, |private| {
            let entry = [format!("/{name} {} RD ", old.len()).as_bytes(), &old].concat();
            let at = private
                .windows(entry.len())
                .position(|window| window == entry.as_slice())
                .unwrap();
            let mut text = format!("/{name} {} RD ", new.len()).into_bytes();
            text.extend_from_slice(&new);
            private.splice(at..at + entry.len(), text);
        })
        .unwrap();
        let _ = replaced.subset([65, 66], "X+");
        // An empty program is shorter than `lenIV`: FreeType rejects the font.
        let Ok(face) = Type1Face::new(&replaced) else {
            continue;
        };
        if let Ok(loaded) = face.load_glyph(glyph) {
            let mut sink = OutlineRecorder::default();
            loaded.outline(&mut sink);
        }
        let size = face.size(100.0, 72.0);
        if let Ok(hinted) = face.load_glyph_hinted(glyph, &size) {
            hinted.outline(&mut OutlineRecorder::default());
        }
    }
}
