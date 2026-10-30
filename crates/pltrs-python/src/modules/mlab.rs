//! `_native.mlab`: spectral estimates, detrending and kernel density
//! estimation (matplotlib's `mlab`), computed by `pltrs::mlab`.
//!
//! A complex signal crosses as its real and imaginary parts, `(re, im)`,
//! with `im` `None` for a real signal; complex results come back the same
//! way. The spectral options cross as one dict keyed by matplotlib's
//! parameter names (`NFFT`, `Fs`, `noverlap`...), already defaulted by the
//! Python signature. A detrend or window function written in Python runs
//! through a [`Callback`].

use std::sync::Arc;

use pltrs::mlab::{
    Complex, Detrend, GaussianKde, MlabError, Specgram, SpectralMode, SpectralOptions,
    SpectrumSides, Window, detrend_grid, spectral_segments,
};
use pltrs::numeric::Grid;
use pltrs::stats::{BandwidthMethod, StatsError};

use crate::binding::prelude::*;
use crate::binding::{dict_value, registered_error};

native_value!(GaussianKde);

impl From<MlabError> for PyErr {
    /// matplotlib's `ValueError`, with its message where pltrs (Rust)'s differs.
    fn from(err: MlabError) -> Self {
        match err {
            MlabError::InvalidOverlap { .. } => {
                PyErr::value_error("noverlap must be less than NFFT")
            }
            MlabError::WindowLength { .. } => {
                PyErr::value_error("The window length must match the data's first dimension")
            }
            MlabError::AxisOutOfBounds { axis, .. } => {
                PyErr::value_error(format!("axis(={axis}) out of bounds"))
            }
            other => PyErr::value_error(other.to_string()),
        }
    }
}

impl From<StatsError> for PyErr {
    /// numpy's `LinAlgError` (registered by `pltrs.mlab`) for a singular
    /// covariance, `ValueError` otherwise, with matplotlib's messages.
    fn from(err: StatsError) -> Self {
        match err {
            StatsError::SingularCovariance => registered_error("LinAlgError", "Singular matrix"),
            StatsError::TooFewSamples => {
                PyErr::value_error("`dataset` input should have multiple elements.")
            }
            StatsError::DimensionMismatch { expected, actual } => PyErr::value_error(format!(
                "points have dimension {actual}, dataset has dimension {expected}"
            )),
            other => PyErr::value_error(other.to_string()),
        }
    }
}

/// A signal, real or complex.
pub(super) enum Samples {
    Real(Vec<f64>),
    Complex(Vec<Complex>),
}

impl FromPy<'_> for Samples {
    /// `(re, im)`: two arrays of equal length, or an array and `None`.
    fn from_py(object: &Bound<'_>) -> PyResult<Self> {
        let (re, im): (NdArray, Option<NdArray>) = object.extract()?;
        let Some(im) = im else {
            return Ok(Self::Real(re.data));
        };
        if im.data.len() != re.data.len() {
            return Err(PyErr::value_error(
                "the real and imaginary parts of a signal differ in length",
            ));
        }
        let samples = re
            .data
            .into_iter()
            .zip(im.data)
            .map(|(re, im)| Complex::new(re, im))
            .collect();
        Ok(Self::Complex(samples))
    }
}

/// The real and imaginary parts of complex values, as arrays of `shape`.
pub(super) type ComplexArrays = (NdArray, NdArray);

/// Splits complex values into their parts, shaped `shape`.
pub(super) fn complex_arrays(values: Vec<Complex>, shape: Vec<usize>) -> PyResult<ComplexArrays> {
    let (re, im): (Vec<f64>, Vec<f64>) = values.into_iter().map(|z| (z.re, z.im)).unzip();
    Ok((
        NdArray::with_shape(re, shape.clone())?,
        NdArray::with_shape(im, shape)?,
    ))
}

/// A detrend method: `"none"`, `"mean"`, `"linear"`, or a Python function
/// of one segment.
pub(super) fn detrend(object: &Bound<'_>) -> PyResult<Detrend> {
    if let Ok(name) = object.extract::<String>() {
        return named_detrend(&name);
    }
    let function: Callback = object.extract()?;
    Ok(Detrend::Custom(Arc::new(move |segment: &[f64]| {
        function
            .call::<NdArray>((NdArray::from_vec(segment.to_vec()),))
            .map(|values| values.data)
            .unwrap_or_default()
    })))
}

/// The detrend method named `"none"`, `"mean"` or `"linear"`.
fn named_detrend(name: &str) -> PyResult<Detrend> {
    match name {
        "none" => Ok(Detrend::None),
        "mean" => Ok(Detrend::Mean),
        "linear" => Ok(Detrend::Linear),
        _ => Err(PyErr::value_error(format!(
            "Unknown value for key: {name:?}, must be one of: 'default', 'constant', 'mean', \
             'linear', or a function"
        ))),
    }
}

