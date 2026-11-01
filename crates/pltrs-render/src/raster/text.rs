//! Text on the raster canvas, following matplotlib's
//! `RendererAgg._draw_text_glyphs_and_boxes`.
//!
//! matplotlib renders each glyph with `FreeType` at its pen position rounded
//! to 1/64 pixel and blits the coverage bitmap unsnapped; here the glyph
//! outline is filled at that same quantized position (never snapped). The
//! outline is grid-fitted as `text.hinting` asked when the run was laid
//! out (its source's font request), as matplotlib loads glyphs with
//! `get_hinting_flag()`.
//! Without anti-aliasing, glyphs are rendered by a port of `FreeType`'s
//! monochrome rasterizer ([`mono`](super::mono)) and their pixels painted
//! at full coverage, as matplotlib blits the bitmap.
//! Rules (mathtext fraction bars and the like) are drawn as anti-aliased
//! paths with the text's snapping, grown to at least one pixel as
//! matplotlib does.

use pltrs_core::{Affine2D, Path, Point};
use pltrs_font::ScanOutline;

use pltrs_raster::{Clip, CoverageRounding, GrayBitmap};
use pltrs_text::{Hinting, TextEngine, TextRun};

use super::RasterRenderer;
use super::bitmap::{GlyphShape, smooth_bitmap};
use super::glyph::{GlyphTransform, is_axis_aligned};
use super::mono;
use crate::gc::{Dashes, GraphicsContext, SnapMode};
use crate::renderer::Renderer;
use crate::renderer::text_placement;

/// `FreeType`'s 26.6 pen position: Python's `round(0x40 * v) / 0x40`.
fn quantize(v: f64) -> f64 {
    (v * 64.0).round_ties_even() / 64.0
}

/// The farthest from the canvas origin, in pixels, that glyphs are drawn
/// (2^31, where matplotlib's integer text position overflows). A glyph
/// placed beyond cannot reach any canvas, and its fixed-point coordinates
/// would overflow.
const MAX_GLYPH_REACH: f64 = 2_147_483_648.0;

/// Whether `p` lies within [`MAX_GLYPH_REACH`] of the origin (NaN does
/// not).
pub(super) fn within_reach(p: Point) -> bool {
    p.x.abs() <= MAX_GLYPH_REACH && p.y.abs() <= MAX_GLYPH_REACH
}

/// The farthest, in pixels, any point of the tagged `outline` lies from
/// the glyph origin along either axis.
fn outline_reach(outline: &ScanOutline) -> f64 {
    let reach = (outline.points.iter())
        .map(|p| p.x.unsigned_abs().max(p.y.unsigned_abs()))
        .max()
        .unwrap_or(0);
    reach as f64 / 64.0
}

/// Whether a glyph reaching `reach` pixels from its pen at `pen`, at any
/// rotation, can paint on a canvas of `width × height` pixels. Glyphs that
/// cannot are skipped: text at sizes `FreeType` clamps to 65 535 pixels per
/// em would otherwise flatten and rasterize every glyph of a long line.
fn touches_canvas(pen: Point, reach: f64, (width, height): (f64, f64)) -> bool {
    let reach = reach * std::f64::consts::SQRT_2 + 1.0;
    pen.x + reach >= 0.0
        && pen.x - reach <= width
        && pen.y + reach >= 0.0
        && pen.y - reach <= height
}

/// The graphics contexts text is drawn with: rules with a fresh,
/// anti-aliased context clipping nothing, glyphs as Agg's
/// `draw_text_image` blits them, clipped to the clip rectangle only.
pub(super) fn text_contexts(gc: &GraphicsContext) -> (GraphicsContext, GraphicsContext) {
    let fill_gc = GraphicsContext {
        linewidth: 0.0,
        dashes: Dashes::solid(),
        hatch: None,
        sketch: None,
        antialiased: true,
        clip_rect: None,
        clip_path: None,
        ..gc.clone()
    };
    let glyph_gc = GraphicsContext {
        snap: SnapMode::Never,
        clip_rect: gc.clip_rect,
        ..fill_gc.clone()
    };
    (fill_gc, glyph_gc)
}

