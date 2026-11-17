//! Behavioural tests of the raster backend; pixel parity with matplotlib
//! is checked by `tests/agg_backend.rs`.

use pltrs_core::{Affine2D, Bbox, Path, Point, Rgba};

use super::*;
use crate::gc::GraphicsContext;
use crate::quad_mesh::QuadMesh;
use crate::renderer::Renderer;

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Path {
    Path::polygon([
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ])
}

fn alpha_at(r: &RasterRenderer, x: u32, y: u32) -> u8 {
    r.pixmap().pixel(x, y).expect("in bounds").a
}

#[test]
fn text_is_drawn_as_paths_while_the_thread_asks() {
    use pltrs_text::{FontQuery, FontRequest, TextEngine};

    let request = FontRequest::new(FontQuery {
        size: 20.0,
        ..FontQuery::families(["DejaVu Sans"])
    });
    let run = TextEngine::global()
        .layout_line("fi", &request, 72.0)
        .expect("bundled font");
    let gc = GraphicsContext::default();
    let origin = Point::new(5.0, 8.0);
    set_texts_as_paths(Some(Rc::new(|| true)));
    let mut paths = RasterRenderer::new(40.0, 30.0, 72.0).unwrap();
    paths.draw_text(&gc, &run, origin, 0.0);
    set_texts_as_paths(None);
    let mut expected = RasterRenderer::new(40.0, 30.0, 72.0).unwrap();
    crate::renderer::draw_text_as_path(&mut expected, &gc, &run, origin, 0.0);
    assert_eq!(paths.pixmap(), expected.pixmap());
    assert!(paths.pixmap().data().iter().any(|&byte| byte != 0));
    let mut glyphs = RasterRenderer::new(40.0, 30.0, 72.0).unwrap();
    glyphs.draw_text(&gc, &run, origin, 0.0);
    assert!(glyphs.pixmap().data().iter().any(|&byte| byte != 0));
}

#[test]
fn construction_validates_and_truncates() {
    assert!(matches!(
        RasterRenderer::new(10.0, 10.0, 0.0),
        Err(RasterBackendError::Dpi(_))
    ));
    let r = RasterRenderer::new(640.000_000_000_000_1, 480.9, 100.0).unwrap();
    assert_eq!((r.pixmap().width(), r.pixmap().height()), (640, 480));
    assert_eq!(r.canvas_size(), (640.000_000_000_000_1, 480.9));
    assert_eq!(
        r.pixmap().pixel(0, 0),
        Some(pltrs_raster::Rgba8::TRANSPARENT_WHITE)
    );
}

#[test]
fn draws_in_display_coordinates_with_y_up() {
    let mut r = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
    let gc = GraphicsContext {
        linewidth: 0.0,
        ..GraphicsContext::default()
    };
    r.draw_path(
        &gc,
        &square(0.0, 0.0, 4.0, 2.0),
        &Affine2D::identity(),
        Some(Rgba::BLACK),
    );
    // The bottom-left square ends up in the bottom rows of the buffer.
    assert_eq!(alpha_at(&r, 1, 9), 255);
    assert_eq!(alpha_at(&r, 1, 0), 0);
    assert_eq!(r.take_errors(), []);
}

#[test]
fn infinite_sketches_of_filled_paths_exceed_aggs_cell_limit() {
    let mut r = RasterRenderer::new(10.0, 10.0, 72.0).unwrap();
    let sketch = |scale| crate::gc::Sketch {
        scale,
        length: 128.0,
        randomness: 16.0,
    };
    let gc = GraphicsContext {
        sketch: Some(sketch(f64::INFINITY)),
        ..GraphicsContext::default()
    };
    let path = square(0.0, 0.0, 4.0, 2.0);
    r.draw_path(&gc, &path, &Affine2D::identity(), None);
    assert_eq!(r.take_errors(), [], "a stroke alone stays within it");
    r.draw_path(&gc, &path, &Affine2D::identity(), Some(Rgba::BLACK));
    let errors = r.take_errors();
    assert_eq!(
        errors,
        [RasterError::UnsplittablePath {
            filled: true,
            hatched: false,
            simplifiable: false,
        }]
    );
    assert_eq!(
        errors[0].to_string(),
        "Exceeded cell block limit in Agg, however for the following reasons:\n\n\
         - cannot split filled path\n- path.should_simplify is False\n\n\
         we cannot automatically split up this path to draw.\n\n\
         Please manually simplify your path."
    );
    let nan = GraphicsContext {
        sketch: Some(sketch(f64::NAN)),
        ..GraphicsContext::default()
    };
    r.draw_path(&nan, &path, &Affine2D::identity(), Some(Rgba::BLACK));
    assert_eq!(r.take_errors(), [], "a NaN sketch stays within it");
}

