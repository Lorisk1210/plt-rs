use super::*;
use crate::tex::test_support::{DEJAVU_SANS, DviWriter, files, sample_dvi};
use crate::tex::tfm::tests::tfm_bytes;

#[test]
fn pages_hold_glyphs_rules_and_their_extent() {
    let files = files(&[]);
    let pages = read_dvi(&sample_dvi(), &files).unwrap();
    let [page] = pages.as_slice() else {
        panic!("one page")
    };
    let baseline = 100 + 100 + 100 + 1000;
    assert_eq!(page.baseline, Some(baseline));
    let text: Vec<(i128, i128, i128, i128)> = page
        .text
        .iter()
        .map(|t| (t.x, t.y, t.glyph, t.width))
        .collect();
    // Widths of 0.5 and 0.7 design sizes at a scale of 10 pt.
    let a = ((1_i128 << 19) * (10 << 16)) >> 20;
    let b = (734_003_i128 * (10 << 16)) >> 20;
    assert_eq!(text, [(0, baseline, 65, a), (a, baseline, 66, b)]);
    assert_eq!(
        page.boxes,
        [RawRule {
            x: a + b,
            y: baseline,
            height: 20_000,
            width: 30_000
        }]
    );
    let at = page.at(72.27 * 65536.0).unwrap();
    let h = ((100_i128 * (10 << 16)) >> 20) as f64;
    let d = ((30_i128 * (10 << 16)) >> 20) as f64;
    assert_eq!(at.width, (a + b + 30_000) as f64);
    assert_eq!(at.height, 20_000_f64.max(h));
    assert_eq!(at.descent, d);
    assert_eq!((at.text[0].x, at.text[0].y), (0.0, 0.0));
    assert_eq!(at.text[1].x, a as f64);
    assert_eq!(at.boxes[0].y, 0.0);
    assert_eq!(at.text[0].font.size(), 10.0 * 72.0 / 72.27);
}

#[test]
fn empty_pages_measure_nothing() {
    let mut dvi = DviWriter::dvi();
    dvi.bop().end();
    let pages = read_dvi(&dvi.0, &files(&[])).unwrap();
    assert_eq!(pages[0].at(100.0).unwrap(), DviPage::default());
}

#[test]
fn the_minus_of_cmsy_has_no_depth() {
    let tfm = Arc::new(Tfm::parse(&tfm_bytes(0, 1, &[100, 100], 0)).unwrap());
    let cmsy = DviFont::new(
        1 << 20,
        DviMetrics::Tfm(Arc::clone(&tfm)),
        b"cmsy10".to_vec(),
        None,
    );
    let cmr = DviFont::new(1 << 20, DviMetrics::Tfm(tfm), b"cmr10".to_vec(), None);
    assert_eq!(cmsy.height_depth_of(0).unwrap(), (100, 0));
    assert_eq!(cmsy.height_depth_of(1).unwrap(), (100, 30));
    assert_eq!(cmr.height_depth_of(0).unwrap(), (100, 30));
    assert!(is_cmsy(b"cmsy7\n") && !is_cmsy(b"cmsy") && !is_cmsy(b"cmsyx"));
    assert_eq!(cmr.height_depth_of(99).unwrap(), (0, 0));
    assert_eq!(cmr.width_of(99).unwrap(), 0);
}

#[test]
fn fonts_made_by_hand_measure_as_those_of_dvi_files() {
    let tfm = Arc::new(Tfm::parse(&tfm_bytes(65, 67, &[1 << 19, 734_003], 0)).unwrap());
    let font = DviFont::new(10 << 16, DviMetrics::Tfm(tfm), b"test".to_vec(), None);
    assert!((font.size() - 10.0 * 72.0 / 72.27).abs() < 1e-12);
    let widths = font.widths();
    assert_eq!(widths.len(), 68);
    assert_eq!(widths[..65], [0; 65]);
    assert_eq!(widths[65], 500);
}

