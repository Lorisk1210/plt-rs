//! Test helpers shared by the crate's unit tests.

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{GraphicsContext, Renderer, RgbaImage};

/// One recorded renderer call.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Call {
    /// `draw_path` with the path's vertices already transformed to display space.
    Path {
        /// Transformed vertices.
        points: Vec<Point>,
        /// Fill colour.
        fill: Option<Rgba>,
        /// Stroke width in points.
        linewidth: f64,
    },
    /// `draw_image` origin and size.
    Image {
        /// Bottom-left corner.
        origin: Point,
        /// Width and height in pixels.
        size: (usize, usize),
    },
    /// `draw_gouraud_triangles` triangle count.
    Gouraud(usize),
}

/// A renderer that records every call, for asserting on draw output.
#[derive(Debug)]
pub(crate) struct RecordingRenderer {
    pub(crate) size: (f64, f64),
    pub(crate) dpi: f64,
    pub(crate) calls: Vec<Call>,
}

impl RecordingRenderer {
    pub(crate) fn new(width: f64, height: f64, dpi: f64) -> Self {
        Self { size: (width, height), dpi, calls: Vec::new() }
    }
}

impl Renderer for RecordingRenderer {
    fn canvas_size(&self) -> (f64, f64) {
        self.size
    }

    fn dpi(&self) -> f64 {
        self.dpi
    }

    fn draw_path(&mut self, gc: &GraphicsContext, path: &Path, transform: &Affine2D, fill: Option<Rgba>) {
        self.calls.push(Call::Path {
            points: path.transformed(transform).vertices().to_vec(),
            fill,
            linewidth: gc.linewidth,
        });
    }

    fn draw_gouraud_triangles(
        &mut self,
        _gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        _colors: &[[Rgba; 3]],
        _transform: &Affine2D,
    ) {
        self.calls.push(Call::Gouraud(triangles.len()));
    }

    fn draw_image(&mut self, _gc: &GraphicsContext, origin: Point, image: &RgbaImage, _transform: Option<&Affine2D>) {
        self.calls.push(Call::Image { origin, size: (image.width(), image.height()) });
    }
}
