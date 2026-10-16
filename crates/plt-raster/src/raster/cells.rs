//! Cell accumulation — a port of Agg's `rasterizer_cells_aa`.
//!
//! Edges arrive in 24.8 fixed point. Each pixel cell an edge crosses
//! accumulates the signed vertical extent of the edge inside it (`cover`)
//! and twice the signed area to the left of the edge within the cell
//! (`area`, in subpixel² units). Sweeping a row then turns running cover
//! sums into coverage.
//!
//! The integer arithmetic is reproduced operation for operation so that
//! coverage matches Agg bit for bit. Accumulations wrap on overflow, as the
//! C++ code does in practice. Heavy geometry inside a known frame switches
//! to dense accumulation (see [`super::dense`]).

use super::dense::{DenseGrid, Frame};
use super::scanline::Scanline;

/// Bits of subpixel precision (`poly_subpixel_shift`).
pub(crate) const SUBPIXEL_SHIFT: i32 = 8;
/// Subpixels per pixel.
pub(crate) const SUBPIXEL_SCALE: i32 = 1 << SUBPIXEL_SHIFT;
const SUBPIXEL_MASK: i32 = SUBPIXEL_SCALE - 1;

/// Fewest stored cells before dense accumulation is considered.
const MIN_DENSE_CELLS: usize = 1 << 16;

/// Longest horizontal extent rendered in one piece; longer edges are split
/// so that intermediate products cannot overflow.
const DX_LIMIT: i32 = 16384 << SUBPIXEL_SHIFT;

/// One pixel's accumulated edge contributions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) cover: i32,
    pub(crate) area: i32,
}

impl Cell {
    const INITIAL: Self = Self {
        x: i32::MAX,
        y: i32::MAX,
        cover: 0,
        area: 0,
    };
}

/// Collects cells for a set of edges and sorts them into rows.
#[derive(Debug)]
pub(crate) struct CellBuffer {
    cells: Vec<Cell>,
    sorted: Vec<Cell>,
    /// `row_start[i]..row_start[i + 1]` indexes `sorted` for row `min_y + i`.
    row_start: Vec<usize>,
    curr: Cell,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    is_sorted: bool,
    limit: usize,
    overflowed: bool,
    /// Bounds guaranteed to contain every cell (known when clipping).
    frame: Option<Frame>,
    /// Stored cell count at which accumulation switches to `dense`.
    dense_threshold: usize,
    dense: DenseGrid,
    dense_active: bool,
}

impl CellBuffer {
    pub(crate) fn new(limit: usize) -> Self {
        Self {
            cells: Vec::new(),
            sorted: Vec::new(),
            row_start: Vec::new(),
            curr: Cell::INITIAL,
            min_x: i32::MAX,
            min_y: i32::MAX,
            max_x: -i32::MAX,
            max_y: -i32::MAX,
            is_sorted: false,
            limit,
            overflowed: false,
            frame: None,
            dense_threshold: usize::MAX,
            dense: DenseGrid::default(),
            dense_active: false,
        }
    }

    /// Declares bounds that all future cells fall within (or none). Dense
    /// accumulation kicks in once storing cells would take more memory than
    /// a grid over the frame.
    pub(crate) fn set_frame(&mut self, frame: Option<Frame>) {
        self.frame = frame;
        self.dense_threshold = frame.map_or(usize::MAX, |f| (f.area() / 2).max(MIN_DENSE_CELLS));
    }

    pub(crate) fn reset(&mut self) {
        self.cells.clear();
        self.curr = Cell::INITIAL;
        self.min_x = i32::MAX;
        self.min_y = i32::MAX;
        self.max_x = -i32::MAX;
        self.max_y = -i32::MAX;
        self.is_sorted = false;
        self.overflowed = false;
        if self.dense_active {
            self.dense.clear();
            self.dense_active = false;
        }
    }

    pub(crate) const fn is_sorted(&self) -> bool {
        self.is_sorted
    }

    pub(crate) const fn overflowed(&self) -> bool {
        self.overflowed
    }

    pub(crate) const fn limit(&self) -> usize {
        self.limit
    }

