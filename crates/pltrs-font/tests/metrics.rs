//! Face-level metrics, names, character maps and kerning compared with
//! FreeType as seen through matplotlib (`tests/fixtures/metrics.json`,
//! `kerning.json`).

mod common;

use common::json::{Fixture, Json};
use common::{fixture, font_bytes};
use pltrs_font::{Face, GlyphId, KerningMode, Size};

fn check_names(name: &str, face: &Face<'_>, expected: &Json) {
    assert_eq!(
        face.family_name().as_deref(),
        Some(expected.field("family_name").str()),
        "{name}"
    );
    assert_eq!(
        face.style_name().as_deref(),
        Some(expected.field("style_name").str()),
        "{name}"
    );
    let postscript = expected.field("postscript_name").str();
    assert_eq!(
        face.postscript_name().as_deref(),
        Some(postscript),
        "{name}"
    );
}

fn check_metrics(name: &str, face: &Face<'_>, expected: &Json) {
    let bbox = face.bbox();
    let (position, thickness) = face.underline_metrics().expect("post table");
    let integers = [
        ("units_per_em", i64::from(face.units_per_em())),
        ("ascender", i64::from(face.ascender())),
        ("descender", i64::from(face.descender())),
        ("height", i64::from(face.height())),
        ("num_glyphs", i64::from(face.num_glyphs())),
        // FreeType reports the centre of the underline.
        (
            "underline_position",
            i64::from(position) - i64::from(thickness) / 2,
        ),
        ("underline_thickness", i64::from(thickness)),
        ("num_charmaps", face.charmaps().len() as i64),
    ];
    for (key, actual) in integers {
        assert_eq!(actual, expected.field(key).i64(), "{name}: {key}");
    }
    let corners = [bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max].map(i64::from);
    assert_eq!(
        corners.to_vec(),
        expected.field("bbox").i64s(),
        "{name}: bbox"
    );
    let flags = face.style_flags();
    assert_eq!(
        flags.italic,
        expected.field("italic").bool(),
        "{name}: italic"
    );
    assert_eq!(flags.bold, expected.field("bold").bool(), "{name}: bold");
    let angle = expected.field("italic_angle").f64();
    assert!(
        (face.italic_angle() - angle).abs() < 1e-6,
        "{name}: italic angle"
    );
}

fn check_glyph_names_and_cmap(name: &str, face: &Face<'_>, expected: &Json) {
    assert_eq!(
        face.has_glyph_names(),
        expected.field("has_glyph_names").bool(),
        "{name}"
    );
    for (gid, glyph_name) in expected.field("glyph_names").object() {
        let gid = GlyphId(gid.parse().expect("glyph id"));
        let glyph_name = glyph_name.str();
        assert_eq!(
            face.glyph_name(gid),
            Some(glyph_name),
            "{name}: glyph {}",
            gid.0
        );
        let found = face
            .glyph_index_by_name(glyph_name)
            .and_then(|g| face.glyph_name(g));
        assert_eq!(found, Some(glyph_name), "{name}: lookup of {glyph_name}");
    }
    for (code, gid) in expected.field("chars").object() {
        let code: u32 = code.parse().expect("character code");
        let found = face
            .glyph_index_for_code(code)
            .map_or(0, |g| i64::from(g.0));
        assert_eq!(found, gid.i64(), "{name}: cmap U+{code:04X}");
    }
}

fn check_os2(name: &str, face: &Face<'_>, expected: &Json) {
    let Some(wanted) = expected.opt("os2") else {
        return;
    };
    let os2 = face.os2().expect("OS/2 table");
    let fields = [
        ("version", i64::from(os2.version)),
        ("usWeightClass", i64::from(os2.weight_class)),
        ("usWidthClass", i64::from(os2.width_class)),
        ("fsSelection", i64::from(os2.fs_selection)),
        ("sTypoAscender", i64::from(os2.typo_ascender)),
        ("sTypoDescender", i64::from(os2.typo_descender)),
        ("usWinAscent", i64::from(os2.win_ascent)),
    ];
    for (key, actual) in fields {
        assert_eq!(actual, wanted.field(key).i64(), "{name}: OS/2 {key}");
    }
    if let Some(x_height) = wanted.opt("sxHeight") {
        assert_eq!(os2.x_height.map(i64::from), Some(x_height.i64()), "{name}");
    }
}

#[test]
fn face_metrics_match_freetype() {
    let fixtures = fixture("metrics.json");
    for (name, expected) in fixtures.object() {
        let data = font_bytes(name);
        let face = Face::parse(&data, 0).unwrap();
        check_names(name, &face, expected);
        check_metrics(name, &face, expected);
        check_glyph_names_and_cmap(name, &face, expected);
        check_os2(name, &face, expected);
    }
}

#[test]
fn kerning_matches_freetype() {
    let fixtures = fixture("kerning.json");
    for (name, pairs) in fixtures.object() {
        let data = font_bytes(name);
        let face = Face::parse(&data, 0).unwrap();
        for row in pairs.array() {
            let pair: Vec<char> = row.field("pair").str().chars().collect();
            let left = face.glyph_index(pair[0]).unwrap_or_default();
            let right = face.glyph_index(pair[1]).unwrap_or_default();
            let label = format!("{name} {pair:?}");
            let unscaled = i64::from(face.legacy_kerning(left, right));
            assert_eq!(unscaled, row.field("unscaled").i64(), "{label}");
            for (points, dpi) in [(12.0, 72.0), (10.0, 100.0), (48.0, 72.0)] {
                let size = Size::new(&face, points, dpi);
                for (mode, key) in [
                    (KerningMode::Default, "default"),
                    (KerningMode::Unfitted, "unfitted"),
                ] {
                    let wanted = row.field(&format!("{key}_{points:?}_{dpi:?}")).i64();
                    assert_eq!(
                        size.kerning(&face, left, right, mode),
                        wanted,
                        "{label} {key} {points}"
                    );
                }
            }
        }
    }
}
