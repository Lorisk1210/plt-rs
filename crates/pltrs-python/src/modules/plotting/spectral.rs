//! Spectral plots: `psd`, `csd`, `cohere`, the single-segment spectra,
//! `specgram` and `xcorr` (`acorr` is `xcorr` of a signal with itself).

use pltrs::axes::{
    Axes, CorrelationArtists, ImshowOptions, SpecgramError, SpecgramOptions, SpectralPlotOptions,
    SpectrumPlotOptions, SpectrumScale,
};
use pltrs::collections::Collection;
use pltrs::image::AxesImage;
use pltrs::lines::Line2D;
use pltrs::mlab::{SpecgramMode, Spectrum, SpectrumOptions, XcorrOptions};

use super::super::lines::LineKwargs;
use super::super::mlab::{
    ComplexArrays, Samples, complex_arrays, detrend, sides, spectral_options, window,
};
use super::mapping::{Mapping, colorizer};
use crate::binding::prelude::*;

impl From<SpecgramError> for PyErr {
    /// matplotlib's `ValueError` for spectrograms it cannot compute or
    /// draw.
    fn from(err: SpecgramError) -> Self {
        PyErr::value_error(err.to_string())
    }
}

/// The Welch estimate's options (the dict `mlab._spectral_helper` makes)
/// and the centre frequency: `(options, Fc)`.
type Welch = (Owned, f64);

/// The plot options of a Welch estimate drawn with `line`.
fn plot_options((options, fc): Welch, line: LineKwargs) -> PyResult<SpectralPlotOptions> {
    Ok(SpectralPlotOptions {
        spectral: Python::with_gil(|py| spectral_options(&options.bind(py)))?,
        fc,
        line: line.0,
    })
}

/// A real spectrum as `(values, freqs)` arrays.
fn arrays(spectrum: Spectrum) -> (NdArray, NdArray) {
    (
        NdArray::from_vec(spectrum.values),
        NdArray::from_vec(spectrum.freqs),
    )
}

/// The power spectral density of `x`, real or complex, plotted in
/// decibels (matplotlib's `psd`): `(Pxx, freqs)`.
pub(super) fn psd(
    axes: Axes,
    x: Samples,
    welch: Welch,
    line: LineKwargs,
) -> PyResult<(NdArray, NdArray)> {
    let options = plot_options(welch, line)?;
    let spectrum = match x {
        Samples::Real(x) => axes.psd(&x, options)?,
        Samples::Complex(x) => axes.psd(&x, options)?,
    };
    Ok(arrays(spectrum))
}

/// The cross spectral density of `x` and `y` plotted in decibels
/// (matplotlib's `csd`): `((re, im) of Pxy, freqs)`.
pub(super) fn csd(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    welch: Welch,
    line: LineKwargs,
) -> PyResult<(ComplexArrays, NdArray)> {
    let spectrum = axes.csd(x.into_vec()?, y.into_vec()?, plot_options(welch, line)?)?;
    let n = spectrum.values.len();
    Ok((
        complex_arrays(spectrum.values, vec![n])?,
        NdArray::from_vec(spectrum.freqs),
    ))
}

/// The coherence of `x` and `y` (matplotlib's `cohere`): `(Cxy, freqs)`.
pub(super) fn cohere(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    welch: Welch,
    line: LineKwargs,
) -> PyResult<(NdArray, NdArray)> {
    let spectrum = axes.cohere(x.into_vec()?, y.into_vec()?, plot_options(welch, line)?)?;
    Ok(arrays(spectrum))
}

/// A single-segment spectrum's options: `(Fs, window, pad_to, sides)`.
type SingleOptions = (f64, Owned, Option<usize>, String);

