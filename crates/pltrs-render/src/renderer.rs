//! The drawing interface every output backend implements.

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_text::TEXT_PATH_FONT_SCALE;
use pltrs_text::tex::{TexError, TexText};
use pltrs_text::{TextEngine, TextRun};

use crate::collection::PathCollection;
use crate::filter::{FilterFn, SharedFilter};
use crate::gc::GraphicsContext;
use crate::image::RgbaImage;
use crate::quad_mesh::QuadMesh;

/// One line of a text as matplotlib's `Text.draw` hands it to a
/// renderer's `draw_text` or `draw_tex`: the line laid out (its
/// [`TextRun::source`] says what it is) and the left end of its baseline,
/// in display coordinates.
#[derive(Clone, Copy, Debug)]
pub struct TextLine<'a> {
    /// The line laid out.
    pub run: &'a TextRun,
    /// The left end of its baseline.
    pub origin: Point,
}

/// A text as matplotlib's `Text.draw` draws it through a renderer: its
/// lines, its rotation, and what drew it (its artist and its font, as the
/// crate drawing it describes them), for renderers that take texts whole
/// (see [`Renderer::draw_text_lines`]).
#[derive(Clone, Copy)]
pub struct TextLines<'a> {
    /// The lines, first to last.
    pub lines: &'a [TextLine<'a>],
    /// The rotation in degrees counter-clockwise.
    pub angle: f64,
    /// What drew the text, as the crate drawing it describes it.
    pub source: &'a dyn std::any::Any,
}

impl std::fmt::Debug for TextLines<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextLines")
            .field("lines", &self.lines)
            .field("angle", &self.angle)
            .finish_non_exhaustive()
    }
}

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

    /// Draws `text` whole, as matplotlib's `Text.draw` hands a renderer a
    /// text's lines one by one, each with its string, font and artist
    /// (`draw_text(gc, x, y, s, prop, angle, ismath, mtext)`); whether
    /// the renderer drew it. The renderers of this crate don't (they draw
    /// each line through [`draw_text`](Self::draw_text) or
    /// [`draw_tex`](Self::draw_tex)), a renderer of the program's does.
    fn draw_text_lines(&mut self, _gc: &GraphicsContext, _text: &TextLines<'_>) -> bool {
        false
    }

    /// Whether the renderer takes texts whole
    /// ([`draw_text_lines`](Self::draw_text_lines)), which those drawing
    /// texts ask before they describe one.
    fn takes_texts_whole(&self) -> bool {
        false
    }

    /// Draws `tex`, a line of TeX text that is the source of `run`
    /// ([`TextSource::Tex`](pltrs_text::TextSource::Tex)), with its origin at
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

    /// Whether the renderer rounds where it puts each glyph (and each line
    /// of TeX) to whole numbers as Python rounds them, as matplotlib's Agg
    /// renderer does (`round(0x40 * x)`), which refuses a position that is
    /// not finite. Renderers drawing glyphs as paths don't.
    fn rounds_glyph_positions(&self) -> bool {
        false
    }

    /// Starts a named group of draw calls (vector backends emit a group).
    fn open_group(&mut self, _name: &str, _gid: Option<&str>) {}

    /// Ends the most recently opened group.
    fn close_group(&mut self, _name: &str) {}

    /// Starts a named group of the draw calls of a part of the artist
    /// being drawn, which draws with that artist's clip regions (the
    /// background of a figure or an axes; an axis' ticks and their lines
    /// and labels, whose gridlines matplotlib clips to one box): a group as
    /// [`open_group`](Self::open_group) starts, but the PDF backend does
    /// not set the clip regions again for the parts.
    fn open_part_group(&mut self, name: &str, gid: Option<&str>) {
        self.open_group(name, gid);
    }

    /// Ends the most recently opened group of a part.
    fn close_part_group(&mut self, name: &str) {
        self.close_group(name);
    }

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

    /// [`stop_filter`](Self::stop_filter) with a filter the renderer may
    /// keep, as matplotlib hands renderers the artist's filter itself: a
    /// renderer passing it on (to a program's renderer, say) overrides
    /// this; the others filter as `stop_filter` does.
    fn stop_shared_filter(&mut self, filter: &SharedFilter) {
        self.stop_filter(filter.as_ref());
    }

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
    let scale = renderer.points_to_pixels(tex.size()) / TEXT_PATH_FONT_SCALE;
    let transform = Affine2D::scaling(scale, scale)
        .rotate_deg(angle)
        .translate(origin.x, origin.y);
    let mut fill_gc = gc.clone();
    fill_gc.linewidth = 0.0;
    renderer.draw_path(&fill_gc, &path, &transform, Some(gc.stroke_color()));
    Ok(())
}

/// Fills `run` as one path in the colour of `gc`, laid out again from its
/// source (matplotlib's `RendererBase._draw_text_as_path`, how path
/// effects and Agg without its own `draw_text` draw text): typeset by
/// `TextToPath` at [`TEXT_PATH_FONT_SCALE`] points and
/// [`TEXT_PATH_DPI`](pltrs_text::TEXT_PATH_DPI), unhinted, then scaled to
/// the text's size in pixels, rotated by `angle` degrees about `origin`
/// and placed there. A run without a source is filled as it is.
pub fn draw_text_as_path<R: Renderer + ?Sized>(
    renderer: &mut R,
    gc: &GraphicsContext,
    run: &TextRun,
    origin: Point,
    angle: f64,
) {
    let engine = TextEngine::global();
    let scaled = run.source.as_ref().and_then(|source| {
        let laid_out = source
            .layout(engine, TEXT_PATH_FONT_SCALE, pltrs_text::TEXT_PATH_DPI)
            .ok()?;
        let scale = renderer.points_to_pixels(source.size()) / TEXT_PATH_FONT_SCALE;
        Some((laid_out.outline(engine), scale))
    });
    let (outline, scale) = scaled.unwrap_or_else(|| (run.outline(engine), 1.0));
    let placement = Affine2D::scaling(scale, scale)
        .rotate_deg(angle)
        .translate(origin.x, origin.y);
    let gc = GraphicsContext {
        linewidth: 0.0,
        ..gc.clone()
    };
    renderer.draw_path(&gc, &outline, &placement, Some(gc.foreground));
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
