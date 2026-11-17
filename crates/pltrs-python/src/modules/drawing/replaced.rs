//! The canvas' renderer with drawing methods the program replaced on
//! `RendererAgg` (on the class, or on the canvas' renderer): matplotlib's
//! artists call the renderer's methods, so a program logging
//! `RendererAgg.draw_path` sees every path drawn. pltrs (Rust) draws onto
//! the canvas itself; while methods are replaced, the canvas draws
//! through a renderer that hands those methods to the program's code, as
//! a renderer of the program's receives them (see [`PythonRenderer`]),
//! recording what the original methods the program calls draw, and draws
//! that onto the canvas, while every other call stays on the canvas.

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::{
    FilterFn, GraphicsContext, PathCollection, QuadMesh, Renderer, RgbaImage, SharedFilter,
    TextLines,
};
use pltrs_text::TextRun;
use pltrs_text::tex::{TexError, TexText};

use super::foreign::PythonRenderer;

/// A drawing method the program may replace on `RendererAgg`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::modules) enum Replaced {
    /// `draw_path`.
    Path,
    /// `draw_markers`.
    Markers,
    /// `draw_path_collection`.
    PathCollection,
    /// `draw_quad_mesh`.
    QuadMesh,
    /// `draw_gouraud_triangles`.
    GouraudTriangles,
    /// `draw_image`.
    Image,
    /// `draw_text`.
    Text,
    /// `open_group` and `close_group`.
    Groups,
}

impl Replaced {
    /// The method of matplotlib's name `name`, if the program may replace
    /// it here.
    pub(in crate::modules) fn named(name: &str) -> Option<Self> {
        Some(match name {
            "draw_path" => Self::Path,
            "draw_markers" => Self::Markers,
            "draw_path_collection" => Self::PathCollection,
            "draw_quad_mesh" => Self::QuadMesh,
            "draw_gouraud_triangles" => Self::GouraudTriangles,
            "draw_image" => Self::Image,
            "draw_text" => Self::Text,
            "open_group" | "close_group" => Self::Groups,
            _ => return None,
        })
    }
}

/// The canvas' renderer `canvas`, its methods `replaced` handed to the
/// program's code through `python` (the canvas' `RendererAgg`).
pub(in crate::modules) struct ReplacingRenderer<'r> {
    canvas: &'r mut dyn Renderer,
    python: PythonRenderer,
    replaced: Vec<Replaced>,
}

impl<'r> ReplacingRenderer<'r> {
    /// The canvas' renderer `canvas` with the methods `replaced` going
    /// through `python`, recording.
    pub(in crate::modules) fn new(
        canvas: &'r mut dyn Renderer,
        python: PythonRenderer,
        replaced: Vec<Replaced>,
    ) -> Self {
        Self {
            canvas,
            python,
            replaced,
        }
    }

    /// Whether the program replaced `method`.
    fn replaces(&self, method: Replaced) -> bool {
        self.replaced.contains(&method)
    }

    /// Draws what `call` makes the program's method record onto the
    /// canvas.
    fn through(&mut self, call: impl FnOnce(&mut PythonRenderer)) {
        let drawing = self.python.recording(call);
        if let Some(drawing) = drawing {
            // Its problems raise as the program's code raised: none here.
            drop(drawing.draw_onto(self.canvas));
        }
    }
}

impl Renderer for ReplacingRenderer<'_> {
    fn canvas_size(&self) -> (f64, f64) {
        self.canvas.canvas_size()
    }

    fn dpi(&self) -> f64 {
        self.canvas.dpi()
    }

    fn points_to_pixels(&self, points: f64) -> f64 {
        self.canvas.points_to_pixels(points)
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        if self.replaces(Replaced::Path) {
            self.through(|python| python.draw_path(gc, path, transform, fill));
        } else {
            self.canvas.draw_path(gc, path, transform, fill);
        }
    }

    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        if self.replaces(Replaced::Markers) {
            self.through(|python| {
                python.draw_markers(gc, marker, marker_transform, path, transform, fill);
            });
        } else {
            self.canvas
                .draw_markers(gc, marker, marker_transform, path, transform, fill);
        }
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        if self.replaces(Replaced::PathCollection) {
            self.through(|python| python.draw_path_collection(gc, collection));
        } else {
            self.canvas.draw_path_collection(gc, collection);
        }
    }

    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        if self.replaces(Replaced::QuadMesh) {
            self.through(|python| python.draw_quad_mesh(gc, mesh));
        } else {
            self.canvas.draw_quad_mesh(gc, mesh);
        }
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        if self.replaces(Replaced::GouraudTriangles) {
            self.through(|python| python.draw_gouraud_triangles(gc, triangles, colors, transform));
        } else {
            self.canvas
                .draw_gouraud_triangles(gc, triangles, colors, transform);
        }
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        if self.replaces(Replaced::Image) {
            self.through(|python| python.draw_image(gc, origin, image, transform));
        } else {
            self.canvas.draw_image(gc, origin, image, transform);
        }
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.canvas.draw_text(gc, run, origin, angle);
    }

    fn draw_text_lines(&mut self, gc: &GraphicsContext, text: &TextLines<'_>) -> bool {
        let mut drawn = false;
        self.through(|python| drawn = python.draw_text_lines(gc, text));
        drawn
    }

    fn takes_texts_whole(&self) -> bool {
        self.replaces(Replaced::Text)
    }

    fn draw_tex(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        self.canvas.draw_tex(gc, run, tex, origin, angle)
    }

    fn rounds_glyph_positions(&self) -> bool {
        self.canvas.rounds_glyph_positions()
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        if self.replaces(Replaced::Groups) {
            self.through(|python| python.open_group(name, gid));
        }
        self.canvas.open_group(name, gid);
    }

    fn close_group(&mut self, name: &str) {
        if self.replaces(Replaced::Groups) {
            self.through(|python| python.close_group(name));
        }
        self.canvas.close_group(name);
    }

    fn open_part_group(&mut self, name: &str, gid: Option<&str>) {
        if self.replaces(Replaced::Groups) {
            self.through(|python| python.open_group(name, gid));
        }
        self.canvas.open_part_group(name, gid);
    }

    fn close_part_group(&mut self, name: &str) {
        if self.replaces(Replaced::Groups) {
            self.through(|python| python.close_group(name));
        }
        self.canvas.close_part_group(name);
    }

    fn start_rasterizing(&mut self) {
        self.canvas.start_rasterizing();
    }

    fn stop_rasterizing(&mut self) {
        self.canvas.stop_rasterizing();
    }

    fn raster_dpi(&self) -> Option<f64> {
        self.canvas.raster_dpi()
    }

    fn start_filter(&mut self) {
        self.canvas.start_filter();
    }

    fn stop_filter(&mut self, filter: &FilterFn<'_>) {
        self.canvas.stop_filter(filter);
    }

    fn stop_shared_filter(&mut self, filter: &SharedFilter) {
        self.canvas.stop_shared_filter(filter);
    }

    fn option_image_nocomposite(&self) -> bool {
        self.canvas.option_image_nocomposite()
    }

    fn option_scale_image(&self) -> bool {
        self.canvas.option_scale_image()
    }

    fn image_magnification(&self) -> f64 {
        self.canvas.image_magnification()
    }
}
