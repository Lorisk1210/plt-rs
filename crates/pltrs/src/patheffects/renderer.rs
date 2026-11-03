//! The renderer proxy applying path effects (matplotlib's
//! `PathEffectRenderer`).

use std::borrow::Cow;

use pltrs_core::{Affine2D, Path, Point, Rgba};
use pltrs_render::{
    FilterFn, GraphicsContext, PathCollection, Renderer, RgbaImage, SharedFilter, draw_text_as_path,
};
use pltrs_text::TextRun;

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

    /// Draws `path` through each effect in turn, the effects sharing `gc`
    /// (an effect of the program's may change it for those after it).
    fn draw_path_shared(
        &mut self,
        gc: &mut Cow<'_, GraphicsContext>,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        for (effect, colors) in &self.effects {
            effect.draw_path(self.inner, gc, path, transform, fill, *colors);
        }
    }

    /// Draws `marker` at every vertex of `path` through the effects, which
    /// share `gc` over all the markers, one effect after the other
    /// (matplotlib hands every marker of every effect the same graphics
    /// context).
    fn draw_markers_shared(
        &mut self,
        gc: &mut Cow<'_, GraphicsContext>,
        marker: &Path,
        marker_transform: &Affine2D,
        path: &Path,
        transform: &Affine2D,
        fill: Option<Rgba>,
    ) {
        if self.effects.len() > 1 {
            return self.per_effect(|single| {
                single.draw_markers_shared(gc, marker, marker_transform, path, transform, fill);
            });
        }
        for &vertex in path.vertices() {
            let at = transform.transform_point(vertex);
            if at.is_finite() {
                let placed = marker_transform.translate(at.x, at.y);
                self.draw_path_shared(gc, marker, &placed, fill);
            }
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
        self.draw_path_shared(&mut Cow::Borrowed(gc), path, transform, fill);
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
        let mut shared = Cow::Borrowed(gc);
        self.draw_markers_shared(&mut shared, marker, marker_transform, path, transform, fill);
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
        draw_text_as_path(self, gc, run, origin, angle);
    }

    fn open_group(&mut self, name: &str, gid: Option<&str>) {
        self.inner.open_group(name, gid);
    }

    fn close_group(&mut self, name: &str) {
        self.inner.close_group(name);
    }

    fn open_part_group(&mut self, name: &str, gid: Option<&str>) {
        self.inner.open_part_group(name, gid);
    }

    fn close_part_group(&mut self, name: &str) {
        self.inner.close_part_group(name);
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

    fn stop_shared_filter(&mut self, filter: &SharedFilter) {
        self.inner.stop_shared_filter(filter);
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