    /// Returns `true` when no cell carries any coverage.
    pub(crate) fn is_empty(&self) -> bool {
        !self.dense_active && self.cells.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn total_cells(&self) -> usize {
        self.cells.len()
    }

    #[cfg(test)]
    pub(crate) const fn is_dense(&self) -> bool {
        self.dense_active
    }

    /// `(min_x, min_y, max_x, max_y)` cell bounds of the edges added so far.
    pub(crate) const fn bounds(&self) -> (i32, i32, i32, i32) {
        (self.min_x, self.min_y, self.max_x, self.max_y)
    }

    /// The sorted cells of row `y` (valid after [`sort`](Self::sort), in
    /// sparse mode).
    pub(super) fn row(&self, y: i32) -> &[Cell] {
        let i = (y - self.min_y) as usize;
        &self.sorted[self.row_start[i]..self.row_start[i + 1]]
    }

    /// Sweeps row `y` (after [`sort`](Self::sort)) into `sl`, mapping
    /// accumulated area to coverage with `alpha` (Agg's `sweep_scanline`).
    pub(crate) fn sweep_row(&self, y: i32, sl: &mut Scanline, alpha: impl Fn(i32) -> u8) {
        if self.dense_active {
            self.dense.sweep_row(y, sl, alpha);
        } else {
            sweep_cells(self.row(y), sl, alpha);
        }
    }

    fn add_curr_cell(&mut self) {
        if self.curr.area | self.curr.cover == 0 {
            return;
        }
        if self.dense_active {
            self.dense.add(&self.curr);
        } else if self.cells.len() >= self.limit {
            self.overflowed = true;
        } else {
            self.cells.push(self.curr);
            if self.cells.len() >= self.dense_threshold {
                self.densify();
            }
        }
    }

    /// Moves the stored cells into the dense grid over the frame.
    fn densify(&mut self) {
        let Some(frame) = self.frame else {
            return;
        };
        self.dense.prepare(frame);
        for cell in &self.cells {
            self.dense.add(cell);
        }
        self.cells.clear();
        self.dense_active = true;
    }

    fn set_curr_cell(&mut self, x: i32, y: i32) {
        if self.curr.x != x || self.curr.y != y {
            self.add_curr_cell();
            self.curr = Cell {
                x,
                y,
                cover: 0,
                area: 0,
            };
        }
    }

    fn accumulate(&mut self, cover: i32, area: i32) {
        self.curr.cover = self.curr.cover.wrapping_add(cover);
        self.curr.area = self.curr.area.wrapping_add(area);
    }

    /// Renders the part of an edge lying within the cell row `ey`; `y1`
    /// and `y2` are subpixel offsets inside that row.
    fn render_hline(&mut self, ey: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
        let mut ex1 = x1 >> SUBPIXEL_SHIFT;
        let ex2 = x2 >> SUBPIXEL_SHIFT;
        let fx1 = x1 & SUBPIXEL_MASK;
        let fx2 = x2 & SUBPIXEL_MASK;

        // Trivial case: a horizontal piece contributes nothing.
        if y1 == y2 {
            self.set_curr_cell(ex2, ey);
            return;
        }

        // Everything inside a single cell.
        if ex1 == ex2 {
            let delta = y2 - y1;
            self.accumulate(delta, (fx1 + fx2) * delta);
            return;
        }

        // A run of adjacent cells on the same row.
        let mut p = (SUBPIXEL_SCALE - fx1) * (y2 - y1);
        let mut first = SUBPIXEL_SCALE;
        let mut incr = 1;
        let mut dx = x2 - x1;
        if dx < 0 {
            p = fx1 * (y2 - y1);
            first = 0;
            incr = -1;
            dx = -dx;
        }
        let mut delta = p / dx;
        let mut modulo = p % dx;
        if modulo < 0 {
            delta -= 1;
            modulo += dx;
        }
        self.accumulate(delta, (fx1 + first) * delta);

        ex1 += incr;
        self.set_curr_cell(ex1, ey);
        let mut y1 = y1 + delta;

        if ex1 != ex2 {
            let p = SUBPIXEL_SCALE * (y2 - y1 + delta);
            let mut lift = p / dx;
            let mut rem = p % dx;
            if rem < 0 {
                lift -= 1;
                rem += dx;
            }
            modulo -= dx;
            while ex1 != ex2 {
                delta = lift;
                modulo += rem;
                if modulo >= 0 {
                    modulo -= dx;
                    delta += 1;
                }
                self.accumulate(delta, SUBPIXEL_SCALE * delta);
                y1 += delta;
                ex1 += incr;
                self.set_curr_cell(ex1, ey);
            }
        }
        let delta = y2 - y1;
        self.accumulate(delta, (fx2 + SUBPIXEL_SCALE - first) * delta);
    }

    /// Adds the edge from `(x1, y1)` to `(x2, y2)` in 24.8 fixed point.
    pub(crate) fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let dx = x2 - x1;
        if dx >= DX_LIMIT || dx <= -DX_LIMIT {
            // Overflow-safe midpoint.
            let cx = (x1 >> 1) + (x2 >> 1) + (x1 & 1 & x2 & 1);
            let cy = (y1 >> 1) + (y2 >> 1) + (y1 & 1 & y2 & 1);
            self.line(x1, y1, cx, cy);
            self.line(cx, cy, x2, y2);
            return;
        }

        let mut dy = y2 - y1;
        let ex1 = x1 >> SUBPIXEL_SHIFT;
        let ex2 = x2 >> SUBPIXEL_SHIFT;
        let mut ey1 = y1 >> SUBPIXEL_SHIFT;
        let ey2 = y2 >> SUBPIXEL_SHIFT;
        let fy1 = y1 & SUBPIXEL_MASK;
        let fy2 = y2 & SUBPIXEL_MASK;

        self.min_x = self.min_x.min(ex1).min(ex2);
        self.max_x = self.max_x.max(ex1).max(ex2);
        self.min_y = self.min_y.min(ey1).min(ey2);
        self.max_y = self.max_y.max(ey1).max(ey2);

        self.set_curr_cell(ex1, ey1);

        // Everything on a single row.
        if ey1 == ey2 {
            self.render_hline(ey1, x1, fy1, x2, fy2);
            return;
        }

        let mut incr = 1;

        // Vertical line: one cell per row with a common cover and area.
        if dx == 0 {
            let ex = x1 >> SUBPIXEL_SHIFT;
            let two_fx = (x1 - (ex << SUBPIXEL_SHIFT)) << 1;
            let mut first = SUBPIXEL_SCALE;
            if dy < 0 {
                first = 0;
                incr = -1;
            }
            let delta = first - fy1;
            self.accumulate(delta, two_fx * delta);
            ey1 += incr;
            self.set_curr_cell(ex, ey1);

            let delta = first + first - SUBPIXEL_SCALE;
            let area = two_fx * delta;
            while ey1 != ey2 {
                self.curr.cover = delta;
                self.curr.area = area;
                ey1 += incr;
                self.set_curr_cell(ex, ey1);
            }
            let delta = fy2 - SUBPIXEL_SCALE + first;
            self.accumulate(delta, two_fx * delta);
            return;
        }

        // Several rows.
        let mut p = (SUBPIXEL_SCALE - fy1) * dx;
        let mut first = SUBPIXEL_SCALE;
        if dy < 0 {
            p = fy1 * dx;
            first = 0;
            incr = -1;
            dy = -dy;
        }
        let mut delta = p / dy;
        let mut modulo = p % dy;
        if modulo < 0 {
            delta -= 1;
            modulo += dy;
        }
        let mut x_from = x1 + delta;
        self.render_hline(ey1, x1, fy1, x_from, first);
        ey1 += incr;
        self.set_curr_cell(x_from >> SUBPIXEL_SHIFT, ey1);

        if ey1 != ey2 {
            let p = SUBPIXEL_SCALE * dx;
            let mut lift = p / dy;
            let mut rem = p % dy;
            if rem < 0 {
                lift -= 1;
                rem += dy;
            }
            modulo -= dy;
            while ey1 != ey2 {
                delta = lift;
                modulo += rem;
                if modulo >= 0 {
                    modulo -= dy;
                    delta += 1;
                }
                let x_to = x_from + delta;
                self.render_hline(ey1, x_from, SUBPIXEL_SCALE - first, x_to, first);
                x_from = x_to;
                ey1 += incr;
                self.set_curr_cell(x_from >> SUBPIXEL_SHIFT, ey1);
            }
        }
        self.render_hline(ey1, x_from, SUBPIXEL_SCALE - first, x2, fy2);
    }

