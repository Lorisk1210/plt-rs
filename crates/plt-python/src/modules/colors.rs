//! `_native.colors`: colour specifications (matplotlib's `colors`).
//!
//! A colour converts from what matplotlib accepts as one: a string (a
//! name, a hex code, a grey level, `"Cn"`, `"none"`) or a sequence of 3
//! or 4 numbers; it converts back as the string it was given, or as an
//! RGBA tuple.

use plt::colors::{
    Channel, Color, ColorError, ColormapError, Rgba, SegmentProblem, color_sequence,
    color_sequence_names, create_lookup_table, hsv_to_rgb, rgb_to_hsv,
};

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
    module.add("color_sequence", sequence)?;
    module.add("color_sequence_names", sequence_names)
}
