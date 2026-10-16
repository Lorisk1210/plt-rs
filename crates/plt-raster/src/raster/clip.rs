//! Edge clipping against a rectangle in floating point — a port of Agg's
//! `rasterizer_sl_clip<ras_conv_dbl>` (`rasterizer_sl_clip_dbl`), the
//! clipper matplotlib's rasterizer uses.
//!
//! Parts of an edge left or right of the box are not discarded but
//! projected onto the box's vertical sides, which preserves the winding
//! contribution for everything inside. Parts above or below are dropped.

use super::cells::{CellBuffer, SUBPIXEL_SCALE, SUBPIXEL_SHIFT};
use super::dense::Frame;

/// Largest magnitude of a fixed-point coordinate (`poly_max_coord`);
/// coordinates are saturated to it so integer arithmetic cannot overflow.
const POLY_MAX_COORD: f64 = ((1 << 30) - 1) as f64;

/// Converts a pixel coordinate to 24.8 fixed point with Agg's `iround`,
/// saturating at `±POLY_MAX_COORD`.
fn to_fixed(v: f64) -> i32 {
    let s = v * f64::from(SUBPIXEL_SCALE);
    let r = if s < 0.0 { s - 0.5 } else { s + 0.5 };
    r.clamp(-POLY_MAX_COORD, POLY_MAX_COORD) as i32
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

// Bit flags locating a point relative to the clip box (Agg's
// `clipping_flags`).
const RIGHT: u32 = 0b0001;
const BELOW: u32 = 0b0010;
const LEFT: u32 = 0b0100;
const ABOVE: u32 = 0b1000;
const X_FLAGS: u32 = RIGHT | LEFT;
const Y_FLAGS: u32 = BELOW | ABOVE;

fn flags(x: f64, y: f64, r: &Rect) -> u32 {
    let flag = |cond: bool, bit: u32| if cond { bit } else { 0 };
    flag(x > r.x2, RIGHT) | flag(x < r.x1, LEFT) | flags_y(y, r)
}

fn flags_y(y: f64, r: &Rect) -> u32 {
    (if y > r.y2 { BELOW } else { 0 }) | (if y < r.y1 { ABOVE } else { 0 })
}

/// The clipping stage between floating-point vertices and the cell buffer.
#[derive(Clone, Debug, Default)]
pub(crate) struct Clipper {
    clip: Option<Rect>,
    x1: f64,
    y1: f64,
    f1: u32,
}

impl Clipper {
    pub(crate) fn set_clip_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.clip = Some(Rect {
            x1: x1.min(x2),
            y1: y1.min(y2),
            x2: x1.max(x2),
            y2: y1.max(y2),
        });
    }

    pub(crate) fn reset_clipping(&mut self) {
        self.clip = None;
    }

    /// Cell bounds containing every cell the clipped geometry can touch
    /// (with a one-cell margin), or `None` when not clipping.
    pub(crate) fn cell_frame(&self) -> Option<Frame> {
        let r = self.clip?;
        let cell = |v: f64| to_fixed(v) >> SUBPIXEL_SHIFT;
        Some(Frame {
            x0: cell(r.x1) - 1,
            y0: cell(r.y1) - 1,
            x1: cell(r.x2) + 1,
            y1: cell(r.y2) + 1,
        })
    }

    pub(crate) fn move_to(&mut self, x: f64, y: f64) {
        self.x1 = x;
        self.y1 = y;
        if let Some(r) = &self.clip {
            self.f1 = flags(x, y, r);
        }
    }

    pub(crate) fn line_to(&mut self, cells: &mut CellBuffer, x2: f64, y2: f64) {
        let Some(r) = self.clip else {
            cells.line(
                to_fixed(self.x1),
                to_fixed(self.y1),
                to_fixed(x2),
                to_fixed(y2),
            );
            self.x1 = x2;
            self.y1 = y2;
            return;
        };
        let f2 = flags(x2, y2, &r);
        let (x1, y1, f1) = (self.x1, self.y1, self.f1);
        self.x1 = x2;
        self.y1 = y2;
        self.f1 = f2;
        if (f1 & Y_FLAGS) == (f2 & Y_FLAGS) && (f1 & Y_FLAGS) != 0 {
            // Invisible by y.
            return;
        }
        let at_x = |cx: f64| y1 + (cx - x1) * (y2 - y1) / (x2 - x1);
        // Agg's case analysis on ((f1 & X_FLAGS) << 1) | (f2 & X_FLAGS).
        match ((f1 & X_FLAGS) << 1) | (f2 & X_FLAGS) {
            0 => clip_y(cells, &r, x1, y1, x2, y2, f1, f2),
            1 => {
                let y3 = at_x(r.x2);
                let f3 = flags_y(y3, &r);
                clip_y(cells, &r, x1, y1, r.x2, y3, f1, f3);
                clip_y(cells, &r, r.x2, y3, r.x2, y2, f3, f2);
            }
            2 => {
                let y3 = at_x(r.x2);
                let f3 = flags_y(y3, &r);
                clip_y(cells, &r, r.x2, y1, r.x2, y3, f1, f3);
                clip_y(cells, &r, r.x2, y3, x2, y2, f3, f2);
            }
            3 => clip_y(cells, &r, r.x2, y1, r.x2, y2, f1, f2),
            4 => {
                let y3 = at_x(r.x1);
                let f3 = flags_y(y3, &r);
                clip_y(cells, &r, x1, y1, r.x1, y3, f1, f3);
                clip_y(cells, &r, r.x1, y3, r.x1, y2, f3, f2);
            }
            6 => {
                let y3 = at_x(r.x2);
                let y4 = at_x(r.x1);
                let f3 = flags_y(y3, &r);
                let f4 = flags_y(y4, &r);
                clip_y(cells, &r, r.x2, y1, r.x2, y3, f1, f3);
                clip_y(cells, &r, r.x2, y3, r.x1, y4, f3, f4);
                clip_y(cells, &r, r.x1, y4, r.x1, y2, f4, f2);
            }
            8 => {
                let y3 = at_x(r.x1);
                let f3 = flags_y(y3, &r);
                clip_y(cells, &r, r.x1, y1, r.x1, y3, f1, f3);
                clip_y(cells, &r, r.x1, y3, x2, y2, f3, f2);
            }
            9 => {
                let y3 = at_x(r.x1);
                let y4 = at_x(r.x2);
                let f3 = flags_y(y3, &r);
                let f4 = flags_y(y4, &r);
                clip_y(cells, &r, r.x1, y1, r.x1, y3, f1, f3);
                clip_y(cells, &r, r.x1, y3, r.x2, y4, f3, f4);
                clip_y(cells, &r, r.x2, y4, r.x2, y2, f4, f2);
            }
            12 => clip_y(cells, &r, r.x1, y1, r.x1, y2, f1, f2),
            // Flags 1 and 4 are mutually exclusive, so no other value occurs.
            _ => {}
        }
    }
}

