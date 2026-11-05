//! `_native.axes`: axes and their plotting methods (matplotlib's
//! `axes`).

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Axes, AxisName, Limits, PlotError};
use pltrs::lines::Line2D;
use pltrs::patches::{Patch, Rectangle};

use super::lines::LineKwargs;
use crate::binding::prelude::*;
use crate::binding::{retyped, wrap_artist};

mod callbacks;
mod children;
mod titles;

/// The native object of any axes converts to it, and that of a 3-D axes
/// to its projection plane: matplotlib's `Axes3D` is an `Axes` whose 2-D
/// side draws in projected coordinates (`artist_handle!` otherwise).
impl FromPy<'_> for Axes {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        match object.extract::<Native<Axes>>() {
            Ok(native) => Ok(native.borrow().clone()),
            Err(mismatch) => match object.extract::<pltrs::mplot3d::Axes3D>() {
                Ok(axes3d) => Ok(axes3d.projection_plane()),
                Err(_) => retyped(object, mismatch),
            },
        }
    }
}

impl IntoPy for Axes {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        wrap_artist(py, self)
    }
}

impl From<PlotError> for PyErr {
    /// matplotlib's `ValueError` for data it cannot plot (with its message
    /// for limits, which names no axis), and the errors statistics raise
    /// for their problems.
    fn from(err: PlotError) -> Self {
        match err {
            PlotError::Stats(err) => err.into(),
            PlotError::NonFiniteLimit(_) => PyErr::value_error("Axis limits cannot be NaN or Inf"),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The x-axis label.
fn get_xlabel(axes: Axes) -> PyResult<String> {
    Ok(axes.xaxis().label_text())
}

/// The y-axis label.
fn get_ylabel(axes: Axes) -> PyResult<String> {
    Ok(axes.yaxis().label_text())
}

/// The x limits.
fn get_xlim(axes: Axes) -> PyResult<(f64, f64)> {
    Ok(axes.xlim())
}

/// Sets the view limits of the axis `name` (`"x"` or `"y"`) to `(low,
/// high)`, switching its autoscaling to `auto` when given; with `emit`,
/// the axes sharing the axis follow and the limit callbacks run
/// (matplotlib's `Axis._set_lim`). Returns the limits as set.
fn set_lim(
    axes: Axes,
    name: String,
    (low, high): (f64, f64),
    auto: Option<bool>,
    emit: bool,
) -> PyResult<(f64, f64)> {
    let axis = match name.as_str() {
        "x" => AxisName::X,
        "y" => AxisName::Y,
        _ => {
            return Err(PyErr::value_error(format!(
                "{name:?} is not a valid value for name; supported values are 'x', 'y'"
            )));
        }
    };
    let limits = Limits {
        low: Some(low),
        high: Some(high),
        auto,
    };
    Ok(if emit {
        axes.set_limits_with(axis, limits)?
    } else {
        axes.set_limits_quietly(axis, limits)?
    })
}

/// The y limits.
fn get_ylim(axes: Axes) -> PyResult<(f64, f64)> {
    Ok(axes.ylim())
}

/// The lines of the axes (`ax.lines`).
fn get_lines(axes: Axes) -> PyResult<Vec<Line2D>> {
    Ok(axes.lines())
}

/// A horizontal (`orientation` `"h"`) or vertical (`"v"`) line across the
/// axes at data position `at`, from axes fraction `start` to `end`
/// (`axhline`, `axvline`).
fn axline(
    axes: Axes,
    orientation: String,
    at: f64,
    (start, end): (f64, f64),
    kwargs: LineKwargs,
) -> PyResult<Line2D> {
    let props = kwargs.over_format(None)?;
    Ok(match orientation.as_str() {
        "h" => axes.axhline_between(at, start, end, props),
        _ => axes.axvline_between(at, start, end, props),
    })
}

/// A horizontal (`orientation` `"h"`) or vertical (`"v"`) band across the
/// axes between data positions `(low, high)`, from axes fraction `start`
/// to `end` (`axhspan`, `axvspan`).
fn axspan(
    axes: Axes,
    orientation: String,
    (low, high): (f64, f64),
    (start, end): (f64, f64),
) -> PyResult<Patch<Rectangle>> {
    Ok(match orientation.as_str() {
        "h" => axes.axhspan_between(low, high, start, end),
        _ => axes.axvspan_between(low, high, start, end),
    })
}

/// The data coordinates of the display point `(x, y)`
/// (`ax.transData.inverted().transform((x, y))`); `ValueError` when the
/// data transform cannot be inverted.
fn display_to_data(axes: Axes, x: f64, y: f64) -> PyResult<(f64, f64)> {
    let figure = axes.figure();
    let to_display = figure
        .resolve_transform_as_is(&axes.trans_data())
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    let to_data = to_display
        .inverted()
        .ok_or_else(|| PyErr::value_error("the data transform of the axes is singular"))?;
    let point = to_data.transform_point(pltrs::transforms::Point::new(x, y));
    Ok((point.x, point.y))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    titles::register(module)?;
    children::register(module)?;
    callbacks::register(module)?;
    module.add("display_to_data", display_to_data)?;
    module.add("axline", axline)?;
    module.add("axspan", axspan)?;
    module.add("get_xlabel", get_xlabel)?;
    module.add("get_ylabel", get_ylabel)?;
    module.add("get_xlim", get_xlim)?;
    module.add("set_lim", set_lim)?;
    module.add("get_ylim", get_ylim)?;
    module.add("get_lines", get_lines)
}
