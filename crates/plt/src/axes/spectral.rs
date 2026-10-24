//! Spectral plots (matplotlib's `psd`, `csd`, `cohere`, the single-segment
//! spectra, `acorr` and `xcorr`): thin plotting layers over [`crate::mlab`].

use super::{Axes, PlotError};
use crate::collections::Collection;
use crate::lines::{Line2D, LineProps, LineStyle};
use crate::markers::{Marker, MarkerSymbol};
use crate::mlab::{
    self, Complex, Correlation, SignalValue, SpectralOptions, Spectrum, SpectrumOptions,
    XcorrOptions,
};
use crate::numeric::arange;

/// Options of [`Axes::psd`], [`Axes::csd`] and [`Axes::cohere`].
#[derive(Clone, Debug, Default)]
pub struct SpectralPlotOptions {
    /// The Welch estimate.
    pub spectral: SpectralOptions,
    /// Centre frequency added to the frequencies (`Fc`).
    pub fc: f64,
    /// Style of the line.
    pub line: LineProps,
}

/// How [`Axes::magnitude_spectrum`] scales the magnitudes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpectrumScale {
    /// The magnitudes themselves (matplotlib's `'linear'`, "energy").
    #[default]
    Linear,
    /// `20 log10` of the magnitudes.
    Decibels,
}

/// Options of the single-segment spectrum plots.
#[derive(Clone, Debug, Default)]
pub struct SpectrumPlotOptions {
    /// The spectrum.
    pub spectrum: SpectrumOptions,
    /// Centre frequency added to the frequencies (`Fc`).
    pub fc: f64,
    /// Style of the line.
    pub line: LineProps,
}

/// The artists of [`Axes::xcorr`] and [`Axes::acorr`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CorrelationArtists {
    /// One vertical line per lag and a horizontal line at zero.
    Stems {
        /// The vertical lines.
        lines: Collection,
        /// The horizontal line.
        zero: Line2D,
    },
    /// Markers at the correlations.
    Markers(Line2D),
}

impl Axes {
    /// Plots the power spectral density in decibels (matplotlib's `psd`)
    /// of real or [`Complex`] samples; the spectrum of complex samples is
    /// two-sided by default.
    ///
    /// # Errors
    ///
    /// When the estimate's options are invalid for `x`.
    pub fn psd<S: SignalValue>(
        &self,
        x: &[S],
        options: SpectralPlotOptions,
    ) -> Result<Spectrum, PlotError> {
        let scale_by_freq = options.spectral.scale_by_freq;
        let spectrum = mlab::psd(x, options.spectral).map_err(PlotError::Mlab)?;
        let db: Vec<f64> = spectrum.values.iter().map(|p| 10.0 * p.log10()).collect();
        self.plot_with(shifted(&spectrum.freqs, options.fc), db, options.line)?;
        let units = if scale_by_freq { "dB/Hz" } else { "dB" };
        self.set_xlabel("Frequency")
            .set_ylabel(format!("Power Spectral Density ({units})"));
        self.grid(true);
        self.decade_yticks();
        Ok(spectrum)
    }

    /// Plots the magnitude of the cross spectral density in decibels
    /// (matplotlib's `csd`).
    ///
    /// # Errors
    ///
    /// When the estimate's options are invalid for the signals.
    pub fn csd(
        &self,
        x: &[f64],
        y: &[f64],
        options: SpectralPlotOptions,
    ) -> Result<Spectrum<Complex>, PlotError> {
        let spectrum = mlab::csd(x, y, options.spectral).map_err(PlotError::Mlab)?;
        let db: Vec<f64> = spectrum
            .values
            .iter()
            .map(|p| 10.0 * p.abs().log10())
            .collect();
        self.plot_with(shifted(&spectrum.freqs, options.fc), db, options.line)?;
        self.set_xlabel("Frequency")
            .set_ylabel("Cross Spectrum Magnitude (dB)");
        self.grid(true);
        self.decade_yticks();
        Ok(spectrum)
    }

