//! Rasterized parts of vector documents (matplotlib's `MixedModeRenderer`).
//!
//! A [`MixedModeRenderer`] forwards drawing to a vector backend until it is
//! asked to [start rasterizing](Renderer::start_rasterizing): from then on
//! drawing goes to a raster canvas covering the page at the save
//! resolution, and when rasterizing [stops](Renderer::stop_rasterizing)
//! the drawn part of that canvas is placed in the document as one image.
//!
//! As matplotlib changes the figure's resolution while rasterizing, the
//! artists drawn meanwhile compute their geometry in raster pixels at
//! [`Renderer::raster_dpi`]: the canvas receives exactly the coordinates
//! (rounding included) that a raster save at that resolution draws.

use plt_core::path::convert::SimplifySettings;
use plt_core::{Affine2D, Path, Point, Rgba};
use plt_text::TextRun;
use plt_text::tex::TexError;

use crate::collection::PathCollection;
use crate::filter::FilterFn;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::quad_mesh::QuadMesh;
use crate::raster::{RasterBackendError, RasterRenderer};
use crate::renderer::Renderer;

/// A vector renderer that draws rasterized artists into a raster canvas
/// placed in the document as an image (matplotlib's `MixedModeRenderer`).
pub struct MixedModeRenderer<'a> {
    vector: &'a mut dyn Renderer,
    /// The page size in inches.
    size: (f64, f64),
    dpi: f64,
    simplify: SimplifySettings,
    raster: Option<RasterRenderer>,
    problem: Option<RasterBackendError>,
}

impl std::fmt::Debug for MixedModeRenderer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MixedModeRenderer")
            .field("size", &self.size)
            .field("dpi", &self.dpi)
            .field("rasterizing", &self.raster.is_some())
            .finish_non_exhaustive()
    }
}

impl<'a> MixedModeRenderer<'a> {
    /// Draws into `vector`, whose page is `width × height` inches,
    /// rasterizing at `dpi` with the `path.simplify` settings `simplify`.
    pub fn new(
        vector: &'a mut dyn Renderer,
        width: f64,
        height: f64,
        dpi: f64,
        simplify: SimplifySettings,
    ) -> Self {
        Self {
            vector,
            size: (width, height),
            dpi,
            simplify,
            raster: None,
            problem: None,
        }
    }

    /// Places what is still being rasterized; drawing is over.
    ///
    /// # Errors
    ///
    /// When a raster canvas could not be created (its part of the drawing
    /// then went to the vector renderer).
    pub fn finish(mut self) -> Result<(), RasterBackendError> {
        self.stop_rasterizing();
        self.problem.map_or(Ok(()), Err)
    }

    /// The renderer drawing calls currently go to.
    fn active(&mut self) -> &mut dyn Renderer {
        match &mut self.raster {
            Some(canvas) => canvas,
            None => &mut *self.vector,
        }
    }

    /// The renderer drawing calls currently go to, for queries.
    fn current(&self) -> &dyn Renderer {
        match &self.raster {
            Some(canvas) => canvas,
            None => &*self.vector,
        }
    }
}

