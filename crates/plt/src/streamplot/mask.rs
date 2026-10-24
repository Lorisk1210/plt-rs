//! The occupancy mask that keeps streamlines apart (matplotlib's
//! `StreamMask` and `_gen_starting_points`).

use super::StreamplotError;
use crate::numeric::Grid;

/// The most cells of the occupancy mask (a density of about 136 each
/// way): matplotlib allocates whatever mask the density asks for.
pub const MAX_STREAMLINE_CELLS: usize = 1 << 24;

/// Cells crossed by streamlines so far: a streamline may only enter free
/// cells, and marks each cell it enters.
#[derive(Debug)]
pub(crate) struct StreamMask {
    cells: Grid<bool>,
    /// The cells marked by the trajectory being traced, `(row, column)`.
    trajectory: Vec<(usize, usize)>,
    /// The cell the trajectory is in (kept between trajectories, as in
    /// matplotlib).
    current: Option<(i64, i64)>,
}

impl StreamMask {
    /// A free mask of `30 × density` cells in each direction (truncated
    /// towards zero).
    pub(crate) fn new((density_x, density_y): (f64, f64)) -> Result<Self, StreamplotError> {
        let cells = |density: f64| {
            let n = (30.0 * density).trunc();
            if n >= 0.0 {
                Ok(n as usize)
            } else {
                Err(StreamplotError::InvalidDensity(density))
            }
        };
        let (nx, ny) = (cells(density_x)?, cells(density_y)?);
        if nx
            .checked_mul(ny)
            .is_none_or(|count| count > MAX_STREAMLINE_CELLS)
        {
            return Err(StreamplotError::TooManyCells(density_x, density_y));
        }
        Ok(Self {
            cells: Grid::filled(ny, nx, false),
            trajectory: Vec::new(),
            current: None,
        })
    }

    /// `(rows, columns)`.
    pub(crate) fn shape(&self) -> (usize, usize) {
        (self.cells.rows(), self.cells.cols())
    }

    /// Whether the cell at column `xm`, row `ym` is free.
    pub(crate) fn is_free(&self, xm: usize, ym: usize) -> bool {
        self.cells.get(ym, xm).is_some_and(|taken| !taken)
    }

    /// Starts recording a trajectory in a cell; `false` if it is taken.
    pub(crate) fn start_trajectory(&mut self, xm: i64, ym: i64, broken: bool) -> bool {
        self.trajectory.clear();
        self.update_trajectory(xm, ym, broken)
    }

    /// Frees the cells of the trajectory being traced.
    pub(crate) fn undo_trajectory(&mut self) {
        for &(row, col) in &self.trajectory {
            self.cells[(row, col)] = false;
        }
    }

    /// Makes `(xm, ym)` the current cell without marking it.
    pub(crate) fn set_current(&mut self, xm: i64, ym: i64) {
        self.current = Some((xm, ym));
    }

    /// Marks the cell the trajectory moved into; `false` if it is taken
    /// and streamlines may be broken there. Cells outside the mask count
    /// as taken.
    pub(crate) fn update_trajectory(&mut self, xm: i64, ym: i64, broken: bool) -> bool {
        if self.current == Some((xm, ym)) {
            return true;
        }
        let free = usize::try_from(xm)
            .ok()
            .zip(usize::try_from(ym).ok())
            .filter(|&(col, row)| self.is_free(col, row));
        match free {
            Some((col, row)) => {
                self.trajectory.push((row, col));
                self.cells[(row, col)] = true;
                self.current = Some((xm, ym));
                true
            }
            None => !broken,
        }
    }
}

/// The mask cells `(column, row)` in seeding order: from the corner,
/// spiralling inwards along the boundary, which gives better streamlines
/// than starting inside.
pub(crate) fn starting_points((rows, cols): (usize, usize)) -> Vec<(usize, usize)> {
    #[derive(Clone, Copy)]
    enum Heading {
        Right,
        Up,
        Left,
        Down,
    }
    let (mut x, mut y) = (0_i64, 0_i64);
    let (mut x_first, mut y_first) = (0_i64, 1_i64);
    let (mut x_last, mut y_last) = (cols as i64 - 1, rows as i64 - 1);
    let mut heading = Heading::Right;
    let mut points = Vec::with_capacity(rows * cols);
    for _ in 0..rows * cols {
        points.push((x as usize, y as usize));
        match heading {
            Heading::Right => {
                x += 1;
                if x >= x_last {
                    x_last -= 1;
                    heading = Heading::Up;
                }
            }
            Heading::Up => {
                y += 1;
                if y >= y_last {
                    y_last -= 1;
                    heading = Heading::Left;
                }
            }
            Heading::Left => {
                x -= 1;
                if x <= x_first {
                    x_first += 1;
                    heading = Heading::Down;
                }
            }
            Heading::Down => {
                y -= 1;
                if y <= y_first {
                    y_first += 1;
                    heading = Heading::Right;
                }
            }
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densities_set_the_cell_count() {
        assert_eq!(StreamMask::new((1.0, 1.0)).unwrap().shape(), (30, 30));
        assert_eq!(StreamMask::new((0.5, 1.5)).unwrap().shape(), (45, 15));
        assert_eq!(StreamMask::new((0.6, 0.7)).unwrap().shape(), (21, 18));
        assert_eq!(StreamMask::new((0.0, 0.0)).unwrap().shape(), (0, 0));
        assert_eq!(StreamMask::new((-0.01, 1.0)).unwrap().shape(), (30, 0));
        assert!(StreamMask::new((-1.0, 1.0)).is_err());
        assert!(StreamMask::new((f64::NAN, 1.0)).is_err());
        // Found by fuzzing: huge densities asked for unbounded masks.
        assert_eq!(
            StreamMask::new((1e300, 1.0)).err(),
            Some(StreamplotError::TooManyCells(1e300, 1.0))
        );
        assert!(StreamMask::new((136.0, 136.0)).is_ok());
    }

    #[test]
    fn trajectories_mark_and_free_cells() {
        let mut mask = StreamMask::new((0.1, 0.1)).unwrap();
        assert!(mask.start_trajectory(0, 0, true));
        assert!(mask.update_trajectory(1, 0, true));
        assert!(mask.update_trajectory(1, 0, true), "staying in a cell");
        assert!(!mask.is_free(1, 0));
        mask.set_current(2, 2);
        assert!(!mask.update_trajectory(0, 0, true), "taken");
        assert!(mask.update_trajectory(0, 0, false), "unbroken lines pass");
        assert!(!mask.update_trajectory(-1, 0, true), "outside");
        mask.undo_trajectory();
        assert!(mask.is_free(0, 0) && mask.is_free(1, 0));
    }

    #[test]
    fn seeds_spiral_inwards_from_the_corner() {
        let points = starting_points((3, 3));
        assert_eq!(
            points,
            [
                (0, 0),
                (1, 0),
                (2, 0),
                (2, 1),
                (2, 2),
                (1, 2),
                (0, 2),
                (0, 1),
                (1, 1)
            ]
        );
        assert_eq!(starting_points((0, 0)), []);
        assert_eq!(starting_points((1, 4)).len(), 4);
    }
}
