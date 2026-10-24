//! The high-level drawing API, mirroring what matplotlib's `RendererAgg`
//! does for each primitive.
//!
//! [`Painter`] is defined here with the plumbing its operations share;
//! the operations themselves live in [`draw`] (into a target pixmap) and
//! [`rasterize`] (into fresh tiles, stamps and masks).

mod draw;
mod rasterize;
#[cfg(test)]
mod tests;

use plt_core::{Path, Point};

use crate::clip::ClipBox;
use crate::color::Rgba8;
use crate::error::RasterError;
use crate::geometry::dash::{DashSink, Dasher};
use crate::geometry::stroke::{StrokeSink, Stroker};
use crate::geometry::{PathSink, feed_path};
use crate::mask::Mask;
use crate::pixmap::Pixmap;
use crate::raster::{CoverageRounding, Gamma, Rasterizer, Scanline};
use crate::render::Surface;
use crate::style::{Antialias, FillRule, StrokeStyle};

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

    /// Sets how cell areas are rounded to coverage for subsequent
    /// operations ([`CoverageRounding::FreeType`] reproduces glyphs).
    pub fn set_coverage_rounding(&mut self, rounding: CoverageRounding) {
        self.rasterizer.set_coverage_rounding(rounding);
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
                feed_path(path, &mut dash);
                dash.finish();
            }
            None => feed_path(path, &mut stroke),
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
