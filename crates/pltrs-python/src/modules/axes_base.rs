//! `_native.axes_base`: what an axes is apart from its plotting methods
//! (matplotlib's `axes._base`): its position, the axes it shares an axis
//! with, its twins.

use pltrs::axes::{AspectError, Axes, AxisName, WhichPosition};
use pltrs::colors::Color;
use pltrs::rc::{Cycler, CyclerError};

use crate::binding::prelude::*;

mod aspect;
mod decorations;
mod limits;

impl From<CyclerError> for PyErr {
    fn from(err: CyclerError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<AspectError> for PyErr {
    fn from(err: AspectError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The axis called `name` (`"x"` or `"y"`).
fn axis_name(name: &str) -> PyResult<AxisName> {
    match name {
        "x" => Ok(AxisName::X),
        "y" => Ok(AxisName::Y),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for name; supported values are 'x', 'y'"
        ))),
    }
}

/// `(left, bottom, width, height)` of the axes in figure fractions: as
/// drawn, or before any aspect adjustment when `original`
/// (`get_position`).
fn position(axes: Axes, original: bool) -> PyResult<(f64, f64, f64, f64)> {
    let bbox = if original {
        axes.original_position()
    } else {
        axes.position()
    };
    Ok(bbox.bounds())
}

/// Moves the axes to `rect`, `[left, bottom, width, height]` in figure
/// fractions, setting the position `which` names (`"both"`, `"active"`,
/// `"original"`; `set_position`).
fn set_position(axes: Axes, rect: (f64, f64, f64, f64), which: String) -> PyResult<()> {
    let which = match which.as_str() {
        "both" => WhichPosition::Both,
        "active" => WhichPosition::Active,
        "original" => WhichPosition::Original,
        _ => {
            return Err(PyErr::value_error(format!(
                "{which:?} is not a valid value for which; supported values are 'both', \
                 'active', 'original'"
            )));
        }
    };
    let (left, bottom, width, height) = rect;
    axes.set_position_which([left, bottom, width, height], which);
    Ok(())
}

/// Shares axis `name` of `axes` with `other` (`sharex`, `sharey`).
fn share(axes: Axes, name: String, other: Axes) -> PyResult<()> {
    match axis_name(&name)? {
        AxisName::X => axes.share_x(&other)?,
        AxisName::Y => axes.share_y(&other)?,
    };
    Ok(())
}

/// The axes sharing axis `name` with `axes`, itself included
/// (`get_shared_x_axes().get_siblings(ax)`).
fn shared(axes: Axes, name: String) -> PyResult<Vec<Axes>> {
    Ok(axes.shared_axes(axis_name(&name)?))
}

/// A twin of `axes` sharing axis `name`, the other axis on the far side
/// (`twinx` for `"x"`, `twiny` for `"y"`).
fn twin(axes: Axes, name: String) -> PyResult<Axes> {
    Ok(match axis_name(&name)? {
        AxisName::X => axes.twinx()?,
        AxisName::Y => axes.twiny()?,
    })
}

/// Only shows the outer labels and tick labels of the axes' grid, and the
/// outer ticks too unless `remove_inner_ticks` is false (`label_outer`).
fn label_outer(axes: Axes, remove_inner_ticks: bool) -> PyResult<()> {
    axes.label_outer_with(remove_inner_ticks);
    Ok(())
}

/// Hides the inner labels of axis `name` of grid axes that share it,
/// unless the axes are not rectangular (`_label_outer_xaxis(
/// skip_non_rectangular_axes=True)`).
fn label_outer_shared(axes: Axes, name: String) -> PyResult<()> {
    axes.label_outer_shared(axis_name(&name)?);
    Ok(())
}

/// Replaces the property cycle with the cycler written `cycle` (as
/// `repr` gives a `Cycler`), or restores `axes.prop_cycle` for `None`
/// (`set_prop_cycle`).
fn set_prop_cycle(axes: Axes, cycle: Option<String>) -> PyResult<()> {
    let cycle = cycle.map(|text| text.parse::<Cycler>()).transpose()?;
    axes.set_prop_cycle(cycle);
    Ok(())
}

/// Gives axis `name` the built-in scale called `scale` with its default
/// options (`set_xscale`, `set_yscale`).
fn set_scale(axes: Axes, name: String, scale: String) -> PyResult<()> {
    match axis_name(&name)? {
        AxisName::X => axes.set_xscale(scale.as_str())?,
        AxisName::Y => axes.set_yscale(scale.as_str())?,
    };
    Ok(())
}

/// The name of the scale of axis `name` (`get_xscale`, `get_yscale`).
fn get_scale(axes: Axes, name: String) -> PyResult<String> {
    Ok(match axis_name(&name)? {
        AxisName::X => axes.xscale(),
        AxisName::Y => axes.yscale(),
    }
    .to_string())
}

/// Sets the colour of the axes background (`set_facecolor`).
fn set_facecolor(axes: Axes, color: Color) -> PyResult<()> {
    axes.set_facecolor(color);
    Ok(())
}

/// The colour of the axes background as an RGBA tuple.
fn get_facecolor(axes: Axes) -> PyResult<(f64, f64, f64, f64)> {
    let color = axes
        .facecolor()
        .resolve()
        .map_err(|err| PyErr::value_error(err.to_string()))?;
    Ok((color.r, color.g, color.b, color.a))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("position", position)?;
    module.add("set_position", set_position)?;
    module.add("share", share)?;
    module.add("shared", shared)?;
    module.add("twin", twin)?;
    module.add("label_outer", label_outer)?;
    module.add("label_outer_shared", label_outer_shared)?;
    module.add("set_prop_cycle", set_prop_cycle)?;
    module.add("set_scale", set_scale)?;
    module.add("get_scale", get_scale)?;
    module.add("set_facecolor", set_facecolor)?;
    module.add("get_facecolor", get_facecolor)?;
    decorations::register(module)?;
    limits::register(module)?;
    aspect::register(module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn axes_share_and_twin_by_axis_name() {
        let fig = Figure::new();
        let (left, right) = (
            fig.add_subplot(1, 2, 1).unwrap(),
            fig.add_subplot(1, 2, 2).unwrap(),
        );
        share(right.clone(), "y".into(), left.clone()).unwrap();
        assert_eq!(shared(left.clone(), "y".into()).unwrap().len(), 2);
        assert_eq!(
            shared(left.clone(), "x".into()).unwrap(),
            std::slice::from_ref(&left)
        );
        let twin = twin(left.clone(), "x".into()).unwrap();
        assert!(shared(left.clone(), "x".into()).unwrap().contains(&twin));
        assert!(axis_name("z").is_err());
        set_position(left.clone(), (0.1, 0.2, 0.3, 0.4), "both".into()).unwrap();
        let (x, y, w, h) = position(left, true).unwrap();
        assert!((x - 0.1).abs() + (y - 0.2).abs() + (w - 0.3).abs() + (h - 0.4).abs() < 1e-12);
    }
}
