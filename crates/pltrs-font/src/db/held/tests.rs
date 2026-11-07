//! Held files: every face reads as it reads from the whole file, while the
//! glyph programs stay on disk until they are drawn.

use std::fs;

use super::*;
use crate::db::{bundled_font_data, bundled_font_files};
use crate::face::Face as ParsedFace;
use crate::outline::OutlineRecorder;
use crate::subset::subset;
use crate::tables::directory::face_count;
use crate::types::GlyphId;

/// A file of `bytes` in the temporary directory, removed when dropped.
struct TempFont(PathBuf);

impl TempFont {
    fn new(name: &str, bytes: &[u8]) -> Self {
        let path =
            std::env::temp_dir().join(format!("pltrs-font-held-{}-{name}", std::process::id()));
        fs::write(&path, bytes).unwrap();
        Self(path)
    }
}

impl Drop for TempFont {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// The outline commands of every glyph of `face`, or why it has none.
fn outlines(face: &ParsedFace<'_>) -> Vec<std::result::Result<String, FontError>> {
    (0..face.num_glyphs())
        .map(|glyph| {
            let mut recorder = OutlineRecorder::default();
            face.outline(GlyphId(glyph), &mut recorder)
                .map(|bbox| format!("{bbox:?} {:?}", recorder.commands))
        })
        .collect()
}

/// Checks that every face of the file `bytes`, held, reads as it reads
/// from the whole file; returns the held file.
fn assert_holds(name: &str, bytes: &[u8]) -> HeldFile {
    let font = TempFont::new(name, bytes);
    let held = HeldFile::open(&font.0).unwrap();
    for index in 0..face_count(bytes).unwrap() {
        let (whole, lazy) = (
            ParsedFace::parse(bytes, index).unwrap(),
            ParsedFace::parse_held(&held, index).unwrap(),
        );
        assert!(whole.table_tags().eq(lazy.table_tags()), "{name}");
        for tag in whole.table_tags() {
            assert_eq!(lazy.table(tag), whole.table(tag), "{name}: {tag:?}");
        }
        assert_eq!(outlines(&lazy), outlines(&whole), "{name}");
        assert_eq!(lazy.data().unwrap(), bytes, "{name}");
        let some: Vec<GlyphId> = "Ag%é"
            .chars()
            .filter_map(|c| whole.glyph_index(c))
            .collect();
        assert_eq!(
            subset(&lazy, some.iter().copied()).map(|font| font.data),
            subset(&whole, some.iter().copied()).map(|font| font.data),
            "{name}"
        );
    }
    held
}

#[test]
fn bundled_fonts_hold_all_but_their_outlines() {
    for (name, bytes) in bundled_font_files().filter(|(_, bytes)| face_count(bytes).is_ok()) {
        let held = assert_holds(name, bytes);
        let glyf = held
            .table(0, Tag::new(b"glyf"))
            .expect("outlines elsewhere");
        assert!(held.held().len() + glyf.len() <= bytes.len() + 64, "{name}");
    }
}

#[test]
fn glyphs_are_read_as_they_are_drawn() {
    let bytes = bundled_font_data("DejaVuSans.ttf").unwrap();
    let font = TempFont::new("drawn.ttf", bytes);
    let held = HeldFile::open(&font.0).unwrap();
    let face = ParsedFace::parse_held(&held, 0).unwrap();
    assert_eq!(held.pieces.len(), 0, "nothing read but the tables held");
    let a = face.glyph_index('a').unwrap();
    for _ in 0..3 {
        face.outline(a, &mut OutlineRecorder::default()).unwrap();
    }
    assert_eq!(held.pieces.len(), 1, "one glyph, read once");
    let reparsed = ParsedFace::parse_held(&held, 0).unwrap();
    reparsed
        .outline(a, &mut OutlineRecorder::default())
        .unwrap();
    assert_eq!(held.pieces.len(), 1, "kept for later faces");
}

#[test]
fn cff_fonts_hold_all_but_their_charstrings() {
    let bytes = include_bytes!("../../../tests/fixtures/synthetic-cff.otf");
    let held = assert_holds("synthetic.otf", bytes);
    let parts = held.cff(0).expect("split around its CharStrings");
    let table = held.table(0, Tag(*b"CFF ")).unwrap();
    assert_eq!(
        parts.head.len() + parts.charstrings.len() + parts.tail.len(),
        table.len()
    );
    assert!(parts.charstrings.len() > 0);
    let whole = ParsedFace::parse(bytes, 0).unwrap();
    let cff = whole.table(Tag(*b"CFF ")).unwrap();
    assert_eq!(parts.head, &cff[..parts.head.len()]);
    assert_eq!(parts.tail, &cff[cff.len() - parts.tail.len()..]);
}

#[test]
fn collections_hold_every_face() {
    let bytes = include_bytes!("../../../tests/fixtures/synthetic.ttc");
    assert!(face_count(bytes).unwrap() > 1);
    assert_holds("synthetic.ttc", bytes);
}

#[test]
fn other_files_are_held_whole() {
    let text = b"not a font at all, but long enough to have a header".repeat(3);
    let font = TempFont::new("text.ttf", &text);
    let held = HeldFile::open(&font.0).unwrap();
    assert_eq!(held.held(), text);
    assert_eq!(held.elsewhere, []);
    assert!(ParsedFace::parse_held(&held, 0).is_err());
    assert!(HeldFile::open(Path::new("/nonexistent/font.ttf")).is_err());
}

#[test]
fn files_cut_short_after_opening_fail_cleanly() {
    let bytes = bundled_font_data("DejaVuSans.ttf").unwrap();
    let font = TempFont::new("cut.ttf", bytes);
    let held = HeldFile::open(&font.0).unwrap();
    fs::write(&font.0, &bytes[..bytes.len() / 2]).unwrap();
    let face = ParsedFace::parse_held(&held, 0).unwrap();
    let late = GlyphId(face.num_glyphs() - 1);
    assert!(face.outline(late, &mut OutlineRecorder::default()).is_err());
    assert_eq!(held.read(bytes.len() - 10..bytes.len() + 10), None);
    let (start, end) = (20, 10);
    assert_eq!(held.read(start..end), None);
    assert_eq!(held.read(5..5), Some(&[][..]));
}
