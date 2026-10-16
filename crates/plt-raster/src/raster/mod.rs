//! The anti-aliased scanline rasterizer — Agg's `rasterizer_scanline_aa`
//! with the floating-point clipper (`rasterizer_sl_clip_dbl`) that
//! matplotlib's `RendererAgg` uses.
//!
//! Polygons are fed as straight segments in pixel coordinates (y down);
//! every sub-path is implicitly closed. Coverage is computed exactly in
//! 24.8 fixed point and swept row by row into [`Scanline`]s.

mod cells;
mod clip;
mod dense;
mod gamma;
mod scanline;

use plt_core::{Path, Point};

use self::cells::CellBuffer;
use self::clip::Clipper;
pub use self::gamma::Gamma;
pub use self::scanline::{Covers, Scanline, Span};
use crate::error::RasterError;
use crate::geometry::PathSink;
use crate::geometry::curve::CurveFlattener;
use crate::style::FillRule;

/// Default maximum number of coverage cells: matplotlib builds its
/// rasterizer with 32768 blocks of 4096 cells.
pub const DEFAULT_CELL_LIMIT: usize = 32768 * 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Initial,
    MoveTo,
    LineTo,
    Closed,
}

/// Accumulates polygons and sweeps them into anti-aliased scanlines.
///
/// The rasterizer is reusable: [`reset`](Self::reset) (or simply starting a
/// new sub-path after a sweep) discards the geometry but keeps the clip
/// box, fill rule and gamma, and all internal buffers.
#[derive(Debug)]
pub struct Rasterizer {
    cells: CellBuffer,
    clipper: Clipper,
    gamma: [u8; 256],
    fill_rule: FillRule,
    start: Point,
    status: Status,
    flattener: CurveFlattener,
}

impl Default for Rasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer {
    /// A rasterizer with matplotlib's settings: non-zero rule, linear
    /// gamma, no clipping, [`DEFAULT_CELL_LIMIT`].
    pub fn new() -> Self {
        Self::with_cell_limit(DEFAULT_CELL_LIMIT)
    }

    /// A rasterizer that fails with [`RasterError::CellLimitExceeded`]
    /// once the geometry needs more than `limit` cells.
    pub fn with_cell_limit(limit: usize) -> Self {
        Self {
            cells: CellBuffer::new(limit),
            clipper: Clipper::default(),
            gamma: Gamma::Linear.table(),
            fill_rule: FillRule::NonZero,
            start: Point::ORIGIN,
            status: Status::Initial,
            flattener: CurveFlattener::default(),
        }
    }

    /// Discards all geometry.
    pub fn reset(&mut self) {
        self.cells.reset();
        self.status = Status::Initial;
    }

    /// Clips all subsequent geometry to the rectangle spanned by the two
    /// corners (pixel coordinates; the order of the corners is irrelevant).
    /// Like Agg, this also discards the current geometry.
    pub fn set_clip_box(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.reset();
        self.clipper.set_clip_box(x0, y0, x1, y1);
        self.cells.set_frame(self.clipper.cell_frame());
    }

    /// Removes the clip box (and discards the current geometry).
    pub fn reset_clipping(&mut self) {
        self.reset();
        self.clipper.reset_clipping();
        self.cells.set_frame(None);
    }

    /// Sets the rule deciding which regions are inside.
    pub fn set_fill_rule(&mut self, rule: FillRule) {
        self.fill_rule = rule;
    }

    /// The current fill rule.
    pub const fn fill_rule(&self) -> FillRule {
        self.fill_rule
    }

    /// Sets the coverage-to-alpha transfer function.
    pub fn set_gamma(&mut self, gamma: Gamma) {
        self.gamma = gamma.table();
    }

    /// Adds every sub-path of `path`, flattening curves like Agg's
    /// `conv_curve` (approximation scale 1).
    pub fn add_path(&mut self, path: &Path) {
        let mut flattener = std::mem::take(&mut self.flattener);
        flattener.feed(path, self);
        self.flattener = flattener;
    }

