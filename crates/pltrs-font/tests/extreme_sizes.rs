//! Robustness: hostile point sizes and resolutions (zero, negative, NaN,
//! infinite, huge) and long strings must not overflow FreeType's
//! fixed-point arithmetic into a panic.

mod common;

use common::font_bytes;
use pltrs_font::{
    AutohintTarget, Autohinter, Face, FontChain, FontError, GlyphId, Hinting, HintingInstance,
    KerningMode, OutlineRecorder, ShapeOptions, Size, SizeError, shape_at_size,
};

const SIZES: [f64; 10] = [
    0.0,
    -12.0,
    1e-300,
    12.0,
    1e6,
    1e17,
    1e300,
    f64::MAX,
    f64::NAN,
    f64::INFINITY,
];

#[test]
fn extreme_sizes_do_not_panic() {
    for name in ["DejaVuSans.ttf", "cmr10.ttf", "STIXGeneral.ttf"] {
        let data = font_bytes(name);
        let face = Face::parse(&data, 0).expect("bundled font");
        let chain = FontChain::new(vec![face.clone()]);
        let autohinter = Autohinter::new(&face);
        for points in SIZES {
            for dpi in [72.0, 0.0, -1.0, 1e12, f64::NAN] {
                let size = Size::new(&face, points, dpi);
                let hinting = HintingInstance::new(&face, &size);
                for glyph in [GlyphId(0), GlyphId(36), GlyphId(68)] {
                    if let Ok(hinted) = hinting.load_glyph(&face, glyph) {
                        hinted.outline(&mut OutlineRecorder::default());
                    }
                    for target in [AutohintTarget::Normal, AutohintTarget::Mono] {
                        if let Ok(hinted) = autohinter.load_glyph(&face, &size, glyph, target) {
                            hinted.outline(&mut OutlineRecorder::default());
                        }
                    }
                    let _ = size.glyph_cbox(&face, glyph);
                    let _ = size.outline(&face, glyph, &mut OutlineRecorder::default());
                    let _ = (
                        size.advance(&face, glyph),
                        size.linear_advance(&face, glyph),
                    );
                    let _ = size.kerning(&face, glyph, GlyphId(36), KerningMode::Default);
                    let _ = (size.em_scale_x(i32::MAX), size.em_scale_y(i32::MIN));
                }
                let _ = shape_at_size(&face, "AVATAR fi", &size, &ShapeOptions::default());
                for hinting in [Hinting::None, Hinting::Native, Hinting::Auto] {
                    let _ =
                        chain.layout("Wide text", points, dpi, &ShapeOptions::default(), hinting);
                }
            }
        }
    }
}

#[test]
fn long_strings_at_huge_sizes_do_not_overflow() {
    let data = font_bytes("DejaVuSans.ttf");
    let face = Face::parse(&data, 0).expect("bundled font");
    let chain = FontChain::new(vec![face]);
    let text = "W".repeat(5_000);
    for points in [1e4, 1e6, f64::MAX] {
        for hinting in [Hinting::None, Hinting::Auto] {
            let _ = chain.layout(&text, points, 1e4, &ShapeOptions::default(), hinting);
        }
    }
}

#[test]
fn sizes_freetype_refuses_are_refused() {
    let data = font_bytes("DejaVuSans.ttf");
    let face = Face::parse(&data, 0).expect("bundled font");
    let chain = FontChain::new(vec![face.clone()]);
    let options = ShapeOptions::default();
    // No pixel per em at a resolution of 2 dots per inch.
    let refused = chain.layout("ab", 10.0, 2.5, &options, Hinting::None);
    assert_eq!(refused.err(), Some(FontError::Size(SizeError::InvalidPpem)));
    assert!(
        chain
            .layout("ab", 10.0, 72.0, &options, Hinting::None)
            .is_ok()
    );
    // A resolution of zero is FreeType's default of 72; more than 65 535
    // pixels per em are refused too.
    assert!(Size::checked(&face, 10.0, 0.0).is_ok());
    assert_eq!(
        Size::checked(&face, 1e6, 72.0),
        Err(SizeError::InvalidPixelSize)
    );
    assert_eq!(SizeError::InvalidPpem.code(), 0x97);
    assert_eq!(
        SizeError::InvalidPixelSize.to_string(),
        "invalid pixel size"
    );
}
