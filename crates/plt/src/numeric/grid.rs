//! A dense, row-major two-dimensional array.

use std::fmt;
use std::ops::{Index, IndexMut};

/// A dense `rows × cols` array stored row-major, the plt-rs counterpart of
/// a 2-D numpy array (images, meshes, contour fields, spectrograms).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    data: Vec<T>,
}

/// A shape inconsistency when building a [`Grid`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridError {
    /// The flat data length is not `rows * cols`.
    LengthMismatch {
        /// Expected element count.
        expected: usize,
        /// Supplied element count.
        actual: usize,
    },
    /// Nested rows have different lengths.
    RaggedRows,
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { expected, actual } => {
                write!(f, "grid needs {expected} elements, got {actual}")
            }
            Self::RaggedRows => f.write_str("all grid rows must have the same length"),
        }
    }
}

impl std::error::Error for GridError {}

impl<T> Grid<T> {
    /// Wraps row-major `data` of shape `rows × cols`.
    pub fn from_vec(rows: usize, cols: usize, data: Vec<T>) -> Result<Self, GridError> {
        let expected = rows * cols;
        if data.len() == expected {
            Ok(Self { rows, cols, data })
        } else {
            Err(GridError::LengthMismatch {
                expected,
                actual: data.len(),
            })
        }
    }

    /// Builds a grid by evaluating `f(row, col)` for every cell.
    pub fn from_fn(rows: usize, cols: usize, mut f: impl FnMut(usize, usize) -> T) -> Self {
        let data = (0..rows)
            .flat_map(|r| (0..cols).map(move |c| (r, c)))
            .map(|(r, c)| f(r, c))
            .collect();
        Self { rows, cols, data }
    }

    /// Builds a grid from nested rows.
    pub fn from_rows<R: IntoIterator<Item = T>>(
        rows: impl IntoIterator<Item = R>,
    ) -> Result<Self, GridError> {
        let mut data = Vec::new();
        let mut cols = None;
        let mut count = 0;
        for row in rows {
            let before = data.len();
            data.extend(row);
            let width = data.len() - before;
            if *cols.get_or_insert(width) != width {
                return Err(GridError::RaggedRows);
            }
            count += 1;
        }
        Ok(Self {
            rows: count,
            cols: cols.unwrap_or(0),
            data,
        })
    }

    /// Number of rows.
    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    pub const fn cols(&self) -> usize {
        self.cols
    }

    /// `(rows, cols)`.
    pub const fn shape(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    /// Returns `true` when the grid has no cells.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// The row-major cell data.
    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    /// Mutable row-major cell data.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    /// Consumes the grid, returning its row-major data.
    pub fn into_vec(self) -> Vec<T> {
        self.data
    }

    /// The cell at `(row, col)`, if in bounds.
    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        (row < self.rows && col < self.cols).then(|| &self.data[row * self.cols + col])
    }

    /// One row as a slice.
    pub fn row(&self, row: usize) -> &[T] {
        &self.data[row * self.cols..(row + 1) * self.cols]
    }

    /// Iterates the rows as slices.
    pub fn iter_rows(&self) -> impl ExactSizeIterator<Item = &[T]> + '_ {
        (0..self.rows).map(move |r| self.row(r))
    }

    /// Applies `f` to every cell.
    pub fn map<U>(&self, f: impl FnMut(&T) -> U) -> Grid<U> {
        Grid {
            rows: self.rows,
            cols: self.cols,
            data: self.data.iter().map(f).collect(),
        }
    }
}

impl<T: Clone> Grid<T> {
    /// A grid with every cell set to `value`.
    pub fn filled(rows: usize, cols: usize, value: T) -> Self {
        Self {
            rows,
            cols,
            data: vec![value; rows * cols],
        }
    }

    /// The transposed grid.
    #[must_use]
    pub fn transposed(&self) -> Self {
        Self::from_fn(self.cols, self.rows, |r, c| {
            self.data[c * self.cols + r].clone()
        })
    }

    /// One column, copied.
    pub fn column(&self, col: usize) -> Vec<T> {
        (0..self.rows)
            .map(|r| self.data[r * self.cols + col].clone())
            .collect()
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, (row, col): (usize, usize)) -> &T {
        assert!(
            row < self.rows && col < self.cols,
            "grid index ({row}, {col}) out of bounds"
        );
        &self.data[row * self.cols + col]
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut T {
        assert!(
            row < self.rows && col < self.cols,
            "grid index ({row}, {col}) out of bounds"
        );
        &mut self.data[row * self.cols + col]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_and_indexing() {
        let g = Grid::from_rows([[1, 2, 3], [4, 5, 6]]).unwrap();
        assert_eq!(g.shape(), (2, 3));
        assert_eq!(g[(1, 0)], 4);
        assert_eq!(g.row(0), &[1, 2, 3]);
        assert_eq!(g.column(2), vec![3, 6]);
        assert_eq!(g.get(2, 0), None);
        assert_eq!(g.transposed()[(2, 1)], 6);
        assert_eq!(g.iter_rows().count(), 2);
    }

    #[test]
    fn shape_errors() {
        assert_eq!(
            Grid::from_vec(2, 2, vec![1, 2, 3]),
            Err(GridError::LengthMismatch {
                expected: 4,
                actual: 3
            })
        );
        assert_eq!(
            Grid::from_rows(vec![vec![1, 2], vec![3]]),
            Err(GridError::RaggedRows)
        );
    }

    #[test]
    fn empty_and_zero_width() {
        let g: Grid<f64> = Grid::from_rows(Vec::<Vec<f64>>::new()).unwrap();
        assert!(g.is_empty());
        let z: Grid<u8> = Grid::filled(3, 0, 0);
        assert_eq!(
            z.iter_rows().map(<[u8]>::len).collect::<Vec<_>>(),
            vec![0, 0, 0]
        );
    }

    #[test]
    fn map_preserves_shape() {
        let g = Grid::from_fn(2, 2, |r, c| (r * 2 + c) as f64);
        assert_eq!(g.map(|v| v * 10.0).into_vec(), vec![0.0, 10.0, 20.0, 30.0]);
    }
}
