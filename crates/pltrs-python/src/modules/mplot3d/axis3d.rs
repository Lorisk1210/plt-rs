//! The axes of a 3-D axes (matplotlib's `mpl_toolkits.mplot3d.axis3d`):
//! their ticks and the edges their ticks and labels go to (their tick
//! helpers and the objects of their parts are `_native.axis`'s).

use pltrs::colors::Color;
use pltrs::mplot3d::{Axes3D, AxisPosition};

use super::super::lines::PyLineStyle;
use super::axis3;
use crate::binding::prelude::*;

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

/// Raises what computing the ticks of axis `name` meets, after the
/// runtime warnings its failure met, as matplotlib's measures of the axis
/// for a tight box raise it (`_update_ticks`).
fn check_ticks(axes: Axes3D, name: String) -> PyResult<()> {
    axes.axis(axis3(&name)?).ticks()?;
    Ok(())
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

/// Whether the line of axis `name` is drawn, its width in points and its
/// colour (`axis.line`).
fn line_state(axes: Axes3D, name: String) -> PyResult<(bool, f64, Color)> {
    let axis = axes.axis(axis3(&name)?);
    let (width, color) = axis.line_style();
    Ok((axis.line_visible(), width, color))
}

/// Shows or hides the line of axis `name` and draws it `width` points
/// wide in `color` (the setters of `axis.line`).
fn push_line(
    axes: Axes3D,
    name: String,
    (visible, width, color): (bool, f64, Color),
) -> PyResult<()> {
    axes.axis(axis3(&name)?)
        .set_line_visible(visible)
        .set_line_style(width, color);
    Ok(())
}

/// The coordinate the tick marks of axis `name` extend along on the edge
/// called `edge` (`_get_tickdir`).
fn tick_direction(axes: Axes3D, name: String, edge: String) -> PyResult<usize> {
    Ok(axes.axis(axis3(&name)?).tick_direction(position(&edge)?))
}

/// The corners of a pane, projected: x, y and depth each.
type PaneCorners = Vec<(f64, f64, f64)>;

/// The pane of axis `name` seen in the view of the last draw: its four
/// corners projected (x, y and depth) and its position along the axis;
/// `None` before the first draw (`active_pane`).
fn active_pane(axes: Axes3D, name: String) -> PyResult<Option<(PaneCorners, f64)>> {
    Ok(axes
        .axis(axis3(&name)?)
        .drawn_active_pane()
        .map(|(corners, at)| (corners.iter().map(|&[x, y, z]| (x, y, z)).collect(), at)))
}

/// The colour of the pane normal to axis `name`, as RGBA.
fn get_pane_color(axes: Axes3D, name: String) -> PyResult<(f64, f64, f64, f64)> {
    let color = axes
        .pane_color(axis3(&name)?)
        .resolve()
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok((color.r, color.g, color.b, color.a))
}

/// A gridline: its three points `((x, y), ...)` in projected
/// coordinates, its RGBA colour, width in points and line style.
type Gridline = (Vec<(f64, f64)>, (f64, f64, f64, f64), f64, PyLineStyle);

/// The gridlines of axis `name` the axes draws, in view, as the last
/// layout placed them (what `draw_grid` draws).
fn gridlines(axes: Axes3D, name: String) -> PyResult<Vec<Gridline>> {
    let lines = axes
        .axis(axis3(&name)?)
        .gridlines()
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    lines
        .into_iter()
        .filter(|line| line.in_view && line.visible)
        .map(|line| {
            let color = line
                .color
                .resolve()
                .map_err(|err| PyErr::value_error(err.to_string()))?;
            Ok((
                line.points.iter().map(|p| (p.x, p.y)).collect(),
                (color.r, color.g, color.b, color.a),
                line.width,
                PyLineStyle(line.linestyle),
            ))
        })
        .collect()
}

/// Whether the label of axis `name` follows the axis: `True`, `False`, or
/// `None` for labels longer than four characters (`set_rotate_label`).
fn set_rotate_label(axes: Axes3D, name: String, rotate: Option<bool>) -> PyResult<()> {
    axes.set_rotate_label(axis3(&name)?, rotate);
    Ok(())
}

/// Whether a label `text` of axis `name` would follow the axis
/// (`get_rotate_label(text)`).
fn get_rotate_label(axes: Axes3D, name: String, text: String) -> PyResult<bool> {
    Ok(axes.rotates_label(axis3(&name)?, &text))
}

/// The interval the data along axis `name` spans (`get_data_interval`).
fn get_data_interval(axes: Axes3D, name: String) -> PyResult<(f64, f64)> {
    Ok(axes.data_interval(axis3(&name)?))
}

/// Replaces the interval the data along axis `name` spans
/// (`set_data_interval(..., ignore=True)`).
fn set_data_interval(axes: Axes3D, name: String, (low, high): (f64, f64)) -> PyResult<()> {
    axes.set_data_interval(axis3(&name)?, low, high);
    Ok(())
}

/// Sets the view limits of axis `name` to `(low, high)` as they are, NaN
/// included (`set_view_interval`, which sets the limit box).
fn set_view_interval(axes: Axes3D, name: String, (low, high): (f64, f64)) -> PyResult<()> {
    axes.set_view_interval(axis3(&name)?, low, high);
    Ok(())
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
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticklocs", get_ticklocs)?;
    module.add("check_ticks", check_ticks)?;
    module.add("set_ticks_position", set_ticks_position)?;
    module.add("get_ticks_position", get_ticks_position)?;
    module.add("set_label_position", set_label_position)?;
    module.add("get_label_position", get_label_position)?;
    module.add("axis_line_data", axis_line_data)?;
    module.add("line_state", line_state)?;
    module.add("push_line", push_line)?;
    module.add("tick_direction", tick_direction)?;
    module.add("set_ticklabels", set_ticklabels)?;
    module.add("active_pane", active_pane)?;
    module.add("get_pane_color", get_pane_color)?;
    module.add("gridlines", gridlines)?;
    module.add("set_rotate_label", set_rotate_label)?;
    module.add("get_rotate_label", get_rotate_label)?;
    module.add("get_data_interval", get_data_interval)?;
    module.add("set_data_interval", set_data_interval)?;
    module.add("set_view_interval", set_view_interval)
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
