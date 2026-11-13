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
use crate::binding::prelude::*;
use crate::binding::{dict_items, lookup};

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
/// Python scale then installs. It keeps the Python scale, which a pickle
/// of its figure holds in its place.
#[derive(Debug)]
pub(crate) struct PyScale {
    name: String,
    map: Rc<FuncTransform>,
    limit_range: Callback,
    object: Owned,
    three_d: bool,
}

impl PyScale {
    /// The Python scale.
    pub(crate) fn object(&self) -> &Owned {
        &self.object
    }

    /// Whether it scales an axis of 3-D axes.
    pub(crate) fn three_d(&self) -> bool {
        self.three_d
    }
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
/// `scale` (see [`python_scale`]).
fn set_python_scale(axes: Axes, name: String, scale: Owned) -> PyResult<()> {
    let scale = Python::with_gil(|py| python_scale(&scale.bind(py), false))?;
    axes.set_scale(axis_name(&name)?, scale);
    Ok(())
}

/// The Python scale `scale` of the program's, of an axis of 3-D axes if
/// `three_d`: its name, the forward and inverse functions it maps through
/// (called with arrays) and its range limits
/// (`limit_range(vmin, vmax, minpos)`), as the registered
/// `program_scale(scale, three_d)` gives them.
pub(crate) fn python_scale(scale: &Bound<'_>, three_d: bool) -> PyResult<Box<dyn Scale>> {
    let py = scale.py();
    let parts = lookup(py, "program_scale")?.call(&[scale.clone(), three_d.into_py(py)?], &[])?;
    let (name, (forward, inverse), limit_range): (String, (Bound<'_>, Bound<'_>), Callback) =
        parts.extract()?;
    Ok(Box::new(PyScale {
        name,
        map: Rc::new(func_transform(&forward, &inverse)?),
        limit_range,
        object: scale.clone().unbind(),
        three_d,
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
    use crate::binding::testing::{eval, with_python};
    use crate::binding::{new_dict, register};
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
        let scale = with_python(|py| {
            let hook = eval(
                py,
                "",
                "lambda scale, three_d: ('double', (lambda x: 2 * x, lambda x: x / 2), \
                 lambda vmin, vmax, minpos: (vmin, vmax))",
            );
            register("program_scale".into(), hook.unbind()).unwrap();
            eval(py, "", "object()").unbind()
        });
        set_python_scale(ax.clone(), "y".into(), scale).unwrap();
        assert_eq!(ax.yscale(), ScaleName::Custom("double".into()));
    }
}
