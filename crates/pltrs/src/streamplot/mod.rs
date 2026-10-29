//! Streamlines of 2-D vector fields (matplotlib.streamplot).
//!
//! `Axes::streamplot` traces streamlines through a [`StreamField`] sampled
//! on an evenly spaced grid, exactly like matplotlib: start points spiral
//! inwards from the domain edge (or are given), each streamline is
//! integrated with an adaptive second-order Runge-Kutta scheme until it
//! leaves the domain, stagnates, reaches its maximum length or enters a
//! cell of the occupancy mask already crossed by another streamline (the
//! mask resolution is the `density`). Streamlines can be coloured and
//! sized by fields on the same grid and carry arrows.
//!
//! ```
//! use pltrs::numeric::{Grid, linspace};
//! use pltrs::prelude::*;
//! use pltrs::streamplot::{StreamField, StreamplotOptions};
//!
//! let x = linspace(-1.0, 1.0, 20);
//! let u = Grid::from_fn(20, 20, |i, _| -x[i]);
//! let v = Grid::from_fn(20, 20, |_, j| x[j]);
//! let ax = Figure::new().subplot();
//! let set = ax.streamplot(&StreamField::new(x.clone(), x.clone(), u, v), StreamplotOptions::default())?;
//! assert!(!set.arrows.is_empty());
//! # Ok::<(), pltrs::streamplot::StreamplotError>(())
//! ```

mod arrow;
mod domain;
mod error;
mod integrate;
mod mask;
mod plan;

pub use arrow::{StreamArrow, StreamArrowHead, StreamArrowStyle, UnknownArrowStyle};
pub use domain::GridAxis;
pub use error::StreamplotError;
pub use integrate::IntegrationDirection;
pub use integrate::MAX_STREAMLINE_STEPS;
pub use mask::MAX_STREAMLINE_CELLS;
pub use plan::MAX_STREAMLINE_ARROWS;

pub(crate) use arrow::{ArrowLook, StreamArrowArtist};
pub(crate) use plan::StreamPlan;

use pltrs_core::Point;

use crate::collections::Collection;
use crate::colors::{Color, ColorizerOptions, impl_colorize};
use crate::numeric::Grid;
use crate::transforms::Transform;

/// A vector field sampled on an evenly spaced grid (streamplot's `x, y,
/// u, v`). Non-finite components are masked: streamlines stop there.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamField {
    /// Strictly increasing, evenly spaced column coordinates.
    pub x: Vec<f64>,
    /// Strictly increasing, evenly spaced row coordinates.
    pub y: Vec<f64>,
    /// x components, one row per y.
    pub u: Grid<f64>,
    /// y components, one row per y.
    pub v: Grid<f64>,
}

impl StreamField {
    /// The components `u`, `v` on the grid spanned by `x` and `y`.
    pub fn new(x: Vec<f64>, y: Vec<f64>, u: Grid<f64>, v: Grid<f64>) -> Self {
        Self { x, y, u, v }
    }

    /// The components on a mesh of coordinates like those of
    /// [`meshgrid`](crate::numeric::meshgrid).
    ///
    /// # Errors
    ///
    /// When the rows of `xx` or the columns of `yy` differ.
    pub fn from_mesh(
        xx: &Grid<f64>,
        yy: &Grid<f64>,
        u: Grid<f64>,
        v: Grid<f64>,
    ) -> Result<Self, StreamplotError> {
        let x = common_row(xx).ok_or(StreamplotError::MeshRowsDiffer)?;
        let y = common_row(&yy.transposed()).ok_or(StreamplotError::MeshColumnsDiffer)?;
        Ok(Self::new(x, y, u, v))
    }
}

/// The first row of `grid` if every row is close to it (numpy's
/// `allclose(grid[0], grid)`).
fn common_row(grid: &Grid<f64>) -> Option<Vec<f64>> {
    let first = grid
        .iter_rows()
        .next()
        .map_or_else(Vec::new, <[f64]>::to_vec);
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-8 + 1e-5 * b.abs();
    grid.iter_rows()
        .all(|row| row.iter().zip(&first).all(|(&b, &a)| close(a, b)))
        .then_some(first)
}

/// Values on the field's grid mapped to streamline colours, through a
/// norm or limits autoscaled to the range of the finite values (matplotlib
/// takes no limits; like elsewhere, a norm excludes them).
#[derive(Clone, Debug)]
pub struct StreamColorMap {
    /// One value per grid point (non-finite values are masked).
    pub values: Grid<f64>,
    /// The colormap, norm and limits.
    pub colorizer: ColorizerOptions,
}

