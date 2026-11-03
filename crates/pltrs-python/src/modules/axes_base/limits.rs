//! The view bounds of an axes and how it autoscales (matplotlib's
//! `get_xbound`/`set_xbound`, `get_autoscalex_on`/`set_autoscalex_on`,
//! `autoscale`, `margins`, `relim`, `update_datalim`, `use_sticky_edges`),
//! which pltrs (Rust) keeps.

use pltrs::axes::Axes;

use super::axis_name;
use crate::binding::prelude::*;

/// The view bounds `(lower, upper)` of axis `name`, increasing
/// (`get_xbound`, `get_ybound`).
fn get_bound(axes: Axes, name: String) -> PyResult<(f64, f64)> {
    Ok(axes.bound(axis_name(&name)?))
}

/// Sets the view bounds of axis `name` keeping its orientation, `None`
/// keeping a bound (`set_xbound`, `set_ybound`).
fn set_bound(axes: Axes, name: String, lower: Option<f64>, upper: Option<f64>) -> PyResult<()> {
    axes.set_bound(axis_name(&name)?, lower, upper)?;
    Ok(())
}

/// Whether axis `name` (both for `None`) autoscales
/// (`get_autoscale_on`, `get_autoscalex_on`).
fn get_autoscale_on(axes: Axes, name: Option<String>) -> PyResult<bool> {
    let axis = name.as_deref().map(axis_name).transpose()?;
    Ok(axes.autoscale_on(axis))
}

/// Turns autoscaling of axis `name` (both for `None`) on or off
/// (`set_autoscale_on`, `set_autoscalex_on`).
fn set_autoscale_on(axes: Axes, name: Option<String>, on: bool) -> PyResult<()> {
    match name.as_deref().map(axis_name).transpose()? {
        Some(axis) => axes.set_axis_autoscale_on(axis, on),
        None => axes.set_autoscale_on(on),
    };
    Ok(())
}

/// matplotlib's `autoscale(enable, axis, tight)`, `axis` `None` for both.
fn autoscale(
    axes: Axes,
    enable: Option<bool>,
    name: Option<String>,
    tight: Option<bool>,
) -> PyResult<()> {
    let axis = name.as_deref().map(axis_name).transpose()?;
    axes.autoscale_with(enable, axis, tight);
    Ok(())
}

/// Autoscales the axes `scalex` and `scaley` ask for now
/// (`autoscale_view`), `tight` replacing the tight setting when given.
fn autoscale_view(axes: Axes, tight: Option<bool>, scalex: bool, scaley: bool) -> PyResult<()> {
    axes.autoscale_view_along(tight, (scalex, scaley));
    Ok(())
}

/// The margin of axis `name` (`get_xmargin`, `get_ymargin`).
fn get_margin(axes: Axes, name: String) -> PyResult<f64> {
    Ok(axes.margin(axis_name(&name)?))
}

/// Sets the margins given, `None` keeping one, and the tight setting
/// when `tight` is given (`margins`, `set_xmargin`, `set_ymargin`).
fn set_margins(axes: Axes, x: Option<f64>, y: Option<f64>, tight: Option<bool>) -> PyResult<()> {
    axes.set_margins_with(x, y, tight)?;
    Ok(())
}

/// Recomputes the data limits from the children (`relim`).
fn relim(axes: Axes, visible_only: bool) -> PyResult<()> {
    axes.relim(visible_only);
    Ok(())
}

/// Whether lines, patches, collections or images were added (`has_data`).
fn has_data(axes: Axes) -> PyResult<bool> {
    Ok(axes.has_data())
}

/// Extends the data limits to the points of the `(N, 2)` array `xys`, in
/// x when `updatex` and in y when `updatey` (`update_datalim`).
fn update_datalim(axes: Axes, xys: NdArray, updatex: bool, updatey: bool) -> PyResult<()> {
    axes.update_datalim(&crate::modules::transforms::points(&xys)?, updatex, updatey);
    Ok(())
}

/// Whether autoscaling stops at the sticky edges (`use_sticky_edges`).
fn get_use_sticky_edges(axes: Axes) -> PyResult<bool> {
    Ok(axes.use_sticky_edges())
}

/// Sets whether autoscaling stops at the sticky edges.
fn set_use_sticky_edges(axes: Axes, on: bool) -> PyResult<()> {
    axes.set_use_sticky_edges(on);
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_bound", get_bound)?;
    module.add("set_bound", set_bound)?;
    module.add("get_autoscale_on", get_autoscale_on)?;
    module.add("set_autoscale_on", set_autoscale_on)?;
    module.add("autoscale", autoscale)?;
    module.add("autoscale_view", autoscale_view)?;
    module.add("get_margin", get_margin)?;
    module.add("set_margins", set_margins)?;
    module.add("relim", relim)?;
    module.add("has_data", has_data)?;
    module.add("update_datalim", update_datalim)?;
    module.add("get_use_sticky_edges", get_use_sticky_edges)?;
    module.add("set_use_sticky_edges", set_use_sticky_edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn bounds_keep_the_orientation_and_autoscale_switches_by_axis() {
        let ax = Figure::new().subplot();
        ax.set_xlim(10.0, 0.0).unwrap();
        assert_eq!(get_bound(ax.clone(), "x".into()).unwrap(), (0.0, 10.0));
        set_bound(ax.clone(), "x".into(), Some(2.0), None).unwrap();
        assert_eq!(ax.xlim(), (10.0, 2.0));
        set_autoscale_on(ax.clone(), Some("y".into()), false).unwrap();
        assert!(!get_autoscale_on(ax.clone(), Some("y".into())).unwrap());
        assert!(!get_autoscale_on(ax.clone(), None).unwrap());
        assert!(set_margins(ax.clone(), Some(-1.0), None, None).is_err());
        set_margins(ax.clone(), None, Some(0.25), Some(false)).unwrap();
        assert_eq!(get_margin(ax, "y".into()).unwrap(), 0.25);
    }
}
