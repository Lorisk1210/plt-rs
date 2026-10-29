//! `_native.ticker`: tick locating and formatting (matplotlib's
//! `ticker`).
//!
//! Formatters show how Python code runs inside a plt-rs draw: any Python
//! formatter (a `Formatter` subclass, a `FuncFormatter`) becomes a plt-rs
//! `FuncFormatter` holding it as a [`Callback`], which plt-rs calls with
//! the GIL each time it labels ticks; an exception the formatter raises
//! surfaces from the native call that drew (`savefig`), and the tick
//! labels of that draw stay empty.

use plt::axes::{Axes, Axis};
use plt::ticker::FuncFormatter;

use crate::binding::prelude::*;

/// The axis `name` (`"x"` or `"y"`) of `axes`.
fn axis(axes: &Axes, name: &str) -> PyResult<Axis> {
    match name {
        "x" => Ok(axes.xaxis()),
        "y" => Ok(axes.yaxis()),
        _ => Err(PyErr::value_error(format!("no axis named {name:?}"))),
    }
}

/// Formats the `level` (`"major"` or `"minor"`) ticks of the axis `name`
/// with the Python formatter `formatter`, called as `formatter(value,
/// position)` (`set_major_formatter` and `set_minor_formatter`).
fn set_formatter(axes: Axes, name: String, level: String, formatter: Callback) -> PyResult<()> {
    let axis = axis(&axes, &name)?;
    let formatter = FuncFormatter::new(move |value, position| {
        formatter
            .call::<String>((value, position))
            .unwrap_or_default()
    });
    match level.as_str() {
        "major" => axis.set_major_formatter(formatter),
        "minor" => axis.set_minor_formatter(formatter),
        _ => return Err(PyErr::value_error(format!("no tick level named {level:?}"))),
    };
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_formatter", set_formatter)
}
