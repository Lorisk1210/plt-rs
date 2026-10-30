//! The limits of a 3-D axes: view limits and bounds, autoscaling, margins,
//! scales and inversion (matplotlib's `set_xlim3d`, `set_zbound`,
//! `autoscale`, `margins`, `set_zscale`, `invert_zaxis`, ...).

use pltrs::mplot3d::{Axes3D, Axis3};
use pltrs::scale::{FuncTransform, ScaleOptions};

use super::super::scale::nonpositive;
use super::axis3;
use crate::binding::dict_items;
use crate::binding::prelude::*;

/// The view limits of axis `name` (`get_xlim3d`, ...).
fn get_lim(axes: Axes3D, name: String) -> PyResult<(f64, f64)> {
    Ok(axes.limits(axis3(&name)?))
}

/// Sets the view limits of axis `name`, widened by `view_margin` times
/// their span (`None`: as `axes3d.automargin` says); `auto` turns
/// autoscaling on or off, `None` keeping it as it was (`set_xlim3d`, ...).
fn set_lim(
    axes: Axes3D,
    name: String,
    (low, high): (f64, f64),
    view_margin: Option<f64>,
    auto: Option<bool>,
) -> PyResult<()> {
    let axis = axis3(&name)?;
    let before = axes.autoscale_on(axis);
    axes.set_limits_with_margin(axis, low, high, view_margin)?;
    axes.set_axis_autoscale_on(axis, auto.unwrap_or(before));
    Ok(())
}

/// The bounds of axis `name`, increasing (`get_xbound`, ...).
fn get_bound(axes: Axes3D, name: String) -> PyResult<(f64, f64)> {
    Ok(axes.bound(axis3(&name)?))
}

/// Sets the bounds of axis `name` keeping its orientation, `None` keeping
/// a bound, widened by `view_margin` (`set_xbound`, ...).
fn set_bound(
    axes: Axes3D,
    name: String,
    (lower, upper): (Option<f64>, Option<f64>),
    view_margin: Option<f64>,
) -> PyResult<()> {
    axes.set_bound_with_margin(axis3(&name)?, lower, upper, view_margin)?;
    Ok(())
}

/// The padding of autoscaled limits as a fraction of their span
/// (`_view_margin`).
fn get_view_margin(axes: Axes3D) -> PyResult<f64> {
    Ok(axes.view_margin())
}

/// Sets the padding of autoscaled limits (`_view_margin`).
fn set_view_margin(axes: Axes3D, margin: f64) -> PyResult<()> {
    axes.set_view_margin(margin)?;
    Ok(())
}

/// Whether axis `name` autoscales (`get_autoscalez_on`, ...).
fn get_autoscale_on(axes: Axes3D, name: String) -> PyResult<bool> {
    Ok(axes.autoscale_on(axis3(&name)?))
}

/// Turns autoscaling of axis `name` on or off (`set_autoscalez_on`).
fn set_autoscale_on(axes: Axes3D, name: String, on: bool) -> PyResult<()> {
    axes.set_axis_autoscale_on(axis3(&name)?, on);
    Ok(())
}

/// matplotlib's `autoscale(enable, axis, tight)`: `axis` names one axis
/// or, for `None`, all three.
fn autoscale(
    axes: Axes3D,
    enable: Option<bool>,
    name: Option<String>,
    tight: Option<bool>,
) -> PyResult<()> {
    let selected = match name {
        Some(name) => {
            let axis = axis3(&name)?;
            Axis3::ALL.map(|a| a == axis)
        }
        None => [true; 3],
    };
    axes.autoscale(enable, selected, tight)?;
    Ok(())
}

/// The autoscaling margin of axis `name` (`get_zmargin`, ...).
fn get_margin(axes: Axes3D, name: String) -> PyResult<f64> {
    Ok(axes.margin(axis3(&name)?))
}

/// Sets the margins given, `None` keeping one, and autoscales those axes;
/// `tight` replaces the tight setting (`margins`, `set_zmargin`).
fn set_margins(
    axes: Axes3D,
    (x, y, z): (Option<f64>, Option<f64>, Option<f64>),
    tight: Option<bool>,
) -> PyResult<()> {
    axes.set_margins([x, y, z], tight)?;
    Ok(())
}

