//! Behavioural tests of the PDF backend; conformance with matplotlib's
//! documents is checked by plt-spec's vector tests.

use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_text::{FontQuery, FontRequest, TextEngine};

use super::*;
use crate::testing::square;

fn renderer() -> PdfRenderer {
    PdfRenderer::new(
        2.0,
        1.0,
        72.0,
        PdfOptions {
            compression: 0,
            metadata: Metadata::none(),
            ..PdfOptions::default()
        },
    )
    .expect("valid page")
}

fn text(document: &[u8]) -> String {
    String::from_utf8_lossy(document).into_owned()
}

#[test]
fn page_skeleton() {
    let pdf = text(&renderer().finish().expect("document"));
    assert!(pdf.starts_with("%PDF-1.4\n"));
    assert!(pdf.contains("/MediaBox [ 0 0 144 72 ]"));
    assert!(pdf.contains("stream\n/DeviceRGB CS\n/DeviceRGB cs\n1 j\n\n\nendstream"));
    assert!(pdf.contains("trailer\n<< /Size 14 /Root 1 0 R /Info 13 0 R >>"));
    assert!(PdfRenderer::new(1.0, -1.0, 72.0, PdfOptions::default()).is_err());
}

#[test]
fn paths_carry_state_changes_and_clips() {
    let mut r = renderer();
    let gc = GraphicsContext {
        linewidth: 2.0,
        foreground: Rgba::opaque(1.0, 0.0, 0.0),
        clip_rect: Some(Bbox::from_bounds(1.0, 1.0, 5.0, 5.0)),
        ..GraphicsContext::default()
    };
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    let pdf = text(&r.finish().expect("document"));
    assert!(
        pdf.contains(
            "q 1 1 5 5 re W n 1 g 2 w 1 0 0 RG 1 g\n0 0 m\n10 0 l\n10 10 l\n0 10 l\nh\n\nB\nQ\n"
        ),
        "{pdf}"
    );
}

#[test]
fn markers_become_form_xobjects() {
    let mut r = renderer();
    let positions = Path::polyline((0..10).map(|i| Point::new(f64::from(i), 5.0)).collect());
    r.draw_markers(
        &GraphicsContext::default(),
        &square(),
        &Affine2D::IDENTITY,
        &positions,
        &Affine2D::IDENTITY,
        Some(Rgba::BLACK),
    );
    let pdf = text(&r.finish().expect("document"));
    assert!(
        pdf.contains("1 0 0 1 0 5 cm /M0 Do\n1 0 0 1 1 0 cm /M0 Do"),
        "{pdf}"
    );
    assert!(pdf.contains("/Subtype /Form /BBox [ -5 -5 15 15 ]"));
}

#[test]
fn text_uses_type3_fonts() {
    let run = TextEngine::global()
        .layout_line(
            "1.0",
            &FontRequest::new(FontQuery {
                size: 10.0,
                ..FontQuery::families(["DejaVu Sans"])
            }),
            72.0,
        )
        .expect("bundled font");
    let mut r = renderer();
    r.draw_text(&GraphicsContext::default(), &run, Point::new(3.0, 4.0), 0.0);
    let pdf = text(&r.finish().expect("document"));
    assert!(
        pdf.contains("BT\n/F1 10 Tf\n0 0 Td\n[ (1) 0.29296875 (.) 0.68359375 (0) ] TJ\nET\nQ"),
        "{pdf}"
    );
    assert!(pdf.contains("/Subtype /Type3"));
    assert!(pdf.contains("/Differences [ 46 /period 48 /zero /one ]"));
    assert!(pdf.contains("636 0 109 0 544 730 d1\n"));
}

/// A run of `text` laid out in `families` at 12 points.
fn plain_run(text: &str, families: &[&str]) -> plt_text::TextRun {
    TextEngine::global()
        .layout_line(
            text,
            &FontRequest::new(FontQuery {
                size: 12.0,
                ..FontQuery::families(families.iter().copied())
            }),
            72.0,
        )
        .expect("bundled font")
}

#[test]
fn core_fonts_show_whole_strings_by_name() {
    let mut r = PdfRenderer::new(
        2.0,
        1.0,
        72.0,
        PdfOptions {
            compression: 0,
            metadata: Metadata::none(),
            core_fonts: true,
            ..PdfOptions::default()
        },
    )
    .expect("valid page");
    assert_eq!(r.afm_directory(), Some(AfmDirectory::PdfCore));
    let gc = GraphicsContext::default();
    r.draw_text(
        &gc,
        &plain_run("AVATAR", &["DejaVu Sans"]),
        Point::new(3.0, 4.0),
        0.0,
    );
    r.draw_text(
        &gc,
        &plain_run("caf\u{e9} \u{2212}1", &["Times", "Courier"]),
        Point::new(5.0, 6.0),
        90.0,
    );
    let math = TextEngine::global()
        .layout_math("$x$", &plt_text::mathtext::MathStyle::default())
        .expect("valid math");
    r.draw_text(&gc, &math, Point::new(1.0, 1.0), 0.0);
    let document = r.finish().expect("document");
    // Code page 1252: the e-acute is byte 0xe9, the minus sign unknown.
    let rotated = b"BT /F2 12 Tf\n0 1 -1 0 5 6 Tm\n0 0 Td\n(caf\xe9 ?1) Tj ET\n";
    assert!(document.windows(rotated.len()).any(|w| w == rotated));
    let pdf = text(&document);
    assert!(
        pdf.contains("BT /F1 12 Tf\n3 4 Td\n(AVATAR) Tj ET\n"),
        "{pdf}"
    );
    for font in ["Helvetica", "Times-Roman", "Courier"] {
        assert!(pdf.contains(&format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{font}\n/Encoding /WinAnsiEncoding >>"
        )));
    }
    assert!(
        pdf.contains("/Subtype /Type3"),
        "mathtext still embeds its fonts"
    );
}

#[test]
fn images_and_shadings() {
    let mut r = renderer();
    let image = RgbaImage::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 128]).expect("2x1");
    r.draw_image(
        &GraphicsContext::default(),
        Point::new(1.0, 2.0),
        &image,
        None,
    );
    let triangle = [[
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(0.0, 10.0),
    ]];
    r.draw_gouraud_triangles(
        &GraphicsContext::default(),
        &triangle,
        &[[Rgba::BLACK, Rgba::WHITE, Rgba::new(1.0, 0.0, 0.0, 0.5)]],
        &Affine2D::IDENTITY,
    );
    let pdf = text(&r.finish().expect("document"));
    assert!(pdf.contains("q 2 0 0 1 1 2 cm /I1 Do Q"));
    assert!(pdf.contains("/SMask"));
    assert!(pdf.contains("q /SM1 gs /GT0 sh Q"));
    assert!(pdf.contains("/ShadingType 4"));
}
