//! TeX text on the raster canvas (matplotlib's `RendererAgg.draw_tex`):
//! with the `latex` engine, the glyphs of the DVI file rendered one by one
//! from their Type 1 fonts as `FreeType` renders them, slanted and extended
//! as the font map says, and its rules as paths; with `latex+dvipng`, the
//! image `dvipng` makes, blitted at the text's position and angle.

use plt_core::{Affine2D, Point};
use plt_font::{Dropouts, OutlineSink, ScanOutline, ScanPoint, ScanPointKind, ScanRules};
use plt_raster::{Clip, CoverageRounding, Mask};
use plt_text::Hinting;
use plt_text::tex::{TexEngine, TexError, TexGlyph, TexHinting, TexText};

use super::RasterRenderer;
use super::glyph::GlyphTransform;
use super::text::{GlyphShape, pen_position, text_contexts, within_reach};
use crate::gc::GraphicsContext;
use crate::renderer::text_placement;

/// How `FreeType` loads a Type 1 glyph for `text.hinting`: the Adobe
/// engine hints it unless hinting is off (`FreeType` has no autohinting
/// for the light Type 1 hinter to give way to).
fn type1_hinting(hinting: Hinting) -> TexHinting {
    match hinting {
        Hinting::None => TexHinting::None,
        Hinting::Default | Hinting::Native | Hinting::Auto => TexHinting::Adobe,
    }
}

/// Collects a Type 1 glyph's outline as `FreeType`'s monochrome
/// rasterizer receives it: tagged points in 26.6 pixels.
#[derive(Default)]
struct ScanSink {
    points: Vec<ScanPoint>,
    contour_ends: Vec<usize>,
    start: usize,
}

impl ScanSink {
    fn point(&mut self, x: f32, y: f32, kind: ScanPointKind) {
        self.points.push(ScanPoint {
            x: x.round() as i64,
            y: y.round() as i64,
            kind,
        });
    }
}

impl OutlineSink for ScanSink {
    fn move_to(&mut self, x: f32, y: f32) {
        self.start = self.points.len();
        self.point(x, y, ScanPointKind::On);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.point(x, y, ScanPointKind::On);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.point(x1, y1, ScanPointKind::Conic);
        self.point(x, y, ScanPointKind::On);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.point(x1, y1, ScanPointKind::Cubic);
        self.point(x2, y2, ScanPointKind::Cubic);
        self.point(x, y, ScanPointKind::On);
    }

    /// Ends the contour, without the line back to its start that the
    /// decomposition adds.
    fn close(&mut self) {
        if self.points.len() > self.start + 1
            && self.points.last().map(|p| (p.x, p.y))
                == self.points.get(self.start).map(|p| (p.x, p.y))
        {
            self.points.pop();
        }
        self.contour_ends.push(self.points.len());
    }
}

/// The outline of `glyph` at `dpi` as `FreeType`'s monochrome rasterizer
/// receives it, with the rules the Type 1 loader sets (high precision
/// below 24 pixels per em, simple drop-out control).
fn scan_outline(glyph: &TexGlyph, dpi: f64, hinting: TexHinting) -> Result<ScanOutline, TexError> {
    let size = glyph.file.face.size(glyph.size, dpi);
    let loaded = glyph.load(dpi, hinting)?;
    let mut sink = ScanSink::default();
    loaded.outline(&mut sink);
    Ok(ScanOutline {
        points: sink.points,
        contour_ends: sink.contour_ends,
        rules: ScanRules {
            high_precision: size.y_ppem() < 24,
            dropouts: Dropouts::Simple { stubs: false },
        },
    })
}

impl RasterRenderer {
    /// `draw_tex`: the line typeset at its size, drawn as the engine of
    /// its manager says.
    pub(super) fn draw_tex_text(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        match tex.manager.engine() {
            TexEngine::Latex => self.draw_tex_glyphs(gc, tex, origin, angle),
            TexEngine::LatexDvipng => self.draw_tex_image(gc, tex, origin, angle),
        }
    }

