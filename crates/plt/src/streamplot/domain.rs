//! The coordinate systems of a streamplot: data, grid indices and the
//! occupancy mask (matplotlib's `Grid` and `DomainMap`).

use plt_core::Point;

use super::StreamplotError;
use super::mask::StreamMask;
use crate::numeric::Grid;

/// An evenly spaced, strictly increasing grid (matplotlib's
/// `streamplot.Grid`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct UniformGrid {
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    dx: f64,
    dy: f64,
    pub(crate) origin: Point,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

/// Which grid coordinate a problem concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GridAxis {
    /// The column coordinates.
    X,
    /// The row coordinates.
    Y,
}

impl GridAxis {
    /// The argument name in matplotlib's messages.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Y => "y",
        }
    }
}

impl UniformGrid {
    /// Validates the column coordinates `x` and row coordinates `y`.
    pub(crate) fn new(x: &[f64], y: &[f64]) -> Result<Self, StreamplotError> {
        let (dx, width) = spacing(x, GridAxis::X)?;
        let (dy, height) = spacing(y, GridAxis::Y)?;
        Ok(Self {
            nx: x.len(),
            ny: y.len(),
            dx,
            dy,
            origin: Point::new(x[0], y[0]),
            width,
            height,
        })
    }

    /// Whether the grid coordinates `(xi, yi)` lie on the grid.
    pub(crate) fn contains(&self, xi: f64, yi: f64) -> bool {
        (0.0..=(self.nx - 1) as f64).contains(&xi) && (0.0..=(self.ny - 1) as f64).contains(&yi)
    }

    /// The `(rows, columns)` shape fields on this grid must have.
    pub(crate) fn shape(&self) -> (usize, usize) {
        (self.ny, self.nx)
    }

    /// Whether `grid` has this grid's shape.
    pub(crate) fn fits<T>(&self, grid: &Grid<T>) -> bool {
        (grid.rows(), grid.cols()) == self.shape()
    }
}

/// The step and the span of strictly increasing, evenly spaced
/// coordinates.
fn spacing(values: &[f64], axis: GridAxis) -> Result<(f64, f64), StreamplotError> {
    let [first, second, ..] = values else {
        return Err(StreamplotError::TooFewPoints(axis));
    };
    if !values.windows(2).all(|w| w[1] - w[0] > 0.0) {
        return Err(StreamplotError::NotIncreasing(axis));
    }
    let span = values[values.len() - 1] - first;
    let step = span / (values.len() - 1) as f64;
    // numpy's allclose: |a - b| <= atol + rtol * |b|.
    if values
        .windows(2)
        .any(|w| ((w[1] - w[0]) - step).abs() > 1e-8 + 1e-5 * step.abs())
    {
        return Err(StreamplotError::UnevenSpacing(axis));
    }
    Ok((second - first, span))
}

/// Conversions between data, grid and mask coordinates (matplotlib's
/// `DomainMap`), plus the bookkeeping of the trajectory being traced.
#[derive(Debug)]
pub(crate) struct DomainMap {
    pub(crate) grid: UniformGrid,
    pub(crate) mask: StreamMask,
    x_grid2mask: f64,
    y_grid2mask: f64,
    x_mask2grid: f64,
    y_mask2grid: f64,
    x_data2grid: f64,
    y_data2grid: f64,
}

impl DomainMap {
    pub(crate) fn new(grid: UniformGrid, mask: StreamMask) -> Self {
        let (mask_rows, mask_cols) = mask.shape();
        let x_grid2mask = (mask_cols as f64 - 1.0) / (grid.nx as f64 - 1.0);
        let y_grid2mask = (mask_rows as f64 - 1.0) / (grid.ny as f64 - 1.0);
        Self {
            grid,
            mask,
            x_grid2mask,
            y_grid2mask,
            x_mask2grid: 1.0 / x_grid2mask,
            y_mask2grid: 1.0 / y_grid2mask,
            x_data2grid: 1.0 / grid.dx,
            y_data2grid: 1.0 / grid.dy,
        }
    }

