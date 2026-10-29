//! Ticks drawn along a path (matplotlib's `TickedStroke`).

use plt_core::path::PolygonOptions;
use plt_core::{Affine2D, Path, PathCode, Point, Rgba};
use plt_render::{GraphicsContext, Renderer};

use super::{GcUpdate, offset_affine};
use crate::numeric::interp;

/// Short ticks at regular intervals along a path, for example to mark the
/// forbidden side of a constraint.
#[derive(Clone, Debug, PartialEq)]
pub struct TickedStroke {
    /// Offset of the ticks in points.
    pub offset: (f64, f64),
    /// Distance between ticks in points.
    pub spacing: f64,
    /// Angle between the path and the ticks in degrees, counter-clockwise
    /// from the direction of travel.
    pub angle: f64,
    /// Tick length relative to `spacing`.
    pub length: f64,
    /// Further graphics state changes.
    pub gc: GcUpdate,
}

impl Default for TickedStroke {
    fn default() -> Self {
        Self {
            offset: (0.0, 0.0),
            spacing: 10.0,
            angle: 45.0,
            length: std::f64::consts::SQRT_2,
            gc: GcUpdate::default(),
        }
    }
}

/// The most ticks drawn along one polyline. A path reaching far beyond the
/// canvas would ask for more ticks than memory holds (matplotlib raises
/// `MemoryError`); such a polyline gets none.
const MAX_TICKS: f64 = 1_048_576.0;

impl TickedStroke {
    /// The tick segments along one polyline (display pixels), or `None`
    /// when it is a single point or would take more than 2²⁰ ticks.
    pub fn ticks(&self, polyline: &[Point], spacing_px: f64) -> Option<Path> {
        if polyline.len() < 2 {
            return None;
        }
        let x: Vec<f64> = polyline.iter().map(|p| p.x).collect();
        let y: Vec<f64> = polyline.iter().map(|p| p.y).collect();
        let mut s = vec![0.0];
        for w in polyline.windows(2) {
            let last = s[s.len() - 1];
            s.push(last + (w[1].x - w[0].x).hypot(w[1].y - w[0].y));
        }
        let total = s[s.len() - 1];
        let num = ((total / spacing_px).ceil() - 1.0).max(0.0);
        if num > MAX_TICKS {
            return None;
        }
        let num = num as usize;
        let theta = -self.angle.to_radians();
        let (sin, cos) = theta.sin_cos();
        let delta = self.spacing * 0.001;
        let scale = self.length * spacing_px;
        let mut vertices = Vec::with_capacity(2 * num);
        for s_tick in crate::numeric::linspace(spacing_px / 2.0, total - spacing_px / 2.0, num) {
            let tick = Point::new(interp(s_tick, &s, &x, None), interp(s_tick, &s, &y, None));
            let u = (interp(s_tick + delta, &s, &x, None) - tick.x) / delta;
            let v = (interp(s_tick + delta, &s, &y, None) - tick.y) / delta;
            let n = u.hypot(v);
            let (ux, uy) = if n == 0.0 { (0.0, 0.0) } else { (u / n, v / n) };
            // The row vector (ux, uy) times [[cos, -sin], [sin, cos]].
            let dx = (ux * cos + uy * sin) * scale;
            let dy = (-ux * sin + uy * cos) * scale;
            vertices.extend([tick, Point::new(tick.x + dx, tick.y + dy)]);
        }
        let codes = [PathCode::MoveTo, PathCode::LineTo].repeat(num);
        Some(Path::new(vertices, codes).expect("tick segments start with moves"))
    }

    /// Draws the ticks of `path` (mapped by `affine`) on `renderer`, with
    /// `foreground` the resolved stroke colour of the graphics state
    /// changes.
    pub(super) fn draw_path(
        &self,
        renderer: &mut dyn Renderer,
        gc: &GraphicsContext,
        path: &Path,
        affine: &Affine2D,
        fill: Option<Rgba>,
        foreground: Option<Rgba>,
    ) {
        let mut gc = gc.clone();
        self.gc.apply(&mut gc, foreground);
        let placed = offset_affine(renderer, affine, self.offset);
        let Some(back) = affine.inverted() else {
            return;
        };
        let to_display = back.then(placed);
        let spacing_px = renderer.points_to_pixels(self.spacing);
        let options = PolygonOptions {
            closed_only: false,
            ..PolygonOptions::default()
        };
        let display = path.transformed(affine);
        for polyline in display.to_polygons(&options) {
            if let Some(ticks) = self.ticks(&polyline, spacing_px) {
                renderer.draw_path(&gc, &ticks, &to_display, fill);
            }
        }
    }
}