    /// The `latex` engine: `_draw_text_glyphs_and_boxes` with the page's
    /// glyphs, loaded from their Type 1 fonts at their sizes, and rules.
    fn draw_tex_glyphs(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        let page = tex.page(tex.size(), self.dpi)?;
        let glyphs = tex.glyphs(&page)?;
        let hinting = type1_hinting(tex.font.hinting);
        let placement = text_placement(origin, angle);
        let (fill_gc, glyph_gc) = text_contexts(gc);
        self.painter
            .set_coverage_rounding(CoverageRounding::FreeType);
        let drawn = glyphs.iter().try_for_each(|glyph| {
            let pen = pen_position(&placement, glyph.x, glyph.y);
            if !within_reach(pen) {
                return Ok(());
            }
            let shape = if gc.antialiased {
                GlyphShape::Smooth(glyph.outline(self.dpi, hinting)?)
            } else {
                GlyphShape::Mono(scan_outline(glyph, self.dpi, hinting)?)
            };
            let transform = GlyphTransform::with_effects(
                angle,
                glyph.file.effects.slant.unwrap_or(0.0),
                glyph.file.effects.extend.unwrap_or(1.0),
                pen,
            );
            self.blit_glyph(&glyph_gc, &shape, &transform, pen);
            Ok(())
        });
        self.painter.set_coverage_rounding(CoverageRounding::Agg);
        drawn?;
        let rules = page.boxes.iter().map(|b| (b.x, b.y, b.width, b.height));
        self.draw_rules(gc, &fill_gc, rules, &placement);
        Ok(())
    }

    /// The `latex+dvipng` engine: the image `dvipng` makes at the canvas
    /// resolution, its bottom-left corner moved from the baseline by the
    /// text's descent (perpendicular to the baseline) and rounded to a
    /// pixel, then blitted (`draw_text_image`).
    fn draw_tex_image(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        let grey = tex.manager.grey(&tex.tex, tex.size(), self.dpi)?;
        let (_, _, descent) =
            tex.manager
                .text_width_height_descent(&tex.tex, tex.size(), self.dpi / 72.0)?;
        let (sin, cos) = angle.to_radians().sin_cos();
        // Agg's y goes down: the corner sits below the baseline by the
        // descent, perpendicular to it.
        let x = (origin.x + descent * sin).round_ties_even();
        let y = (f64::from(self.height()) - origin.y + descent * cos).round_ties_even();
        let Ok(coverage) = Mask::from_vec(grey.width, grey.height, grey.data.clone()) else {
            return Ok(());
        };
        self.blit_coverage(gc, &coverage, (x as i32, y as i32), angle);
        Ok(())
    }

    /// `draw_text_image`: blends `coverage` in the text colour with its
    /// bottom-left corner at `corner` (device pixels, y down), rotated by
    /// `angle` degrees about it; clipped to the clip rectangle only.
    fn blit_coverage(
        &mut self,
        gc: &GraphicsContext,
        coverage: &Mask,
        corner: (i32, i32),
        angle: f64,
    ) {
        let color = gc.stroke_color();
        let clip = Clip::rect(self.clip_box(gc).unwrap_or_else(|| self.full_box()));
        let height = coverage.height();
        let result = if angle == 0.0 {
            let top = corner
                .1
                .saturating_sub(i32::try_from(height).unwrap_or(i32::MAX));
            plt_raster::draw_coverage(&mut self.pixmap, coverage, corner.0, top, color, &clip)
        } else {
            let transform = Affine2D::translation(0.0, -f64::from(height))
                .rotate_deg(-angle)
                .translate(f64::from(corner.0), f64::from(corner.1));
            self.painter.draw_coverage_transformed(
                &mut self.pixmap,
                coverage,
                &transform,
                color,
                &clip,
            )
        };
        self.record(result);
    }
}
