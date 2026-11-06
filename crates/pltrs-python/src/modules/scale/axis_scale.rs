//! Setting the scale of an axis (matplotlib's `Axis._set_axes_scale`):
//! one of pltrs (Rust)'s scales with matplotlib's keyword arguments, or a
//! scale of the program's (a `ScaleBase` subclass registered with
//! `register_scale`), whose transform and range limits pltrs (Rust) calls.

use std::rc::Rc;

use pltrs::axes::{Axes, AxisName};
use pltrs::scale::{
    AxisTickers, FuncTransform, LinearScale, Scale, ScaleName, ScaleOptions, scale_factory,
};
use pltrs::ticker::{ScaleHint, TickerSettings};
use pltrs::transforms::AxisMap;

use super::super::archive::func_transform;
use super::nonpositive;
use crate::binding::dict_items;
use crate::binding::prelude::*;

/// The axis `name` (`"x"` or `"y"`).
fn axis_name(name: &str) -> PyResult<AxisName> {
    match name {
        "x" => Ok(AxisName::X),
        "y" => Ok(AxisName::Y),
        _ => Err(PyErr::value_error(format!("no axis {name:?}"))),
    }
}

/// The options of a scale from the keyword arguments the Python scale
/// was made with (`base`, `subs`, `nonpositive`, `linthresh`,
/// `linscale`, `linear_width`, `one_half`, `use_overline`, `functions`).
pub(crate) fn options(kwargs: &Bound<'_>) -> PyResult<ScaleOptions> {
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
                let (forward, inverse): (Bound<'_>, Bound<'_>) = value.extract()?;
                options.functions = Some(func_transform(&forward, &inverse)?);
            }
            _ => {
                return Err(PyErr::type_error(format!(
                    "unexpected scale option {key:?}"
                )));
            }
        }
    }
    Ok(options)
}

/// Gives axis `name` of `axes` (and the axes sharing it) pltrs (Rust)'s
/// scale `scale` with the options `kwargs`.
fn set_scale(axes: Axes, name: String, scale: String, kwargs: Owned) -> PyResult<()> {
    let options = Python::with_gil(|py| options(&kwargs.bind(py)))?;
    axes.set_scale(axis_name(&name)?, scale_factory(scale.as_str(), options)?);
    Ok(())
}

/// A scale of the program's: its name, its transform's map and its range
/// limits, which pltrs (Rust) asks it for; its tick helpers are those the
/// Python scale then installs.
#[derive(Debug)]
struct PyScale {
    name: String,
    map: Rc<FuncTransform>,
    limit_range: Callback,
}

impl Scale for PyScale {
    fn name(&self) -> ScaleName {
        ScaleName::Custom(self.name.clone())
    }

    fn axis_map(&self) -> Option<Rc<dyn AxisMap>> {
        Some(Rc::clone(&self.map) as Rc<dyn AxisMap>)
    }

    fn limit_range_for_scale(&self, vmin: f64, vmax: f64, minpos: f64) -> (f64, f64) {
        self.limit_range
            .call((vmin, vmax, minpos))
            .unwrap_or((vmin, vmax))
    }

    fn hint(&self) -> ScaleHint {
        ScaleHint::Other
    }

    fn default_tickers(&self, axis: AxisName, settings: &TickerSettings) -> AxisTickers {
        LinearScale.default_tickers(axis, settings)
    }
}

/// Gives axis `name` of `axes` (and the axes sharing it) the Python scale
/// called `scale`, mapping through `forward` and `inverse` (called with
/// arrays) and limiting ranges with `limit_range(vmin, vmax, minpos)`.
fn set_python_scale(
    axes: Axes,
    name: String,
    scale: String,
    (forward, inverse): (Owned, Owned),
    limit_range: Callback,
) -> PyResult<()> {
    axes.set_scale(
        axis_name(&name)?,
        python_scale(scale, (forward, inverse), limit_range)?,
    );
    Ok(())
}

/// The scale of the program's called `scale`, mapping through `forward`
/// and `inverse` (called with arrays) and limiting ranges with
/// `limit_range(vmin, vmax, minpos)`.
pub(crate) fn python_scale(
    scale: String,
    (forward, inverse): (Owned, Owned),
    limit_range: Callback,
) -> PyResult<Box<dyn Scale>> {
    let map = Python::with_gil(|py| func_transform(&forward.bind(py), &inverse.bind(py)))?;
    Ok(Box::new(PyScale {
        name: scale,
        map: Rc::new(map),
        limit_range,
    }))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_scale", set_scale)?;
    module.add("set_python_scale", set_python_scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binding::new_dict;
    use crate::binding::testing::{eval, with_python};
    use pltrs::figure::Figure;

    #[test]
    fn scales_take_their_options_and_python_scales_their_maps() {
        let ax = Figure::new().subplot();
        let kwargs = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("base", 2.0).unwrap();
            kwargs.unbind()
        });
        set_scale(ax.clone(), "x".into(), "log".into(), kwargs).unwrap();
        assert_eq!(ax.xscale(), ScaleName::Log);
        let unknown = with_python(|py| {
            let kwargs = new_dict(py).unwrap();
            kwargs.set_item("foo", 1.0).unwrap();
            kwargs.unbind()
        });
        assert!(set_scale(ax.clone(), "y".into(), "log".into(), unknown).is_err());
        let (forward, inverse, limit) = with_python(|py| {
            let object = |expression: &str| eval(py, "", expression).unbind();
            (
                object("lambda x: 2 * x"),
                object("lambda x: x / 2"),
                eval(py, "", "lambda vmin, vmax, minpos: (vmin, vmax)")
                    .extract::<Callback>()
                    .unwrap(),
            )
        });
        set_python_scale(
            ax.clone(),
            "y".into(),
            "double".into(),
            (forward, inverse),
            limit,
        )
        .unwrap();
        assert_eq!(ax.yscale(), ScaleName::Custom("double".into()));
    }
}
