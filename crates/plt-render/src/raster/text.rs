//! Text on the raster canvas, following matplotlib's
//! `RendererAgg._draw_text_glyphs_and_boxes`.
//!
//! matplotlib renders each glyph with `FreeType` at its pen position rounded
//! to 1/64 pixel and blits the coverage bitmap unsnapped; here the glyph
//! outline is filled at that same quantized position (never snapped).
//! Rules (mathtext fraction bars and the like) are drawn as paths with the
//! text's snapping, grown to at least one pixel as matplotlib does.

use plt_core::{Affine2D, Path, Point};
use plt_text::{TextEngine, TextRun};

use super::RasterRenderer;
use crate::gc::{Dashes, GraphicsContext, SnapMode};
use crate::renderer::Renderer;

/// `FreeType`'s 26.6 pen position: Python's `round(0x40 * v) / 0x40`.
fn quantize(v: f64) -> f64 {
    (v * 64.0).round_ties_even() / 64.0
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
        let rotation = Affine2D::identity().rotate_deg(angle);
        let placement = rotation.translate(origin.x, origin.y);
        let color = gc.stroke_color();
        let fill_gc = GraphicsContext {
            linewidth: 0.0,
            dashes: Dashes::solid(),
            hatch: None,
            sketch: None,
            ..gc.clone()
        };
        let glyph_gc = GraphicsContext {
            snap: SnapMode::Never,
            ..fill_gc.clone()
        };
        for glyph in &run.glyphs {
            let Ok(outline) = engine.glyph_outline(glyph.font, glyph.glyph, glyph.size, run.dpi)
            else {
                continue;
            };
            let pen = placement.transform_point(Point::new(glyph.x, glyph.y));
            let at = rotation.translate(quantize(pen.x), quantize(pen.y));
            self.draw_path(&glyph_gc, &outline, &at, Some(color));
        }
        for rect in &run.rects {
            let (mut x, mut y, mut w, mut h) = rect.bounds();
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
            self.draw_path(&fill_gc, &path, &placement, Some(color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pen_positions_round_like_python() {
        assert_eq!(quantize(1.0 / 128.0), 0.0);
        assert_eq!(quantize(3.0 / 128.0), 2.0 / 64.0);
        assert_eq!(quantize(10.3), 659.0 / 64.0);
    }
}