    /// Flushes the current cell and sorts all cells by row, then by column.
    /// Like Agg, an empty buffer is never marked sorted.
    pub(crate) fn sort(&mut self) {
        if self.is_sorted {
            return;
        }
        self.add_curr_cell();
        self.curr = Cell::INITIAL;
        if self.dense_active {
            self.is_sorted = true;
            return;
        }
        if self.cells.is_empty() {
            return;
        }

        let rows = (self.max_y - self.min_y + 1) as usize;
        self.row_start.clear();
        self.row_start.resize(rows + 1, 0);
        for cell in &self.cells {
            self.row_start[(cell.y - self.min_y) as usize + 1] += 1;
        }
        for i in 1..=rows {
            self.row_start[i] += self.row_start[i - 1];
        }

        self.sorted.clear();
        self.sorted.resize(self.cells.len(), Cell::INITIAL);
        let mut next: Vec<usize> = self.row_start[..rows].to_vec();
        for cell in &self.cells {
            let slot = &mut next[(cell.y - self.min_y) as usize];
            self.sorted[*slot] = *cell;
            *slot += 1;
        }
        for row in self.row_start.windows(2) {
            self.sorted[row[0]..row[1]].sort_unstable_by_key(|c| c.x);
        }
        self.is_sorted = true;
    }
}

