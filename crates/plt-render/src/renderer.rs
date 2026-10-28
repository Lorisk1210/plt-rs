//! The drawing interface every output backend implements.

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_text::tex::{TEX_FONT_SCALE, TexError, TexText};
use plt_text::{TextEngine, TextRun};

use crate::collection::PathCollection;
use crate::filter::FilterFn;
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::quad_mesh::QuadMesh;

/// A drawing target: a raster canvas, an SVG/PDF/PS document, a window.
///
/// This mirrors matplotlib's `RendererBase`. All geometry arrives in
/// *display coordinates*: pixels at the renderer's dpi with the origin at
/// the **bottom-left** corner and y pointing up. Transforms passed alongside
/// paths are affine maps into that space; any non-affine part (log scales,
/// polar projections) has already been applied by the caller.
///
/// Only [`draw_path`](Self::draw_path) is mandatory; the batched operations
/// have correct but slow default implementations that backends override
/// when they can do better (e.g. stamping a pre-rasterised marker).
pub trait Renderer {
    /// Canvas width and height in pixels.
    fn canvas_size(&self) -> (f64, f64);

    /// Resolution in dots per inch.
    fn dpi(&self) -> f64;

    /// Converts a length in points (1/72 inch) to pixels.
    fn points_to_pixels(&self, points: f64) -> f64 {
        points * self.dpi() / 72.0
    }

    /// Strokes `path` with `gc` and fills it with `fill`, if given.
    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    );

    /// Draws `marker` (in marker coordinates, mapped by `marker_transform`
    /// to pixel offsets) at every vertex of `path` transformed by `transform`.
    fn draw_markers(
        &mut self,
        gc: &GraphicsContext,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        for &vertex in path.vertices() {
            let at = transform.transform_point(vertex);
            if at.is_finite() {
                let placed = marker_transform.translate(at.x, at.y);
                self.draw_path(gc, marker, &placed, fill);
            }
        }
    }

    /// Draws many paths with per-item properties (scatter, patches, meshes).
    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        for item in collection.items(gc) {
            self.draw_path(&item.gc, item.path, &item.transform, item.fill);
        }
    }

    /// Draws a flat-shaded quad mesh (matplotlib's `draw_quad_mesh`): by
    /// default as a path collection of its cells in matplotlib's order
    /// ([`QuadMesh::cell_paths`]) with the line width of `gc`.
    fn draw_quad_mesh(&mut self, gc: &GraphicsContext, mesh: &QuadMesh<'_>) {
        let paths = mesh.cell_paths();
        let widths = [gc.linewidth];
        let flags = [mesh.antialiased];
        self.draw_path_collection(gc, &mesh.collection(&paths, &widths, &flags));
    }

    /// Draws triangles whose vertex colours are interpolated linearly
    /// (Gouraud shading); `transform` maps the vertices to display space.
    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    );

    /// Draws an image with its **bottom-left** corner at `origin` (display
    /// coordinates). With `transform`, the image's unit square is mapped
    /// through it instead (only used when
    /// [`option_scale_image`](Self::option_scale_image) is `true`).
    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    );

    /// Draws laid-out text with its origin (left end of the baseline) at
    /// `origin`, rotated by `angle` degrees counter-clockwise about it.
    ///
    /// The default fills every glyph outline and rule as a path in the
    /// stroke colour, which is exact for vector output; raster backends
    /// override it to reproduce `FreeType`'s glyph rendering.
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        fill_text(self, gc, run, origin, angle);
    }

    /// Draws `tex`, a line of TeX text that is the source of `run`
    /// ([`TextSource::Tex`](plt_text::TextSource::Tex)), with its origin at
    /// `origin`, rotated by `angle` degrees counter-clockwise about it
    /// (matplotlib's `draw_tex`).
    ///
    /// The default fills the outline matplotlib's `TextToPath` makes of
    /// the typeset text (`RendererBase.draw_tex`).
    ///
    /// # Errors
    ///
    /// When TeX cannot typeset the text or its fonts cannot be drawn.
    fn draw_tex(
        &mut self,
        gc: &GraphicsContext,
        _run: &TextRun,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        fill_tex(self, gc, tex, origin, angle)
    }

    /// Starts a named group of draw calls (vector backends emit a group).
    fn open_group(&mut self, _name: &str, _gid: Option<&str>) {}

    /// Ends the most recently opened group.
    fn close_group(&mut self, _name: &str) {}

    /// Sends the following drawing to a raster canvas until
    /// [`stop_rasterizing`](Self::stop_rasterizing) (matplotlib's
    /// `start_rasterizing`); only vector output distinguishes the modes.
    fn start_rasterizing(&mut self) {}

    /// Places what was drawn since
    /// [`start_rasterizing`](Self::start_rasterizing) in the output as an
    /// image and resumes drawing directly.
    fn stop_rasterizing(&mut self) {}

    /// The resolution of the raster canvas drawing goes to while a vector
    /// renderer is rasterizing, `None` otherwise. Artists drawn meanwhile
    /// compute their geometry at it, in canvas pixels (matplotlib changes
    /// the figure's dpi in `start_rasterizing`).
    fn raster_dpi(&self) -> Option<f64> {
        None
    }

    /// Sends the following drawing to a fresh transparent canvas until
    /// [`stop_filter`](Self::stop_filter) (matplotlib's `start_filter`).
    /// Only raster canvases filter; other backends draw as usual.
    fn start_filter(&mut self) {}

    /// Hands what was drawn since [`start_filter`](Self::start_filter),
    /// cropped to its visible pixels, to `filter` and draws the image it
    /// returns in its place (matplotlib's `stop_filter`).
    fn stop_filter(&mut self, _filter: &FilterFn<'_>) {}

    /// When `true`, images are drawn one by one instead of being
    /// composited into a single image first.
    fn option_image_nocomposite(&self) -> bool {
        false
    }

    /// When `true`, [`draw_image`](Self::draw_image) accepts arbitrary
    /// affine transforms (vector backends); otherwise images arrive
    /// pre-resampled at display resolution.
    fn option_scale_image(&self) -> bool {
        false
    }

    /// The factor by which images are magnified before being drawn
    /// (matplotlib's `get_image_magnification`): vector backends rasterize
    /// images at their own resolution rather than at 72 dpi.
    fn image_magnification(&self) -> f64 {
        1.0
    }
}

