//! Autohinter tests: FreeType's results for bundled and synthetic fonts,
//! the rendering targets, advances, style assignment and degenerate input.

use super::metrics::StyleMetrics;
use super::*;
use crate::face::Face;
use crate::hinting::tests::{TestFont, points, square};
use crate::hinting::{HintedGlyph, Hinter, Hinting};
use crate::scale::Size;
use crate::tables::glyf::tests::simple_glyph;
use crate::types::GlyphId;

const DEJAVU: &[u8] = include_bytes!("../../fonts/DejaVuSans.ttf");
const DEJAVU_MONO: &[u8] = include_bytes!("../../fonts/DejaVuSansMono.ttf");
const STIX: &[u8] = include_bytes!("../../fonts/STIXGeneral.ttf");
const CFF: &[u8] = include_bytes!("../../tests/fixtures/synthetic-cff.otf");

/// `c` of `face` autohinted at `points` and `dpi` for `target`.
fn load(face: &Face<'_>, c: char, points: f64, dpi: f64, target: AutohintTarget) -> HintedGlyph {
    let glyph = face.glyph_index(c).expect("mapped character");
    Autohinter::new(face)
        .load_glyph(face, &Size::new(face, points, dpi), glyph, target)
        .expect("valid glyph")
}

/// A font without a character map: a square and a ring.
fn unmapped_font() -> Vec<u8> {
    TestFont::new(vec![
        square(15, 15, &[]),
        simple_glyph(&[
            &[
                (50, 0, true),
                (50, 700, true),
                (450, 700, true),
                (450, 0, true),
            ],
            &[
                (150, 100, true),
                (350, 100, true),
                (350, 600, true),
                (150, 600, true),
            ],
        ]),
    ])
    .build()
}

#[test]
fn bundled_glyphs_match_freetype() {
    // FreeType 2.14.3: matplotlib's `FT2Font.load_glyph` with
    // `LoadFlags.FORCE_AUTOHINT` (and a target) and `get_path()` for
    // DejaVu Sans at 12 pt and 100 dpi.
    let face = Face::parse(DEJAVU, 0).expect("bundled font");
    let a = load(&face, 'A', 12.0, 100.0, AutohintTarget::Normal);
    assert_eq!(a.advance(), 704);
    assert_eq!(
        points(&a),
        [
            (365, 632),
            (222, 260),
            (508, 260),
            (365, 632),
            (305, 768),
            (425, 768),
            (721, 0),
            (612, 0),
            (541, 176),
            (190, 176),
            (119, 0),
            (8, 0),
            (305, 768),
        ]
    );
    // Light hinting leaves x alone.
    let h = load(&face, 'H', 12.0, 100.0, AutohintTarget::Light);
    assert_eq!(h.advance(), 832);
    assert_eq!(
        points(&h),
        [
            (105, 768),
            (210, 768),
            (210, 452),
            (592, 452),
            (592, 768),
            (698, 768),
            (698, 0),
            (592, 0),
            (592, 368),
            (210, 368),
            (210, 0),
            (105, 0),
            (105, 768),
        ]
    );
}

#[test]
fn unmapped_glyphs_get_the_cjk_fallback_style() {
    // FreeType 2.14.3 under `FORCE_AUTOHINT`, normal and monochrome.
    let data = unmapped_font();
    let face = Face::parse(&data, 0).expect("synthetic font");
    assert_eq!(Hinting::Default.hinter(&face, true), Hinter::Autohinter);
    let hinter = Autohinter::new(&face);
    let style = hinter.glyph_style(&face, GlyphId(1)).expect("glyph 1");
    assert!(matches!(style.metrics, StyleMetrics::Cjk(_)));
    let ring = |points_size, dpi, target| {
        let size = Size::new(&face, points_size, dpi);
        hinter
            .load_glyph(&face, &size, GlyphId(1), target)
            .expect("valid glyph")
    };
    for target in [AutohintTarget::Normal, AutohintTarget::Mono] {
        let glyph = ring(40.0, 72.0, target);
        assert_eq!(glyph.advance(), 1280);
        assert_eq!(
            points(&glyph),
            [
                (128, 0),
                (128, 1792),
                (1152, 1792),
                (1152, 0),
                (128, 0),
                (384, 256),
                (896, 256),
                (896, 1536),
                (384, 1536),
                (384, 256),
            ]
        );
    }
    // Without blue zones, heights are not snapped.
    let glyph = ring(12.0, 100.0, AutohintTarget::Normal);
    assert_eq!(glyph.advance(), 512);
    assert_eq!(
        points(&glyph),
        [
            (64, 0),
            (64, 758),
            (502, 758),
            (502, 0),
            (64, 0),
            (182, 118),
            (384, 118),
            (384, 640),
            (182, 640),
            (182, 118),
        ]
    );
}

