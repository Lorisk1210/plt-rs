//! `_native.colors`: colour specifications (matplotlib's `colors`).
//!
//! A colour converts from what matplotlib accepts as one: a string (a
//! name, a hex code, a grey level, `"Cn"`, `"none"`) or a sequence of 3
//! or 4 numbers; it converts back as the string it was given, or as an
//! RGBA tuple.

use plt::colors::{Color, ColorError, Rgba};

use crate::binding::prelude::*;

impl From<ColorError> for PyErr {
    /// matplotlib's `ValueError` for invalid colours.
    fn from(err: ColorError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl FromPy<'_> for Color {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(spec) = object.extract::<String>() {
            return Ok(Color::from(spec));
        }
        let components: Vec<f64> = object.extract().map_err(|_| {
            PyErr::value_error(format!(
                "{} is not a valid color value.",
                object.repr().unwrap_or_default()
            ))
        })?;
        match components[..] {
            [r, g, b] => Ok(Color::from([r, g, b])),
            [r, g, b, a] => Ok(Color::from([r, g, b, a])),
            _ => Err(PyErr::value_error(format!(
                "{} is not a valid color value.",
                object.repr().unwrap_or_default()
            ))),
        }
    }
}

impl IntoPy for Color {
    /// The string a colour was given as, else its RGBA tuple.
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Color::Spec(spec) => spec.into_py(py),
            Color::Rgba(rgba) => rgba_tuple(rgba).into_py(py),
        }
    }
}

/// `(r, g, b, a)`.
fn rgba_tuple(rgba: Rgba) -> (f64, f64, f64, f64) {
    (rgba.r, rgba.g, rgba.b, rgba.a)
}

/// `to_rgba(c, alpha)`: the RGBA tuple of a colour, with `alpha`
/// replacing its own when given.
fn to_rgba(color: Color, alpha: Option<f64>) -> PyResult<(f64, f64, f64, f64)> {
    Ok(rgba_tuple(color.resolve_alpha(alpha)?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("to_rgba", to_rgba)
}
