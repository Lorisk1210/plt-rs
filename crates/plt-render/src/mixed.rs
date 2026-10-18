//! Rasterized parts of vector documents (matplotlib's `MixedModeRenderer`).
//!
//! A [`MixedModeRenderer`] forwards drawing to a vector backend until it is
//! asked to [start rasterizing](Renderer::start_rasterizing): from then on
//! drawing goes to a raster canvas covering the page at the save
//! resolution, and when rasterizing [stops](Renderer::stop_rasterizing)
//! the drawn part of that canvas is placed in the document as one image.
//!
//! matplotlib changes the figure's resolution while rasterizing so that
//! artists compute their geometry in raster pixels. plt-rs artists keep
//! drawing in points; the raster layer scales every coordinate (and clip
//! region) by `dpi / 72` instead, lays text out again at the raster
//! resolution and asks images for that magnification, which gives the same
//! pixels.

use plt_core::path::convert::SimplifySettings;
use plt_core::{Affine2D, Bbox, Path, Point, Rgba};
use plt_text::{TextEngine, TextRun};

use crate::collection::PathCollection;
use crate::filter::FilterFn;
use crate::gc::{ClipPath, GraphicsContext};
use crate::image::RgbaImage;
use crate::quad_mesh::QuadMesh;
use crate::raster::{RasterBackendError, RasterRenderer};
use crate::renderer::{Renderer, fill_text};
use crate::vector::POINTS_DPI;

/// A raster canvas drawn into in points.
#[derive(Debug)]
struct RasterLayer {
    canvas: RasterRenderer,
    /// Pixels per point.
    scale: f64,
}

impl RasterLayer {
    /// The map from points to raster pixels.
    fn to_pixels(&self) -> Affine2D {
        Affine2D::scaling(self.scale, self.scale)
    }

    /// `gc` with its clip regions in raster pixels.
    fn scaled_gc(&self, gc: &GraphicsContext) -> GraphicsContext {
        GraphicsContext {
            clip_rect: gc.clip_rect.map(|rect| {
                let low = self.pixel(Point::new(rect.x0(), rect.y0()));
                let high = self.pixel(Point::new(rect.x1(), rect.y1()));
                Bbox::from_extents(low.x, low.y, high.x, high.y)
            }),
            clip_path: gc.clip_path.as_ref().map(|clip| ClipPath {
                path: clip.path.clone(),
                transform: clip.transform.then(self.to_pixels()),
            }),
            ..gc.clone()
        }
    }

    /// `point` (in points) in raster pixels, rounded to a billionth of a
    /// pixel: geometry computed in points and scaled carries rounding
    /// noise where matplotlib, computing at the raster resolution, gets
    /// exact values, and on a pixel centre (a mesh cell edge, say) that
    /// noise would decide which side of the edge the pixel falls on.
    fn pixel(&self, point: Point) -> Point {
        let round = |v: f64| (v * self.scale * PIXEL_GRID).round() / PIXEL_GRID;
        Point::new(round(point.x), round(point.y))
    }

    /// `path` mapped by `transform` (to points), in raster pixels.
    fn pixel_path(&self, path: &Path, transform: &Affine2D) -> Path {
        let mut placed = path.transformed(transform);
        for vertex in placed.vertices_mut() {
            *vertex = self.pixel(*vertex);
        }
        placed
    }
}

/// Subdivisions of a pixel raster coordinates are rounded to.
const PIXEL_GRID: f64 = 1e9;

impl Renderer for RasterLayer {
    fn canvas_size(&self) -> (f64, f64) {
        let (width, height) = self.canvas.canvas_size();
        (width / self.scale, height / self.scale)
    }

    fn dpi(&self) -> f64 {
        self.canvas.dpi()
    }