#[test]
fn blue_zones_and_targets_snap_to_the_pixel_grid() {
    let face = Face::parse(DEJAVU, 0).expect("bundled font");
    for (size, dpi) in [(6.0, 72.0), (9.5, 100.0), (12.0, 72.0), (20.0, 150.0)] {
        // Baseline, cap height and x-height are blue zones.
        for c in ['H', 'E', 'x', 'z'] {
            let glyph = load(&face, c, size, dpi, AutohintTarget::Normal);
            let cbox = glyph.control_box().expect("outline");
            assert_eq!((cbox.y_min % 64, cbox.y_max % 64), (0, 0), "{c} at {size}");
            assert_eq!(glyph.advance() % 64, 0);
        }
    }
    // Monochrome hinting snaps every edge of `H` at text sizes.
    for (size, dpi) in [(9.5, 100.0), (12.0, 72.0), (20.0, 150.0)] {
        let glyph = load(&face, 'H', size, dpi, AutohintTarget::Mono);
        assert!(
            points(&glyph)
                .iter()
                .all(|&(x, y)| x % 64 == 0 && y % 64 == 0),
            "H at {size}"
        );
    }
}

#[test]
fn advances_of_digits_and_fixed_pitch_fonts_stay_equal() {
    let advances = |data: &[u8], text: &str, points: f64| -> Vec<i32> {
        let face = Face::parse(data, 0).expect("bundled font");
        text.chars()
            .map(|c| load(&face, c, points, 72.0, AutohintTarget::Normal).advance())
            .collect()
    };
    for points in [7.0, 11.0, 23.0] {
        let digits = advances(DEJAVU, "0123456789", points);
        assert!(digits.iter().all(|&a| a == digits[0]), "{digits:?}");
        let mono = advances(DEJAVU_MONO, "il.mWM0", points);
        assert!(mono.iter().all(|&a| a == mono[0]), "{mono:?}");
    }
}

#[test]
fn styles_carry_digit_and_nonbase_flags() {
    let face = Face::parse(DEJAVU, 0).expect("bundled font");
    let hinter = Autohinter::new(&face);
    let style = |c: char| {
        let glyph = face.glyph_index(c).expect("mapped character");
        hinter.glyph_style(&face, glyph).expect("glyph in range")
    };
    for c in ['a', '\u{3b1}', '\u{416}', '\u{5e9}'] {
        assert!(matches!(style(c).metrics, StyleMetrics::Latin(_)), "{c}");
    }
    assert!(style('7').digit && !style('7').nonbase);
    assert!(style('\u{301}').nonbase && !style('a').digit);
    assert!(
        hinter
            .glyph_style(&face, GlyphId(face.num_glyphs()))
            .is_none()
    );
}

#[test]
fn analyses_are_reused_across_sizes() {
    let face = Face::parse(STIX, 0).expect("bundled font");
    let hinter = Autohinter::new(&face);
    let glyph = face.glyph_index('\u{2211}').expect("mapped character");
    let load = |points| {
        hinter
            .load_glyph(
                &face,
                &Size::new(&face, points, 100.0),
                glyph,
                AutohintTarget::Normal,
            )
            .expect("valid glyph")
    };
    let first = load(12.0);
    assert_ne!(load(17.0), first);
    assert_eq!(load(12.0), first);
}

#[test]
fn degenerate_glyphs_and_fonts() {
    let face = Face::parse(DEJAVU, 0).expect("bundled font");
    let hinter = Autohinter::new(&face);
    let size = Size::new(&face, 12.0, 72.0);
    let space = load(&face, ' ', 12.0, 72.0, AutohintTarget::Normal);
    assert!(space.control_box().is_none());
    assert!(space.advance() > 0 && space.advance() % 64 == 0);
    let beyond = GlyphId(face.num_glyphs());
    assert!(
        hinter
            .load_glyph(&face, &size, beyond, AutohintTarget::Normal)
            .is_err()
    );
    // CFF outlines are autohinted too.
    let cff = Face::parse(CFF, 0).expect("fixture font");
    let hinter = Autohinter::new(&cff);
    let size = Size::new(&cff, 12.0, 72.0);
    for glyph in (0..cff.num_glyphs()).map(GlyphId) {
        assert!(
            hinter
                .load_glyph(&cff, &size, glyph, AutohintTarget::Normal)
                .is_ok()
        );
    }
}