#[test]
fn native_fonts_of_xetex_and_luatex() {
    let files = files(&[]);
    let effects = NativeEffects {
        slant: Some(0.25),
        ..NativeEffects::default()
    };
    let xetex = DviFont::from_xetex(10 << 16, DEJAVU_SANS.as_bytes(), 0, effects, &files).unwrap();
    assert_eq!(xetex.texname(), format!("[{DEJAVU_SANS}]").as_bytes());
    let native = xetex.native_font().expect("a native font");
    assert_eq!((native.subfont, native.effects), (0, effects));
    assert_eq!(xetex.widths(), Vec::<i64>::new());
    let luatex = DviFont::from_luatex(
        10 << 16,
        format!("[{DEJAVU_SANS}]:index=0").into_bytes(),
        &files,
    )
    .unwrap();
    assert_eq!(luatex.width_of(36).unwrap(), xetex.width_of(36).unwrap());
    assert!(DviFont::from_luatex(10 << 16, b"DejaVuSans.ttf".to_vec(), &files).is_err());
}

#[test]
fn missing_fonts_fail_when_a_glyph_is_set_in_them() {
    let mut defined_only = DviWriter::dvi();
    defined_only.font_def(0, "absent", 10 << 16).bop().end();
    assert!(read_dvi(&defined_only.0, &files(&[])).is_ok());
    let mut used = DviWriter::dvi();
    used.font_def(0, "absent", 10 << 16)
        .bop()
        .op(171)
        .op(65)
        .end();
    assert_eq!(
        read_dvi(&used.0, &files(&[])).unwrap_err(),
        TexError::NotFound {
            name: "absent.tfm".into()
        }
    );
}

#[test]
fn malformed_files_are_errors() {
    let files = files(&[]);
    let sample = sample_dvi();
    for len in 0..sample.len() {
        assert!(
            read_dvi(&sample[..len], &files).is_err(),
            "truncated at {len}"
        );
    }
    let mut bad_format = sample.clone();
    bad_format[1] = 3;
    assert!(read_dvi(&bad_format, &files).is_err());
    let mut bad_units = sample.clone();
    bad_units[2] = 0;
    assert!(read_dvi(&bad_units, &files).is_err());
    for opcode in [249, 250, 251, 252, 255] {
        let mut dvi = DviWriter::dvi();
        dvi.bop().op(opcode);
        assert!(read_dvi(&dvi.0, &files).is_err(), "{opcode}");
    }
    let mut char_outside_page = DviWriter::dvi();
    char_outside_page.op(65);
    assert!(read_dvi(&char_outside_page.0, &files).is_err());
    let mut no_font = DviWriter::dvi();
    no_font.bop().op(65).end();
    assert!(read_dvi(&no_font.0, &files).is_err());
    let mut pop_empty = DviWriter::dvi();
    pop_empty.bop().op(142).end();
    assert!(read_dvi(&pop_empty.0, &files).is_err());
}

/// The advance of glyph `units` wide in a 2048-unit em at 10 pt, in DVI
/// units.
fn ten_point(units: i128) -> i128 {
    mul1220(units << 9, 10 << 16).unwrap()
}

#[test]
fn xetex_native_fonts_set_glyphs_at_their_offsets() {
    let mut dvi = DviWriter::dvi();
    dvi.bop();
    // `define_native_font` 5 at 10 pt: face 0 of DejaVu Sans, slanted.
    let name = DEJAVU_SANS.as_bytes();
    dvi.op(252).u4(5).u4(10 << 16).u2(0x2000);
    dvi.u1(u8::try_from(name.len()).expect("a short path"));
    dvi.0.extend_from_slice(name);
    dvi.u4(0).s4(16384);
    dvi.op(235).u1(5);
    // `set_glyphs`: A and g, 1000 units in all.
    dvi.op(253).u4(1000).u2(2);
    dvi.s4(0).s4(0).s4(500).s4(-7).u2(36).u2(74);
    // `set_text_and_glyphs`: the text "A", then A 300 units wide.
    dvi.op(254).u2(1).u2(0x41).u4(300).u2(1).s4(10).s4(0).u2(36);
    dvi.end();
    let pages = read_dvi(&dvi.0, &files(&[])).unwrap();
    let text: Vec<(i128, i128, i128, i128)> = pages[0]
        .text
        .iter()
        .map(|t| (t.x, t.y, t.glyph, t.width))
        .collect();
    let (a, g) = (ten_point(1401), ten_point(1300));
    assert_eq!(text, [(0, 0, 36, a), (500, -7, 74, g), (1010, 0, 36, a)]);
    let font = &pages[0].text[0].font;
    assert_eq!(font.texname(), format!("[{DEJAVU_SANS}]").as_bytes());
    assert_eq!(font.size(), 10.0 * 72.0 / 72.27);
    let native = font.native_font().expect("a native font");
    assert_eq!(native.effects.slant, Some(0.25));
    assert_eq!(font.height_depth_of(74).unwrap().1, ten_point(426));
}