    /// Plots the coherence of two signals (matplotlib's `cohere`).
    ///
    /// # Errors
    ///
    /// When the estimate's options are invalid for the signals.
    pub fn cohere(
        &self,
        x: &[f64],
        y: &[f64],
        options: SpectralPlotOptions,
    ) -> Result<Spectrum, PlotError> {
        let coherence = mlab::cohere(x, y, options.spectral).map_err(PlotError::Mlab)?;
        self.plot_with(
            shifted(&coherence.freqs, options.fc),
            coherence.values.clone(),
            options.line,
        )?;
        self.set_xlabel("Frequency").set_ylabel("Coherence");
        self.grid(true);
        Ok(coherence)
    }

    /// Plots the magnitude spectrum (matplotlib's `magnitude_spectrum`).
    ///
    /// # Errors
    ///
    /// When the spectrum's options are invalid for `x`.
    pub fn magnitude_spectrum(
        &self,
        x: &[f64],
        scale: SpectrumScale,
        options: SpectrumPlotOptions,
    ) -> Result<(Spectrum, Line2D), PlotError> {
        let spectrum = mlab::magnitude_spectrum(x, options.spectrum).map_err(PlotError::Mlab)?;
        let (values, units): (Vec<f64>, _) = match scale {
            SpectrumScale::Linear => (spectrum.values.clone(), "energy"),
            SpectrumScale::Decibels => (
                spectrum.values.iter().map(|m| 20.0 * m.log10()).collect(),
                "dB",
            ),
        };
        let line = self.plot_with(shifted(&spectrum.freqs, options.fc), values, options.line)?;
        self.set_xlabel("Frequency")
            .set_ylabel(format!("Magnitude ({units})"));
        Ok((spectrum, line))
    }

    /// Plots the wrapped phase angle spectrum (matplotlib's
    /// `angle_spectrum`).
    ///
    /// # Errors
    ///
    /// When the spectrum's options are invalid for `x`.
    pub fn angle_spectrum(
        &self,
        x: &[f64],
        options: SpectrumPlotOptions,
    ) -> Result<(Spectrum, Line2D), PlotError> {
        let spectrum = mlab::angle_spectrum(x, options.spectrum).map_err(PlotError::Mlab)?;
        self.phase_line(spectrum, "Angle (radians)", options.fc, options.line)
    }

    /// Plots the unwrapped phase spectrum (matplotlib's `phase_spectrum`).
    ///
    /// # Errors
    ///
    /// When the spectrum's options are invalid for `x`.
    pub fn phase_spectrum(
        &self,
        x: &[f64],
        options: SpectrumPlotOptions,
    ) -> Result<(Spectrum, Line2D), PlotError> {
        let spectrum = mlab::phase_spectrum(x, options.spectrum).map_err(PlotError::Mlab)?;
        self.phase_line(spectrum, "Phase (radians)", options.fc, options.line)
    }

    /// Plots a phase spectrum shifted by `fc`, labelling the y axis
    /// `label`.
    fn phase_line(
        &self,
        spectrum: Spectrum,
        label: &str,
        fc: f64,
        line: LineProps,
    ) -> Result<(Spectrum, Line2D), PlotError> {
        let line = self.plot_with(shifted(&spectrum.freqs, fc), spectrum.values.clone(), line)?;
        self.set_xlabel("Frequency").set_ylabel(label);
        Ok((spectrum, line))
    }

    /// Plots the autocorrelation of `x` (matplotlib's `acorr`); see
    /// [`Axes::xcorr`].
    ///
    /// # Errors
    ///
    /// As [`Axes::xcorr`].
    pub fn acorr(
        &self,
        x: &[f64],
        options: XcorrOptions,
        usevlines: bool,
        line: LineProps,
    ) -> Result<(Correlation, CorrelationArtists), PlotError> {
        self.xcorr(x, x, options, usevlines, line)
    }

