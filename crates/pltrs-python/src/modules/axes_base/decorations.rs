//! What an axes draws around its data (matplotlib's `set_axis_off`,
//! `axis('equal')` and the other presets, `set_frame_on`) and its
//! background patch (`Axes.patch`: visibility, edge, z-order, gid).

use pltrs::axes::{Axes, AxisMode};

use crate::binding::prelude::*;

/// Shows (`on`) or hides the patch, spines and axes (`set_axis_on`,
/// `set_axis_off`).
fn set_axis_on(axes: Axes, on: bool) -> PyResult<()> {
    if on {
        axes.set_axis_on();
    } else {
        axes.set_axis_off();
    }
    Ok(())
}

/// Whether the patch, spines and axes are drawn (`axison`).
fn get_axis_on(axes: Axes) -> PyResult<bool> {
    Ok(axes.axis_on())
}

/// Applies the preset `mode` of `axis(mode)`: `"on"`, `"off"`,
/// `"equal"`, `"scaled"`, `"tight"`, `"auto"`, `"image"` or `"square"`.
fn axis_mode(axes: Axes, mode: String) -> PyResult<()> {
    let mode = match mode.as_str() {
        "on" => AxisMode::On,
        "off" => AxisMode::Off,
        "equal" => AxisMode::Equal,
        "scaled" => AxisMode::Scaled,
        "tight" => AxisMode::Tight,
        "auto" => AxisMode::Auto,
        "image" => AxisMode::Image,
        "square" => AxisMode::Square,
        _ => {
            return Err(PyErr::value_error(format!(
                "Unrecognized string '{mode}' to axis; try 'on' or 'off'"
            )));
        }
    };
    axes.axis_mode(mode)?;
    Ok(())
}

/// Shows or hides the background patch and the spines (`set_frame_on`).
fn set_frame_on(axes: Axes, on: bool) -> PyResult<()> {
    axes.set_frame_on(on);
    Ok(())
}

/// Whether the background patch and the spines are drawn
/// (`get_frame_on`).
fn get_frame_on(axes: Axes) -> PyResult<bool> {
    Ok(axes.frame_on())
}

/// Rasterizes the children at or below `zorder` together in vector
/// output; `None` keeps them vector (`set_rasterization_zorder`).
fn set_rasterization_zorder(axes: Axes, zorder: Option<f64>) -> PyResult<()> {
    axes.set_rasterization_zorder(zorder);
    Ok(())
}

/// The z-order up to which children are rasterized together, if any
/// (`get_rasterization_zorder`).
fn get_rasterization_zorder(axes: Axes) -> PyResult<Option<f64>> {
    Ok(axes.rasterization_zorder())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_rasterization_zorder", set_rasterization_zorder)?;
    module.add("get_rasterization_zorder", get_rasterization_zorder)?;
    module.add("set_axis_on", set_axis_on)?;
    module.add("get_axis_on", get_axis_on)?;
    module.add("axis_mode", axis_mode)?;
    module.add("set_frame_on", set_frame_on)?;
    module.add("get_frame_on", get_frame_on)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn axis_presets_switch_the_decorations() {
        let ax = Figure::new().subplot();
        axis_mode(ax.clone(), "off".into()).unwrap();
        assert!(!get_axis_on(ax.clone()).unwrap());
        set_axis_on(ax.clone(), true).unwrap();
        assert!(get_axis_on(ax.clone()).unwrap());
        assert!(axis_mode(ax, "sideways".into()).is_err());
    }
}
