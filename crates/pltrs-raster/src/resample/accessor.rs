//! Source pixel access outside the image bounds.
//!
//! Filters read through Agg's `image_accessor_wrap` with
//! `wrap_mode_reflect` on both axes (edge pixels are mirrored, the edge
//! itself repeated); nearest-neighbour sampling reads through
//! `image_accessor_clip`, which yields a zero background outside.

use super::grid::GridRef;
use super::pixel::ops::PixelOps;

/// Agg's `wrap_mode_reflect`, with its unsigned wrap-around arithmetic.
#[derive(Clone, Copy, Debug)]
struct Reflect {
    size: u32,
    size2: u32,
    add: u32,
    value: u32,
}

impl Reflect {
    fn new(size: u32) -> Self {
        let size2 = size * 2;
        Self {
            size,
            size2,
            add: size2 * (0x3FFF_FFFF / size2),
            value: 0,
        }
    }

    fn reflect(&self) -> usize {
        let v = if self.value >= self.size {
            self.size2 - self.value - 1
        } else {
            self.value
        };
        v as usize
    }

    /// `operator()(v)`: the reflected index of coordinate `v`.
    fn at(&mut self, v: i32) -> usize {
        self.value = (v as u32).wrapping_add(self.add) % self.size2;
        self.reflect()
    }

    /// `operator++`: the reflected index of the next coordinate.
    fn next(&mut self) -> usize {
        self.value += 1;
        if self.value >= self.size2 {
            self.value = 0;
        }
        self.reflect()
    }
}

/// `image_accessor_wrap<pixfmt, wrap_mode_reflect, wrap_mode_reflect>`,
/// handing out reflected column indices and row offsets: a run of
/// columns (or rows) starts at a coordinate and continues one at a time,
/// as Agg's `span`, `next_x` and `next_y` walk them.
#[derive(Debug)]
pub(super) struct ReflectAccessor<'a, P> {
    pixels: &'a [P],
    width: usize,
    wrap_x: Reflect,
    wrap_y: Reflect,
}

impl<'a, P: Copy> ReflectAccessor<'a, P> {
    /// An accessor over a non-empty grid whose sides fit in `u32 / 2`.
    pub(super) fn new(grid: GridRef<'a, P>) -> Self {
        Self {
            pixels: grid.as_slice(),
            width: grid.width(),
            wrap_x: Reflect::new(grid.width() as u32),
            wrap_y: Reflect::new(grid.height() as u32),
        }
    }

    /// The pixels, row by row.
    pub(super) const fn pixels(&self) -> &'a [P] {
        self.pixels
    }

    /// The column of coordinate `x`, starting a run of columns.
    pub(super) fn first_column(&mut self, x: i32) -> usize {
        self.wrap_x.at(x)
    }

    /// The column after the previous one.
    pub(super) fn next_column(&mut self) -> usize {
        self.wrap_x.next()
    }

    /// The offset of the row of coordinate `y`, starting a run of rows.
    pub(super) fn first_row(&mut self, y: i32) -> usize {
        self.wrap_y.at(y) * self.width
    }

    /// The offset of the row after the previous one.
    pub(super) fn next_row(&mut self) -> usize {
        self.wrap_y.next() * self.width
    }
}

/// `image_accessor_clip` reading a single pixel: the background
/// (`no_color()`) outside the grid.
pub(super) fn clipped<P: PixelOps>(grid: GridRef<'_, P>, x: i32, y: i32) -> P {
    match (usize::try_from(x), usize::try_from(y)) {
        (Ok(x), Ok(y)) => grid.get(x, y).copied().unwrap_or(P::BACKGROUND),
        _ => P::BACKGROUND,
    }
}

#[cfg(test)]
mod tests {
    use super::super::grid::Grid;
    use super::*;

    #[test]
    fn reflect_mirrors_with_repeated_edges() {
        let mut r = Reflect::new(3);
        let idx: Vec<usize> = (-4..7).map(|v| r.at(v)).collect();
        assert_eq!(idx, vec![2, 2, 1, 0, 0, 1, 2, 2, 1, 0, 0]);
        r.at(1);
        let walked: Vec<usize> = (0..6).map(|_| r.next()).collect();
        assert_eq!(walked, vec![2, 2, 1, 0, 0, 1]);
    }

    #[test]
    fn reflect_accessor_walks_rows_and_columns() {
        let g = Grid::from_vec(2, 2, vec![1, 2, 3, 4]).expect("grid");
        let mut a = ReflectAccessor::new(g.view());
        let columns = [a.first_column(-1), a.next_column(), a.next_column()];
        assert_eq!(columns, [0, 0, 1]);
        let rows = [a.first_row(-1), a.next_row(), a.next_row()];
        assert_eq!(rows, [0, 0, 2]);
        assert_eq!(a.pixels()[rows[2] + columns[2]], 4);
    }

    #[test]
    fn clipped_uses_background_outside() {
        let g = Grid::from_vec(1, 1, vec![[9_u8; 4]]).expect("grid");
        assert_eq!(clipped(g.view(), 0, 0), [9; 4]);
        assert_eq!(clipped(g.view(), -1, 0), [0; 4]);
        assert_eq!(clipped(g.view(), 0, 1), [0; 4]);
    }
}