#[test]
fn xetex_glyphs_are_set_in_the_current_font() {
    let mut dvi = DviWriter::dvi();
    dvi.font_def(0, "test", 10 << 16).bop().op(171);
    dvi.op(253).u4(0).u2(1).s4(0).s4(0).u2(65).end();
    // A TFM font measures the glyph as its character.
    let pages = read_dvi(&dvi.0, &files(&[])).unwrap();
    assert_eq!(pages[0].text[0].width, ((1_i128 << 19) * (10 << 16)) >> 20);
    let mut undefined = DviWriter::dvi();
    undefined.bop().op(253).u4(0).u2(0).end();
    assert!(read_dvi(&undefined.0, &files(&[])).is_err());
    let mut missing_file = DviWriter::dvi();
    missing_file.bop().op(252).u4(1).u4(10 << 16).u2(0).u1(8);
    missing_file.0.extend_from_slice(b"none.ttf");
    missing_file.u4(0).end();
    assert!(matches!(
        read_dvi(&missing_file.0, &files(&[])),
        Err(TexError::Io { .. })
    ));
}

#[test]
fn luatex_native_fonts_are_defined_by_their_names() {
    let name = format!("[{DEJAVU_SANS}]:embolden=16384");
    let mut dvi = DviWriter::dvi();
    dvi.op(243).u1(3).u4(0x4c75_6146).u4(10 << 16).u4(10 << 16);
    dvi.u1(0)
        .u1(u8::try_from(name.len()).expect("a short path"));
    dvi.0.extend_from_slice(name.as_bytes());
    dvi.bop().op(171 + 3).op(36).op(36).end();
    let pages = read_dvi(&dvi.0, &files(&[])).unwrap();
    let text: Vec<(i128, i128)> = pages[0].text.iter().map(|t| (t.x, t.glyph)).collect();
    assert_eq!(text, [(0, 36), (ten_point(1401), 36)]);
    let font = &pages[0].text[0].font;
    assert_eq!(font.texname(), name.as_bytes());
    let native = font.native_font().expect("a native font");
    assert_eq!(native.effects.embolden, Some(0.25));
    // A bracketed name without LuaTeX's checksum is the name of a TFM file.
    let mut tfm_named = DviWriter::dvi();
    tfm_named
        .font_def(0, "[x]", 10 << 16)
        .bop()
        .op(171)
        .op(65)
        .end();
    assert_eq!(
        read_dvi(&tfm_named.0, &files(&[])).unwrap_err(),
        TexError::NotFound {
            name: "[x].tfm".into()
        }
    );
}

#[test]
fn checksums_must_agree() {
    let mut dvi = DviWriter::dvi();
    dvi.op(243).u1(0).u4(7).u4(1 << 20).u4(1 << 20).u1(0).u1(5);
    dvi.0.extend_from_slice(b"check");
    dvi.bop().end();
    let files = files(&[("check.tfm", tfm_bytes(65, 65, &[1], 9))]);
    assert!(
        read_dvi(&dvi.0, &files)
            .unwrap_err()
            .to_string()
            .contains("tfm checksum mismatch: b'check'")
    );
}