impl_colorize!(StreamColorMap);

impl StreamColorMap {
    /// `values` mapped through `image.cmap` and a linear norm spanning
    /// their finite range.
    pub fn new(values: Grid<f64>) -> Self {
        Self {
            values,
            colorizer: ColorizerOptions::default(),
        }
    }
}

/// How streamlines are coloured (matplotlib's `color`).
#[derive(Debug)]
pub enum StreamColors {
    /// One colour for every streamline and arrow.
    Single(Color),
    /// Colours interpolated from values on the grid.
    Mapped(Box<StreamColorMap>),
}

/// How wide streamlines are (matplotlib's `linewidth`).
#[derive(Clone, Debug, PartialEq)]
pub enum StreamWidths {
    /// One width in points.
    Uniform(f64),
    /// Widths in points interpolated from values on the grid.
    Varying(Grid<f64>),
}

/// Options of `Axes::streamplot` (matplotlib's `streamplot` keywords).
#[derive(Debug)]
pub struct StreamplotOptions {
    /// Closeness of streamlines in x and y: the occupancy mask has
    /// `30 × density` cells per direction.
    pub density: (f64, f64),
    /// Streamline widths (`None` = `lines.linewidth`).
    pub linewidth: Option<StreamWidths>,
    /// Streamline colours (`None` = the next colour of the line cycle).
    pub color: Option<StreamColors>,
    /// Scale of the arrow heads.
    pub arrowsize: f64,
    /// The arrow heads.
    pub arrowstyle: StreamArrowStyle,
    /// Shortest streamline kept, in axes-fraction lengths.
    pub minlength: f64,
    /// Longest streamline, in axes-fraction lengths.
    pub maxlength: f64,
    /// Start points in data coordinates (`None` = seeded over the grid).
    pub start_points: Option<Vec<Point>>,
    /// Drawing order of streamlines and arrows (`None` = that of lines).
    pub zorder: Option<f64>,
    /// Which way streamlines are traced from their start points.
    pub integration_direction: IntegrationDirection,
    /// Whether streamlines stop where they come too close to another one.
    pub broken_streamlines: bool,
    /// Multiplier on the largest integration step (positive).
    pub integration_max_step_scale: f64,
    /// Multiplier on the largest accepted integration error (positive).
    pub integration_max_error_scale: f64,
    /// Arrows per streamline, spaced evenly along its integration steps.
    pub num_arrows: usize,
    /// The coordinates of the grid (matplotlib's `transform`; default the
    /// axes' data coordinates). When they end in the data coordinates
    /// after a fixed affine map, the streamlines bound the data mapped
    /// into them; in other coordinates they do not bound the data.
    pub transform: Option<Transform>,
}

impl Default for StreamplotOptions {
    fn default() -> Self {
        Self {
            density: (1.0, 1.0),
            linewidth: None,
            color: None,
            arrowsize: 1.0,
            arrowstyle: StreamArrowStyle::default(),
            minlength: 0.1,
            maxlength: 4.0,
            start_points: None,
            zorder: None,
            integration_direction: IntegrationDirection::Both,
            broken_streamlines: true,
            integration_max_step_scale: 1.0,
            integration_max_error_scale: 1.0,
            num_arrows: 1,
            transform: None,
        }
    }
}

/// What `Axes::streamplot` returns (matplotlib's `StreamplotSet`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamplotSet {
    /// The streamlines, as one line collection.
    pub lines: Collection,
    /// The arrows, in streamline order.
    pub arrows: Vec<StreamArrow>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::meshgrid;

    #[test]
    fn meshes_reduce_to_their_axes() {
        let (xx, yy) = meshgrid(&[0.0, 1.0, 2.0], &[5.0, 6.0]);
        let zero = Grid::filled(2, 3, 0.0);
        let field = StreamField::from_mesh(&xx, &yy, zero.clone(), zero.clone()).unwrap();
        assert_eq!((field.x, field.y), (vec![0.0, 1.0, 2.0], vec![5.0, 6.0]));
        assert_eq!(
            StreamField::from_mesh(&yy, &yy, zero.clone(), zero.clone()),
            Err(StreamplotError::MeshRowsDiffer)
        );
        assert_eq!(
            StreamField::from_mesh(&xx, &xx, zero.clone(), zero),
            Err(StreamplotError::MeshColumnsDiffer)
        );
        let empty = Grid::filled(2, 0, 0.0);
        assert!(StreamField::from_mesh(&empty, &empty, empty.clone(), empty.clone()).is_ok());
    }
}
