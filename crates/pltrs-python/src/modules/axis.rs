//! `_native.axis`: the x and y axes of an axes (matplotlib's `axis`):
//! their ticks, tick labels, label and direction. A Python `Axis` reaches
//! its pltrs (Rust) axis through its axes and its name (`"x"`, `"y"`).

use pltrs::axes::{Axes, Axis, TickPosition};
use pltrs::spines::SpineSide;

use super::spines::side_name;
use crate::binding::prelude::*;

/// The axis `name` (`"x"` or `"y"`) of `axes`.
fn axis(axes: &Axes, name: &str) -> PyResult<Axis> {
    match name {
        "x" => Ok(axes.xaxis()),
        "y" => Ok(axes.yaxis()),
        _ => Err(PyErr::value_error(format!("no axis {name:?}"))),
    }
}

/// The side called `name` (`"left"`, `"right"`, `"bottom"`, `"top"`).
fn side(name: &str) -> PyResult<SpineSide> {
    SpineSide::from_name(name).ok_or_else(|| PyErr::value_error(format!("no side {name:?}")))
}

/// The tick locations, major or `minor` (`get_ticklocs`).
fn get_ticklocs(axes: Axes, name: String, minor: bool) -> PyResult<NdArray> {
    Ok(NdArray::from_vec(axis(&axes, &name)?.ticklocs(minor)?))
}

/// Puts the major or `minor` ticks at `ticks`, widening the view to show
/// them (`set_ticks`).
fn set_ticks(axes: Axes, name: String, ticks: Vec<f64>, minor: bool) -> PyResult<()> {
    let axis = axis(&axes, &name)?;
    if minor {
        axis.set_minor_ticks(ticks);
    } else {
        axis.set_ticks(ticks);
    }
    Ok(())
}

/// The tick label strings, major or `minor`.
fn get_ticklabel_texts(axes: Axes, name: String, minor: bool) -> PyResult<Vec<String>> {
    Ok(axis(&axes, &name)?.ticklabels(minor)?)
}

/// Labels the major or `minor` ticks with `labels` (`set_ticklabels`).
fn set_ticklabels(axes: Axes, name: String, labels: Vec<String>, minor: bool) -> PyResult<()> {
    let axis = axis(&axes, &name)?;
    if minor {
        axis.set_minor_ticklabels(labels)?;
    } else {
        axis.set_ticklabels(labels)?;
    }
    Ok(())
}

/// Where the ticks are: a side, `"default"` or `"unknown"`
/// (`get_ticks_position`).
fn get_ticks_position(axes: Axes, name: String) -> PyResult<String> {
    Ok(match axis(&axes, &name)?.ticks_position() {
        Some(TickPosition::Side(side)) => side_name(side).to_owned(),
        Some(TickPosition::Default) => "default".to_owned(),
        _ => "unknown".to_owned(),
    })
}

/// Puts the ticks on a side, on `"both"`, `"none"` or back to
/// `"default"` (`set_ticks_position`).
fn set_ticks_position(axes: Axes, name: String, position: String) -> PyResult<()> {
    let position = match position.as_str() {
        "both" => TickPosition::Both,
        "none" => TickPosition::None,
        "default" => TickPosition::Default,
        other => TickPosition::Side(side(other)?),
    };
    axis(&axes, &name)?.set_ticks_position(position)?;
    Ok(())
}

/// The side of the axis label (`get_label_position`).
fn get_label_position(axes: Axes, name: String) -> PyResult<String> {
    Ok(side_name(axis(&axes, &name)?.label_position()).to_owned())
}

/// Moves the axis label to a side (`set_label_position`).
fn set_label_position(axes: Axes, name: String, position: String) -> PyResult<()> {
    axis(&axes, &name)?.set_label_position(side(&position)?)?;
    Ok(())
}

/// Whether the axis runs from high to low values (`get_inverted`).
fn get_inverted(axes: Axes, name: String) -> PyResult<bool> {
    Ok(axis(&axes, &name)?.inverted())
}

/// Turns the axis around (`set_inverted`).
fn set_inverted(axes: Axes, name: String, inverted: bool) -> PyResult<()> {
    axis(&axes, &name)?.set_inverted(inverted);
    Ok(())
}

/// The tick keywords matplotlib keeps per tick level (`_major_tick_kw`):
/// `(tick1On, tick2On, label1On, label2On, gridOn)`.
fn tick_switches(
    axes: Axes,
    name: String,
    minor: bool,
) -> PyResult<(bool, bool, bool, bool, bool)> {
    let style = axis(&axes, &name)?.tick_style(minor);
    Ok((
        style.ticks.first,
        style.ticks.second,
        style.labels.first,
        style.labels.second,
        style.grid_on,
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_ticklocs", get_ticklocs)?;
    module.add("set_ticks", set_ticks)?;
    module.add("get_ticklabel_texts", get_ticklabel_texts)?;
    module.add("set_ticklabels", set_ticklabels)?;
    module.add("get_ticks_position", get_ticks_position)?;
    module.add("set_ticks_position", set_ticks_position)?;
    module.add("get_label_position", get_label_position)?;
    module.add("set_label_position", set_label_position)?;
    module.add("get_inverted", get_inverted)?;
    module.add("set_inverted", set_inverted)?;
    module.add("tick_switches", tick_switches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn ticks_and_sides_cross_by_name() {
        let ax = Figure::new().subplot();
        set_ticks(ax.clone(), "x".into(), vec![0.0, 0.5, 2.0], false).unwrap();
        let locs = get_ticklocs(ax.clone(), "x".into(), false).unwrap();
        assert_eq!(locs.into_vec().unwrap(), [0.0, 0.5, 2.0]);
        set_ticks_position(ax.clone(), "x".into(), "top".into()).unwrap();
        assert_eq!(get_ticks_position(ax.clone(), "x".into()).unwrap(), "top");
        assert!(set_ticks_position(ax.clone(), "x".into(), "left".into()).is_err());
        set_label_position(ax.clone(), "y".into(), "right".into()).unwrap();
        assert_eq!(get_label_position(ax.clone(), "y".into()).unwrap(), "right");
        set_inverted(ax.clone(), "y".into(), true).unwrap();
        assert!(get_inverted(ax, "y".into()).unwrap());
        assert!(side("middle").is_err());
    }
}
