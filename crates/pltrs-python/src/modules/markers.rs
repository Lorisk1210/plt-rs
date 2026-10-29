//! `_native.markers`: marker shapes (matplotlib's `markers`), resolved by
//! `pltrs::markers`.
//!
//! A marker arrives as `(kind, value)`: `("code", "o")` or `("code", 4)`
//! for matplotlib's named markers, `("polygon", (sides, style, angle))`,
//! `("vertices", array)`, `("path", Path)` or `("math", "$...$")`. It
//! resolves into the paths and transforms matplotlib draws.

use std::str::FromStr;

use pltrs::markers::{FillStyle, Marker, MarkerError, MarkerStyle, PolygonStyle};
use pltrs_render::{CapStyle, JoinStyle};

use super::path::{PathArrays, PyPath, path_arrays};
use super::transforms::{matrix, points};
use crate::binding::prelude::*;

impl From<MarkerError> for PyErr {
    /// matplotlib's `ValueError` for unrecognized markers.
    fn from(err: MarkerError) -> Self {
        match err {
            MarkerError::Unrecognized(marker) => {
                PyErr::value_error(format!("Unrecognized marker style {marker}"))
            }
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The marker `(kind, value)` describes.
pub(super) fn marker_of_spec(spec: &Bound<'_>) -> PyResult<Marker> {
    let (kind, value): (String, Bound<'_>) = spec.extract()?;
    Ok(match kind.as_str() {
        "code" => match value.extract::<String>() {
            Ok(code) => Marker::from_str(&code)?,
            Err(_) => Marker::from_code(value.extract()?)?,
        },
        "polygon" => {
            let (sides, style, angle): (usize, i64, f64) = value.extract()?;
            let style = PolygonStyle::from_code(style).ok_or_else(|| {
                PyErr::value_error(format!("Unexpected tuple marker style {style}"))
            })?;
            Marker::polygon(sides, style, angle)
        }
        "vertices" => {
            let vertices: NdArray = value.extract()?;
            Marker::Vertices(points(&vertices)?)
        }
        "path" => Marker::Path(value.extract::<PyPath>()?.path),
        "math" => Marker::Math(value.extract()?),
        _ => return Err(PyErr::value_error(format!("no marker kind {kind:?}"))),
    })
}

/// A path and its transform's matrix.
type Shape = (PathArrays, NdArray);

/// `(path, transform)`, `(alt_path, alt_transform)` (`None` without an
/// alternate path), and `(snap_threshold, joinstyle, capstyle, filled)`.
type Resolved = (Shape, Option<Shape>, (Option<f64>, String, String, bool));

/// The marker `spec` resolved with the fill style `fillstyle` and the
/// user's cap and join styles (`None` for the marker's own).
fn resolve(
    spec: Owned,
    fillstyle: String,
    capstyle: Option<String>,
    joinstyle: Option<String>,
) -> PyResult<Resolved> {
    let marker = Python::with_gil(|py| marker_of_spec(&spec.bind(py)))?;
    let fillstyle = FillStyle::from_str(&fillstyle)?;
    let capstyle = capstyle
        .map(|name| {
            CapStyle::from_name(&name)
                .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid CapStyle")))
        })
        .transpose()?;
    let joinstyle = joinstyle
        .map(|name| {
            JoinStyle::from_name(&name)
                .ok_or_else(|| PyErr::value_error(format!("{name:?} is not a valid JoinStyle")))
        })
        .transpose()?;
    let style = MarkerStyle::with_styles(marker, fillstyle, capstyle, joinstyle)?;
    let alt = match (style.alt_path(), style.alt_transform()) {
        (Some(path), Some(transform)) => Some((path_arrays(path, false)?, matrix(transform)?)),
        _ => None,
    };
    Ok((
        (
            path_arrays(style.path(), false)?,
            matrix(style.transform())?,
        ),
        alt,
        (
            style.snap_threshold(),
            style.joinstyle().name().to_owned(),
            style.capstyle().name().to_owned(),
            style.is_filled(),
        ),
    ))
}

/// Whether `text` is mathtext (an even number of unescaped `$`s, at least
/// two): the markers drawn from text.
fn is_math_text(text: String) -> PyResult<bool> {
    Ok(pltrs_text::mathtext::is_math_text(&text))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("resolve", resolve)?;
    module.add("is_math_text", is_math_text)
}