    /// Plots the cross-correlation of `x` and `y` by lag (matplotlib's
    /// `xcorr`): as vertical lines from zero (`usevlines`) or as markers.
    ///
    /// # Errors
    ///
    /// When the signals differ in length or `maxlags` is out of range.
    pub fn xcorr(
        &self,
        x: &[f64],
        y: &[f64],
        options: XcorrOptions,
        usevlines: bool,
        line: LineProps,
    ) -> Result<(Correlation, CorrelationArtists), PlotError> {
        let correlation = mlab::xcorr(x, y, options).map_err(PlotError::Mlab)?;
        let lags: Vec<f64> = correlation.lags.iter().map(|&lag| lag as f64).collect();
        let artists = if usevlines {
            let lines = self.vlines(&lags, [0.0], &correlation.values)?;
            style_lines(&lines, &line);
            CorrelationArtists::Stems {
                lines,
                zero: self.axhline(0.0, line),
            }
        } else {
            let mut line = line;
            if !line.is_set("marker") {
                line = line.marker(Marker::Symbol(MarkerSymbol::Circle));
            }
            if !line.is_set("linestyle") {
                line = line.linestyle(LineStyle::None);
            }
            CorrelationArtists::Markers(self.plot_with(&lags, &correlation.values, line)?)
        };
        Ok((correlation, artists))
    }

    /// matplotlib's decibel tick placement after `psd`/`csd`: ticks every
    /// `10 · ⌊log10(span)⌋` (at least 1) units over the y bounds.
    fn decade_yticks(&self) {
        let (low, high) = self.ylim();
        let (vmin, vmax) = (low.min(high), low.max(high));
        let step = (10.0 * (vmax - vmin).log10().trunc()).max(1.0);
        self.set_yticks(arange(vmin.floor(), vmax.ceil() + 1.0, step));
    }
}

/// Applies the colour, width and style of `props` to line collection
/// `lines` (matplotlib passes the same keyword arguments to `vlines`).
fn style_lines(lines: &Collection, props: &LineProps) {
    if let Some(color) = &props.color {
        lines.set_edgecolor(color.clone());
    }
    if let Some(width) = props.linewidth {
        lines.set_linewidths(vec![width]);
    }
    if let Some(style) = &props.linestyle {
        lines.set_linestyles(vec![style.clone()]);
    }
}

/// `freqs + fc`.
fn shifted(freqs: &[f64], fc: f64) -> Vec<f64> {
    freqs.iter().map(|f| f + fc).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::AxisName;
    use crate::figure::Figure;

    fn tone(n: usize) -> Vec<f64> {
        (0..n).map(|i| (i as f64 * 0.7).sin()).collect()
    }

    #[test]
    fn psd_labels_grid_and_decibel_ticks() {
        let ax = Figure::new().subplot();
        let spectrum = ax.psd(&tone(512), SpectralPlotOptions::default()).unwrap();
        assert_eq!(spectrum.freqs.len(), 129);
        assert_eq!(ax.label(AxisName::Y), "Power Spectral Density (dB/Hz)");
        let ticks = ax.ticks(AxisName::Y).unwrap();
        let steps: Vec<f64> = ticks
            .major
            .windows(2)
            .map(|w| w[1].loc - w[0].loc)
            .collect();
        assert!(steps.iter().all(|&s| s >= 1.0 && s.fract() == 0.0));
    }

    #[test]
    fn psd_of_complex_samples_is_two_sided() {
        let ax = Figure::new().subplot();
        let samples: Vec<Complex> = (0..256)
            .map(|i| {
                let phase = f64::from(i) * 0.7;
                Complex::new(phase.cos(), phase.sin())
            })
            .collect();
        let spectrum = ax.psd(&samples, SpectralPlotOptions::default()).unwrap();
        assert_eq!(spectrum.freqs.len(), 256);
        assert_eq!(spectrum.freqs[0], -1.0);
        let lines = ax.lines();
        assert_eq!(lines[0].xdata(), spectrum.freqs);
    }

    #[test]
    fn xcorr_markers_default_to_circles_without_line() {
        let ax = Figure::new().subplot();
        let (correlation, artists) = ax
            .xcorr(
                &tone(50),
                &tone(50),
                XcorrOptions::default(),
                false,
                LineProps::default(),
            )
            .unwrap();
        assert_eq!(correlation.lags.len(), 21);
        let CorrelationArtists::Markers(line) = artists else {
            panic!("markers requested");
        };
        assert_eq!(line.marker(), Marker::Symbol(MarkerSymbol::Circle));
        assert_eq!(line.linestyle(), LineStyle::None);
        assert!(
            ax.xcorr(
                &tone(5),
                &tone(6),
                XcorrOptions::default(),
                true,
                LineProps::default()
            )
            .is_err()
        );
    }
}
