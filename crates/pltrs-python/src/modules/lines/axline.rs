//! Infinite lines (matplotlib's `AxLine`, which `axline` makes): their
//! points or slope.

use pltrs::lines::{AxLineError, Line2D};
use pltrs_core::Point;

use crate::binding::prelude::*;

impl From<AxLineError> for PyErr {
    /// matplotlib's `ValueError`s for a second point given to a line fixed
    /// by its slope, and the other way round.
    fn from(err: AxLineError) -> Self {
        PyErr::value_error(match err {
            AxLineError::NotInfinite => err.to_string(),
            AxLineError::SlopeIsSet | AxLineError::PointIsSet => {
                format!("{err}; they differ but their functionalities overlap")
            }
        })
    }
}

/// A point as Python gives it.
type Pair = (f64, f64);

/// A point as a pair.
fn pair(point: Point) -> Pair {
    (point.x, point.y)
}

/// `(xy1, xy2, slope)` of an infinite line, `xy2` or `slope` `None` as
/// the line was given the other.
type AxLineFields = (Option<Pair>, Option<Pair>, Option<f64>);

/// The points and slope of an infinite line (see [`AxLineFields`]).
fn axline(line: Line2D) -> PyResult<AxLineFields> {
    Ok((
        line.axline_xy1().map(pair),
        line.axline_xy2().map(pair),
        line.axline_slope(),
    ))
}

/// Moves the first point of an infinite line.
fn set_axline_xy1(line: Line2D, (x, y): Pair) -> PyResult<()> {
    line.set_axline_xy1(Point::new(x, y))?;
    Ok(())
}

/// Moves the second point of an infinite line given one.
fn set_axline_xy2(line: Line2D, (x, y): Pair) -> PyResult<()> {
    line.set_axline_xy2(Point::new(x, y))?;
    Ok(())
}

/// Changes the slope of an infinite line given one.
fn set_axline_slope(line: Line2D, slope: f64) -> PyResult<()> {
    line.set_axline_slope(slope)?;
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("axline", axline)?;
    module.add("set_axline_xy1", set_axline_xy1)?;
    module.add("set_axline_xy2", set_axline_xy2)?;
    module.add("set_axline_slope", set_axline_slope)
}
