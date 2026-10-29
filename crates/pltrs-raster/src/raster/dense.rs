//! Dense cell accumulation for heavy geometry.
//!
//! Agg stores every cell and sorts them before sweeping. When the clip
//! frame is known and the geometry produces more cells than a frame-sized
//! grid would occupy, the rasterizer instead sums cells straight into
//! per-pixel cover/area arrays: integer addition is order independent, and
//! a per-pixel "touched" flag records where Agg would have had cells, so
//! the sweep below reproduces Agg's scanlines exactly — without the sort
//! and with memory bounded by the frame.

use super::cells::Cell;
use super::scanline::Scanline;

/// Inclusive cell bounds that every cell is guaranteed to fall within.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Frame {
    pub(crate) x0: i32,
    pub(crate) y0: i32,
    pub(crate) x1: i32,
    pub(crate) y1: i32,
}

impl Frame {
    fn width(&self) -> usize {
        (i64::from(self.x1) - i64::from(self.x0) + 1).max(0) as usize
    }

    fn height(&self) -> usize {
        (i64::from(self.y1) - i64::from(self.y0) + 1).max(0) as usize
    }

    /// Offset of cell `(x, y)` (known to be inside) in row-major order.
    fn index(&self, x: i32, y: i32) -> usize {
        (y - self.y0) as usize * self.width() + (x - self.x0) as usize
    }

    /// Number of cells in the frame (saturating).
    pub(crate) fn area(&self) -> usize {
        self.width().saturating_mul(self.height())
    }
}

/// Untouched-row marker: an empty column range.
const EMPTY_ROW: (i32, i32) = (i32::MAX, i32::MIN);

/// Per-pixel accumulated cover and area over a [`Frame`].
#[derive(Debug, Default)]
pub(crate) struct DenseGrid {
    frame: Option<Frame>,
    cover: Vec<i32>,
    area: Vec<i32>,
    touched: Vec<bool>,
    /// Touched column range of each row.
    rows: Vec<(i32, i32)>,
}

impl DenseGrid {
    /// Readies an all-zero grid covering `frame`.
    pub(crate) fn prepare(&mut self, frame: Frame) {
        if self.frame == Some(frame) {
            self.clear();
            return;
        }
        let n = frame.area();
        self.cover = vec![0; n];
        self.area = vec![0; n];
        self.touched = vec![false; n];
        self.rows = vec![EMPTY_ROW; frame.height()];
        self.frame = Some(frame);
    }

    /// Adds a cell's contributions. Cells outside the frame cannot occur
    /// (the frame contains the clip box with a margin) and are ignored.
    pub(crate) fn add(&mut self, cell: &Cell) {
        let Some(f) = self.frame else {
            return;
        };
        if cell.x < f.x0 || cell.x > f.x1 || cell.y < f.y0 || cell.y > f.y1 {
            return;
        }
        let i = f.index(cell.x, cell.y);
        self.cover[i] = self.cover[i].wrapping_add(cell.cover);
        self.area[i] = self.area[i].wrapping_add(cell.area);
        self.touched[i] = true;
        let row = &mut self.rows[(cell.y - f.y0) as usize];
        row.0 = row.0.min(cell.x);
        row.1 = row.1.max(cell.x);
    }

    /// Zeroes everything touched since the last clear.
    pub(crate) fn clear(&mut self) {
        let Some(f) = self.frame else {
            return;
        };
        for (r, range) in self.rows.iter_mut().enumerate() {
            if range.0 <= range.1 {
                let base = r * f.width();
                let span = base + (range.0 - f.x0) as usize..=base + (range.1 - f.x0) as usize;
                self.cover[span.clone()].fill(0);
                self.area[span.clone()].fill(0);
                self.touched[span].fill(false);
                *range = EMPTY_ROW;
            }
        }
    }

    /// Sweeps row `y` into `sl` exactly like Agg's `sweep_scanline` over
    /// the cells of that row; `alpha` maps accumulated area to coverage.
    pub(crate) fn sweep_row(&self, y: i32, sl: &mut Scanline, alpha: impl Fn(i32) -> u8) {
        let Some(f) = self.frame else {
            return;
        };
        if y < f.y0 || y > f.y1 {
            return;
        }
        let (min_x, max_x) = self.rows[(y - f.y0) as usize];
        if min_x > max_x {
            return;
        }
        let base = f.index(f.x0, y);
        let at = |x: i32| base + (x - f.x0) as usize;
        let mut cover = 0i32;
        let mut x = min_x;
        loop {
            let i = at(x);
            cover = cover.wrapping_add(self.cover[i]);
            let area = self.area[i];
            let mut next = x + 1;
            while next <= max_x && !self.touched[at(next)] {
                next += 1;
            }
            let mut px = x;
            if area != 0 {
                let a = alpha((cover << 9).wrapping_sub(area));
                if a != 0 {
                    sl.add_cell(px, a);
                }
                px += 1;
            }
            if next > max_x {
                return;
            }
            if next > px {
                let a = alpha(cover << 9);
                if a != 0 {
                    sl.add_span(px, next - px, a);
                }
            }
            x = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(x: i32, y: i32, cover: i32, area: i32) -> Cell {
        Cell { x, y, cover, area }
    }

    #[test]
    fn accumulates_sweeps_and_clears() {
        let mut g = DenseGrid::default();
        g.prepare(Frame {
            x0: -1,
            y0: -1,
            x1: 5,
            y1: 2,
        });
        // A unit-height band from x = 1 to x = 3 in row 0.
        g.add(&cell(1, 0, 256, 0));
        g.add(&cell(3, 0, -256, 0));
        g.add(&cell(9, 9, 1, 1)); // outside: ignored
        let mut sl = Scanline::new();
        sl.reset(0);
        g.sweep_row(0, &mut sl, |a| (a >> 9).unsigned_abs().min(255) as u8);
        let spans: Vec<_> = sl.spans().map(|s| (s.x, s.len(), s.cover_at(0))).collect();
        assert_eq!(spans, vec![(1, 2, 255)]);
        g.clear();
        sl.reset(0);
        g.sweep_row(0, &mut sl, |_| 255);
        assert!(sl.is_empty());
        assert_eq!(
            Frame {
                x0: 0,
                y0: 0,
                x1: -5,
                y1: 3
            }
            .area(),
            0
        );
    }
}
