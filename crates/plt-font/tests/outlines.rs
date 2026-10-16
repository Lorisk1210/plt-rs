//! Glyph outlines, control boxes and linear advances compared with
//! FreeType's unhinted output through matplotlib
//! (`tests/fixtures/outlines.json`).

mod common;

use common::{fixture, font_bytes, path_codes};
use plt_font::{Face, GlyphId, OutlineRecorder, Size};

/// Asserts two flat paths are equal up to `tolerance` per coordinate.
fn assert_paths(actual: &[i64], expected: &[i64], tolerance: i64, label: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{label}: path length\n actual {actual:?}\n expected {expected:?}"
    );
    for (index, (a, e)) in actual.iter().zip(expected).enumerate() {
        let allowed = if index % 3 == 0 { 0 } else { tolerance };
        assert!(
            (a - e).abs() <= allowed,
            "{label}: element {index}: {a} vs {e}"
        );
    }
}

#[test]
fn outlines_and_boxes_match_freetype() {
    let fixtures = fixture("outlines.json");
    let mut checked = 0;
    for (name, glyphs) in fixtures.as_object() {
        let data = font_bytes(name);
        let face = Face::parse(&data, 0).unwrap();
        let size_12 = Size::new(&face, 12.0, 72.0);
        let size_10 = Size::new(&face, 10.0, 100.0);
        for entry in glyphs.as_array() {
            let glyph = GlyphId(entry.get("gid").as_i64() as u16);
            let label = format!("{name} glyph {}", glyph.0);
            let mut units = OutlineRecorder::default();
            face.outline(glyph, &mut units).unwrap();
            // At ppem == units per em FreeType's coordinates are exact font
            // units (implied midpoints are half units).
            assert_paths(
                &path_codes(&units.commands, 2.0),
                &entry.get("units_x2").i64s(),
                0,
                &label,
            );

            let cbox = |size: &Size| {
                size.glyph_cbox(&face, glyph)
                    .unwrap()
                    .map_or(vec![0, 0, 0, 0], |r| {
                        vec![
                            i64::from(r.x_min),
                            i64::from(r.y_min),
                            i64::from(r.x_max),
                            i64::from(r.y_max),
                        ]
                    })
            };
            assert_eq!(
                cbox(&size_12),
                entry.get("bbox_12").i64s(),
                "{label}: bbox at 12/72"
            );
            assert_eq!(
                cbox(&size_10),
                entry.get("bbox_10_100").i64s(),
                "{label}: bbox at 10/100"
            );
            assert_eq!(
                size_12.linear_advance(&face, glyph),
                entry.get("linear_advance_12").as_i64(),
                "{label}: advance"
            );
            if let Some(expected) = entry.opt("pixels_12_x64") {
                let mut pixels = OutlineRecorder::default();
                size_12.outline(&face, glyph, &mut pixels).unwrap();
                assert_paths(
                    &path_codes(&pixels.commands, 64.0),
                    &expected.i64s(),
                    0,
                    &format!("{label} at 12/72"),
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 300, "only {checked} glyphs checked");
}