#[test]
fn virtual_fonts_expand_into_their_glyphs_and_rules() {
    // Character 65 of `virt` sets `B` of `test` and a rule, at half scale.
    let mut vf = DviWriter::vf();
    vf.font_def(0, "test", 1 << 19);
    let mut packet = DviWriter::default();
    packet.op(66).rule(1 << 20, 1 << 20);
    vf.op(packet.0.len() as u8).u1(65).op(0).op(0).op(0);
    vf.0.extend_from_slice(&packet.0);
    vf.op(248);
    let files = files(&[
        ("virt.tfm", tfm_bytes(65, 65, &[1 << 20], 0)),
        ("virt.vf", vf.0),
    ]);
    let mut dvi = DviWriter::dvi();
    dvi.font_def(0, "virt", 10 << 16).bop().op(171).op(65).end();
    let pages = read_dvi(&dvi.0, &files).unwrap();
    let page = &pages[0];
    assert_eq!(page.text.len(), 1);
    let glyph = &page.text[0];
    assert_eq!(glyph.font.texname(), b"test");
    assert_eq!(glyph.font.scale(), ((1 << 19) * (10 << 16)) >> 20);
    assert_eq!(glyph.glyph, 66);
    assert_eq!(page.boxes.len(), 1);
    let width = (734_003_i128 * (1 << 19)) >> 20;
    assert_eq!(page.boxes[0].x, (width * (10 << 16)) >> 20);
}

#[test]
fn self_referring_virtual_fonts_are_errors() {
    let mut vf = DviWriter::vf();
    vf.font_def(0, "loop", 1 << 20).op(248);
    let files = files(&[("loop.tfm", tfm_bytes(65, 65, &[1], 0)), ("loop.vf", vf.0)]);
    let mut dvi = DviWriter::dvi();
    dvi.font_def(0, "loop", 1 << 20).bop().end();
    assert!(read_dvi(&dvi.0, &files).is_err());
}

#[test]
fn glyphs_of_a_virtual_font_share_their_scaled_font() {
    // Character 65 of `virt` sets `B` of `test`.
    let mut vf = DviWriter::vf();
    vf.font_def(0, "test", 1 << 20);
    vf.op(1).u1(65).op(0).op(0).op(0).op(66).op(248);
    let files = files(&[
        ("virt.tfm", tfm_bytes(65, 65, &[1 << 20], 0)),
        ("virt.vf", vf.0),
    ]);
    let mut dvi = DviWriter::dvi();
    dvi.font_def(0, "virt", 10 << 16)
        .font_def(1, "virt", 12 << 16)
        .bop()
        .op(171)
        .op(65)
        .op(65)
        .op(172)
        .op(65)
        .end();
    let pages = read_dvi(&dvi.0, &files).unwrap();
    let [first, second, larger] = pages[0].glyphs() else {
        panic!("three glyphs")
    };
    assert!(Arc::ptr_eq(&first.font, &second.font));
    assert!(!Arc::ptr_eq(&first.font, &larger.font));
    assert_eq!(larger.font.scale(), 12 << 16);
}

#[test]
fn extents_too_large_for_the_arithmetic_are_errors() {
    let rule = |x| RawRule {
        x,
        y: 0,
        height: 1,
        width: 1,
    };
    let page = RawPage {
        text: Vec::new(),
        boxes: vec![rule(i128::MIN / 2 - 10), rule(i128::MAX / 2 + 10)],
        baseline: None,
    };
    assert!(page.extent().is_err());
    assert!(page.at(72.0).is_err());
}

#[test]
fn rules_outline_as_rectangles_going_right_first() {
    let rule = DviBox {
        x: 1.0,
        y: 2.0,
        height: 3.0,
        width: 4.0,
    };
    let outline = rule.outline();
    let corners = [(1.0, 2.0), (5.0, 2.0), (5.0, 5.0), (1.0, 5.0), (1.0, 2.0)];
    assert_eq!(
        outline.vertices()[..5],
        corners.map(|(x, y)| Point::new(x, y))
    );
    assert_eq!(outline.codes().map(<[_]>::len), Some(6), "closed");
}
