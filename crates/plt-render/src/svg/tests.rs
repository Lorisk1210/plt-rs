//! Behavioural tests of the SVG backend; conformance with matplotlib's
//! documents is checked by plt-spec's vector tests.

use plt_core::{Affine2D, Bbox, Path, Point, Rgba};

use super::*;
use crate::gc::{ClipPath, Hatch};

fn renderer() -> SvgRenderer {
    SvgRenderer::new(
        100.0,
        50.0,
        72.0,
        SvgOptions {
            hashsalt: Some("test".to_owned()),
            metadata: Metadata::none(),
            ..SvgOptions::default()
        },
    )
    .expect("valid canvas")
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
fn document_skeleton() {
    let svg = renderer().finish().expect("document");
    assert!(svg.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\" standalone=\"no\"?>"));
    assert!(svg.contains(
        "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"100pt\" height=\"50pt\" \
         viewBox=\"0 0 100 50\" xmlns=\"http://www.w3.org/2000/svg\" version=\"1.1\">"
    ));
    assert!(svg.contains("<dc:format>image/svg+xml</dc:format>"));
    assert!(svg.ends_with("</svg>\n"));
    assert!(SvgRenderer::new(0.0, 1.0, 72.0, SvgOptions::default()).is_err());
}

#[test]
fn paths_are_flipped_and_styled() {
    let mut r = renderer();
    let gc = GraphicsContext {
        linewidth: 0.0,
        ..GraphicsContext::default()
    };
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    let svg = r.finish().expect("document");
    assert!(svg.contains(
        "<path d=\"M 0 50 \nL 10 50 \nL 10 40 \nL 0 40 \nz\n\" style=\"fill: #ffffff\"/>"
    ));
}

#[test]
fn clips_and_hatches_are_defined_at_the_end() {
    let mut r = renderer();
    let gc = GraphicsContext {
        clip_rect: Some(Bbox::from_extents(1.0, 2.0, 11.0, 22.0)),
        hatch: Some(Hatch {
            pattern: "/".to_owned(),
            color: Rgba::BLACK,
            linewidth: 1.0,
        }),
        ..GraphicsContext::default()
    };
    r.draw_path(&gc, &square(), &Affine2D::IDENTITY, Some(Rgba::WHITE));
    let clipped = GraphicsContext {
        clip_path: Some(ClipPath {
            path: square(),
            transform: Affine2D::IDENTITY,
        }),
        ..GraphicsContext::default()
    };
    r.draw_path(&clipped, &square(), &Affine2D::IDENTITY, None);
    let svg = r.finish().expect("document");
    let rect = svg.find("<rect x=\"1\" y=\"28\" width=\"10\" height=\"20\"/>");
    let pattern = svg.find("<pattern id=\"h");
    let clip_path = svg.find("<clipPath id=\"p");
    assert!(rect.is_some() && pattern.is_some() && clip_path.is_some());
    assert!(svg.contains("style=\"fill: url(#h"));
    assert!(svg.contains("stroke-width: 1.0; stroke-linecap: butt"));
}

#[test]
fn markers_are_defined_once() {
    let mut r = renderer();
    let gc = GraphicsContext::default();
    let positions = Path::polyline(vec![
        Point::new(1.0, 1.0),
        Point::new(f64::NAN, 2.0),
        Point::new(3.0, 3.0),
    ]);
    let marker = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(0.0, 3.5)]);
    for _ in 0..2 {
        r.draw_markers(
            &gc,
            &marker,
            &Affine2D::IDENTITY,
            &positions,
            &Affine2D::IDENTITY,
            None,
        );
    }
    let svg = r.finish().expect("document");
    assert_eq!(svg.matches("<path id=\"m").count(), 1);
    assert_eq!(svg.matches("<use ").count(), 4);
    assert!(svg.contains("x=\"3\" y=\"47\""));
}

#[test]
fn images_are_inlined() {
    let mut r = renderer();
    let image = RgbaImage::new(2, 1, vec![255, 0, 0, 255, 0, 0, 255, 255]).expect("2x1");
    r.draw_image(
        &GraphicsContext::default(),
        Point::new(5.0, 5.0),
        &image,
        None,
    );
    let svg = r.finish().expect("document");
    assert!(svg.contains("xlink:href=\"data:image/png;base64,\niVBORw0KGgo"));
    assert!(svg.contains("transform=\"scale(1 -1) translate(0 -1)\" x=\"5\" y=\"-44\""));
}
