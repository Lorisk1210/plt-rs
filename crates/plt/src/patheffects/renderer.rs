//! The renderer proxy applying path effects (matplotlib's
//! `PathEffectRenderer`).

use plt_core::{Affine2D, Path, Point, Rgba};
use plt_render::{FilterFn, GraphicsContext, PathCollection, Renderer, RgbaImage};
use plt_text::{TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE, TextEngine, TextRun};

use super::{EffectColors, PathEffect};
use crate::colors::ColorError;

/// Wraps a renderer so that every path drawn goes through each effect in
/// turn; markers and collections are drawn path by path, once per effect,
/// and text is drawn as its glyph outlines.
pub struct PathEffectRenderer<'r> {
    /// The effects with their colours resolved.
    effects: Vec<(PathEffect, EffectColors)>,
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
    ///
    /// # Errors
    ///
    /// When a colour of an effect does not resolve.
    pub fn new(effects: Vec<PathEffect>, inner: &'r mut dyn Renderer) -> Result<Self, ColorError> {
        let effects = effects
            .into_iter()
            .map(|effect| effect.colors().map(|colors| (effect, colors)))
            .collect::<Result<_, _>>()?;
        Ok(Self::resolved(effects, inner))
    }

    /// A proxy drawing through `effects`, whose colours are resolved, onto
    /// `inner`.
    pub(super) fn resolved(
        effects: Vec<(PathEffect, EffectColors)>,
        inner: &'r mut dyn Renderer,
    ) -> Self {
        Self { effects, inner }
    }

    /// Runs `draw` once per effect, each time on a proxy holding only that
    /// effect, so that every item is drawn with one effect before the next
    /// (matplotlib's `copy_with_path_effect` recursion).
    fn per_effect(&mut self, mut draw: impl FnMut(&mut PathEffectRenderer<'_>)) {
        for effect in self.effects.clone() {
            draw(&mut PathEffectRenderer::resolved(
                vec![effect],
                &mut *self.inner,
            ));
        }
    }
}

impl Renderer for PathEffectRenderer<'_> {
    fn canvas_size(&self) -> (f64, f64) {
        self.inner.canvas_size()
    }

    fn dpi(&self) -> f64 {
        self.inner.dpi()
    }

    fn points_to_pixels(&self, points: f64) -> f64 {
        self.inner.points_to_pixels(points)
    }

    fn draw_path(
        &mut self,
        gc: &GraphicsContext,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        for (effect, colors) in &self.effects {
            effect.draw_path(self.inner, gc, path, transform, fill, *colors);
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
    /// stroke (matplotlib's `_draw_text_as_path`): the text laid out at
    /// 100 points and 72 dpi (`TextToPath.get_text_path`), scaled to the
    /// text size; runs without a source keep their own outlines.
    fn draw_text(&mut self, gc: &GraphicsContext, run: &TextRun, origin: Point, angle: f64) {
        let engine = TextEngine::global();
        let scaled = run.source.as_ref().and_then(|source| {
            let laid_out = source
                .layout(engine, TEXT_PATH_FONT_SCALE, TEXT_PATH_DPI)
                .ok()?;
            let scale = self.points_to_pixels(source.size()) / TEXT_PATH_FONT_SCALE;
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
        self.draw_path(&gc, &outline, &placement, Some(gc.foreground));
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.inner.open_group(name, gid);
    }

    fn close_group(&mut self, name: &str) {
        self.inner.close_group(name);
    }

    fn start_rasterizing(&mut self) {
        self.inner.start_rasterizing();
    }

    fn stop_rasterizing(&mut self) {
        self.inner.stop_rasterizing();
    }

    fn raster_dpi(&self) -> Option<f64> {
        self.inner.raster_dpi()
    }

    fn start_filter(&mut self) {
        self.inner.start_filter();
    }

    fn stop_filter(&mut self, filter: &FilterFn<'_>) {
        self.inner.stop_filter(filter);
    }

    fn option_image_nocomposite(&self) -> bool {
        self.inner.option_image_nocomposite()
    }

    fn option_scale_image(&self) -> bool {
        self.inner.option_scale_image()
    }

    fn image_magnification(&self) -> f64 {
        self.inner.image_magnification()
    }
}