impl RasterRenderer {
    pub(super) fn draw_text_run(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        let engine = TextEngine::global();
        let placement = self.glyph_placement(origin, angle);
        let (fill_gc, glyph_gc) = text_contexts(gc);
        let hinting = run
            .source
            .as_ref()
            .map_or(Hinting::None, |source| source.font().hinting);
        let axis_aligned = is_axis_aligned(angle);
        // Glyphs are rendered by FreeType, which rounds coverage its own way.
        self.painter
            .set_coverage_rounding(CoverageRounding::FreeType);
        for glyph in &run.glyphs {
            let pen = pen_position(&placement, glyph.x, glyph.y);
            if !within_reach(pen) {
                continue;
            }
            let outline = engine.scan_glyph_outline(
                glyph.font,
                glyph.glyph,
                glyph.size,
                run.dpi,
                hinting,
                axis_aligned,
            );
            if let Ok(outline) = outline {
                let shape = if gc.antialiased {
                    GlyphShape::Smooth(outline)
                } else {
                    GlyphShape::Mono(outline)
                };
                self.blit_glyph(&glyph_gc, &shape, &GlyphTransform::new(angle, pen), pen);
            }
        }
        self.painter.set_coverage_rounding(CoverageRounding::Agg);
        let rules = run.rects.iter().map(pltrs_core::Bbox::bounds);
        self.draw_rules(gc, &fill_gc, rules, &placement);
    }

    /// Where the glyphs and rules of a text anchored at `origin` (display
    /// pixels) and turned by `angle` go: matplotlib flips the anchor
    /// downwards in `Text.draw` (`canvash - y`) and back up in
    /// `_draw_text_glyphs_and_boxes` (`self.height - y`), which lands it on
    /// the bits of the flipped value.
    pub(super) fn glyph_placement(&self, origin: Point, angle: f64) -> Affine2D {
        let height = f64::from(self.height());
        text_placement(Point::new(origin.x, height - (height - origin.y)), angle)
    }

    /// Blits one glyph whose pen is at `pen` (display pixels, on the 26.6
    /// grid), placed by `transform`, in the colour of `glyph_gc`, as
    /// `_render_glyph` and `draw_text_image` do. Glyphs that cannot reach
    /// the canvas are skipped.
    pub(super) fn blit_glyph(
        &mut self,
        glyph_gc: &GraphicsContext,
        shape: &GlyphShape,
        transform: &GlyphTransform,
        pen: Point,
    ) {
        match shape {
            GlyphShape::Smooth(outline) => {
                let reach = outline_reach(outline);
                if reach > MAX_GLYPH_REACH || !touches_canvas(pen, reach, self.size) {
                    return;
                }
                match smooth_bitmap(outline, transform) {
                    Ok(Some(bitmap)) => self.blit_bitmap(glyph_gc, &bitmap),
                    Ok(None) => {}
                    Err(err) => self.record(Err(err)),
                }
            }
            GlyphShape::Mono(outline) => {
                let Some(bitmap) = mono::render(outline, transform) else {
                    return;
                };
                let color = glyph_gc.stroke_color();
                self.draw_path(
                    glyph_gc,
                    &bitmap.to_path(),
                    &Affine2D::IDENTITY,
                    Some(color),
                );
            }
        }
    }

    /// `draw_text_image` at angle 0: blends a glyph's coverage in the
    /// colour of `glyph_gc`, clipped to its clip rectangle only.
    fn blit_bitmap(&mut self, glyph_gc: &GraphicsContext, bitmap: &GrayBitmap) {
        let color = glyph_gc.stroke_color();
        let clip = Clip::rect(self.clip_box(glyph_gc).unwrap_or_else(|| self.full_box()));
        let Some(top) = i32::try_from(self.height())
            .ok()
            .and_then(|height| height.checked_sub(bitmap.top))
        else {
            return;
        };
        let result = pltrs_raster::draw_coverage(
            &mut self.pixmap,
            &bitmap.coverage,
            bitmap.left,
            top,
            color,
            &clip,
        );
        self.record(result);
    }

    /// Draws rules `(x, y, width, height)` placed by `placement` as
    /// matplotlib's `_draw_text_glyphs_and_boxes` does: thin rules grow to
    /// a pixel unless the text is never snapped.
    pub(super) fn draw_rules(
        &mut self,
        gc: &GraphicsContext,
        fill_gc: &GraphicsContext,
        rules: impl Iterator<Item = (f64, f64, f64, f64)>,
        placement: &Affine2D,
    ) {
        let color = gc.stroke_color();
        for (mut x, mut y, mut w, mut h) in rules {
            if gc.snap != SnapMode::Never {
                // Keep thin rules visible by growing them symmetrically.
                if w < 1.0 {
                    x -= (1.0 - w) / 2.0;
                    w = 1.0;
                }
                if h < 1.0 {
                    y -= (1.0 - h) / 2.0;
                    h = 1.0;
                }
            }
            let corners = [(x, y), (x + w, y), (x + w, y + h), (x, y + h)];
            let path = Path::polygon(corners.map(|(cx, cy)| Point::new(cx, cy)));
            self.draw_path(fill_gc, &path, placement, Some(color));
        }
    }
}