#[test]
fn clip_rect_and_clip_path_restrict_drawing() {
    let mut r = RasterRenderer::new(20.0, 20.0, 72.0).unwrap();
    let gc = GraphicsContext {
        linewidth: 0.0,
        clip_rect: Some(Bbox::from_extents(0.0, 0.0, 10.0, 20.0)),
        clip_path: Some(crate::gc::ClipPath::new(
            square(0.0, 0.0, 20.0, 10.0),
            Affine2D::identity(),
        )),
        ..GraphicsContext::default()
    };
    r.draw_path(
        &gc,
        &square(0.0, 0.0, 20.0, 20.0),
        &Affine2D::identity(),
        Some(Rgba::BLACK),
    );
    assert_eq!(alpha_at(&r, 5, 15), 255);
    assert_eq!(alpha_at(&r, 15, 15), 0, "outside the clip rectangle");
    assert_eq!(alpha_at(&r, 5, 5), 0, "outside the clip path");
}

#[test]
fn markers_images_and_triangles_draw() {
    let mut r = RasterRenderer::new(20.0, 20.0, 72.0).unwrap();
    let gc = GraphicsContext::default();
    let positions = Path::polyline(vec![
        Point::new(5.0, 5.0),
        Point::new(f64::NAN, 1.0),
        Point::new(15.0, 15.0),
    ]);
    r.draw_markers(
        &gc,
        &square(-2.0, -2.0, 2.0, 2.0),
        &Affine2D::identity(),
        &positions,
        &Affine2D::identity(),
        Some(Rgba::opaque(1.0, 0.0, 0.0)),
    );
    assert_eq!(r.pixmap().pixel(5, 14).unwrap().r, 255);
    assert_eq!(alpha_at(&r, 10, 10), 0);

    let image = RgbaImage::new(2, 1, vec![0, 0, 255, 255, 0, 255, 0, 255]).unwrap();
    r.draw_image(&gc, Point::new(0.0, 0.0), &image, None);
    assert_eq!(r.pixmap().pixel(0, 19).unwrap().b, 255);
    assert_eq!(r.pixmap().pixel(1, 19).unwrap().g, 255);

    let triangle = [[
        Point::new(10.0, 1.0),
        Point::new(19.0, 1.0),
        Point::new(19.0, 9.0),
    ]];
    r.draw_gouraud_triangles(&gc, &triangle, &[[Rgba::BLACK; 3]], &Affine2D::identity());
    assert_eq!(alpha_at(&r, 17, 16), 255);
}

#[test]
fn png_round_trip() {
    let mut r = RasterRenderer::new(3.0, 2.0, 100.0).unwrap();
    r.draw_path(
        &GraphicsContext::default(),
        &square(0.0, 0.0, 3.0, 2.0),
        &Affine2D::identity(),
        Some(Rgba::WHITE),
    );
    let png = r.to_png().unwrap();
    let decoded = pltrs_codec::png::decode(&png).unwrap();
    assert_eq!(decoded.to_rgba8(), r.pixmap().data());
    assert_eq!(r.to_image().data(), r.pixmap().data());
    r.clear();
    assert_eq!(
        r.into_pixmap().pixel(0, 0),
        Some(pltrs_raster::Rgba8::TRANSPARENT_WHITE)
    );
}

#[test]
fn large_quad_meshes_draw_as_one_collection_would() {
    // More cells than one block, a wavy grid and per-cell colours, some
    // with edges: drawing block by block must match one collection.
    let (rows, columns) = (40, 150);
    let coordinates: Vec<Point> = (0..=rows)
        .flat_map(|r| {
            (0..=columns).map(move |c| {
                let (r, c) = (f64::from(r), f64::from(c));
                Point::new(c * 0.37 + (r * 0.3).sin(), r * 0.41 + (c * 0.2).cos())
            })
        })
        .collect();
    let faces: Vec<Rgba> = (0..rows * columns)
        .map(|i| Rgba::new(f64::from(i % 7) / 7.0, f64::from(i % 11) / 11.0, 0.5, 1.0))
        .collect();
    let mesh = QuadMesh {
        master_transform: Affine2D::scaling(1.1, 1.1),
        columns: columns as usize,
        coordinates: &coordinates,
        facecolors: &faces,
        edgecolors: &[Rgba::BLACK],
        antialiased: true,
    };
    let gc = GraphicsContext {
        linewidth: 0.3,
        ..GraphicsContext::default()
    };
    let mut blocks = RasterRenderer::new(64.0, 24.0, 72.0).unwrap();
    blocks.draw_quad_mesh(&gc, &mesh);
    let mut whole = RasterRenderer::new(64.0, 24.0, 72.0).unwrap();
    let paths = mesh.agg_cell_paths();
    whole.draw_path_collection(&gc, &mesh.collection(&paths, &[0.3], &[true]));
    assert_eq!(blocks.pixmap(), whole.pixmap());
}