    /// Bounds `(min_x, min_y, max_x, max_y)` of the cells touched by the
    /// geometry added so far, or `None` when nothing was added.
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let bounds = self.cells.bounds();
        (bounds.0 <= bounds.2).then_some(bounds)
    }

    fn close_polygon(&mut self) {
        if self.status == Status::LineTo {
            self.clipper
                .line_to(&mut self.cells, self.start.x, self.start.y);
            self.status = Status::Closed;
        }
    }

    fn reset_if_swept(&mut self) {
        if self.cells.is_sorted() {
            self.reset();
        }
    }

    /// Converts accumulated area to an 8-bit alpha under the fill rule and
    /// gamma (Agg's `calculate_alpha`).
    fn calculate_alpha(&self, area: i32) -> u8 {
        let mut cover = (area >> 9).unsigned_abs();
        if self.fill_rule == FillRule::EvenOdd {
            cover &= 511;
            if cover > 256 {
                cover = 512 - cover;
            }
        }
        self.gamma[cover.min(255) as usize]
    }

    /// Closes the open polygon, sorts the cells and calls `render` for
    /// every non-empty scanline from top to bottom. The geometry is kept
    /// (a second sweep yields the same scanlines) until the next sub-path
    /// starts.
    pub fn sweep<F: FnMut(&Scanline)>(
        &mut self,
        sl: &mut Scanline,
        mut render: F,
    ) -> Result<(), RasterError> {
        self.close_polygon();
        self.cells.sort();
        if self.cells.overflowed() {
            return Err(RasterError::CellLimitExceeded {
                limit: self.cells.limit(),
            });
        }
        if self.cells.is_empty() {
            return Ok(());
        }
        let (_, min_y, _, max_y) = self.cells.bounds();
        for y in min_y..=max_y {
            sl.reset(y);
            self.cells
                .sweep_row(y, sl, |area| self.calculate_alpha(area));
            if !sl.is_empty() {
                render(sl);
            }
        }
        Ok(())
    }
}

impl PathSink for Rasterizer {
    /// Starts a polygon, closing the previous one. Non-finite points are
    /// ignored.
    fn move_to(&mut self, p: Point) {
        if !p.is_finite() {
            return;
        }
        self.reset_if_swept();
        self.close_polygon();
        self.start = p;
        self.clipper.move_to(p.x, p.y);
        self.status = Status::MoveTo;
    }

    /// Adds an edge. Non-finite points are ignored.
    fn line_to(&mut self, p: Point) {
        if !p.is_finite() {
            return;
        }
        self.reset_if_swept();
        self.clipper.line_to(&mut self.cells, p.x, p.y);
        self.status = Status::LineTo;
    }

