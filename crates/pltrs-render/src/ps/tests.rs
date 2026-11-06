//! Behavioural tests of the PostScript backend; conformance with
//! matplotlib's documents is checked by pltrs-spec's vector tests.

use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};
use pltrs_text::tex::{TexEngine, TexFonts, TexManager};
use pltrs_text::{FontQuery, FontRequest, TextEngine};

use super::*;
use crate::gc::Hatch;
use crate::testing::square;

/// The document `r` finishes (no usetex).
fn document(r: PsRenderer) -> PsDocument {
    match r.finish().expect("document") {
        PsOutput::Document(document) => document,
        PsOutput::Psfrag(figure) => panic!("a psfrag figure: {figure:?}"),
    }
}

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

#[test]
fn documents_are_centred_on_the_paper() {
    let centred = document(renderer(false));
    let (x0, y0) = (36.0 * (8.5 - 6.4), 36.0 * (11.0 - 4.8));
    assert_eq!(
        centred.bbox,
        Bbox::from_extents(x0, y0, x0 + 6.4 * 72.0, y0 + 4.8 * 72.0)
    );
    let ps = centred.text;
    assert!(ps.starts_with("%!PS-Adobe-3.0\n%%DocumentPaperSizes: letter\n%%Pages: 1\n"));
    assert!(ps.contains("%%BoundingBox: 75 223 537 569\n"));
    assert!(ps.contains("%%HiResBoundingBox: 75.600000 223.200000 536.400000 568.800000\n"));
    assert!(ps.contains("/mpldict 8 dict def\n"));
    assert!(
        ps.contains("%%Page: 1 1\nmpldict begin\n75.6 223.2 translate\n0 0 460.8 345.6 rectclip\n")
    );
    assert!(ps.ends_with("\nend\nshowpage\n%%EOF\n"));
    let eps = document(renderer(true)).text;
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
    let ps = document(r).text;
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
    let ps = document(r).text;
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
    let ps = document(r).text;
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
    assert_eq!(r.options.afm_directory(), Some(AfmDirectory::PostScript));
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
        .layout_math("$x$", &pltrs_text::mathtext::MathStyle::default())
        .expect("valid math");
    r.draw_text(&gc, &math, Point::new(1.0, 1.0), 0.0);
    let ps = document(r).text;
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
    let ps = document(r).text;
    assert!(ps.contains(
        "1 2 translate\n[1 0 0 1 0 0] concat\n1.44 0.72 scale\n/DataString 2 string def"
    ));
    assert!(ps.contains("bind false 3 colorimage\nff00000000ff\ngrestore\n"));
    assert!(ps.contains("/Decode [ -4096 4106 -4096 4106 0 1 0 1 0 1 ]"));
}

#[test]
fn usetex_texts_are_psfrag_tags_in_a_figure_without_fonts() {
    let mut r = PsRenderer::new(
        3.2,
        2.0,
        100.0,
        PsOptions {
            orientation: PageOrientation::Landscape,
            metadata: Metadata::none(),
            usetex: true,
            ..PsOptions::default()
        },
    )
    .expect("valid figure");
    let fonts = TexFonts {
        family: vec!["sans-serif".to_owned()],
        ..TexFonts::default()
    };
    let manager = TexManager::new(&std::env::temp_dir(), fonts, "", TexEngine::Latex);
    // Blank text has no extent, so TeX need not run.
    let font = FontRequest::new(FontQuery {
        size: 12.0,
        ..FontQuery::families(["DejaVu Sans"])
    });
    let tex = TexText::new(" ", font, manager);
    let red = GraphicsContext {
        foreground: Rgba::opaque(1.0, 0.0, 0.0),
        ..GraphicsContext::default()
    };
    let run = TextRun::default();
    r.draw_tex(&red, &run, &tex, Point::new(10.0, 20.0), 30.0)
        .expect("tagged");
    let clear = GraphicsContext {
        alpha: Some(0.0),
        ..GraphicsContext::default()
    };
    r.draw_tex(&clear, &run, &tex, Point::new(1.0, 2.0), 0.0)
        .expect("not drawn");
    r.draw_path(
        &GraphicsContext::default(),
        &square(),
        &Affine2D::IDENTITY,
        None,
    );
    let PsOutput::Psfrag(figure) = r.finish().expect("figure") else {
        panic!("not a psfrag figure");
    };
    assert_eq!(
        figure.psfrags,
        [
            r"\psfrag{psmarker0}[bl][bl][1][30.000000]{\fontsize{12.000000}{15.000000}\color[rgb]{1,0,0} {\sffamily  }}"
        ]
    );
    assert!(figure.figure.starts_with(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%LanguageLevel: 3\n%%BoundingBox: 0 0 231 144\n\
         %%HiResBoundingBox: 0.000000 0.000000 230.400000 144.000000\n%%EndComments\n\
         %%BeginProlog\n/mpldict 8 dict def\nmpldict begin\n/_d { bind def } bind def/m { moveto } _d"
    ));
    assert!(figure.figure.contains(
        "/sc { setcachedevice } _d\nend\n%%EndProlog\nmpldict begin\n0 0 translate\n\
         0 0 230.4 144 rectclip\ngsave\n10 20 moveto\n(psmarker0)\nshow\ngrestore\n"
    ));
    assert!(
        figure
            .figure
            .ends_with("stroke\ngrestore\n\nend\nshowpage\n")
    );
    assert_eq!((figure.width, figure.height), (3.2, 2.0));
    assert_eq!(figure.bbox, Bbox::from_extents(0.0, 0.0, 144.0, 3.2 * 72.0));
}

#[test]
fn usetex_texts_without_usetex_are_drawn_plain_and_said_so() {
    let manager = TexManager::new(
        &std::env::temp_dir(),
        TexFonts::default(),
        "",
        TexEngine::Latex,
    );
    let font = FontRequest::new(FontQuery {
        size: 12.0,
        ..FontQuery::families(["DejaVu Sans"])
    });
    let tex = TexText::new("foo", font, manager);
    let run = TextRun::default();
    let clear = GraphicsContext {
        alpha: Some(0.0),
        ..GraphicsContext::default()
    };
    let mut r = renderer(false);
    r.draw_tex(&clear, &run, &tex, Point::new(1.0, 2.0), 0.0)
        .expect("not drawn");
    assert!(
        !document(r).tex_drawn_as_plain,
        "transparent text is skipped"
    );
    let mut r = renderer(false);
    r.draw_tex(
        &GraphicsContext::default(),
        &run,
        &tex,
        Point::new(1.0, 2.0),
        0.0,
    )
    .expect("drawn");
    let plain = document(r);
    assert!(plain.tex_drawn_as_plain);
    assert!(plain.text.contains("0 0 m /f glyphshow"), "{}", plain.text);
}

#[test]
fn documents_note_partially_transparent_colours() {
    let mut r = renderer(false);
    let gc = GraphicsContext::default();
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::TRANSPARENT));
    assert!(!document(r).partially_transparent, "opaque or invisible");
    let mut r = renderer(false);
    let half = Rgba::new(1.0, 0.0, 0.0, 0.5);
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(half));
    let drawn = document(r);
    assert!(drawn.partially_transparent);
    assert!(drawn.text.contains("1 0 0 setrgbcolor"), "drawn opaque");
}
