//! Subsets of the fixtures' CFF and CFF2 tables draw their glyphs as the
//! originals do; a small font built here exercises the flattening.

use super::*;
use crate::cff::CffScale;
use crate::face::Face;
use crate::outline::Outline;

/// The `CFF `/`CFF2` table of fixture `file`, its version and the face's
/// units per em.
fn fixture(file: &str) -> (Vec<u8>, Version, u16) {
    let path = format!("{}/tests/fixtures/{file}", env!("CARGO_MANIFEST_DIR"));
    let data = std::fs::read(path).unwrap();
    let face = Face::parse(&data, 0).unwrap();
    let upem = face.units_per_em();
    for version in [Version::Cff2, Version::Cff] {
        if let Some(table) = face.table(version.tag()) {
            return (table.to_vec(), version, upem);
        }
    }
    panic!("{file} has no CFF table");
}

/// The outline and advance of `glyph` in font units.
fn outline(cff: &Cff<'_>, glyph: GlyphId, upem: u16) -> (Outline, i64) {
    cff.load(glyph, 500, CffScale::Units, upem).unwrap()
}

fn assert_same_glyphs(file: &str, kept: impl Fn(usize) -> bool) {
    let (data, version, upem) = fixture(file);
    let subsetter = CffSubsetter::new(&data, version, upem).unwrap();
    let original = Cff::parse(&data, version, upem, &[]).unwrap();
    let mut kept: Vec<GlyphId> = (0..original.charstrings.len())
        .filter(|&g| kept(g))
        .map(|g| GlyphId(g as u16))
        .collect();
    // With the `seac` components the subset keeps.
    let components: Vec<GlyphId> = kept
        .iter()
        .flat_map(|&g| subsetter.components(g).unwrap())
        .collect();
    kept.extend(components);
    kept.sort_unstable();
    kept.dedup();
    let table = subsetter.table(&kept).unwrap();
    let subset = Cff::parse(&table, version, upem, &[]).unwrap();
    assert_eq!(subset.charstrings.len(), kept.len());
    assert_eq!(subset.global_subrs.len(), 0);
    for (new, &old) in kept.iter().enumerate() {
        let new = GlyphId(new as u16);
        let (drawn, advance) = outline(&subset, new, upem);
        let (expected, expected_advance) = outline(&original, old, upem);
        assert_eq!(drawn, expected, "{file} glyph {}", old.0);
        assert_eq!(advance, expected_advance);
        assert_eq!(subset.glyph_name(new), original.glyph_name(old));
        assert!(subset.subfonts.iter().all(|font| font.subrs.len() == 0));
    }
}

#[test]
fn cff_subsets_draw_as_the_original() {
    assert_same_glyphs("synthetic-cff.otf", |_| true);
    assert_same_glyphs("synthetic-cff.otf", |g| g == 0 || g % 2 == 1);
}

