//! Source pixel access outside the image bounds.
//!
//! Filters read through Agg's `image_accessor_wrap` with
//! `wrap_mode_reflect` on both axes (edge pixels are mirrored, the edge
//! itself repeated); nearest-neighbour sampling reads through
//! `image_accessor_clip`, which yields a zero background outside.

use super::grid::Grid;
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

/// `image_accessor_wrap<pixfmt, wrap_mode_reflect, wrap_mode_reflect>`.
#[derive(Debug)]
pub(super) struct ReflectAccessor<'a, P> {
    pixels: &'a [P],
    width: usize,
    row: usize,
    x: i32,
    wrap_x: Reflect,
    wrap_y: Reflect,
}

impl<'a, P: Copy> ReflectAccessor<'a, P> {
    /// An accessor over a non-empty grid whose sides fit in `u32 / 2`.
    pub(super) fn new(grid: &'a Grid<P>) -> Self {
        Self {
            pixels: grid.as_slice(),
            width: grid.width(),
            row: 0,
            x: 0,
            wrap_x: Reflect::new(grid.width() as u32),
            wrap_y: Reflect::new(grid.height() as u32),
        }
    }

    /// Starts reading at `(x, y)`.
    pub(super) fn span(&mut self, x: i32, y: i32) -> P {
        self.x = x;
        self.row = self.wrap_y.at(y) * self.width;
        self.pixels[self.row + self.wrap_x.at(x)]
    }

    /// The pixel to the right of the previous one.
    pub(super) fn next_x(&mut self) -> P {
        self.pixels[self.row + self.wrap_x.next()]
    }

    /// The first pixel of the span in the next row.
    pub(super) fn next_y(&mut self) -> P {
        self.row = self.wrap_y.next() * self.width;
        self.pixels[self.row + self.wrap_x.at(self.x)]
    }
}

/// `image_accessor_clip` reading a single pixel: the background
/// (`no_color()`) outside the grid.
pub(super) fn clipped<P: PixelOps>(grid: &Grid<P>, x: i32, y: i32) -> P {
    match (usize::try_from(x), usize::try_from(y)) {
        (Ok(x), Ok(y)) => grid.get(x, y).copied().unwrap_or(P::BACKGROUND),
        _ => P::BACKGROUND,
    }
}

#[cfg(test)]
mod tests {
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
    fn reflect_accessor_walks_rows() {
        let g = Grid::from_vec(2, 2, vec![1, 2, 3, 4]).expect("grid");
        let mut a = ReflectAccessor::new(&g);
        assert_eq!(a.span(-1, -1), 1);
        assert_eq!(a.next_x(), 1);
        assert_eq!(a.next_x(), 2);
        assert_eq!(a.next_y(), 1);
        assert_eq!(a.next_y(), 3);
        assert_eq!(a.next_x(), 3);
    }

    #[test]
    fn clipped_uses_background_outside() {
        let g = Grid::from_vec(1, 1, vec![[9_u8; 4]]).expect("grid");
        assert_eq!(clipped(&g, 0, 0), [9; 4]);
        assert_eq!(clipped(&g, -1, 0), [0; 4]);
        assert_eq!(clipped(&g, 0, 1), [0; 4]);
    }
}
