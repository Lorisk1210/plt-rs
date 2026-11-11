//! `_native.axes_base`: what an axes is apart from its plotting methods
//! (matplotlib's `axes._base`): its position, the axes it shares an axis
//! with, its twins.

use pltrs::axes::{AspectError, Axes, AxisName, PropCycle, WhichPosition};
use pltrs::colors::Color;
use pltrs::mplot3d::SharedAxes;
use pltrs::rc::{Cycler, CyclerError};

use crate::binding::prelude::*;

mod across;
pub(super) mod aspect;
mod decorations;
mod limits;
mod twins;

impl From<CyclerError> for PyErr {
    fn from(err: CyclerError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<AspectError> for PyErr {
    /// matplotlib's `ValueError`s: that of `set_aspect` (which `imshow` and
    /// `spy` call) for a ratio that is not positive, with its trailing
    /// space; for axes fixing their data ratio, that of
    /// `set_adjustable('datalim')`, which every path reaches.
    fn from(err: AspectError) -> Self {
        match err {
            AspectError::NotPositive(_) => {
                PyErr::value_error("aspect must be finite and positive ")
            }
            AspectError::FixedDataRatio(_) => PyErr::value_error(
                "Cannot set Axes adjustable to 'datalim' for Axes which override 'get_data_ratio'",
            ),
            AspectError::NonFiniteLimits | AspectError::BoxNotPositive => {
                PyErr::value_error(err.to_string())
            }
        }
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

/// `(left, bottom, right, top)` of the axes in figure fractions: as
/// drawn, or before any aspect adjustment when `original`
/// (`get_position`, which keeps the edges as they are, as matplotlib's
/// `Bbox` does).
fn position(axes: Axes, original: bool) -> PyResult<(f64, f64, f64, f64)> {
    let bbox = if original {
        axes.original_position()
    } else {
        axes.try_position()?
    };
    Ok(bbox.extents())
}

/// The active position where the axes was last placed, its aspect not
/// applied first (`get_position` of an axes with a locator).
fn located_position(axes: Axes) -> PyResult<(f64, f64, f64, f64)> {
    Ok(axes.located_position().extents())
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

/// Shares axis `name` of `axes` with `other`, an axes of either kind
/// (`sharex`, `sharey`).
fn share(axes: Axes, name: String, other: SharedAxes) -> PyResult<()> {
    axes.share_with(axis_name(&name)?, &other)?;
    Ok(())
}

/// The axes, of either kind, whose axis `name` `axes` shares (`_sharex`,
/// `_sharey`).
fn shared_from(axes: Axes, name: String) -> PyResult<Option<SharedAxes>> {
    Ok(axes.shared_from(axis_name(&name)?))
}

/// The axes of either kind sharing axis `name` with `axes`, itself
/// included (`get_shared_x_axes().get_siblings(ax)`).
fn shared(axes: Axes, name: String) -> PyResult<Vec<SharedAxes>> {
    Ok(axes.sharing(axis_name(&name)?))
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

/// The property cycle called `kind`: `"lines"` (`_get_lines`) or
/// `"patches"` (`_get_patches_for_fill`).
fn prop_cycle(kind: &str) -> PyResult<PropCycle> {
    match kind {
        "lines" => Ok(PropCycle::Lines),
        "patches" => Ok(PropCycle::Patches),
        other => Err(PyErr::value_error(format!("no property cycle {other:?}"))),
    }
}

/// Makes `axes` draw from the property cycle `kind` that `other` draws
/// from (`axes._get_lines = other._get_lines`).
fn share_prop_cycle(axes: Axes, kind: String, other: Axes) -> PyResult<()> {
    axes.share_prop_cycle(prop_cycle(&kind)?, &other);
    Ok(())
}

/// The next colour of the property cycle `kind`
/// (`_get_lines.get_next_color()`).
fn next_cycle_color(axes: Axes, kind: String) -> PyResult<Color> {
    Ok(match prop_cycle(&kind)? {
        PropCycle::Lines => axes.next_line_color(),
        PropCycle::Patches => axes.next_patch_color(),
    })
}

/// The entry the property cycle `kind` gives next
/// (`_get_lines._prop_cycle._idx`).
fn cycle_position(axes: Axes, kind: String) -> PyResult<usize> {
    Ok(axes.prop_cycle_position(prop_cycle(&kind)?))
}

/// Makes the property cycle `kind` give its entry `position` next
/// (`_get_lines._prop_cycle._idx = position`).
fn set_cycle_position(axes: Axes, kind: String, position: usize) -> PyResult<()> {
    axes.set_prop_cycle_position(prop_cycle(&kind)?, position);
    Ok(())
}

/// Replaces the cycler of the property cycle `kind` alone, given as rc
/// text, `None` for `axes.prop_cycle`'s (`_get_lines.set_prop_cycle`).
fn set_prop_cycle_of(axes: Axes, kind: String, cycle: Option<String>) -> PyResult<()> {
    let cycle = cycle.map(|text| text.parse::<Cycler>()).transpose()?;
    axes.set_prop_cycle_of(prop_cycle(&kind)?, cycle);
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

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("position", position)?;
    module.add("located_position", located_position)?;
    module.add("set_position", set_position)?;
    module.add("share", share)?;
    module.add("shared_from", shared_from)?;
    module.add("shared", shared)?;
    module.add("label_outer", label_outer)?;
    module.add("label_outer_shared", label_outer_shared)?;
    module.add("set_prop_cycle", set_prop_cycle)?;
    module.add("share_prop_cycle", share_prop_cycle)?;
    module.add("next_cycle_color", next_cycle_color)?;
    module.add("cycle_position", cycle_position)?;
    module.add("set_cycle_position", set_cycle_position)?;
    module.add("set_prop_cycle_of", set_prop_cycle_of)?;
    module.add("get_scale", get_scale)?;
    across::register(module)?;
    decorations::register(module)?;
    limits::register(module)?;
    twins::register(module)?;
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
        share(right.clone(), "y".into(), left.clone().into()).unwrap();
        assert_eq!(shared(left.clone(), "y".into()).unwrap().len(), 2);
        assert_eq!(
            shared(left.clone(), "x".into()).unwrap(),
            [SharedAxes::Flat(left.clone())]
        );
        let twin = left.twinx().unwrap();
        assert!(
            shared(left.clone(), "x".into())
                .unwrap()
                .contains(&SharedAxes::Flat(twin))
        );
        let deep = fig.subplot_3d();
        share(right.clone(), "x".into(), deep.clone().into()).unwrap();
        assert_eq!(
            shared_from(right.clone(), "x".into()).unwrap(),
            Some(SharedAxes::Deep(deep))
        );
        assert!(axis_name("z").is_err());
        set_position(left.clone(), (0.1, 0.2, 0.3, 0.4), "both".into()).unwrap();
        let (x0, y0, x1, y1) = position(left, true).unwrap();
        assert!((x0 - 0.1).abs() + (y0 - 0.2).abs() + (x1 - 0.4).abs() + (y1 - 0.6).abs() < 1e-12);
    }
}