/// The pen of a glyph at `(x, y)` in a text placed by `placement` (see
/// [`RasterRenderer::glyph_placement`]), on `FreeType`'s 26.6 grid, summed
/// as matplotlib sums it: `x + dx * cos - dy * sin` and
/// `y + dx * sin + dy * cos`.
pub(super) fn pen_position(placement: &Affine2D, x: f64, y: f64) -> Point {
    let [cos, sin, minus_sin, _, origin_x, origin_y] = placement.values();
    Point::new(
        quantize(origin_x + x * cos + y * minus_sin),
        quantize(origin_y + x * sin + y * cos),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether drawing "W" with `gc` paints any pixel.
    fn paints_glyph(gc: &GraphicsContext) -> bool {
        let run = TextEngine::global()
            .layout_line(
                "W",
                &pltrs_text::FontRequest::new(pltrs_text::FontQuery {
                    size: 20.0,
                    ..pltrs_text::FontQuery::families(["DejaVu Sans"])
                }),
                72.0,
            )
            .expect("bundled font");
        let mut r = RasterRenderer::new(40.0, 40.0, 72.0).expect("valid canvas");
        r.draw_text(gc, &run, Point::new(5.0, 10.0), 0.0);
        let pixmap = r.pixmap();
        (0..40).any(|x| (0..40).any(|y| pixmap.pixel(x, y).is_some_and(|p| p.a > 0)))
    }

    #[test]
    fn glyphs_ignore_clip_paths_but_not_clip_rectangles() {
        let nowhere = pltrs_core::Bbox::from_extents(100.0, 100.0, 101.0, 101.0);
        let clip_path = Some(crate::gc::ClipPath {
            path: Path::unit_rectangle(),
            transform: Affine2D::identity().translate(100.0, 100.0),
        });
        let path_clipped = GraphicsContext {
            clip_path,
            ..GraphicsContext::default()
        };
        assert!(paints_glyph(&path_clipped), "Agg ignores the clip path");
        let rect_clipped = GraphicsContext {
            clip_rect: Some(nowhere),
            ..GraphicsContext::default()
        };
        assert!(!paints_glyph(&rect_clipped));
    }

    #[test]
    fn glyphs_far_off_the_canvas_are_skipped() {
        // Found by fuzzing: pens this far overflowed FreeType's fixed point.
        let run = TextEngine::global()
            .layout_line(
                "W",
                &pltrs_text::FontRequest::new(pltrs_text::FontQuery::families(["DejaVu Sans"])),
                72.0,
            )
            .expect("bundled font");
        for origin in [1e308, 8.4e16, -3e9, f64::NAN] {
            let mut r = RasterRenderer::new(10.0, 10.0, 72.0).expect("valid canvas");
            r.draw_text(
                &GraphicsContext::default(),
                &run,
                Point::new(origin, 5.0),
                30.0,
            );
        }
    }

    #[test]
    fn pen_positions_round_like_python() {
        assert_eq!(quantize(1.0 / 128.0), 0.0);
        assert_eq!(quantize(3.0 / 128.0), 2.0 / 64.0);
        assert_eq!(quantize(10.3), 659.0 / 64.0);
    }

    /// An anchor just below a tie of the 26.6 grid lands on the tie once
    /// flipped down and up on a 600-pixel canvas, as in matplotlib, and
    /// rounds to even.
    #[test]
    fn anchors_are_flipped_down_and_up() {
        let renderer = RasterRenderer::new(800.0, 600.0, 100.0).expect("valid canvas");
        let below_tie = 215.835_937_499_999_97;
        assert!(below_tie < 215.835_937_5);
        let placement = renderer.glyph_placement(Point::new(87.5, below_tie), 0.0);
        let pen = pen_position(&placement, 0.0, 0.0);
        assert_eq!(pen, Point::new(87.5, 13814.0 / 64.0));
    }
}