impl Renderer for MixedModeRenderer<'_> {
    fn canvas_size(&self) -> (f64, f64) {
        self.current().canvas_size()
    }

    fn dpi(&self) -> f64 {
        self.current().dpi()
    }

    fn points_to_pixels(&self, points: f64) -> f64 {
        self.current().points_to_pixels(points)
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        self.active().draw_path(gc, path, transform, fill);
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
        self.active()
            .draw_markers(gc, marker, marker_transform, path, transform, fill);
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        self.active().draw_path_collection(gc, collection);
    }

    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        self.active().draw_quad_mesh(gc, mesh);
    }

    fn draw_tex(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        self.active().draw_tex(gc, run, origin, angle)
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.active()
            .draw_gouraud_triangles(gc, triangles, colors, transform);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.active().draw_image(gc, origin, image, transform);
    }

    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        self.active().draw_text(gc, run, origin, angle);
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.active().open_group(name, gid);
    }

    /// Filters only while rasterizing, where the raster canvas draws
    /// (matplotlib's mixed renderer hands `start_filter` to its current
    /// renderer).
    fn start_filter(&mut self) {
        self.active().start_filter();
    }

    fn stop_filter(&mut self, filter: &FilterFn<'_>) {
        self.active().stop_filter(filter);
    }

    fn close_group(&mut self, name: &str) {
        self.active().close_group(name);
    }

    /// A raster canvas of the page at the save resolution receives the
    /// drawing from now on.
    fn start_rasterizing(&mut self) {
        let (width, height) = self.size;
        match RasterRenderer::new(width * self.dpi, height * self.dpi, self.dpi) {
            Ok(canvas) => self.raster = Some(canvas.with_simplify(self.simplify)),
            Err(err) => {
                self.problem.get_or_insert(err);
            }
        }
    }

    /// The drawn part of the raster canvas becomes an image of the
    /// document, placed with a default graphics context.
    fn stop_rasterizing(&mut self) {
        let Some(canvas) = self.raster.take() else {
            return;
        };
        let image = canvas.to_image();
        let Some(bounds) = image.opaque_bounds() else {
            return;
        };
        let (left, _, _, bottom) = bounds;
        let height = self.size.1 * self.dpi;
        // From raster pixels to the vector canvas's units (points, or the
        // figure's pixels for PGF): matplotlib's `_figdpi / dpi`.
        let to_canvas = self.vector.canvas_size().1 / self.size.1 / self.dpi;
        let origin = Point::new(
            left as f64 * to_canvas,
            (height - bottom as f64) * to_canvas,
        );
        self.vector.draw_image(
            &GraphicsContext::default(),
            origin,
            &image.crop(bounds),
            None,
        );
    }

    /// The save resolution while a raster canvas receives the drawing.
    fn raster_dpi(&self) -> Option<f64> {
        self.raster.as_ref().map(|_| self.dpi)
    }

    fn option_image_nocomposite(&self) -> bool {
        self.current().option_image_nocomposite()
    }

    fn option_scale_image(&self) -> bool {
        self.current().option_scale_image()
    }

    fn image_magnification(&self) -> f64 {
        self.current().image_magnification()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::{SvgOptions, SvgRenderer};

    #[test]
    fn rasterized_drawing_becomes_one_cropped_image() {
        let mut svg = SvgRenderer::new(72.0, 36.0, 144.0, SvgOptions::default()).unwrap();
        let mut mixed =
            MixedModeRenderer::new(&mut svg, 1.0, 0.5, 144.0, SimplifySettings::default());
        assert_eq!(mixed.raster_dpi(), None);
        assert_eq!(
            mixed.points_to_pixels(1.0),
            1.0,
            "the document is in points"
        );
        mixed.start_rasterizing();
        assert_eq!(mixed.raster_dpi(), Some(144.0));
        assert_eq!(mixed.points_to_pixels(1.0), 2.0, "the canvas is in pixels");
        assert_eq!(mixed.canvas_size(), (144.0, 72.0));
        assert_eq!(mixed.image_magnification(), 1.0);
        assert!(!mixed.option_scale_image());
        let gc = GraphicsContext {
            linewidth: 0.0,
            ..GraphicsContext::default()
        };
        // A 20 × 10 pixel red rectangle at (40, 20) pixels: 10 × 5 pt at
        // (20, 10) pt.
        let place = Affine2D::scaling(20.0, 10.0).translate(40.0, 20.0);
        mixed.draw_path(
            &gc,
            &Path::unit_rectangle(),
            &place,
            Some(Rgba::new(1.0, 0.0, 0.0, 1.0)),
        );
        mixed.stop_rasterizing();
        assert_eq!(mixed.raster_dpi(), None);
        mixed.finish().unwrap();
        let document = svg.finish().unwrap();
        assert_eq!(document.matches("<image").count(), 1);
        assert!(document.contains("x=\"20\""), "{document}");
        assert!(document.contains("width=\"10\" height=\"5\""), "{document}");
    }

    #[test]
    fn nothing_drawn_places_no_image() {
        let mut svg = SvgRenderer::new(72.0, 36.0, 72.0, SvgOptions::default()).unwrap();
        let mut mixed =
            MixedModeRenderer::new(&mut svg, 1.0, 0.5, 72.0, SimplifySettings::default());
        mixed.start_rasterizing();
        mixed.stop_rasterizing();
        mixed.finish().unwrap();
        let document = svg.finish().unwrap();
        assert!(!document.contains("<image"));
    }
}
