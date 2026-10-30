//! The axes of a 3-D axes (matplotlib's `mpl_toolkits.mplot3d.axis3d`):
//! their formatters, ticks, and the edges their ticks and labels go to.

use pltrs::mplot3d::{Axes3D, AxisPosition};

use super::axis3;
use crate::binding::prelude::*;

/// Labels the `level` (`"major"` or `"minor"`) ticks of axis `name` with
/// the Python formatter `formatter`, called as `formatter(value,
/// position)` (`set_major_formatter`, `set_minor_formatter`).
fn set_formatter(axes: Axes3D, name: String, level: String, formatter: Owned) -> PyResult<()> {
    let axis = axes.axis(axis3(&name)?);
    let formatter =
        Python::with_gil(|py| crate::modules::archive::tick_formatter(&formatter.bind(py)))?;
    match level.as_str() {
        "major" => axis.set_major_formatter(formatter),
        "minor" => axis.set_minor_formatter(formatter),
        _ => return Err(PyErr::value_error(format!("no tick level named {level:?}"))),
    };
    Ok(())
}

/// Puts the `level` ticks of axis `name` at `ticks` (`set_ticks`).
fn set_ticks(axes: Axes3D, name: String, ticks: Vec<f64>, minor: bool) -> PyResult<()> {
    let axis = axes.axis(axis3(&name)?);
    if minor {
        axis.set_minor_ticks(ticks);
    } else {
        axis.set_ticks(ticks);
    }
    Ok(())
}

/// The major (or `minor`) tick locations of axis `name` (`get_ticklocs`).
fn get_ticklocs(axes: Axes3D, name: String, minor: bool) -> PyResult<Vec<f64>> {
    axes.axis(axis3(&name)?)
        .ticklocs(minor)
        .map_err(|err| PyErr::value_error(err.to_string()))
}

/// The edge called `name`: `"lower"`, `"upper"`, `"both"`, `"default"` or
/// `"none"`.
fn position(name: &str) -> PyResult<AxisPosition> {
    match name {
        "lower" => Ok(AxisPosition::Lower),
        "upper" => Ok(AxisPosition::Upper),
        "both" => Ok(AxisPosition::Both),
        "default" => Ok(AxisPosition::Default),
        "none" => Ok(AxisPosition::None),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for position; supported values are 'lower', \
             'upper', 'both', 'default', 'none'"
        ))),
    }
}

/// The name of `position`.
fn position_name(position: AxisPosition) -> &'static str {
    match position {
        AxisPosition::Lower => "lower",
        AxisPosition::Upper => "upper",
        AxisPosition::Both => "both",
        AxisPosition::Default => "default",
        AxisPosition::None => "none",
    }
}

/// Puts the ticks of axis `name` on the edge called `edge`
/// (`set_ticks_position`).
fn set_ticks_position(axes: Axes3D, name: String, edge: String) -> PyResult<()> {
    axes.axis(axis3(&name)?)
        .set_ticks_position(position(&edge)?);
    Ok(())
}

/// The edge the ticks of axis `name` are on (`get_ticks_position`).
fn get_ticks_position(axes: Axes3D, name: String) -> PyResult<&'static str> {
    Ok(position_name(axes.axis(axis3(&name)?).ticks_position()))
}

/// Puts the label of axis `name` by the edge called `edge`
/// (`set_label_position`).
fn set_label_position(axes: Axes3D, name: String, edge: String) -> PyResult<()> {
    axes.axis(axis3(&name)?)
        .set_label_position(position(&edge)?);
    Ok(())
}

/// The edge the label of axis `name` is by (`get_label_position`).
fn get_label_position(axes: Axes3D, name: String) -> PyResult<&'static str> {
    Ok(position_name(axes.axis(axis3(&name)?).label_position()))
}

/// The x and y coordinates of the two ends of a line.
type LineEnds = ((f64, f64), (f64, f64));

/// The projected end points of the line of axis `name`, as x and y
/// coordinates; `None` for the tick position `none` (`axis.line`).
fn axis_line_data(axes: Axes3D, name: String) -> PyResult<Option<LineEnds>> {
    Ok(axes
        .axis(axis3(&name)?)
        .line_data()
        .map(|([x0, x1], [y0, y1])| ((x0, x1), (y0, y1))))
}

/// Shows or hides the line of axis `name` (`axis.line.set_visible`).
fn set_line_visible(axes: Axes3D, name: String, visible: bool) -> PyResult<()> {
    axes.axis(axis3(&name)?).set_line_visible(visible);
    Ok(())
}

/// Whether the line of axis `name` is drawn (`axis.line.get_visible`).
fn get_line_visible(axes: Axes3D, name: String) -> PyResult<bool> {
    Ok(axes.axis(axis3(&name)?).line_visible())
}

/// The coordinate the tick marks of axis `name` extend along on the edge
/// called `edge` (`_get_tickdir`).
fn tick_direction(axes: Axes3D, name: String, edge: String) -> PyResult<usize> {
    Ok(axes.axis(axis3(&name)?).tick_direction(position(&edge)?))
}

/// Labels the major (or `minor`) ticks of axis `name` with `labels`
/// (`set_ticklabels`).
fn set_ticklabels(axes: Axes3D, name: String, labels: Vec<String>, minor: bool) -> PyResult<()> {
    let axis = axes.axis(axis3(&name)?);
    if minor {
        axis.set_minor_ticklabels(labels)?;
    } else {
        axis.set_ticklabels(labels)?;
    }
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_formatter", set_formatter)?;
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticklocs", get_ticklocs)?;
    module.add("set_ticks_position", set_ticks_position)?;
    module.add("get_ticks_position", get_ticks_position)?;
    module.add("set_label_position", set_label_position)?;
    module.add("get_label_position", get_label_position)?;
    module.add("axis_line_data", axis_line_data)?;
    module.add("set_line_visible", set_line_visible)?;
    module.add("get_line_visible", get_line_visible)?;
    module.add("tick_direction", tick_direction)?;
    module.add("set_ticklabels", set_ticklabels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn ticks_and_edges_round_trip() {
        let ax = Figure::new().subplot_3d();
        set_ticks(ax.clone(), "z".into(), vec![0.0, 0.5, 1.0], false).unwrap();
        assert_eq!(
            get_ticklocs(ax.clone(), "z".into(), false).unwrap(),
            [0.0, 0.5, 1.0]
        );
        set_ticks_position(ax.clone(), "x".into(), "upper".into()).unwrap();
        assert_eq!(get_ticks_position(ax.clone(), "x".into()).unwrap(), "upper");
        set_label_position(ax.clone(), "y".into(), "both".into()).unwrap();
        assert_eq!(get_label_position(ax.clone(), "y".into()).unwrap(), "both");
        assert!(position("sideways").is_err());
    }
}
