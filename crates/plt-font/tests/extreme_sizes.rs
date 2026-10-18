//! Robustness: hostile point sizes and resolutions (zero, negative, NaN,
//! infinite, huge) and long strings must not overflow FreeType's
//! fixed-point arithmetic into a panic.

mod common;

use common::font_bytes;
use plt_font::{
    Face, FontChain, GlyphId, KerningMode, OutlineRecorder, ShapeOptions, Size, shape_at_size,
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
        for points in SIZES {
            for dpi in [72.0, 0.0, -1.0, 1e12, f64::NAN] {
                let size = Size::new(&face, points, dpi);
                for glyph in [GlyphId(0), GlyphId(36), GlyphId(68)] {
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
                let _ = chain.layout("Wide text", points, dpi, &ShapeOptions::default());
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
        let _ = chain.layout(&text, points, 1e4, &ShapeOptions::default());
    }
}