/// The single-segment spectrum `kind` (`"magnitude"`, `"angle"` or
/// `"phase"`) of `x`, magnitudes scaled `"linear"` or `"dB"`, plotted
/// (matplotlib's `magnitude_spectrum`, `angle_spectrum` and
/// `phase_spectrum`): `(spectrum, freqs, line)`.
pub(super) fn single_spectrum(
    axes: Axes,
    kind: String,
    x: NdArray,
    ((fs, window_spec, pad_to, sides_name), fc): (SingleOptions, f64),
    scale: String,
    line: LineKwargs,
) -> PyResult<(NdArray, NdArray, Line2D)> {
    let options = SpectrumPlotOptions {
        spectrum: SpectrumOptions {
            fs,
            window: Python::with_gil(|py| window(&window_spec.bind(py)))?,
            pad_to,
            sides: sides(&sides_name)?,
        },
        fc,
        line: line.0,
    };
    let x = x.into_vec()?;
    let (spectrum, line) = match kind.as_str() {
        "angle" => axes.angle_spectrum(x, options)?,
        "phase" => axes.phase_spectrum(x, options)?,
        _ => axes.magnitude_spectrum(
            x,
            if scale == "dB" {
                SpectrumScale::Decibels
            } else {
                SpectrumScale::Linear
            },
            options,
        )?,
    };
    let (values, freqs) = arrays(spectrum);
    Ok((values, freqs, line))
}

/// What a spectrogram shows: `(mode, scale)`, the scale `None` for the
/// mode's default.
type SpecgramLook = (String, Option<String>);

/// A spectrogram: `(spectrum (freqs × segments), freqs, times, image)`.
type Spectrogram = (NdArray, NdArray, NdArray, AxesImage);

/// The spectrogram of `x` drawn as an image (matplotlib's `specgram`),
/// mapped as `mapping` says, spanning `xextent` (default the segment
/// times).
pub(super) fn specgram(
    axes: Axes,
    x: NdArray,
    welch: Welch,
    (mode, scale): SpecgramLook,
    xextent: Option<(f64, f64)>,
    mapping: Mapping,
) -> PyResult<Spectrogram> {
    let (options, fc) = welch;
    let options = SpecgramOptions {
        spectral: Python::with_gil(|py| spectral_options(&options.bind(py)))?,
        fc,
        mode: match mode.as_str() {
            "magnitude" => SpecgramMode::Magnitude,
            "angle" => SpecgramMode::Angle,
            "phase" => SpecgramMode::Phase,
            _ => SpecgramMode::Psd,
        },
        scale: match scale.as_deref() {
            Some("dB") => Some(SpectrumScale::Decibels),
            Some("linear") => Some(SpectrumScale::Linear),
            _ => None,
        },
        xextent,
        image: ImshowOptions {
            colorizer: colorizer(mapping),
            ..ImshowOptions::default()
        },
    };
    let plot = axes.specgram(x.into_vec()?, options)?;
    let (rows, cols) = plot.spectrogram.values.shape();
    Ok((
        NdArray::with_shape(plot.spectrogram.values.into_vec(), vec![rows, cols])?,
        NdArray::from_vec(plot.spectrogram.freqs),
        NdArray::from_vec(plot.spectrogram.times),
        plot.image,
    ))
}

/// The correlation's options: `(normed, detrend, maxlags)`.
type Correlate = (bool, Owned, Option<usize>);

/// The artists of a correlation plot: `(vertical lines, zero line,
/// markers)`, the first two or the last `None`.
type CorrelationPlot = (Option<Collection>, Option<Line2D>, Option<Line2D>);

/// The cross-correlation of `x` and `y` by lag (matplotlib's `xcorr`):
/// `(lags, correlations, artists)`, drawn as vertical lines from zero
/// (`usevlines`) or markers styled by `line`.
pub(super) fn xcorr(
    axes: Axes,
    (x, y): (NdArray, NdArray),
    (normed, detrend_spec, maxlags): Correlate,
    usevlines: bool,
    line: LineKwargs,
) -> PyResult<(NdArray<i64>, NdArray, CorrelationPlot)> {
    let options = XcorrOptions {
        normed,
        detrend: Python::with_gil(|py| detrend(&detrend_spec.bind(py)))?,
        maxlags,
    };
    let (correlation, artists) =
        axes.xcorr(x.into_vec()?, y.into_vec()?, options, usevlines, line.0)?;
    let artists = match artists {
        CorrelationArtists::Stems { lines, zero } => (Some(lines), Some(zero), None),
        CorrelationArtists::Markers(markers) => (None, None, Some(markers)),
    };
    Ok((
        NdArray::from_vec(correlation.lags),
        NdArray::from_vec(correlation.values),
        artists,
    ))
}