/// Agg's `sweep_scanline` over one row of sorted cells.
fn sweep_cells(row: &[Cell], sl: &mut Scanline, alpha: impl Fn(i32) -> u8) {
    let mut cover = 0i32;
    let mut i = 0;
    while i < row.len() {
        let mut x = row[i].x;
        let mut area = row[i].area;
        cover = cover.wrapping_add(row[i].cover);
        i += 1;
        while i < row.len() && row[i].x == x {
            area = area.wrapping_add(row[i].area);
            cover = cover.wrapping_add(row[i].cover);
            i += 1;
        }
        if area != 0 {
            let a = alpha((cover << 9).wrapping_sub(area));
            if a != 0 {
                sl.add_cell(x, a);
            }
            x += 1;
        }
        if i < row.len() && row[i].x > x {
            let a = alpha(cover << 9);
            if a != 0 {
                sl.add_span(x, row[i].x - x, a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells_of(buffer: &mut CellBuffer) -> Vec<Cell> {
        buffer.sort();
        let (_, min_y, _, max_y) = buffer.bounds();
        (min_y..=max_y)
            .flat_map(|y| buffer.row(y).to_vec())
            .collect()
    }

    #[test]
    fn vertical_edge_covers_whole_rows() {
        let mut b = CellBuffer::new(usize::MAX);
        // x = 1.5 px, from y = 0 to y = 2 px.
        b.line(384, 0, 384, 512);
        let cells = cells_of(&mut b);
        assert_eq!(
            cells,
            vec![
                Cell {
                    x: 1,
                    y: 0,
                    cover: 256,
                    area: 256 * 256
                },
                Cell {
                    x: 1,
                    y: 1,
                    cover: 256,
                    area: 256 * 256
                }
            ]
        );
    }

    #[test]
    fn closed_square_cancels_cover() {
        let mut b = CellBuffer::new(usize::MAX);
        let pts = [(0, 0), (512, 0), (512, 512), (0, 512), (0, 0)];
        for w in pts.windows(2) {
            b.line(w[0].0, w[0].1, w[1].0, w[1].1);
        }
        let cells = cells_of(&mut b);
        let total: i32 = cells.iter().map(|c| c.cover).sum();
        assert_eq!(total, 0);
        assert_eq!(b.bounds(), (0, 0, 2, 2));
    }

    #[test]
    fn diagonal_edge_preserves_total_cover() {
        let mut b = CellBuffer::new(usize::MAX);
        b.line(10, 20, 3000, 1500);
        let cells = cells_of(&mut b);
        let total: i32 = cells.iter().map(|c| c.cover).sum();
        assert_eq!(total, 1480);
        assert!(
            cells
                .windows(2)
                .all(|w| (w[0].y, w[0].x) <= (w[1].y, w[1].x))
        );
    }

    #[test]
    fn huge_horizontal_extent_is_split() {
        let mut b = CellBuffer::new(usize::MAX);
        b.line(0, 0, DX_LIMIT * 4, 256);
        let cells = cells_of(&mut b);
        assert_eq!(cells.iter().map(|c| c.cover).sum::<i32>(), 256);
    }

    #[test]
    fn limit_sets_overflow_flag() {
        let mut b = CellBuffer::new(3);
        b.line(0, 0, 0, 256 * 10);
        b.sort();
        assert!(b.overflowed());
        assert_eq!(b.total_cells(), 3);
    }

    #[test]
    fn empty_buffer_is_not_sorted() {
        let mut b = CellBuffer::new(10);
        b.sort();
        assert!(!b.is_sorted());
        assert_eq!(b.total_cells(), 0);
    }
}