/// A window: `"hanning"`, `"none"`, or its values.
pub(super) fn window(object: &Bound<'_>) -> PyResult<Window> {
    match object.extract::<String>().ok().as_deref() {
        Some("hanning") => Ok(Window::Hanning),
        Some("none") => Ok(Window::Rectangular),
        Some(other) => Err(PyErr::value_error(format!("no window named {other:?}"))),
        None => Ok(Window::Values(object.extract::<NdArray>()?.data)),
    }
}

/// The sides of a spectrum: `"default"`, `"onesided"` or `"twosided"`.
pub(super) fn sides(name: &str) -> PyResult<SpectrumSides> {
    match name {
        "default" => Ok(SpectrumSides::Default),
        "onesided" => Ok(SpectrumSides::OneSided),
        "twosided" => Ok(SpectrumSides::TwoSided),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for sides; supported values are 'default', \
             'onesided', 'twosided'"
        ))),
    }
}

/// What a spectrum reports: matplotlib's `mode` names.
fn mode(name: &str) -> PyResult<SpectralMode> {
    match name {
        "psd" => Ok(SpectralMode::Psd),
        "complex" => Ok(SpectralMode::Complex),
        "magnitude" => Ok(SpectralMode::Magnitude),
        "angle" => Ok(SpectralMode::Angle),
        "phase" => Ok(SpectralMode::Phase),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a valid value for mode; supported values are 'default', 'psd', \
             'complex', 'magnitude', 'angle', 'phase'"
        ))),
    }
}

