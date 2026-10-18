//! Behavioural tests of the PostScript backend; conformance with
//! matplotlib's documents is checked by plt-spec's vector tests.

use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_text::{FontQuery, FontRequest, TextEngine};

use super::*;
use crate::gc::Hatch;

fn renderer(eps: bool) -> PsRenderer {
    PsRenderer::new(
        6.4,
        4.8,
        100.0,
        PsOptions {
            eps,
            metadata: Metadata::none(),
            ..PsOptions::default()
        },
    )
    .expect("valid figure")
}

fn square() -> Path {
    Path::polygon([
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
        Point::new(0.0, 10.0),
    ])
}

#[test]
fn documents_are_centred_on_the_paper() {
    let ps = renderer(false).finish().expect("document");
    assert!(ps.starts_with("%!PS-Adobe-3.0\n%%DocumentPaperSizes: letter\n%%Pages: 1\n"));
    assert!(ps.contains("%%BoundingBox: 75 223 537 569\n"));
    assert!(ps.contains("%%HiResBoundingBox: 75.600000 223.200000 536.400000 568.800000\n"));
    assert!(ps.contains("/mpldict 8 dict def\n"));
    assert!(
        ps.contains("%%Page: 1 1\nmpldict begin\n75.6 223.2 translate\n0 0 460.8 345.6 rectclip\n")
    );
    assert!(ps.ends_with("\nend\nshowpage\n%%EOF\n"));
    let eps = renderer(true).finish().expect("document");
    assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n%%LanguageLevel: 3\n"));
    assert!(eps.contains("%%BoundingBox: 0 0 461 346\n"));
    assert!(!eps.contains("%%EOF"));
    assert_eq!(
        Paper::from_name("A4").map(|p| matches!(p, Paper::Named { .. })),
        Some(true)
    );
    assert_eq!(Paper::from_name("nope"), None);
}

#[test]
fn paths_write_changed_properties_only() {
    let mut r = renderer(true);
    let gc = GraphicsContext {
        linewidth: 2.0,
        clip_rect: Some(Bbox::from_bounds(1.0, 2.0, 3.0, 4.0)),
        hatch: Some(Hatch {
            pattern: "/".to_owned(),
            color: Rgba::BLACK,
            linewidth: 1.0,
        }),
        ..GraphicsContext::default()
    };
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    let ps = r.finish().expect("document");
    assert_eq!(ps.matches("2 setlinewidth").count(), 1);
    assert_eq!(ps.matches("makepattern").count(), 1);
    assert!(ps.contains(
        "gsave\n1 2 3 4 rectclip\n0 0 m\n10 0 l\n10 10 l\n0 10 l\ncl\ngsave\n1 setgray\nfill\ngrestore\n"
    ));
    assert!(ps.contains("gsave\n0 0 0 H0 setpattern fill grestore\nstroke\ngrestore\n"));
}

#[test]
fn markers_are_procedures() {
    let mut r = renderer(true);
    let positions = Path::polyline(vec![Point::new(1.5, 2.0), Point::new(3.0, 4.0)]);
    r.draw_markers(
        &GraphicsContext::default(),
        &square(),
        &Affine2D::IDENTITY,
        &positions,
        &Affine2D::IDENTITY,
        Some(Rgba::opaque(1.0, 0.0, 0.0)),
    );
    let ps = r.finish().expect("document");
    assert!(ps.contains(
        "/o {\ngsave\nnewpath\ntranslate\n1.0 setlinewidth\n1 setlinejoin\n\n0 setlinecap\n"
    ));
    assert!(ps.contains(
        "gsave\n1 0 0 setrgbcolor\nfill\ngrestore\nstroke\ngrestore\n} bind def\n1.5 2 o\n3 4 o\n"
    ));
}

#[test]
fn text_embeds_type3_fonts() {
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
    let mut r = renderer(true);
    r.draw_text(
        &GraphicsContext::default(),
        &run,
        Point::new(65.8874, 23.4183),
        0.0,
    );
    let ps = r.finish().expect("document");
    assert!(ps.contains("/mpldict 9 dict def"));
    assert!(ps.contains("/FontName /DejaVuSans-0 def"));
    assert!(ps.contains(
        "/DejaVuSans-0 10.000 selectfont\ngsave\n\n65.8874 23.4183 translate\n0 rotate\n\
         0 0 m /one glyphshow\n6.35938 0 m /period glyphshow\n9.53125 0 m /zero glyphshow\ngrestore\n"
    ));
}

#[test]
fn afm_text_is_placed_by_its_metrics_and_nothing_is_embedded() {
    let mut r = PsRenderer::new(
        6.4,
        4.8,
        100.0,
        PsOptions {
            eps: true,
            metadata: Metadata::none(),
            afm: true,
            ..PsOptions::default()
        },
    )
    .expect("valid figure");
    assert_eq!(r.options.afm_directory(), Some(AfmDirectory::PdfCore));
    let run = TextEngine::global()
        .layout_line(
            "AV",
            &FontRequest::new(FontQuery {
                size: 10.0,
                ..FontQuery::families(["DejaVu Sans"])
            }),
            72.0,
        )
        .expect("bundled font");
    let gc = GraphicsContext::default();
    r.draw_text(&gc, &run, Point::new(1.0, 2.0), 0.0);
    let math = TextEngine::global()
        .layout_math("$x$", &plt_text::mathtext::MathStyle::default())
        .expect("valid math");
    r.draw_text(&gc, &math, Point::new(1.0, 1.0), 0.0);
    let ps = r.finish().expect("document");
    assert!(ps.contains(
        "/Helvetica 10.000 selectfont\ngsave\n\n1 2 translate\n0 rotate\n\
         0 0 m /A glyphshow\n5.97 0 m /V glyphshow\ngrestore\n"
    ));
    assert!(ps.contains("/mpldict 8 dict def"));
    assert!(
        !ps.contains("/FontName"),
        "not even mathtext's fonts are embedded"
    );
}

#[test]
fn images_and_shadings() {
    let mut r = renderer(true);
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
        &[[Rgba::BLACK, Rgba::WHITE, Rgba::BLACK]],
        &Affine2D::IDENTITY,
    );
    let ps = r.finish().expect("document");
    assert!(ps.contains(
        "1 2 translate\n[1 0 0 1 0 0] concat\n1.44 0.72 scale\n/DataString 2 string def"
    ));
    assert!(ps.contains("bind false 3 colorimage\nff00000000ff\ngrestore\n"));
    assert!(ps.contains("/Decode [ -4096 4106 -4096 4106 0 1 0 1 0 1 ]"));
}
