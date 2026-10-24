//! Probe operations for the `stats` area: box/violin statistics, kernel
//! density estimates, numpy histograms, `Axes.hist`, windows, detrending,
//! spectral estimates and correlations.
//!
//! Argument conventions (see `spec/reference/probes/test_stats.py`): lists
//! made only of JSON integers are numpy `int64` arrays, booleans `bool`
//! arrays; "datasets" follow `cbook._reshape_2D` (a flat list is one
//! dataset, a list of lists one dataset per inner list, and with the
//! `*_ndarray` flag the columns of a 2-D array); complex values are
//! `[re, im]` pairs.

mod distributions;
mod histogram;
mod signal;
mod spectral;

pub(crate) use distributions::{boxplot_stats, gaussian_kde};
pub(crate) use signal::{detrend, detrend_mean, window_hanning, window_none};
pub(crate) use spectral::{cohere, psd, spectral_options, spectrum_options};

use plt::mlab::Complex;
use plt::numeric::Grid;

use crate::json::Json;
use crate::probes::encode::object;
use crate::probes::{Op, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("cbook.boxplot_stats", distributions::boxplot_stats),
    Op::new("cbook.violin_stats", distributions::violin_stats),
    Op::new("mlab.GaussianKDE", distributions::gaussian_kde),
    Op::new("numpy.histogram", histogram::histogram),
    Op::new("numpy.histogram_bin_edges", histogram::histogram_bin_edges),
    Op::new("numpy.histogram_bin_width", histogram::histogram_bin_width),
    Op::new("Axes.hist", histogram::axes_hist),
    Op::new("mlab.window_hanning", signal::window_hanning),
    Op::new("mlab.window_none", signal::window_none),
    Op::new("mlab.detrend", signal::detrend),
    Op::new("mlab.detrend_mean", signal::detrend_mean),
    Op::new("mlab.detrend_linear", signal::detrend_linear),
    Op::new("mlab.detrend_none", signal::detrend_none),
    Op::new("mlab._stride_windows", signal::stride_windows),
    Op::new("Axes.acorr", signal::acorr),
    Op::new("Axes.xcorr", signal::xcorr),
    Op::new("mlab.psd", spectral::psd),
    Op::new("mlab.csd", spectral::csd),
    Op::new("mlab.specgram", spectral::specgram),
    Op::new("mlab.cohere", spectral::cohere),
    Op::new("mlab.complex_spectrum", spectral::complex_spectrum),
    Op::new("mlab.magnitude_spectrum", spectral::magnitude_spectrum),
    Op::new("mlab.angle_spectrum", spectral::angle_spectrum),
    Op::new("mlab.phase_spectrum", spectral::phase_spectrum),
];

/// A flat numeric list as `np.asarray` types it.
enum Numbers {
    Int(Vec<i64>),
    Float(Vec<f64>),
    Bool(Vec<bool>),
}

impl Numbers {
    fn parse(value: &Json) -> Result<Self, ProbeError> {
        let items = value
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs(format!("expected a list, got {value}")))?;
        if !items.is_empty() && items.iter().all(|v| v.as_bool().is_some()) {
            return Ok(Self::Bool(items.iter().filter_map(Json::as_bool).collect()));
        }
        if !items.is_empty() && items.iter().all(Json::is_integer) {
            return Ok(Self::Int(items.iter().filter_map(Json::as_i64).collect()));
        }
        Ok(Self::Float(floats(value)?))
    }
}

/// A flat list of numbers (any JSON numeric literal).
fn floats(value: &Json) -> Result<Vec<f64>, ProbeError> {
    value
        .as_f64_vec()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected a list of numbers, got {value}")))
}

/// Datasets per `cbook._reshape_2D`; `as_ndarray` first converts the value
/// to a 2-D array whose columns are the datasets. The flag reports whether
/// every sample was an integer literal (an `int64` array in numpy).
fn datasets(value: &Json, as_ndarray: bool) -> Result<(Vec<Vec<f64>>, bool), ProbeError> {
    let integer = all_integers(value);
    let sets = match value.as_array() {
        None => vec![vec![number(value)?]],
        Some([]) => vec![Vec::new()],
        Some(items) if as_ndarray => ndarray_columns(items)?,
        Some(items) if items.iter().all(|v| v.as_array().is_none()) => vec![floats(value)?],
        Some(items) => items
            .iter()
            .map(|item| match item.as_array() {
                None => Ok(vec![number(item)?]),
                Some(_) => floats(item)
                    .map_err(|_| ProbeError::rejected("datasets must have 2 or fewer dimensions")),
            })
            .collect::<Result<_, _>>()?,
    };
    Ok((sets, integer))
}

fn ndarray_columns(items: &[Json]) -> Result<Vec<Vec<f64>>, ProbeError> {
    if items.iter().all(|v| v.as_array().is_none()) {
        return Ok(vec![items.iter().map(number).collect::<Result<_, _>>()?]);
    }
    let rows = items.iter().map(floats).collect::<Result<Vec<_>, _>>()?;
    let grid = Grid::from_rows(rows).map_err(ProbeError::rejected)?;
    Ok((0..grid.cols()).map(|c| grid.column(c)).collect())
}

fn number(value: &Json) -> Result<f64, ProbeError> {
    value
        .as_f64()
        .ok_or_else(|| ProbeError::BadArgs(format!("expected a number, got {value}")))
}

fn all_integers(value: &Json) -> bool {
    match value {
        Json::Integer(_) => true,
        Json::Array(items) => items.iter().all(all_integers),
        _ => false,
    }
}

/// A 2-D grid as nested rows.
fn grid_json(grid: &Grid<f64>) -> Json {
    Json::Array(
        grid.iter_rows()
            .map(|row| Json::from(row.to_vec()))
            .collect(),
    )
}

/// A complex number as `[re, im]`.
fn complex_json(z: Complex) -> Json {
    Json::from(vec![z.re, z.im])
}

/// A list of `[re, im]` pairs or a flat list of real numbers.
enum SignalArg {
    Real(Vec<f64>),
    Complex(Vec<Complex>),
}

impl SignalArg {
    fn parse(value: &Json) -> Result<Self, ProbeError> {
        let items = value
            .as_array()
            .ok_or_else(|| ProbeError::BadArgs(format!("expected a signal, got {value}")))?;
        if items.first().is_some_and(|v| v.as_array().is_some()) {
            items
                .iter()
                .map(|pair| match pair.as_f64_vec().as_deref() {
                    Some([re, im]) => Ok(Complex::new(*re, *im)),
                    _ => Err(ProbeError::BadArgs(format!("bad complex value {pair}"))),
                })
                .collect::<Result<_, _>>()
                .map(Self::Complex)
        } else {
            floats(value).map(Self::Real)
        }
    }
}