#[test]
fn cff2_subsets_draw_as_the_original() {
    assert_same_glyphs("synthetic-var.otf", |_| true);
    assert_same_glyphs("synthetic-var.otf", |g| g != 1);
    // The variation store stays: the subset still varies.
    let (data, version, upem) = fixture("synthetic-var.otf");
    let table = CffSubsetter::new(&data, version, upem)
        .unwrap()
        .table(&[GlyphId(0), GlyphId(2)])
        .unwrap();
    let file = std::fs::read(format!(
        "{}/tests/fixtures/synthetic-var.otf",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let axes = Face::parse(&file, 0).unwrap().variation_axes().len();
    let coords = vec![0x10000; axes];
    let varied = Cff::parse(&table, version, upem, &coords).unwrap();
    let original = Cff::parse(&data, version, upem, &coords).unwrap();
    assert_eq!(
        outline(&varied, GlyphId(1), upem).0.points,
        outline(&original, GlyphId(2), upem).0.points
    );
}

#[test]
fn seac_components() {
    let (data, version, upem) = fixture("synthetic-cff.otf");
    let subsetter = CffSubsetter::new(&data, version, upem).unwrap();
    let cff = &subsetter.cff;
    let named = |name: &str| {
        (0..cff.charstrings.len() as u16)
            .map(GlyphId)
            .find(|&g| cff.glyph_name(g) == Some(name))
            .unwrap()
    };
    assert_eq!(
        subsetter.components(named("Aacute")).unwrap(),
        [named("A"), named("acute")]
    );
    assert_eq!(subsetter.components(named("A")).unwrap(), []);
}

fn n(value: i32) -> u8 {
    (value + 139) as u8
}

/// A name-keyed CFF (`ISOAdobe` charset) of `glyphs` with the local
/// subroutines `subrs`.
fn font(glyphs: &[&[u8]], subrs: &[&[u8]]) -> Vec<u8> {
    let names = index([b"T".as_slice()], CountSize::Short);
    let charstrings = index(glyphs.iter().copied(), CountSize::Short);
    let private = entry(OP_SUBRS, &[entry_length(OP_SUBRS, 1)]);
    let top_length = entry_length(OP_CHARSTRINGS, 1) + entry_length(OP_PRIVATE, 2);
    let charstrings_at = 4 + names.len() + index_length([top_length], CountSize::Short) + 4;
    let private_at = charstrings_at + charstrings.len();
    let mut top = entry(OP_CHARSTRINGS, &[charstrings_at]);
    top.extend(entry(OP_PRIVATE, &[private.len(), private_at]));
    let mut out = vec![1, 0, 4, 4];
    out.extend(names);
    out.extend(index([top.as_slice()], CountSize::Short));
    out.extend(index([], CountSize::Short));
    out.extend(index([], CountSize::Short));
    assert_eq!(out.len(), charstrings_at);
    out.extend(charstrings);
    out.extend(private);
    out.extend(index(subrs.iter().copied(), CountSize::Short));
    out
}

/// Glyph 1 of a font of `glyph` and `subrs`, flattened.
fn flattened(glyph: &[u8], subrs: &[&[u8]]) -> Result<Flattened> {
    let data = font(&[&[14], glyph], subrs);
    CffSubsetter::new(&data, Version::Cff, 1000)?.flattened(GlyphId(1))
}

#[test]
fn subroutines_are_inlined() {
    // rmoveto, callsubr 0 (bias 107), endchar; the subroutine draws and
    // returns.
    let glyph = [n(0), n(0), 21, n(-107), 10, 14];
    let subr: &[u8] = &[n(10), 6, 11];
    let flat = flattened(&glyph, &[subr]).unwrap();
    assert_eq!(flat.program, [n(0), n(0), 21, n(10), 6, 14]);
    assert_eq!(flat.seac, None);
    // A hint mask inside a subroutine keeps its mask bytes: one
    // horizontal and one implied vertical stem.
    let glyph = [n(0), n(10), 1, n(-107), 10, 14];
    let subr: &[u8] = &[n(0), n(10), 19, 0xC0, 11];
    let flat = flattened(&glyph, &[subr]).unwrap();
    assert_eq!(flat.program, [n(0), n(10), 1, n(0), n(10), 19, 0xC0, 14]);
    // `endchar` with four operands implies a `seac` of codes 65 and 194.
    let glyph = [n(0), n(1), n(65), 247, 86, 14];
    let flat = flattened(&glyph, &[]).unwrap();
    assert_eq!(flat.seac, Some((65, 194)));
}

#[test]
fn subroutines_that_cannot_be_inlined() {
    let calls_itself: &[u8] = &[n(-107), 10, 11];
    // An index computed by `add`, a missing subroutine, endless nesting,
    // a `return` outside a subroutine and a mask cut short.
    assert!(flattened(&[n(-108), n(1), 12, 10, 10, 14], &[calls_itself]).is_err());
    assert!(flattened(&[n(-106), 10, 14], &[calls_itself]).is_err());
    assert!(flattened(&[n(-107), 10, 14], &[calls_itself]).is_err());
    assert!(flattened(&[n(0), 11], &[]).is_err());
    assert!(flattened(&[n(0), n(10), 1, n(0), n(10), 19], &[]).is_err());
    assert!(flattened(&[10, 14], &[]).is_err());
}