    /// Lengths are points, as in the vector document.
    fn points_to_pixels(&self, points: f64) -> f64 {
        points
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        let gc = self.scaled_gc(gc);
        let placed = self.pixel_path(path, transform);
        self.canvas
            .draw_path(&gc, &placed, &Affine2D::IDENTITY, fill);
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
        let gc = self.scaled_gc(gc);
        let positions = self.pixel_path(path, transform);
        self.canvas.draw_markers(
            &gc,
            marker,
            &marker_transform.then(self.to_pixels()),
            &positions,
            &Affine2D::IDENTITY,
            fill,
        );
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        let gc = self.scaled_gc(gc);
        let offsets: Vec<Point> = collection.offsets.iter().map(|&p| self.pixel(p)).collect();
        let scaled = PathCollection {
            master_transform: collection.master_transform.then(self.to_pixels()),
            offsets: &offsets,
            ..collection.clone()
        };
        self.canvas.draw_path_collection(&gc, &scaled);
    }

    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        let gc = self.scaled_gc(gc);
        let coordinates: Vec<Point> = mesh
            .coordinates
            .iter()
            .map(|&p| self.pixel(mesh.master_transform.transform_point(p)))
            .collect();
        let scaled = QuadMesh {
            master_transform: Affine2D::IDENTITY,
            coordinates: &coordinates,
            ..*mesh
        };
        self.canvas.draw_quad_mesh(&gc, &scaled);
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        let gc = self.scaled_gc(gc);
        let placed: Vec<[Point; 3]> = triangles
            .iter()
            .map(|triangle| triangle.map(|p| self.pixel(transform.transform_point(p))))
            .collect();
        self.canvas
            .draw_gouraud_triangles(&gc, &placed, colors, &Affine2D::IDENTITY);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        let gc = self.scaled_gc(gc);
        let transform = transform.map(|t| t.then(self.to_pixels()));
        self.canvas
            .draw_image(&gc, self.pixel(origin), image, transform.as_ref());
    }

    /// Text is laid out again at the raster resolution; runs without a
    /// source are filled as scaled outlines.
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let relaid = run.source.as_ref().and_then(|source| {
            source
                .layout(TextEngine::global(), source.size(), self.canvas.dpi())
                .ok()
        });
        match relaid {
            Some(relaid) => {
                let gc = self.scaled_gc(gc);
                self.canvas
                    .draw_text(&gc, &relaid, self.pixel(origin), angle);
            }
            None => fill_text(self, gc, run, origin, angle),
        }
    }

    fn option_image_nocomposite(&self) -> bool {
        self.canvas.option_image_nocomposite()
    }

    fn start_filter(&mut self) {
        self.canvas.start_filter();
    }

    fn stop_filter(&mut self, filter: &FilterFn<'_>) {
        self.canvas.stop_filter(filter);
    }

    /// Images are resampled at the raster resolution.
    fn image_magnification(&self) -> f64 {
        self.scale
    }
}

/// A vector renderer that draws rasterized artists into a raster canvas
/// placed in the document as an image (matplotlib's `MixedModeRenderer`).
pub struct MixedModeRenderer<'a> {
    vector: &'a mut dyn Renderer,
    /// The page size in inches.
    size: (f64, f64),
    dpi: f64,
    simplify: SimplifySettings,
    raster: Option<RasterLayer>,
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
            Some(layer) => layer,
            None => &mut *self.vector,
        }
    }
}

impl Renderer for MixedModeRenderer<'_> {
    fn canvas_size(&self) -> (f64, f64) {
        self.vector.canvas_size()
    }

    fn dpi(&self) -> f64 {
        self.vector.dpi()
    }

    fn points_to_pixels(&self, points: f64) -> f64 {
        self.vector.points_to_pixels(points)
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
        let scale = self.dpi / POINTS_DPI;
        match RasterRenderer::new(width * self.dpi, height * self.dpi, self.dpi) {
            Ok(canvas) => {
                self.raster = Some(RasterLayer {
                    canvas: canvas.with_simplify(self.simplify),
                    scale,
                });
            }
            Err(err) => {
                self.problem.get_or_insert(err);
            }
        }
    }

    /// The drawn part of the raster canvas becomes an image of the
    /// document, placed with a default graphics context.
    fn stop_rasterizing(&mut self) {
        let Some(layer) = self.raster.take() else {
            return;
        };
        let image = layer.canvas.to_image();
        let Some(bounds) = image.opaque_bounds() else {
            return;
        };
        let (left, _, _, bottom) = bounds;
        let height = self.size.1 * self.dpi;
        let to_points = POINTS_DPI / self.dpi;
        let origin = Point::new(
            left as f64 * to_points,
            (height - bottom as f64) * to_points,
        );
        self.vector.draw_image(
            &GraphicsContext::default(),
            origin,
            &image.crop(bounds),
            None,
        );
    }

    fn option_image_nocomposite(&self) -> bool {
        match &self.raster {
            Some(layer) => layer.option_image_nocomposite(),
            None => self.vector.option_image_nocomposite(),
        }
    }

    fn option_scale_image(&self) -> bool {
        self.raster.is_none() && self.vector.option_scale_image()
    }

    fn image_magnification(&self) -> f64 {
        match &self.raster {
            Some(layer) => layer.image_magnification(),
            None => self.vector.image_magnification(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::{SvgOptions, SvgRenderer};

    fn square() -> Path {
        Path::unit_rectangle()
    }

    #[test]
    fn rasterized_drawing_becomes_one_cropped_image() {
        let mut svg = SvgRenderer::new(72.0, 36.0, 144.0, SvgOptions::default()).unwrap();
        let mut mixed =
            MixedModeRenderer::new(&mut svg, 1.0, 0.5, 144.0, SimplifySettings::default());
        mixed.start_rasterizing();
        assert_eq!(mixed.image_magnification(), 2.0);
        assert!(!mixed.option_scale_image());
        let gc = GraphicsContext {
            linewidth: 0.0,
            ..GraphicsContext::default()
        };
        // A 10 × 5 pt red rectangle at (20, 10) pt: 20 × 10 pixels.
        let place = Affine2D::scaling(10.0, 5.0).translate(20.0, 10.0);
        mixed.draw_path(&gc, &square(), &place, Some(Rgba::new(1.0, 0.0, 0.0, 1.0)));
        mixed.stop_rasterizing();
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
