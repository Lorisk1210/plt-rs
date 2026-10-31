//! Tick parameters (matplotlib's `Axis.set_tick_params`): the keywords
//! `Axis._translate_tick_params` gives (`size`, `tickdir`, `tick1On`,
//! `labelrotation`, `grid_color`, ...) as pltrs (Rust)'s [`TickParams`].

use pltrs::axes::{AxisName, GridOptions, TickLevel, TickParams};
use pltrs::fonts::FontSize;
use pltrs::mplot3d::{Axis3, Axis3D};

use super::owner::{AnyAxis, AxisOwner, not_on_3d};
use crate::binding::dict_items;
use crate::binding::prelude::*;
use crate::modules::lines::{PyLineStyle, parse};
use crate::modules::patheffects::path_effects;

/// A font size given as points or as a keyword (`"large"`).
fn font_size(value: &Bound<'_>) -> PyResult<FontSize> {
    match value.extract::<f64>() {
        Ok(points) => Ok(FontSize::Points(points)),
        Err(_) => parse(value),
    }
}

/// Sets `params.labelrotation` from matplotlib's `labelrotation`: an
/// angle, `"auto"`, `"default"` or `(mode, angle)`.
fn label_rotation(params: &mut TickParams, value: &Bound<'_>) -> PyResult<()> {
    let (mode, angle) = if let Ok(angle) = value.extract::<f64>() {
        ("default".to_owned(), angle)
    } else if let Ok(mode) = value.extract::<String>() {
        (mode, 0.0)
    } else {
        value.extract::<(String, f64)>()?
    };
    params.labelrotation_auto = Some(match mode.as_str() {
        "auto" => true,
        "default" => false,
        _ => {
            return Err(PyErr::value_error(format!(
                "{mode:?} is not a valid value for labelrotation; supported values are \
                 'auto', 'default'"
            )));
        }
    });
    params.labelrotation = Some(angle);
    Ok(())
}

/// Sets the side switch `key` (`tick1On`, `label2On`, ...) of axis
/// `name`.
fn side_switch(params: &mut TickParams, name: AxisName, key: &str, on: bool) {
    let slot = match (key, name) {
        ("tick1On", AxisName::X) => &mut params.bottom,
        ("tick2On", AxisName::X) => &mut params.top,
        ("label1On", AxisName::X) => &mut params.labelbottom,
        ("label2On", AxisName::X) => &mut params.labeltop,
        ("tick1On", AxisName::Y) => &mut params.left,
        ("tick2On", AxisName::Y) => &mut params.right,
        ("label1On", AxisName::Y) => &mut params.labelleft,
        _ => &mut params.labelright,
    };
    *slot = Some(on);
}

/// The tick parameters of the translated keywords `kwargs` for axis
/// `name`, and the gridline switch (`gridOn`) when given.
fn tick_params(name: AxisName, kwargs: &Bound<'_>) -> PyResult<(TickParams, Option<bool>)> {
    let mut params = TickParams::default();
    let mut grid_on = None;
    for (key, value) in dict_items(kwargs)? {
        let key: String = key.extract()?;
        match key.as_str() {
            "size" => params.length = Some(value.extract()?),
            "width" => params.width = Some(value.extract()?),
            "pad" => params.pad = Some(value.extract()?),
            "color" => params.color = Some(value.extract()?),
            "labelcolor" => params.labelcolor = Some(value.extract()?),
            "tickdir" => params.direction = Some(parse(&value)?),
            "labelsize" => params.labelsize = Some(font_size(&value)?),
            "labelfontfamily" => params.labelfontfamily = Some(value.extract()?),
            "labelrotation" => label_rotation(&mut params, &value)?,
            "labelrotation_mode" => params.labelrotation_mode = Some(parse(&value)?),
            "gridOn" => grid_on = Some(value.is_truthy()?),
            "tick1On" | "tick2On" | "label1On" | "label2On" => {
                side_switch(&mut params, name, &key, value.is_truthy()?);
            }
            "grid_color" | "grid_c" => params.grid_color = Some(value.extract()?),
            "grid_alpha" => params.grid_alpha = Some(value.extract()?),
            "grid_linewidth" | "grid_lw" => params.grid_linewidth = Some(value.extract()?),
            "grid_linestyle" | "grid_ls" => {
                params.grid_linestyle = Some(value.extract::<PyLineStyle>()?.0);
            }
            "grid_path_effects" => params.grid_path_effects = Some(path_effects(&value)?),
            _ => {
                return Err(PyErr::new(
                    ExceptionKind::NotImplementedError,
                    format!("pltrs: tick_params({key}=...)"),
                ));
            }
        }
    }
    Ok((params, grid_on))
}

/// Sets the tick parameters `kwargs` (translated keywords) of the major,
/// minor or both (`which`) tick levels of axis `name`, after restoring
/// the rc defaults when `reset` (`Axis.set_tick_params`).
fn set_tick_params(
    owner: AxisOwner,
    name: String,
    which: String,
    reset: bool,
    kwargs: Owned,
) -> PyResult<()> {
    let which: TickLevel =
        which
            .parse()
            .map_err(|err: <TickLevel as std::str::FromStr>::Err| {
                PyErr::value_error(err.to_string())
            })?;
    let axis = match owner.axis(&name)? {
        AnyAxis::Flat(axis) => axis,
        AnyAxis::Deep(axis) => return set_deep_tick_params(&axis, which, reset, &kwargs),
    };
    let (params, grid_on) = Python::with_gil(|py| tick_params(axis.name(), &kwargs.bind(py)))?;
    axis.set_tick_params(TickParams {
        which,
        reset,
        ..params
    });
    if let Some(visible) = grid_on {
        axis.grid(
            visible,
            GridOptions {
                which,
                ..GridOptions::default()
            },
        );
    }
    Ok(())
}

/// [`set_tick_params`] on an axis of a 3-D axes, whose x axis takes the
/// switches of an x axis and whose y and z axes those of a y axis, as
/// `Axes3D.tick_params` routes them; its gridlines are the axes' own.
fn set_deep_tick_params(
    axis: &Axis3D,
    which: TickLevel,
    reset: bool,
    kwargs: &Owned,
) -> PyResult<()> {
    let sides = if axis.dir() == Axis3::X {
        AxisName::X
    } else {
        AxisName::Y
    };
    let (params, grid_on) = Python::with_gil(|py| tick_params(sides, &kwargs.bind(py)))?;
    if grid_on.is_some() {
        return Err(not_on_3d("tick_params(gridOn=...)"));
    }
    axis.set_tick_params(TickParams {
        which,
        reset,
        ..params
    });
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_tick_params", set_tick_params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::new_dict;
    use crate::binding::testing::with_python;
    use pltrs::figure::Figure;

    #[test]
    fn translated_keywords_become_tick_params() {
        let ax = Figure::new().subplot();
        let kwargs = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("size", 7.0).unwrap();
            kwargs.set_item("tick2On", true).unwrap();
            kwargs.set_item("gridOn", true).unwrap();
            kwargs.unbind()
        });
        set_tick_params(
            AxisOwner::Flat(ax.clone()),
            "x".into(),
            "major".into(),
            false,
            kwargs,
        )
        .unwrap();
        let style = ax.xaxis().tick_style(false);
        assert_eq!(style.size, 7.0);
        assert!(style.ticks.second && style.grid_on);
        let unknown = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("zorder", 3.0).unwrap();
            kwargs.unbind()
        });
        assert!(
            set_tick_params(
                AxisOwner::Flat(ax),
                "y".into(),
                "both".into(),
                false,
                unknown
            )
            .is_err()
        );
    }
}