    /// The mask cell nearest to grid coordinates (Python's `round`,
    /// halves to even).
    fn grid2mask(&self, xi: f64, yi: f64) -> (i64, i64) {
        (
            (xi * self.x_grid2mask).round_ties_even() as i64,
            (yi * self.y_grid2mask).round_ties_even() as i64,
        )
    }

    /// The grid coordinates of a mask cell.
    pub(crate) fn mask2grid(&self, xm: usize, ym: usize) -> (f64, f64) {
        (xm as f64 * self.x_mask2grid, ym as f64 * self.y_mask2grid)
    }

    /// Grid coordinates of a data offset from the grid origin (also
    /// converts velocities).
    pub(crate) fn data2grid(&self, xd: f64, yd: f64) -> (f64, f64) {
        (xd * self.x_data2grid, yd * self.y_data2grid)
    }

    /// The data point at grid coordinates.
    pub(crate) fn grid2data(&self, xg: f64, yg: f64) -> Point {
        Point::new(
            xg / self.x_data2grid + self.grid.origin.x,
            yg / self.y_data2grid + self.grid.origin.y,
        )
    }

    /// Starts recording a trajectory at grid coordinates; `false` if its
    /// cell is taken.
    pub(crate) fn start_trajectory(&mut self, xg: f64, yg: f64, broken: bool) -> bool {
        let (xm, ym) = self.grid2mask(xg, yg);
        self.mask.start_trajectory(xm, ym, broken)
    }

    /// Makes the cell of the start point current again, before tracing the
    /// second direction.
    pub(crate) fn reset_start_point(&mut self, xg: f64, yg: f64) {
        let (xm, ym) = self.grid2mask(xg, yg);
        self.mask.set_current(xm, ym);
    }

    /// Records that the trajectory reached grid coordinates; `false` if it
    /// left the grid or entered a taken cell.
    pub(crate) fn update_trajectory(&mut self, xg: f64, yg: f64, broken: bool) -> bool {
        if !self.grid.contains(xg, yg) {
            return false;
        }
        let (xm, ym) = self.grid2mask(xg, yg);
        self.mask.update_trajectory(xm, ym, broken)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::linspace;

    #[test]
    fn grids_must_increase_evenly() {
        let x = linspace(-3.0, 3.0, 40);
        let grid = UniformGrid::new(&x, &[0.0, 0.5, 1.0]).unwrap();
        assert_eq!(grid.shape(), (3, 40));
        assert_eq!(grid.width, 6.0);
        assert!(grid.contains(39.0, 2.0));
        assert!(!grid.contains(39.0001, 0.0));
        assert!(!grid.contains(f64::NAN, 0.0));
        assert_eq!(
            UniformGrid::new(&[0.0, 1.0, 3.0], &[0.0, 1.0]),
            Err(StreamplotError::UnevenSpacing(GridAxis::X))
        );
        assert_eq!(
            UniformGrid::new(&[0.0, 1.0], &[1.0, 0.0]),
            Err(StreamplotError::NotIncreasing(GridAxis::Y))
        );
        assert_eq!(
            UniformGrid::new(&[0.0], &[0.0, 1.0]),
            Err(StreamplotError::TooFewPoints(GridAxis::X))
        );
    }

    #[test]
    fn coordinate_conversions() {
        let grid = UniformGrid::new(&linspace(0.0, 2.0, 5), &linspace(0.0, 1.0, 3)).unwrap();
        let map = DomainMap::new(grid, StreamMask::new((1.0, 1.0)).unwrap());
        assert_eq!(map.mask2grid(29, 29), (4.0, 2.0));
        assert_eq!(map.grid2mask(2.0, 1.0), (14, 14), "14.5 rounds to even");
        assert_eq!(map.data2grid(1.0, 0.5), (2.0, 1.0));
        assert_eq!(map.grid2data(2.0, 1.0), Point::new(1.0, 0.5));
    }
}
