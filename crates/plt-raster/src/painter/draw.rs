//! The painter's operations that blend into a target pixmap: solid fills
//! and strokes, hatch-pattern fills, Gouraud triangles and transformed
//! coverage bitmaps.

use plt_core::{Affine2D, Path, Point, Rgba};

use super::{Painter, Scanlines};
use crate::clip::Clip;
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::geometry::{feed_path, is_flat};
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::render::gouraud::GouraudTriangle;
use crate::render::image::CoverageSampler;
use crate::style::{Antialias, FillRule, StrokeStyle};

impl Painter {
    /// Fills `path` with a solid colour (matplotlib's path face).
    pub fn fill_path(
        &mut self,
        target: &mut Pixmap,
        path: &Path,
        color: Rgba,
        rule: FillRule,
        aa: Antialias,
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        let color = Rgba8::from(color);
        if color.a == 0 {
            return Ok(());
        }
        if is_flat(path) {
            // Nothing to cover (dense meshes snap many cells flat).
            return check_mask(target, clip.mask);
        }
        self.begin(target, clip.rect, rule);
        feed_path(path, &mut self.rasterizer);
        self.render_solid(target, color, aa, clip.mask)
    }

    /// Strokes `path` with a solid colour, dashing it first if the style
    /// has a dash pattern (matplotlib's path edge).
    ///
    /// matplotlib rounds the width of non-anti-aliased *dashed* strokes
    /// (`max(0.5, round(w))`) before calling Agg; that is left to the caller.
    pub fn stroke_path(
        &mut self,
        target: &mut Pixmap,
        path: &Path,
        color: Rgba,
        style: &StrokeStyle,
        aa: Antialias,
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        let color = Rgba8::from(color);
        if color.a == 0 {
            return Ok(());
        }
        self.begin(target, clip.rect, FillRule::NonZero);
        self.add_stroke(path, style);
        self.render_solid(target, color, aa, clip.mask)
    }

    /// Fills `path` with `tile` repeated from the target's origin
    /// (matplotlib's hatch transfer: `span_pattern_rgba` over
    /// `image_accessor_wrap`). Always anti-aliased, as in matplotlib, and
    /// like matplotlib's packed scanlines, runs of uniform partial
    /// coverage are drawn at full strength under a mask.
    pub fn fill_path_with_pattern(
        &mut self,
        target: &mut Pixmap,
        path: &Path,
        tile: &Pixmap,
        rule: FillRule,
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        let (tw, th) = (i64::from(tile.width()), i64::from(tile.height()));
        if tw == 0 || th == 0 {
            return Ok(());
        }
        if is_flat(path) {
            return check_mask(target, clip.mask);
        }
        self.begin(target, clip.rect, rule);
        feed_path(path, &mut self.rasterizer);
        let data = tile.data();
        self.render_with(target, clip.mask, Scanlines::Packed, |x, y, out| {
            let row = i64::from(y).rem_euclid(th) * tw;
            for (i, px) in out.iter_mut().enumerate() {
                let col = (i64::from(x) + i as i64).rem_euclid(tw);
                let k = ((row + col) * 4) as usize;
                *px = Rgba8::new(data[k], data[k + 1], data[k + 2], data[k + 3]);
            }
        })
    }

    /// Draws a Gouraud-shaded triangle (matplotlib's
    /// `draw_gouraud_triangles`): the triangle is grown by half a pixel and
    /// colours are interpolated with Agg's `span_gouraud_rgba`. Triangles
    /// with non-finite corners are skipped.
    pub fn gouraud_triangle(
        &mut self,
        target: &mut Pixmap,
        points: [Point; 3],
        colors: [Rgba; 3],
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        if points.iter().any(|p| !p.is_finite()) {
            return Ok(());
        }
        let triangle = GouraudTriangle::new(points, colors.map(Rgba8::from), 0.5);
        self.begin(target, clip.rect, FillRule::NonZero);
        self.add_polygon(triangle.outline());
        let mut spans = triangle.spans();
        self.render_with(target, clip.mask, Scanlines::Unpacked, |x, y, out| {
            spans.generate(x, y, out);
        })
    }

    /// Blends a coverage bitmap in `color` through an affine `transform`
    /// from bitmap pixels (y down, origin at its top-left corner) to the
    /// target, sampling bilinearly like Agg's
    /// `span_image_filter_gray_bilinear` and colouring like matplotlib's
    /// `font_to_rgba`. Non-invertible transforms draw nothing.
    pub fn draw_coverage_transformed(
        &mut self,
        target: &mut Pixmap,
        coverage: &Mask,
        transform: &Affine2D,
        color: Rgba,
        clip: &Clip<'_>,
    ) -> Result<(), RasterError> {
        let Some(sampler) = CoverageSampler::new(coverage, transform, Rgba8::from(color)) else {
            return Ok(());
        };
        self.begin(target, clip.rect, FillRule::NonZero);
        self.add_polygon(&CoverageSampler::outline(
            transform,
            coverage.width(),
            coverage.height(),
        ));
        self.render_with(target, clip.mask, Scanlines::Unpacked, |x, y, out| {
            sampler.generate(x, y, out);
        })
    }
}

/// Fails, as drawing through it would, when `mask` does not cover
/// `target`.
fn check_mask(target: &Pixmap, mask: Option<&Mask>) -> Result<(), RasterError> {
    mask.map_or(Ok(()), |mask| {
        mask.check_covers(target.width(), target.height())
    })
}
