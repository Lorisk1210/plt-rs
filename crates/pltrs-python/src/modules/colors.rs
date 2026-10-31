//! `_native.colors`: colour specifications (matplotlib's `colors`).
//!
//! A colour converts from what matplotlib accepts as one: a string (a
//! name, a hex code, a grey level, `"Cn"`, `"none"`) or a sequence of 3
//! or 4 numbers; it converts back as the string it was given, or as an
//! RGBA tuple.

pub(crate) mod mapping;
mod python_norm;
pub(crate) use python_norm::PythonNorm;

use pltrs::colors::multivar::{BivarColormap, BivarShape, MultivarError};
use pltrs::colors::{
    Channel, Color, ColorError, ColormapError, Rgba, SegmentProblem, color_sequence,
    color_sequence_names, create_lookup_table, hsv_to_rgb, rgb_to_hsv,
};
use pltrs::numeric::Grid;

use crate::binding::prelude::*;

impl From<MultivarError> for PyErr {
    /// matplotlib's `ValueError` for invalid multivariate colormaps.
    fn from(err: MultivarError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

impl From<ColorError> for PyErr {
    /// matplotlib's `ValueError` for invalid colours.
    fn from(err: ColorError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// `object` with a one-row 2D array flattened, as matplotlib's `to_rgba`
/// takes an array of shape `(1, 3)` or `(1, 4)` for its row.
fn single_row<'py>(object: &Bound<'py>) -> PyResult<Bound<'py>> {
    let shape = object
        .getattr("shape")
        .and_then(|shape| shape.extract::<Vec<i64>>());
    match shape.as_deref() {
        Ok([1, _]) => object.call_method("reshape", &[(-1_i64).into_py(object.py())?]),
        _ => Ok(object.clone()),
    }
}

impl FromPy<'_> for Color {
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        if let Ok(spec) = object.extract::<String>() {
            return Ok(Color::from(spec));
        }
        let components: Vec<f64> = single_row(object)?.extract().map_err(|_| {
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

impl From<ColormapError> for PyErr {
    /// matplotlib's `ValueError`s for malformed segment data.
    fn from(err: ColormapError) -> Self {
        match err {
            ColormapError::Segment {
                problem: SegmentProblem::Endpoints,
                ..
            } => PyErr::value_error("data mapping points must start with x=0 and end with x=1"),
            ColormapError::Segment {
                problem: SegmentProblem::Decreasing,
                ..
            } => PyErr::value_error("data mapping points must have x in increasing order"),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// The RGBA rows (`(N, 4)`) of the colours `colors`, with `alpha`
/// replacing theirs when given.
fn to_rgba_array(colors: Vec<Color>, alpha: Option<f64>) -> PyResult<NdArray> {
    let rows: Vec<f64> = colors
        .into_iter()
        .map(|color| color.resolve_alpha(alpha).map(rgba_tuple))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|(r, g, b, a)| [r, g, b, a])
        .collect();
    let count = rows.len() / 4;
    NdArray::with_shape(rows, vec![count, 4])
}

/// The triples of an array whose last axis has length 3.
fn triples(array: &NdArray) -> PyResult<Vec<[f64; 3]>> {
    if array.shape.last() != Some(&3) {
        return Err(PyErr::value_error(format!(
            "Last dimension of input array must be 3; shape {:?} was found.",
            array.shape
        )));
    }
    Ok(array.data.as_chunks::<3>().0.to_vec())
}

/// `rgb_to_hsv(arr)`: HSV values of RGB values (last axis of length 3).
fn rgb_to_hsv_array(rgb: NdArray) -> PyResult<NdArray> {
    let hsv = triples(&rgb)?
        .into_iter()
        .map(rgb_to_hsv)
        .collect::<Result<Vec<_>, _>>()?
        .concat();
    NdArray::with_shape(hsv, rgb.shape)
}

/// `hsv_to_rgb(hsv)`: RGB values of HSV values (last axis of length 3).
fn hsv_to_rgb_array(hsv: NdArray) -> PyResult<NdArray> {
    let rgb = triples(&hsv)?
        .into_iter()
        .map(hsv_to_rgb)
        .collect::<Vec<_>>()
        .concat();
    NdArray::with_shape(rgb, hsv.shape)
}

/// `_create_lookup_table(N, data, gamma)` for anchor rows `(x, y0, y1)`.
fn lookup_table(n: usize, rows: Vec<(f64, f64, f64)>, gamma: f64) -> PyResult<NdArray> {
    let rows: Vec<[f64; 3]> = rows.into_iter().map(|(x, y0, y1)| [x, y0, y1]).collect();
    Ok(NdArray::from_vec(create_lookup_table(
        n,
        &rows,
        gamma,
        Channel::Red,
    )?))
}

/// The `(n, n, 4)` table of `SegmentedBivarColormap(patch, n)`: the RGB
/// patch `(M, K, 3)` interpolated bilinearly.
fn segmented_bivar_lut(patch: NdArray, n: usize) -> PyResult<NdArray> {
    let [rows, cols, 3] = patch.shape[..] else {
        return Err(PyErr::value_error(format!(
            "patch must have shape (M, N, 3), not {:?}",
            patch.shape
        )));
    };
    let cells = patch.data.as_chunks::<3>().0.to_vec();
    let patch =
        Grid::from_vec(rows, cols, cells).map_err(|err| PyErr::value_error(err.to_string()))?;
    let cmap = BivarColormap::segmented("", &patch, n, BivarShape::Square, (0.0, 0.0))?;
    let lut = cmap.lut();
    let (rows, cols) = lut.shape();
    NdArray::with_shape(lut.into_vec().concat(), vec![rows, cols, 4])
}

/// The colours of the built-in colour sequence `name`, as RGBA tuples.
fn sequence(name: String) -> PyResult<Vec<(f64, f64, f64, f64)>> {
    color_sequence(&name)
        .map(|colors| colors.into_iter().map(rgba_tuple).collect())
        .ok_or_else(|| PyErr::key_error(format!("{name:?} is not a known color sequence name")))
}

/// The names of the colour sequences.
fn sequence_names() -> PyResult<Vec<String>> {
    Ok(color_sequence_names())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("to_rgba", to_rgba)?;
    module.add("to_rgba_array", to_rgba_array)?;
    module.add("rgb_to_hsv", rgb_to_hsv_array)?;
    module.add("hsv_to_rgb", hsv_to_rgb_array)?;
    module.add("lookup_table", lookup_table)?;
    module.add("segmented_bivar_lut", segmented_bivar_lut)?;
    module.add("color_sequence", sequence)?;
    module.add("color_sequence_names", sequence_names)
}
