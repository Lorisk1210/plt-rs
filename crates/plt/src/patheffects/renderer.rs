//! The renderer proxy applying path effects (matplotlib's
//! `PathEffectRenderer`).

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{GraphicsContext, PathCollection, Renderer, RgbaImage};
use plt_text::{TextEngine, TextRun};

use super::PathEffect;

/// Wraps a renderer so that every path drawn goes through each effect in
/// turn; markers and collections are drawn path by path, once per effect,
/// and text is drawn as its glyph outlines.
pub struct PathEffectRenderer<'r> {
    effects: Vec<PathEffect>,
    inner: &'r mut dyn Renderer,
}

impl std::fmt::Debug for PathEffectRenderer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PathEffectRenderer")
            .field("effects", &self.effects)
            .finish_non_exhaustive()
    }
}

impl<'r> PathEffectRenderer<'r> {
    /// A proxy drawing through `effects` onto `inner`.
    pub fn new(effects: Vec<PathEffect>, inner: &'r mut dyn Renderer) -> Self {
        Self { effects, inner }
    }

    /// Runs `draw` once per effect, each time on a proxy holding only that
    /// effect, so that every item is drawn with one effect before the next
    /// (matplotlib's `copy_with_path_effect` recursion).
    fn per_effect(&mut self, mut draw: impl FnMut(&mut PathEffectRenderer<'_>)) {
        for effect in self.effects.clone() {
            draw(&mut PathEffectRenderer::new(vec![effect], &mut *self.inner));
        }
    }
}

/// The outlines of a text run as one compound path relative to its origin
/// (matplotlib's `TextToPath.get_text_path`).
fn text_path(run: &TextRun) -> Path {
    let engine = TextEngine::global();
    let glyphs = run.glyphs.iter().filter_map(|glyph| {
        let outline = engine
            .glyph_outline(glyph.font, glyph.glyph, glyph.size, run.dpi)
            .ok()?;
        Some(outline.transformed(&Affine2D::translation(glyph.x, glyph.y)))
    });
    let rects = run.rects.iter().map(|rect| {
        let (x, y, w, h) = rect.bounds();
        Path::polygon([
            Point::new(x, y),
            Point::new(x + w, y),
            Point::new(x + w, y + h),
            Point::new(x, y + h),
        ])
    });
    let parts: Vec<Path> = glyphs.chain(rects).collect();
    Path::compound(&parts)
}

impl Renderer for PathEffectRenderer<'_> {
    fn canvas_size(&self) -> (f64, f64) {
        self.inner.canvas_size()
    }

    fn dpi(&self) -> f64 {
        self.inner.dpi()
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        for effect in &self.effects {
            effect.draw_path(self.inner, gc, path, transform, fill);
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
        if self.effects.len() > 1 {
            return self.per_effect(|single| {
                single.draw_markers(gc, marker, marker_transform, path, transform, fill);
            });
        }
        for &vertex in path.vertices() {
            let at = transform.transform_point(vertex);
            if at.is_finite() {
                self.draw_path(gc, marker, &marker_transform.translate(at.x, at.y), fill);
            }
        }
    }

    fn draw_path_collection(&mut self, gc: &GraphicsContext, collection: &PathCollection<'_>) {
        if self.effects.len() > 1 {
            return self.per_effect(|single| single.draw_path_collection(gc, collection));
        }
        for item in collection.items(gc) {
            self.draw_path(&item.gc, item.path, &item.transform, item.fill);
        }
    }

    fn draw_gouraud_triangles(
        &mut self,
        gc: &GraphicsContext,
        triangles: &[[Point; 3]],
        colors: &[[Rgba; 3]],
        transform: &Affine2D,
    ) {
        self.inner
            .draw_gouraud_triangles(gc, triangles, colors, transform);
    }

    fn draw_image(
        &mut self,
        gc: &GraphicsContext,
        origin: Point,
        image: &RgbaImage,
        transform: Option<&Affine2D>,
    ) {
        self.inner.draw_image(gc, origin, image, transform);
    }

    /// Text becomes its outlines, filled in the text colour without a
    /// stroke (matplotlib's `_draw_text_as_path`).
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let placement = Affine2D::identity()
            .rotate_deg(angle)
            .translate(origin.x, origin.y);
        let gc = GraphicsContext {
            linewidth: 0.0,
            ..gc.clone()
        };
        self.draw_path(&gc, &text_path(run), &placement, Some(gc.foreground));
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.inner.open_group(name, gid);
    }

    fn close_group(&mut self, name: &str) {
        self.inner.close_group(name);
    }

    fn option_image_nocomposite(&self) -> bool {
        self.inner.option_image_nocomposite()
    }

    fn option_scale_image(&self) -> bool {
        self.inner.option_scale_image()
    }
}
