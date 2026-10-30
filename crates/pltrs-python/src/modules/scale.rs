//! `_native.scale`: the maps of the non-linear scales (matplotlib's
//! `scale`), computed by `pltrs::scale`.
//!
//! A map is named by its kind and parameters; values cross as an array of
//! any shape, mapped element by element, forwards or back. Setting the
//! scale of an axis is in [`axis_scale`].

mod axis_scale;

use pltrs::scale::{
    AsinhTransform, LogTransform, LogitTransform, NonPositive, ScaleError, SymmetricalLogTransform,
};
use pltrs::transforms::AxisMap;

use crate::binding::prelude::*;

impl From<ScaleError> for PyErr {
    /// matplotlib's `ValueError` for invalid scale parameters.
    fn from(err: ScaleError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// matplotlib's `nonpositive`: `"clip"` or `"mask"`.
pub(super) fn nonpositive(name: &str) -> PyResult<NonPositive> {
    match name {
        "clip" => Ok(NonPositive::Clip),
        "mask" => Ok(NonPositive::Mask),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for nonpositive; supported values are 'clip', 'mask'"
        ))),
    }
}

/// The map of `kind` with `params`: `"log"` `(base, nonpositive)`,
/// `"symlog"` `(base, linthresh, linscale)`, `"logit"` `(nonpositive,)`,
/// `"asinh"` `(linear_width,)`.
fn map(kind: &str, params: &Bound<'_>) -> PyResult<Box<dyn AxisMap>> {
    Ok(match kind {
        "log" => {
            let (base, clip): (f64, String) = params.extract()?;
            Box::new(LogTransform::new(base, nonpositive(&clip)?)?)
        }
        "symlog" => {
            let (base, linthresh, linscale): (f64, f64, f64) = params.extract()?;
            Box::new(SymmetricalLogTransform::new(base, linthresh, linscale)?)
        }
        "logit" => {
            let (clip,): (String,) = params.extract()?;
            Box::new(LogitTransform::new(nonpositive(&clip)?))
        }
        "asinh" => {
            let (linear_width,): (f64,) = params.extract()?;
            Box::new(AsinhTransform::new(linear_width)?)
        }
        _ => return Err(PyErr::value_error(format!("no scale map named {kind:?}"))),
    })
}

/// `values` mapped by the map `kind` of `params`, or by its inverse.
fn transform(kind: String, params: Owned, values: NdArray, inverse: bool) -> PyResult<NdArray> {
    let map = Python::with_gil(|py| map(&kind, &params.bind(py)))?;
    let mapped = values
        .data
        .iter()
        .map(|&v| {
            if inverse {
                map.inverse(v)
            } else {
                map.forward(v)
            }
        })
        .collect();
    NdArray::with_shape(mapped, values.shape)
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("transform", transform)?;
    axis_scale::register(module)
}