/// Clips an edge that is already within the box horizontally against the
/// box's top and bottom (Agg's `line_clip_y`).
#[allow(clippy::too_many_arguments)] // Mirrors Agg's signature one to one.
fn clip_y(cells: &mut CellBuffer, r: &Rect, x1: f64, y1: f64, x2: f64, y2: f64, f1: u32, f2: u32) {
    let f1 = f1 & Y_FLAGS;
    let f2 = f2 & Y_FLAGS;
    if f1 | f2 == 0 {
        cells.line(to_fixed(x1), to_fixed(y1), to_fixed(x2), to_fixed(y2));
        return;
    }
    if f1 == f2 {
        return;
    }
    let at_y = |cy: f64| x1 + (cy - y1) * (x2 - x1) / (y2 - y1);
    let (mut tx1, mut ty1, mut tx2, mut ty2) = (x1, y1, x2, y2);
    if f1 & ABOVE != 0 {
        tx1 = at_y(r.y1);
        ty1 = r.y1;
    }
    if f1 & BELOW != 0 {
        tx1 = at_y(r.y2);
        ty1 = r.y2;
    }
    if f2 & ABOVE != 0 {
        tx2 = at_y(r.y1);
        ty2 = r.y1;
    }
    if f2 & BELOW != 0 {
        tx2 = at_y(r.y2);
        ty2 = r.y2;
    }
    cells.line(to_fixed(tx1), to_fixed(ty1), to_fixed(tx2), to_fixed(ty2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_rounds_half_away_from_zero_and_saturates() {
        assert_eq!(to_fixed(1.5), 384);
        assert_eq!(to_fixed(-0.5 / 256.0), -1);
        assert_eq!(to_fixed(0.5 / 256.0), 1);
        assert_eq!(to_fixed(1e300), (1 << 30) - 1);
        assert_eq!(to_fixed(-1e300), -((1 << 30) - 1));
    }

    #[test]
    fn edges_left_of_box_are_projected_onto_it() {
        let mut cells = CellBuffer::new(usize::MAX);
        let mut c = Clipper::default();
        c.set_clip_box(2.0, 0.0, 10.0, 10.0);
        c.move_to(0.0, 1.0);
        c.line_to(&mut cells, 0.0, 3.0);
        cells.sort();
        let (min_x, min_y, max_x, max_y) = cells.bounds();
        assert_eq!((min_x, max_x), (2, 2));
        assert_eq!((min_y, max_y), (1, 3));
        let cover: i32 = (min_y..=max_y)
            .flat_map(|y| cells.row(y).to_vec())
            .map(|c| c.cover)
            .sum();
        assert_eq!(cover, 512);
    }

    #[test]
    fn edges_above_box_are_dropped() {
        let mut cells = CellBuffer::new(usize::MAX);
        let mut c = Clipper::default();
        c.set_clip_box(0.0, 5.0, 10.0, 10.0);
        c.move_to(1.0, 0.0);
        c.line_to(&mut cells, 8.0, 4.0);
        cells.sort();
        assert_eq!(cells.total_cells(), 0);
    }
}
