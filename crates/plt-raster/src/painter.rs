//! The high-level drawing API, mirroring what matplotlib's `RendererAgg`
//! does for each primitive.

use plt_core::{Affine2D, Path, Point, Rgba};

use crate::clip::{Clip, ClipBox};
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::geometry::PathSink;
use crate::geometry::curve::CurveFlattener;
use crate::geometry::dash::{DashSink, Dasher};
use crate::geometry::stroke::{StrokeSink, Stroker};
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{Gamma, Rasterizer, Scanline};
use crate::render::Surface;
use crate::render::gouraud::GouraudTriangle;
use crate::render::image::CoverageSampler;
use crate::render::marker::MarkerStamp;
use crate::style::{Antialias, FillRule, LineCap, StrokeStyle};

/// Which Agg scanline container matplotlib renders an operation with;
/// it decides how solid runs interact with a clip mask (see
/// `Surface::with_packed_spans`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scanlines {
    /// `scanline_p8`, used for hatch patterns.
    Packed,
    /// `scanline_u8`, used for Gouraud triangles and images.
    Unpacked,
}

/// Draws paths, patterns, markers and shaded triangles into pixmaps.
///
/// A painter owns the rasterizer and all scratch buffers, so reusing one
/// painter for many operations avoids per-call allocation. Paths are in
/// device pixels (y down) and must already be transformed, NaN-free,
/// snapped and simplified as the caller requires; curves are flattened
/// here exactly like Agg's `conv_curve`.
///
/// Clip rectangles clip the geometry (Agg's rasterizer clip box); the
/// target's bounds always clip pixels.
#[derive(Debug, Default)]
pub struct Painter {
    rasterizer: Rasterizer,
    scanline: Scanline,
    flattener: CurveFlattener,
    stroker: Stroker,
    dasher: Dasher,
    colors: Vec<Rgba8>,
}

impl Painter {
    /// A painter with matplotlib's rasterizer settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// A painter whose rasterizer fails with
    /// [`RasterError::CellLimitExceeded`] beyond `limit` cells.
    pub fn with_cell_limit(limit: usize) -> Self {
        Self {
            rasterizer: Rasterizer::with_cell_limit(limit),
            ..Self::default()
        }
    }

    /// Sets the coverage-to-alpha function for subsequent operations
    /// (matplotlib uses the default, [`Gamma::Linear`]).
    pub fn set_gamma(&mut self, gamma: Gamma) {
        self.rasterizer.set_gamma(gamma);
    }

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
        self.begin(target, clip.rect, rule);
        self.flattener.feed(path, &mut self.rasterizer);
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
        self.begin(target, clip.rect, rule);
        self.flattener.feed(path, &mut self.rasterizer);
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