/// The options of a scale from its keyword arguments, which the Python
/// side checked against the scale's class.
fn scale_options(kwargs: &Bound<'_>) -> PyResult<ScaleOptions> {
    let mut options = ScaleOptions::default();
    for (key, value) in dict_items(kwargs)? {
        let key: String = key.extract()?;
        match key.as_str() {
            "base" => options.base = Some(value.extract()?),
            "subs" => options.subs = value.extract()?,
            "nonpositive" => options.nonpositive = Some(nonpositive(&value.extract::<String>()?)?),
            "linthresh" => options.linthresh = Some(value.extract()?),
            "linscale" => options.linscale = Some(value.extract()?),
            "linear_width" => options.linear_width = Some(value.extract()?),
            "one_half" => options.one_half = Some(value.extract()?),
            "use_overline" => options.use_overline = Some(value.extract()?),
            "functions" => {
                let (forward, inverse): (Callback, Callback) = value.extract()?;
                options.functions = Some(FuncTransform::new(
                    move |v| forward.call::<f64>((v,)).unwrap_or(f64::NAN),
                    move |v| inverse.call::<f64>((v,)).unwrap_or(f64::NAN),
                ));
            }
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::NotImplementedError,
                    format!("pltrs: set_xscale({key}=...) on 3-D axes"),
                ));
            }
        }
    }
    Ok(options)
}

/// Gives axis `name` the scale called `scale` with the options `kwargs`
/// (`set_xscale`, `set_yscale`, `set_zscale`).
fn set_scale(axes: Axes3D, name: String, scale: String, kwargs: Owned) -> PyResult<()> {
    let options = Python::with_gil(|py| scale_options(&kwargs.bind(py)))?;
    axes.set_scale_with(axis3(&name)?, scale, options)?;
    Ok(())
}

/// The name of the scale of axis `name` (`get_zscale`, ...).
fn get_scale(axes: Axes3D, name: String) -> PyResult<String> {
    Ok(axes.scale(axis3(&name)?).to_string())
}

/// Whether axis `name` is inverted (`zaxis_inverted`, ...).
fn get_inverted(axes: Axes3D, name: String) -> PyResult<bool> {
    Ok(axes.inverted(axis3(&name)?))
}

/// Inverts axis `name` or not (`set_inverted`).
fn set_inverted(axes: Axes3D, name: String, inverted: bool) -> PyResult<()> {
    axes.set_inverted(axis3(&name)?, inverted);
    Ok(())
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_lim", get_lim)?;
    module.add("set_lim", set_lim)?;
    module.add("get_bound", get_bound)?;
    module.add("set_bound", set_bound)?;
    module.add("get_view_margin", get_view_margin)?;
    module.add("set_view_margin", set_view_margin)?;
    module.add("get_autoscale_on", get_autoscale_on)?;
    module.add("set_autoscale_on", set_autoscale_on)?;
    module.add("autoscale", autoscale)?;
    module.add("get_margin", get_margin)?;
    module.add("set_margins", set_margins)?;
    module.add("set_scale", set_scale)?;
    module.add("get_scale", get_scale)?;
    module.add("get_inverted", get_inverted)?;
    module.add("set_inverted", set_inverted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pltrs::figure::Figure;

    #[test]
    fn limits_keep_or_set_the_autoscale_switch() {
        let ax = Figure::new().subplot_3d();
        set_lim(ax.clone(), "z".into(), (0.0, 2.0), Some(0.0), None).unwrap();
        assert_eq!(get_lim(ax.clone(), "z".into()).unwrap(), (0.0, 2.0));
        assert!(get_autoscale_on(ax.clone(), "z".into()).unwrap());
        set_lim(ax.clone(), "x".into(), (2.0, 1.0), Some(0.0), Some(false)).unwrap();
        assert!(!get_autoscale_on(ax.clone(), "x".into()).unwrap());
        assert!(get_inverted(ax.clone(), "x".into()).unwrap());
        assert_eq!(get_bound(ax.clone(), "x".into()).unwrap(), (1.0, 2.0));
        assert!(set_margins(ax.clone(), (Some(-1.0), None, None), None).is_err());
        set_margins(ax.clone(), (None, Some(0.25), None), Some(true)).unwrap();
        assert_eq!(get_margin(ax.clone(), "y".into()).unwrap(), 0.25);
        assert_eq!(get_scale(ax, "y".into()).unwrap(), "linear");
    }
}
