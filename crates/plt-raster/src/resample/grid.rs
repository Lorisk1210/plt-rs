//! A dense, row-major image grid.

use super::error::ResampleError;

/// A `width × height` grid of pixels stored row by row.
///
/// Pixel `(x, y)` lives at index `y * width + x`; row 0 is the first row in
/// memory (array-index order, as in a `NumPy` array).
#[derive(Clone, Debug, PartialEq)]
pub struct Grid<P> {
    width: usize,
    height: usize,
    data: Vec<P>,
}

impl<P> Grid<P> {
    /// Wraps `data` (row-major, `width × height` pixels) without copying.
    pub fn from_vec(width: usize, height: usize, data: Vec<P>) -> Result<Self, ResampleError> {
        if width.checked_mul(height) != Some(data.len()) {
            return Err(ResampleError::GridSize {
                width,
                height,
                len: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// The grid as a borrowed view.
    pub fn view(&self) -> GridRef<'_, P> {
        GridRef {
            width: self.width,
            height: self.height,
            data: &self.data,
        }
    }

    /// Number of columns.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Whether the grid holds no pixels.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The pixel at column `x`, row `y`, if inside the grid.
    pub fn get(&self, x: usize, y: usize) -> Option<&P> {
        self.view().get(x, y)
    }

    /// Mutable access to the pixel at column `x`, row `y`, if inside.
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut P> {
        if x < self.width && y < self.height {
            self.data.get_mut(y * self.width + x)
        } else {
            None
        }
    }

    /// Row `y`, if inside the grid.
    pub fn row(&self, y: usize) -> Option<&[P]> {
        if y >= self.height {
            return None;
        }
        let start = y * self.width;
        self.data.get(start..start + self.width)
    }

    /// All pixels, row-major.
    pub fn as_slice(&self) -> &[P] {
        &self.data
    }

    /// All pixels, row-major, mutably.
    pub fn as_mut_slice(&mut self) -> &mut [P] {
        &mut self.data
    }

    /// Consumes the grid, returning its row-major pixels.
    pub fn into_vec(self) -> Vec<P> {
        self.data
    }
}

impl<P: Clone> Grid<P> {
    /// A grid with every pixel set to `value`.
    pub fn filled(width: usize, height: usize, value: P) -> Result<Self, ResampleError> {
        let len = width.checked_mul(height).ok_or(ResampleError::GridSize {
            width,
            height,
            len: 0,
        })?;
        Ok(Self {
            width,
            height,
            data: vec![value; len],
        })
    }
}

/// A borrowed `width × height` grid of pixels stored row by row: what
/// [`resample`](super::resample) reads, so that pixels held elsewhere need
/// not be copied into a [`Grid`].
#[derive(Debug, PartialEq)]
pub struct GridRef<'a, P> {
    width: usize,
    height: usize,
    data: &'a [P],
}

// A view copies like the reference it holds, whatever the pixels.
impl<P> Clone for GridRef<'_, P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P> Copy for GridRef<'_, P> {}

impl<'a, P> GridRef<'a, P> {
    /// Views `data` (row-major, `width × height` pixels).
    pub fn new(width: usize, height: usize, data: &'a [P]) -> Result<Self, ResampleError> {
        if width.checked_mul(height) != Some(data.len()) {
            return Err(ResampleError::GridSize {
                width,
                height,
                len: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Number of columns.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Whether the grid holds no pixels.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The pixel at column `x`, row `y`, if inside the grid.
    pub fn get(&self, x: usize, y: usize) -> Option<&'a P> {
        if x < self.width && y < self.height {
            self.data.get(y * self.width + x)
        } else {
            None
        }
    }

    /// All pixels, row-major.
    pub fn as_slice(&self) -> &'a [P] {
        self.data
    }
}

impl<'a, P> From<&'a Grid<P>> for GridRef<'a, P> {
    fn from(grid: &'a Grid<P>) -> Self {
        grid.view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_vec_checks_length() {
        assert!(Grid::from_vec(2, 3, vec![0_u8; 6]).is_ok());
        assert_eq!(
            Grid::from_vec(2, 3, vec![0_u8; 5]),
            Err(ResampleError::GridSize {
                width: 2,
                height: 3,
                len: 5
            })
        );
        assert!(Grid::<u8>::from_vec(usize::MAX, 2, Vec::new()).is_err());
        assert!(Grid::filled(usize::MAX, 2, 0_u8).is_err());
    }

    #[test]
    fn indexing_is_row_major() {
        let mut g = Grid::from_vec(3, 2, (0..6).collect::<Vec<i32>>()).expect("grid");
        assert_eq!(g.get(2, 1), Some(&5));
        assert_eq!(g.get(3, 0), None);
        assert_eq!(g.row(1), Some(&[3, 4, 5][..]));
        assert_eq!(g.row(2), None);
        *g.get_mut(0, 1).expect("inside") = 9;
        assert_eq!(g.as_slice(), &[0, 1, 2, 9, 4, 5]);
        assert!(!g.is_empty());
        assert_eq!((g.width(), g.height()), (3, 2));
        assert_eq!(g.into_vec().len(), 6);
    }

    #[test]
    fn views_borrow_and_check_length() {
        let pixels = [1, 2, 3, 4, 5, 6];
        let view = GridRef::new(3, 2, &pixels).expect("view");
        assert_eq!((view.width(), view.height()), (3, 2));
        assert_eq!(view.get(0, 1), Some(&4));
        assert_eq!(view.get(3, 0), None);
        assert!(!view.is_empty());
        assert!(GridRef::new(4, 2, &pixels).is_err());
        let grid = Grid::from_vec(3, 2, pixels.to_vec()).expect("grid");
        assert_eq!(GridRef::from(&grid), view);
        assert_eq!(view.as_slice(), grid.as_slice());
    }

    #[test]
    fn empty_grids() {
        let g = Grid::filled(0, 4, 1.0_f32).expect("grid");
        assert!(g.is_empty());
        assert_eq!(g.row(0), Some(&[][..]));
        assert_eq!(g.row(4), None);
    }
}