/// Fills the outline of TeX text at `origin`, rotated by `angle`, in the
/// stroke colour of `gc` (matplotlib's `_draw_text_as_path` with
/// `ismath="TeX"`): typeset at `TextToPath`'s size, then scaled to the
/// text's size in pixels.
pub(crate) fn fill_tex<R: Renderer + ?Sized>(
    renderer: &mut R,
    gc: &GraphicsContext,
    tex: &TexText,
    origin: Point,
    angle: f64,
) -> Result<(), TexError> {
    let path = tex.text_path()?;
    let scale = renderer.points_to_pixels(tex.size()) / TEX_FONT_SCALE;
    let transform = Affine2D::scaling(scale, scale)
        .rotate_deg(angle)
        .translate(origin.x, origin.y);
    let mut fill_gc = gc.clone();
    fill_gc.linewidth = 0.0;
    renderer.draw_path(&fill_gc, &path, &transform, Some(gc.stroke_color()));
    Ok(())
}

/// The transform placing text drawn at `origin`, rotated by `angle`
/// degrees counter-clockwise about it.
pub(crate) fn text_placement(origin: Point, angle: f64) -> Affine2D {
    Affine2D::identity()
        .rotate_deg(angle)
        .translate(origin.x, origin.y)
}

/// Fills every glyph outline and rule of `run` as a path in the stroke
/// colour of `gc` (the default [`Renderer::draw_text`], which vector
/// backends also use for runs that carry no source).
pub(crate) fn fill_text<R: Renderer + ?Sized>(
    renderer: &mut R,
    gc: &GraphicsContext,
    run: &TextRun,
    origin: Point,
    angle: f64,
) {
    let engine = TextEngine::global();
    let placement = text_placement(origin, angle);
    let mut fill_gc = gc.clone();
    fill_gc.linewidth = 0.0;
    let color = gc.stroke_color();
    for glyph in &run.glyphs {
        let Ok(outline) = engine.glyph_outline(glyph.font, glyph.glyph, glyph.size, run.dpi) else {
            continue;
        };
        let at = Affine2D::translation(glyph.x, glyph.y).then(placement);
        renderer.draw_path(&fill_gc, &outline, &at, Some(color));
    }
    for rect in &run.rects {
        let at = crate::unit_box(rect).then(placement);
        renderer.draw_path(&fill_gc, &Path::unit_rectangle(), &at, Some(color));
    }
}