    fn close(&mut self) {
        self.close_polygon();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coverage(ras: &mut Rasterizer, width: usize, height: usize) -> Vec<u8> {
        let mut out = vec![0u8; width * height];
        let mut sl = Scanline::new();
        ras.sweep(&mut sl, |sl| {
            for span in sl.spans() {
                for i in 0..span.len() {
                    let x = span.x as usize + i;
                    out[sl.y() as usize * width + x] = span.cover_at(i);
                }
            }
        })
        .unwrap();
        out
    }

    fn square(ras: &mut Rasterizer, x0: f64, y0: f64, x1: f64, y1: f64) {
        ras.move_to(Point::new(x0, y0));
        ras.line_to(Point::new(x1, y0));
        ras.line_to(Point::new(x1, y1));
        ras.line_to(Point::new(x0, y1));
    }

    #[test]
    fn pixel_aligned_square_is_fully_covered() {
        let mut ras = Rasterizer::new();
        square(&mut ras, 1.0, 1.0, 3.0, 3.0);
        let cov = coverage(&mut ras, 4, 4);
        assert_eq!(
            cov,
            vec![0, 0, 0, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 0, 0, 0]
        );
        assert_eq!(ras.bounds(), Some((1, 1, 3, 3)));
    }

    #[test]
    fn half_pixel_offset_gives_half_coverage() {
        let mut ras = Rasterizer::new();
        square(&mut ras, 0.5, 0.0, 1.5, 1.0);
        assert_eq!(coverage(&mut ras, 2, 1), vec![128, 128]);
    }

    #[test]
    fn fill_rules_differ_on_overlaps() {
        let mut ras = Rasterizer::new();
        square(&mut ras, 0.0, 0.0, 2.0, 1.0);
        square(&mut ras, 1.0, 0.0, 3.0, 1.0);
        assert_eq!(coverage(&mut ras, 3, 1), vec![255, 255, 255]);
        ras.reset();
        ras.set_fill_rule(FillRule::EvenOdd);
        square(&mut ras, 0.0, 0.0, 2.0, 1.0);
        square(&mut ras, 1.0, 0.0, 3.0, 1.0);
        assert_eq!(coverage(&mut ras, 3, 1), vec![255, 0, 255]);
    }

    #[test]
    fn clip_box_limits_coverage() {
        let mut ras = Rasterizer::new();
        ras.set_clip_box(1.0, 0.0, 2.0, 1.0);
        square(&mut ras, -5.0, -5.0, 10.0, 10.0);
        assert_eq!(coverage(&mut ras, 3, 1), vec![0, 255, 0]);
    }

    #[test]
    fn threshold_gamma_binarises() {
        let mut ras = Rasterizer::new();
        ras.set_gamma(Gamma::Threshold(0.5));
        square(&mut ras, 0.25, 0.0, 1.75, 1.0);
        assert_eq!(coverage(&mut ras, 2, 1), vec![255, 255]);
        ras.reset();
        square(&mut ras, 0.75, 0.0, 1.25, 1.0);
        assert_eq!(coverage(&mut ras, 2, 1), vec![0, 0]);
    }

    #[test]
    fn non_finite_points_and_empty_paths_are_harmless() {
        let mut ras = Rasterizer::new();
        ras.move_to(Point::new(f64::NAN, 0.0));
        ras.line_to(Point::new(1.0, f64::INFINITY));
        assert_eq!(coverage(&mut ras, 1, 1), vec![0]);
        assert_eq!(ras.bounds(), None);
        ras.add_path(&Path::default());
        assert_eq!(coverage(&mut ras, 1, 1), vec![0]);
    }

    #[test]
    fn cell_limit_is_reported() {
        let mut ras = Rasterizer::with_cell_limit(4);
        square(&mut ras, 0.5, 0.5, 30.5, 30.5);
        let mut sl = Scanline::new();
        assert_eq!(
            ras.sweep(&mut sl, |_| {}),
            Err(RasterError::CellLimitExceeded { limit: 4 })
        );
    }

    /// Every scanline as `(y, [(x, covers, solid)])`.
    type Rows = Vec<(i32, Vec<(i32, Vec<u8>, bool)>)>;

    /// Spans of every scanline, for exact comparisons.
    fn scanlines(ras: &mut Rasterizer) -> Rows {
        let mut out = Vec::new();
        let mut sl = Scanline::new();
        ras.sweep(&mut sl, |sl| {
            let spans = sl
                .spans()
                .map(|s| {
                    let solid = matches!(s.covers, Covers::Solid { .. });
                    (s.x, (0..s.len()).map(|i| s.cover_at(i)).collect(), solid)
                })
                .collect();
            out.push((sl.y(), spans));
        })
        .unwrap();
        out
    }

    #[test]
    fn dense_accumulation_matches_sorted_cells_exactly() {
        // A many-spiked star whose edges produce far more cells than the
        // 64×64 frame holds, so the clipped rasterizer switches to dense.
        let star = |ras: &mut Rasterizer| {
            for i in 0..=6000 {
                let t = f64::from(i) * 0.731;
                let r = if i % 2 == 0 {
                    31.0
                } else {
                    3.0 + f64::from(i % 7)
                };
                let p = Point::new(32.0 + r * t.cos(), 32.0 + r * t.sin());
                if i == 0 {
                    ras.move_to(p);
                } else {
                    ras.line_to(p);
                }
            }
        };
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            let mut sparse = Rasterizer::new();
            sparse.set_fill_rule(rule);
            star(&mut sparse);
            let mut dense = Rasterizer::new();
            dense.set_clip_box(0.0, 0.0, 64.0, 64.0);
            dense.set_fill_rule(rule);
            star(&mut dense);
            let expected = scanlines(&mut sparse);
            assert!(!sparse.cells.is_dense());
            assert_eq!(scanlines(&mut dense), expected);
            assert!(dense.cells.is_dense());
            // Reusing the rasterizer clears the grid.
            square(&mut dense, 1.0, 1.0, 2.0, 2.0);
            assert_eq!(scanlines(&mut dense).len(), 1);
        }
    }

    #[test]
    fn new_geometry_after_a_sweep_starts_fresh() {
        let mut ras = Rasterizer::new();
        square(&mut ras, 0.0, 0.0, 1.0, 1.0);
        let _ = coverage(&mut ras, 2, 1);
        square(&mut ras, 1.0, 0.0, 2.0, 1.0);
        assert_eq!(coverage(&mut ras, 2, 1), vec![0, 255]);
    }
}