/// The spectral options of the dict `options`.
pub(super) fn spectral_options(options: &Bound<'_>) -> PyResult<SpectralOptions> {
    Ok(SpectralOptions {
        nfft: dict_value(options, "NFFT")?,
        fs: dict_value(options, "Fs")?,
        detrend: detrend(&dict_value::<Bound<'_>>(options, "detrend")?)?,
        window: window(&dict_value::<Bound<'_>>(options, "window")?)?,
        noverlap: Some(dict_value(options, "noverlap")?),
        pad_to: Some(dict_value(options, "pad_to")?),
        sides: sides(&dict_value::<String>(options, "sides")?)?,
        scale_by_freq: dict_value(options, "scale_by_freq")?,
    })
}

/// `_spectral_helper(x, y, options, mode)`: the unaveraged spectra of each
/// segment as `(re, im)` arrays of shape `(freqs, segments)`, the
/// frequencies, and the segment times. `y = None` transforms `x` alone.
fn spectral_helper(
    x: Samples,
    y: Option<Samples>,
    options: Owned,
    mode_name: String,
) -> PyResult<(ComplexArrays, NdArray, NdArray)> {
    let options = Python::with_gil(|py| spectral_options(&options.bind(py)))?;
    let mode = mode(&mode_name)?;
    let result = match (&x, &y) {
        (Samples::Real(x), None) => spectral_segments(x, None::<&[f64]>, &options, mode),
        (Samples::Complex(x), None) => spectral_segments(x, None::<&[f64]>, &options, mode),
        (Samples::Real(x), Some(Samples::Real(y))) => {
            spectral_segments(x, Some(y.as_slice()), &options, mode)
        }
        (Samples::Real(x), Some(Samples::Complex(y))) => {
            spectral_segments(x, Some(y.as_slice()), &options, mode)
        }
        (Samples::Complex(x), Some(Samples::Real(y))) => {
            spectral_segments(x, Some(y.as_slice()), &options, mode)
        }
        (Samples::Complex(x), Some(Samples::Complex(y))) => {
            spectral_segments(x, Some(y.as_slice()), &options, mode)
        }
    }?;
    let Specgram {
        values,
        freqs,
        times,
    } = result;
    let (rows, cols) = values.shape();
    Ok((
        complex_arrays(values.into_vec(), vec![rows, cols])?,
        NdArray::from_vec(freqs),
        NdArray::from_vec(times),
    ))
}

/// numpy's `axis` (negative counts from the end) as an index below `ndim`.
fn array_axis(axis: i64, ndim: usize) -> PyResult<usize> {
    let ndim_signed = i64::try_from(ndim).unwrap_or(i64::MAX);
    let index = if axis < 0 { axis + ndim_signed } else { axis };
    usize::try_from(index)
        .ok()
        .filter(|&index| index < ndim)
        .ok_or_else(|| PyErr::value_error(format!("axis(={axis}) out of bounds")))
}

/// `detrend(x, key, axis)` with `key` `"mean"` or `"linear"`, for arrays
/// of up to two dimensions (`axis` `None` detrends the whole array).
fn detrend_array(x: NdArray, key: String, axis: Option<i64>) -> PyResult<NdArray> {
    let method = named_detrend(&key)?;
    let shape = x.shape.clone();
    let axis = axis.map(|axis| array_axis(axis, shape.len())).transpose()?;
    let (rows, cols, axis) = match shape[..] {
        [] if method == Detrend::None => return Ok(x),
        [] => return NdArray::with_shape(vec![0.0], shape),
        [n] => (1, n, Some(1)),
        [rows, cols] => (rows, cols, axis),
        _ => {
            return Err(PyErr::value_error(format!(
                "pltrs detrends arrays of up to 2 dimensions, not {}",
                shape.len()
            )));
        }
    };
    let values = Grid::from_vec(rows, cols, x.data).map_err(shape_error)?;
    NdArray::with_shape(detrend_grid(&values, &method, axis)?.into_vec(), shape)
}

/// A grid that does not have the shape its array claims.
fn shape_error(err: impl std::fmt::Display) -> PyErr {
    PyErr::value_error(err.to_string())
}

/// The bandwidth factor of a dataset of `num_dp` points of `dim`
/// dimensions by the rule `"scott"` or `"silverman"`.
fn bandwidth_factor(rule: String, dim: usize, num_dp: usize) -> PyResult<f64> {
    let method = match rule.as_str() {
        "scott" => BandwidthMethod::Scott,
        "silverman" => BandwidthMethod::Silverman,
        _ => {
            return Err(PyErr::value_error(format!(
                "no bandwidth rule named {rule:?}"
            )));
        }
    };
    Ok(method.factor(dim, num_dp))
}

/// A Gaussian kernel density estimate of `dataset` (`dim × points`) with
/// the bandwidth `factor`.
fn gaussian_kde(dataset: NdArray, factor: f64) -> PyResult<GaussianKde> {
    let [rows, cols] = dataset.shape[..] else {
        return Err(PyErr::value_error("the dataset must be a 2-D array"));
    };
    let grid = Grid::from_vec(rows, cols, dataset.data).map_err(shape_error)?;
    Ok(GaussianKde::from_grid(
        grid,
        BandwidthMethod::Factor(factor),
    )?)
}

/// A square grid as a 2-D array.
fn square_array(grid: &Grid<f64>) -> PyResult<NdArray> {
    let (rows, cols) = grid.shape();
    NdArray::with_shape(grid.as_slice().to_vec(), vec![rows, cols])
}

/// The kernel covariance and its inverse, and the normalisation factor.
fn kde_covariance(kde: Native<GaussianKde>) -> PyResult<(NdArray, NdArray, f64)> {
    let kde = kde.borrow();
    Ok((
        square_array(kde.covariance())?,
        square_array(kde.inv_cov())?,
        kde.norm_factor(),
    ))
}

/// The estimate at the columns of `points` (`dim × m`).
fn kde_evaluate(kde: Native<GaussianKde>, points: NdArray) -> PyResult<NdArray> {
    let [rows, cols] = points.shape[..] else {
        return Err(PyErr::value_error("the points must be a 2-D array"));
    };
    let grid = Grid::from_vec(rows, cols, points.data).map_err(shape_error)?;
    Ok(NdArray::from_vec(kde.borrow().evaluate_grid(&grid)?))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("spectral_helper", spectral_helper)?;
    module.add("detrend", detrend_array)?;
    module.add("bandwidth_factor", bandwidth_factor)?;
    module.add("gaussian_kde", gaussian_kde)?;
    module.add("kde_covariance", kde_covariance)?;
    module.add("kde_evaluate", kde_evaluate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axes_count_from_the_end_and_are_checked() {
        assert_eq!(array_axis(-1, 2).unwrap(), 1);
        assert_eq!(array_axis(0, 1).unwrap(), 0);
        assert!(array_axis(1, 1).is_err());
        assert!(array_axis(-3, 2).is_err());
        assert!(array_axis(0, 0).is_err());
    }

    #[test]
    fn detrending_scalars_rows_and_columns() {
        let scalar = NdArray::with_shape(vec![5.5], vec![]).unwrap();
        assert_eq!(
            detrend_array(scalar, "mean".into(), None).unwrap().data,
            [0.0]
        );
        let rows = NdArray::with_shape(vec![1.0, 3.0, 10.0, 20.0], vec![2, 2]).unwrap();
        let by_row = detrend_array(rows.clone(), "mean".into(), Some(1)).unwrap();
        assert_eq!(by_row.data, [-1.0, 1.0, -5.0, 5.0]);
        let by_column = detrend_array(rows, "mean".into(), Some(-2)).unwrap();
        assert_eq!(by_column.data, [-4.5, -8.5, 4.5, 8.5]);
        let line = NdArray::from_vec(vec![1.0, 2.0, 3.0]);
        let flat = detrend_array(line, "linear".into(), None).unwrap();
        assert!(flat.data.iter().all(|v| v.abs() < 1e-12));
        assert!(named_detrend("spam").is_err());
    }

    #[test]
    fn modes_and_sides_take_matplotlibs_names() {
        assert_eq!(mode("phase").unwrap(), SpectralMode::Phase);
        assert!(mode("default").is_err());
        assert_eq!(sides("twosided").unwrap(), SpectrumSides::TwoSided);
        assert!(sides("eggs").is_err());
    }
}
