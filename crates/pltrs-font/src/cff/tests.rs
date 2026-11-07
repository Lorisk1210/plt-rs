//! Tests of CFF tables: a small name-keyed font built here, and the
//! fixtures' FreeType view of fonts generated with fontTools.

use super::*;
use crate::outline::Outline;

/// Encodes an INDEX with 1-byte offsets.
fn index(items: &[&[u8]]) -> Vec<u8> {
    let mut out = (items.len() as u16).to_be_bytes().to_vec();
    if items.is_empty() {
        return out;
    }
    out.push(1);
    let mut offset = 1u8;
    out.push(offset);
    for item in items {
        offset += item.len() as u8;
        out.push(offset);
    }
    for item in items {
        out.extend_from_slice(item);
    }
    out
}

fn n(v: i32) -> u8 {
    (v + 139) as u8
}

/// A name-keyed CFF with glyphs `.notdef`, `A` (a square) and `Aacute`
/// built with the implied `seac` from `A` and `acute`.
fn font() -> Vec<u8> {
    let notdef: &[u8] = &[14];
    let a: &[u8] = &[n(0), n(0), 21, n(10), 6, n(10), 7, n(-10), 6, 14];
    let acute: &[u8] = &[n(0), n(20), 21, n(2), n(2), 5, 14];
    let aacute: &[u8] = &[n(0), n(1), n(5), n(65), 247, 86, 14];
    let charstrings = index(&[notdef, a, acute, aacute]);
    let strings = index(&[b"Aacute"]);
    // charset format 0: SIDs for glyphs 1..: A=34, acute=125, Aacute=391.
    let charset = [0u8, 0, 34, 0, 125, 1, 135];
    let header = [1u8, 0, 4, 1];
    let name = index(&[b"Test"]);
    let globals = index(&[]);
    // Top DICT with two 2-byte-encoded offsets (filled below).
    let top_len = 3 + 1 + 3 + 1;
    let fixed = header.len() + name.len() + 2 + 1 + 2 + top_len + strings.len() + globals.len();
    let charset_at = fixed;
    let charstrings_at = charset_at + charset.len();
    let top = [28, 0, charset_at as u8, 15, 28, 0, charstrings_at as u8, 17];
    let mut out = header.to_vec();
    out.extend(name);
    out.extend(index(&[&top]));
    out.extend(strings);
    out.extend(globals);
    assert_eq!(out.len(), charset_at);
    out.extend_from_slice(&charset);
    out.extend(charstrings);
    out
}

fn load(cff: &Cff<'_>, glyph: u16, scale: CffScale) -> Result<(Outline, i64)> {
    cff.load(GlyphId(glyph), 500, scale, 1000)
}

#[test]
fn names_outlines_and_seac() {
    let data = font();
    let cff = Cff::parse(&data, Version::Cff, 1000, &[]).unwrap();
    assert!(cff.has_glyph_names());
    assert_eq!(cff.glyph_name(GlyphId(0)), Some(".notdef"));
    assert_eq!(cff.glyph_name(GlyphId(1)), Some("A"));
    assert_eq!(cff.glyph_name(GlyphId(2)), Some("acute"));
    assert_eq!(cff.glyph_name(GlyphId(3)), Some("Aacute"));
    let (square, advance) = load(&cff, 1, CffScale::Units).unwrap();
    assert_eq!((square.points.len(), advance), (4, 500));
    // The accent first, at its offset, then the base.
    let (composite, _) = load(&cff, 3, CffScale::Units).unwrap();
    assert_eq!(composite.contour_ends, vec![2, 6]);
    assert_eq!((composite.points[0].x, composite.points[0].y), (1.0, 25.0));
    let double = CffScale::Scaled {
        x_scale: 0x2_0000,
        y_scale: 0x2_0000,
    };
    let (scaled, advance) = load(&cff, 1, double).unwrap();
    assert_eq!((scaled.points[1].x, advance), (20.0, 1000));
    assert!(load(&cff, 4, CffScale::Units).is_err());
}

#[test]
fn hinted_glyphs_are_in_pixels() {
    let data = font();
    let cff = Cff::parse(&data, Version::Cff, 1000, &[]).unwrap();
    // Four 26.6 units per font unit: 62.5 pixels per em of 1000 units.
    let size = CffScale::Hinted {
        x_scale: 4 << 16,
        y_scale: 4 << 16,
    };
    let (square, advance) = load(&cff, 1, size).unwrap();
    assert_eq!(advance, 500 * 4);
    assert_eq!((square.points[1].x, square.points[1].y), (40.0, 0.0));
    let huge = CffScale::Hinted {
        x_scale: 0x10000 * 64 * 3,
        y_scale: 0x10000 * 64 * 3,
    };
    // Too large for the engine: scaled unhinted.
    let (square, _) = load(&cff, 1, huge).unwrap();
    assert_eq!(square.points[1].x, 10.0 * 64.0 * 3.0);
}

#[test]
fn truncated_cff_errors() {
    let data = font();
    for len in 0..data.len() - 1 {
        if let Ok(cff) = Cff::parse(&data[..len], Version::Cff, 1000, &[]) {
            let _ = load(&cff, 3, CffScale::Units);
        }
    }
    assert!(Cff::parse(&[2, 0, 4, 1], Version::Cff, 1000, &[]).is_err());
    assert!(Cff::parse(&[1, 0, 4, 1], Version::Cff2, 1000, &[]).is_err());
}