    /// Renders a `size × size` hatch tile the way matplotlib does: the
    /// tile is cleared to `background` (matplotlib uses
    /// [`Rgba8::TRANSPARENT_WHITE`]), then `hatch` (tile pixel coordinates)
    /// is filled and stroked in `color` with square caps and Agg's default
    /// joins, without clipping.
    pub fn hatch_tile(
        &mut self,
        size: u32,
        hatch: &Path,
        color: Rgba,
        linewidth: f64,
        background: Rgba8,
    ) -> Result<Pixmap, RasterError> {
        let mut tile = Pixmap::filled(size, size, background)?;
        let color = Rgba8::from(color);
        self.rasterizer.reset_clipping();
        self.rasterizer.set_fill_rule(FillRule::NonZero);
        self.flattener.feed(hatch, &mut self.rasterizer);
        self.render_solid(&mut tile, color, Antialias::On, None)?;
        self.rasterizer.reset();
        self.add_stroke(
            hatch,
            &StrokeStyle::new(linewidth).with_cap(LineCap::Square),
        );
        self.render_solid(&mut tile, color, Antialias::On, None)?;
        Ok(tile)
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

    /// Rasterizes a marker once: the fill of `path` (if `fill`) and its
    /// stroke (if `stroke` is given), both anti-aliased and unclipped, as
    /// matplotlib's `draw_markers` does. Stamp it with [`MarkerStamp::draw`].
    pub fn marker_stamp(
        &mut self,
        path: &Path,
        fill: bool,
        stroke: Option<&StrokeStyle>,
    ) -> Result<MarkerStamp, RasterError> {
        self.rasterizer.reset_clipping();
        self.rasterizer.set_fill_rule(FillRule::NonZero);
        let mut fill_rows = Vec::new();
        if fill {
            self.flattener.feed(path, &mut self.rasterizer);
            self.rasterizer
                .sweep(&mut self.scanline, |sl| fill_rows.push(sl.clone()))?;
            self.rasterizer.reset();
        }
        let mut stroke_rows = Vec::new();
        if let Some(style) = stroke {
            self.add_stroke(path, style);
            self.rasterizer
                .sweep(&mut self.scanline, |sl| stroke_rows.push(sl.clone()))?;
        }
        Ok(MarkerStamp::new(fill_rows, stroke_rows))
    }

    /// Rasterizes `path` into a `width × height` coverage mask — for clip
    /// paths (matplotlib's `render_clippath`) and glyph bitmaps. The
    /// geometry is clipped to `rect` if given, otherwise not at all:
    /// matplotlib rasterizes clip paths with the current clip box (or the
    /// canvas bounds) for paths, images and triangles, but unclipped for
    /// markers. With [`Antialias::Off`] every touched pixel becomes 255.
    pub fn coverage_mask(
        &mut self,
        path: &Path,
        width: u32,
        height: u32,
        rule: FillRule,
        aa: Antialias,
        rect: Option<ClipBox>,
    ) -> Result<Mask, RasterError> {
        let mut mask = Mask::new(width, height)?;
        if let Some(rect) = rect {
            self.set_clip(rect, rule);
        } else {
            self.rasterizer.reset_clipping();
            self.rasterizer.set_fill_rule(rule);
        }
        self.flattener.feed(path, &mut self.rasterizer);
        let full = ClipBox::full(width, height);
        let Some(bounds) = rect.map_or(Some(full), |r| r.within(width, height)) else {
            return Ok(mask);
        };
        self.rasterizer.sweep(&mut self.scanline, |sl| {
            let Some(row) = u32::try_from(sl.y()).ok().and_then(|y| mask.row_mut(y)) else {
                return;
            };
            for span in sl.spans() {
                for i in 0..span.len() {
                    let x = span.x + i as i32;
                    if x >= bounds.x0() && x < bounds.x1() {
                        row[x as usize] = if aa == Antialias::Off {
                            255
                        } else {
                            span.cover_at(i)
                        };
                    }
                }
            }
        })?;
        Ok(mask)
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

    /// Resets the rasterizer for a new operation on `target`.
    fn begin(&mut self, target: &Pixmap, rect: Option<ClipBox>, rule: FillRule) {
        let rect = rect.unwrap_or_else(|| ClipBox::full(target.width(), target.height()));
        self.set_clip(rect, rule);
    }

    fn set_clip(&mut self, rect: ClipBox, rule: FillRule) {
        self.rasterizer.set_clip_box(
            f64::from(rect.x0()),
            f64::from(rect.y0()),
            f64::from(rect.x1()),
            f64::from(rect.y1()),
        );
        self.rasterizer.set_fill_rule(rule);
    }

    fn add_polygon(&mut self, points: &[Point]) {
        if let Some((&first, rest)) = points.split_first() {
            self.rasterizer.move_to(first);
            for &p in rest {
                self.rasterizer.line_to(p);
            }
        }
    }

    /// Feeds the stroke outline of `path` (dashed if requested) to the
    /// rasterizer.
    fn add_stroke(&mut self, path: &Path, style: &StrokeStyle) {
        let mut stroke = StrokeSink::new(&mut self.stroker, style, &mut self.rasterizer);
        match &style.dashes {
            Some(pattern) => {
                let mut dash = DashSink::new(&mut self.dasher, pattern, &mut stroke);
                self.flattener.feed(path, &mut dash);
                dash.finish();
            }
            None => self.flattener.feed(path, &mut stroke),
        }
        stroke.finish();
    }

    fn render_solid(
        &mut self,
        target: &mut Pixmap,
        color: Rgba8,
        aa: Antialias,
        mask: Option<&Mask>,
    ) -> Result<(), RasterError> {
        let mut surface = Surface::new(target, None, mask)?;
        self.rasterizer.sweep(&mut self.scanline, |sl| {
            for span in sl.spans() {
                surface.blend_solid(sl.y(), span, color, aa);
            }
        })
    }

    fn render_with<G: FnMut(i32, i32, &mut [Rgba8])>(
        &mut self,
        target: &mut Pixmap,
        mask: Option<&Mask>,
        scanlines: Scanlines,
        mut generate: G,
    ) -> Result<(), RasterError> {
        let mut surface = Surface::new(target, None, mask)?;
        if scanlines == Scanlines::Packed {
            surface = surface.with_packed_spans();
        }
        let colors = &mut self.colors;
        self.rasterizer.sweep(&mut self.scanline, |sl| {
            for span in sl.spans() {
                colors.clear();
                colors.resize(span.len(), Rgba8::default());
                generate(span.x, sl.y(), colors);
                surface.blend_colors(sl.y(), span, colors);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{DashPattern, LineJoin};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn alpha_sum(p: &Pixmap) -> u32 {
        p.data().chunks(4).map(|c| u32::from(c[3])).sum()
    }

    #[test]
    fn fills_and_clips_a_rectangle() {
        let mut target = Pixmap::new(8, 8).unwrap();
        let mut painter = Painter::new();
        let rect = Path::polygon([p(1.0, 1.0), p(7.0, 1.0), p(7.0, 7.0), p(1.0, 7.0)]);
        let clip = Clip::rect(ClipBox::new(0, 0, 4, 8));
        painter
            .fill_path(
                &mut target,
                &rect,
                Rgba::BLACK,
                FillRule::NonZero,
                Antialias::On,
                &clip,
            )
            .unwrap();
        assert_eq!(alpha_sum(&target), 3 * 6 * 255);
        assert_eq!(target.pixel(3, 3).unwrap().a, 255);
        assert_eq!(target.pixel(4, 3).unwrap().a, 0);
    }

    #[test]
    fn strokes_with_and_without_dashes() {
        let mut painter = Painter::new();
        let line = Path::polyline(vec![p(0.0, 2.0), p(20.0, 2.0)]);
        let mut solid = Pixmap::new(20, 4).unwrap();
        let style = StrokeStyle::new(2.0).with_join(LineJoin::Round);
        painter
            .stroke_path(
                &mut solid,
                &line,
                Rgba::BLACK,
                &style,
                Antialias::On,
                &Clip::NONE,
            )
            .unwrap();
        assert_eq!(alpha_sum(&solid), 40 * 255);
        let mut dashed = Pixmap::new(20, 4).unwrap();
        let style = style.with_dashes(Some(DashPattern::new(vec![(5.0, 5.0)], 0.0).unwrap()));
        painter
            .stroke_path(
                &mut dashed,
                &line,
                Rgba::BLACK,
                &style,
                Antialias::On,
                &Clip::NONE,
            )
            .unwrap();
        assert_eq!(alpha_sum(&dashed), 20 * 255);
    }

    #[test]
    fn pattern_fill_repeats_the_tile() {
        let mut painter = Painter::new();
        let mut tile = Pixmap::new(2, 1).unwrap();
        tile.set_pixel(0, 0, Rgba8::new(255, 0, 0, 255));
        let mut target = Pixmap::new(4, 1).unwrap();
        let rect = Path::polygon([p(0.0, 0.0), p(4.0, 0.0), p(4.0, 1.0), p(0.0, 1.0)]);
        painter
            .fill_path_with_pattern(&mut target, &rect, &tile, FillRule::NonZero, &Clip::NONE)
            .unwrap();
        let alphas: Vec<u8> = target.data().chunks(4).map(|c| c[3]).collect();
        assert_eq!(alphas, vec![255, 0, 255, 0]);
    }

    #[test]
    fn coverage_mask_and_markers() {
        let mut painter = Painter::new();
        let square = Path::polygon([p(1.0, 1.0), p(3.0, 1.0), p(3.0, 3.0), p(1.0, 3.0)]);
        let mask = painter
            .coverage_mask(&square, 4, 4, FillRule::NonZero, Antialias::On, None)
            .unwrap();
        assert_eq!(
            mask.data().iter().map(|&v| u32::from(v)).sum::<u32>(),
            4 * 255
        );
        let stamp = painter
            .marker_stamp(&square, true, Some(&StrokeStyle::new(1.0)))
            .unwrap();
        assert_eq!(stamp.bounds(), Some((0, 0, 3, 3)));
        let mut target = Pixmap::new(10, 10).unwrap();
        stamp
            .draw(
                &mut target,
                &[p(5.0, 5.0)],
                Some(Rgba::BLACK),
                Some(Rgba::BLACK),
                &Clip::NONE,
            )
            .unwrap();
        assert_eq!(target.pixel(7, 7).unwrap().a, 255);
        assert_eq!(target.pixel(2, 2).unwrap().a, 0);
    }

    #[test]
    fn gouraud_and_transformed_coverage_draw_something() {
        let mut painter = Painter::new();
        let mut target = Pixmap::new(16, 16).unwrap();
        let colors = [Rgba::BLACK, Rgba::WHITE, Rgba::opaque(1.0, 0.0, 0.0)];
        painter
            .gouraud_triangle(
                &mut target,
                [p(1.0, 1.0), p(14.0, 2.0), p(3.0, 14.0)],
                colors,
                &Clip::NONE,
            )
            .unwrap();
        assert!(alpha_sum(&target) > 50 * 255);
        let before = target.clone();
        painter
            .gouraud_triangle(
                &mut target,
                [p(f64::NAN, 1.0), p(14.0, 2.0), p(3.0, 14.0)],
                colors,
                &Clip::NONE,
            )
            .unwrap();
        assert_eq!(target, before);

        let mut target = Pixmap::new(16, 16).unwrap();
        let glyph = Mask::from_vec(4, 4, vec![255; 16]).unwrap();
        let t = Affine2D::identity().rotate_deg(30.0).translate(8.0, 4.0);
        painter
            .draw_coverage_transformed(&mut target, &glyph, &t, Rgba::BLACK, &Clip::NONE)
            .unwrap();
        assert!(alpha_sum(&target) > 8 * 254);
    }

    #[test]
    fn hatch_tile_and_errors() {
        let mut painter = Painter::new();
        let diagonal = Path::polyline(vec![p(0.0, 8.0), p(8.0, 0.0)]);
        let tile = painter
            .hatch_tile(8, &diagonal, Rgba::BLACK, 1.0, Rgba8::TRANSPARENT_WHITE)
            .unwrap();
        assert!(alpha_sum(&tile) > 0);
        assert_eq!(tile.pixel(0, 0), Some(Rgba8::TRANSPARENT_WHITE));

        let mut small = Painter::with_cell_limit(2);
        let mut target = Pixmap::new(8, 8).unwrap();
        let big = Path::polygon([p(0.5, 0.5), p(7.5, 0.5), p(7.5, 7.5), p(0.5, 7.5)]);
        assert!(
            small
                .fill_path(
                    &mut target,
                    &big,
                    Rgba::BLACK,
                    FillRule::NonZero,
                    Antialias::On,
                    &Clip::NONE
                )
                .is_err()
        );
        let wrong = Mask::new(2, 2).unwrap();
        assert!(
            painter
                .fill_path(
                    &mut target,
                    &big,
                    Rgba::BLACK,
                    FillRule::NonZero,
                    Antialias::On,
                    &Clip::NONE.with_mask(&wrong)
                )
                .is_err()
        );
    }
}
