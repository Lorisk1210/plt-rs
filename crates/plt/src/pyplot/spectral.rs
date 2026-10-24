//! Spectral analysis and correlation plots on the current axes
//! (matplotlib's `psd`, `csd`, `cohere`, `specgram`,
//! `magnitude_spectrum`, `angle_spectrum`, `phase_spectrum`, `acorr` and
//! `xcorr`).

use super::mappables::make_current;
use super::registry::gca;
use crate::axes::{
    CorrelationArtists, PlotError, SpecgramError, SpecgramOptions, SpecgramPlot,
    SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale,
};
use crate::lines::{Line2D, LineProps};
use crate::mlab::{Complex, Correlation, SignalValue, Spectrum, XcorrOptions};

/// Plots the power spectral density of `x` on the current axes
/// (matplotlib's `psd`).
///
/// # Errors
///
/// As [`Axes::psd`](crate::axes::Axes::psd).
pub fn psd<S: SignalValue>(x: &[S], options: SpectralPlotOptions) -> Result<Spectrum, PlotError> {
    gca().psd(x, options)
}

/// Plots the cross spectral density of `x` and `y` on the current axes
/// (matplotlib's `csd`).
///
/// # Errors
///
/// As [`Axes::csd`](crate::axes::Axes::csd).
pub fn csd(
    x: &[f64],
    y: &[f64],
    options: SpectralPlotOptions,
) -> Result<Spectrum<Complex>, PlotError> {
    gca().csd(x, y, options)
}

/// Plots the coherence of `x` and `y` on the current axes (matplotlib's
/// `cohere`).
///
/// # Errors
///
/// As [`Axes::cohere`](crate::axes::Axes::cohere).
pub fn cohere(x: &[f64], y: &[f64], options: SpectralPlotOptions) -> Result<Spectrum, PlotError> {
    gca().cohere(x, y, options)
}

/// Draws the spectrogram of `x` on the current axes and makes its image
/// the current image (matplotlib's `specgram`).
///
/// # Errors
///
/// As [`Axes::specgram`](crate::axes::Axes::specgram).
pub fn specgram(x: &[f64], options: SpecgramOptions) -> Result<SpecgramPlot, SpecgramError> {
    let ax = gca();
    let plot = ax.specgram(x, options)?;
    make_current(&ax, &plot.image);
    Ok(plot)
}

/// Plots the magnitude spectrum of `x` on the current axes (matplotlib's
/// `magnitude_spectrum`).
///
/// # Errors
///
/// As [`Axes::magnitude_spectrum`](crate::axes::Axes::magnitude_spectrum).
pub fn magnitude_spectrum(
    x: &[f64],
    scale: SpectrumScale,
    options: SpectrumPlotOptions,
) -> Result<(Spectrum, Line2D), PlotError> {
    gca().magnitude_spectrum(x, scale, options)
}

/// Plots the wrapped phase spectrum of `x` on the current axes
/// (matplotlib's `angle_spectrum`).
///
/// # Errors
///
/// As [`Axes::angle_spectrum`](crate::axes::Axes::angle_spectrum).
pub fn angle_spectrum(
    x: &[f64],
    options: SpectrumPlotOptions,
) -> Result<(Spectrum, Line2D), PlotError> {
    gca().angle_spectrum(x, options)
}

/// Plots the unwrapped phase spectrum of `x` on the current axes
/// (matplotlib's `phase_spectrum`).
///
/// # Errors
///
/// As [`Axes::phase_spectrum`](crate::axes::Axes::phase_spectrum).
pub fn phase_spectrum(
    x: &[f64],
    options: SpectrumPlotOptions,
) -> Result<(Spectrum, Line2D), PlotError> {
    gca().phase_spectrum(x, options)
}

/// Plots the autocorrelation of `x` on the current axes (matplotlib's
/// `acorr`).
///
/// # Errors
///
/// As [`Axes::acorr`](crate::axes::Axes::acorr).
pub fn acorr(
    x: &[f64],
    options: XcorrOptions,
    usevlines: bool,
    line: LineProps,
) -> Result<(Correlation, CorrelationArtists), PlotError> {
    gca().acorr(x, options, usevlines, line)
}

/// Plots the cross correlation of `x` and `y` on the current axes
/// (matplotlib's `xcorr`).
///
/// # Errors
///
/// As [`Axes::xcorr`](crate::axes::Axes::xcorr).
pub fn xcorr(
    x: &[f64],
    y: &[f64],
    options: XcorrOptions,
    usevlines: bool,
    line: LineProps,
) -> Result<(Correlation, CorrelationArtists), PlotError> {
    gca().xcorr(x, y, options, usevlines, line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::ColorizingArtist;
    use crate::pyplot::{close_all, gci, subplot};

    fn signal() -> Vec<f64> {
        (0..512)
            .map(|i| (f64::from(i) * 0.3).sin() + (f64::from(i) * 0.05).cos())
            .collect()
    }

    #[test]
    fn spectra_and_correlations() {
        close_all();
        let (x, y) = (signal(), signal().iter().rev().copied().collect::<Vec<_>>());
        let spectral = SpectralPlotOptions::default();
        let spectrum = SpectrumPlotOptions::default();
        psd(&x, spectral.clone()).unwrap();
        subplot(2, 3, 2).unwrap();
        csd(&x, &y, spectral.clone()).unwrap();
        subplot(2, 3, 3).unwrap();
        cohere(&x, &y, spectral).unwrap();
        subplot(2, 3, 4).unwrap();
        magnitude_spectrum(&x, SpectrumScale::default(), spectrum.clone()).unwrap();
        angle_spectrum(&x, spectrum.clone()).unwrap();
        phase_spectrum(&x, spectrum).unwrap();
        subplot(2, 3, 5).unwrap();
        let options = XcorrOptions::default();
        acorr(&x, options.clone(), true, LineProps::default()).unwrap();
        subplot(2, 3, 6).unwrap();
        xcorr(&x, &y, options, false, LineProps::default()).unwrap();
        close_all();
    }

    #[test]
    fn spectrograms_become_the_current_image() {
        close_all();
        let plot = specgram(&signal(), SpecgramOptions::default()).unwrap();
        assert!(matches!(gci(), Some(ColorizingArtist::Image(image)) if image == plot.image));
        close_all();
    }
}
