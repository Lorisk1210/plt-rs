//! Welch spectral estimates, spectrograms, coherence and single-segment
//! spectra (`mlab.psd`, `csd`, `specgram`, `cohere`, `*_spectrum`).

mod helper;

use helper::{Mode, Raw, Signal, spectral_helper};

use super::{Complex, Detrend, MlabError, Window};
use crate::numeric::Grid;
use crate::stats::pairwise;

/// A sample type spectral estimates accept: real (`f64`) or complex.
///
/// Real signals default to one-sided spectra, complex ones to two-sided.
pub trait SignalValue: Copy + Into<Complex> {
    /// `true` for complex samples.
    const COMPLEX: bool;
}

impl SignalValue for f64 {
    const COMPLEX: bool = false;
}

impl SignalValue for Complex {
    const COMPLEX: bool = true;
}

/// Which frequencies a spectrum covers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpectrumSides {
    /// One-sided for real signals, two-sided for complex ones.
    #[default]
    Default,
    /// Non-negative frequencies only (negative-frequency power folded in).
    OneSided,
    /// All frequencies, from negative to positive.
    TwoSided,
}

/// What a spectrogram reports for each segment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpecgramMode {
    /// Power spectral density.
    #[default]
    Psd,
    /// Magnitude `|X| / sum(window)`.
    Magnitude,
    /// Wrapped phase angle.
    Angle,
    /// Phase unwrapped along frequency.
    Phase,
}

/// Parameters of the Welch-style estimates ([`psd`], [`csd`],
/// [`specgram`], [`cohere`]); the defaults are matplotlib's.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectralOptions {
    /// Samples per segment (default 256); shorter signals are zero padded.
    pub nfft: usize,
    /// Sampling frequency (default 2).
    pub fs: f64,
    /// Detrending applied to each segment (default none).
    pub detrend: Detrend,
    /// Window applied to each segment (default Hann).
    pub window: Window,
    /// Samples shared by consecutive segments; `None` means 0, except for
    /// [`specgram`] where it means 128.
    pub noverlap: Option<usize>,
    /// FFT length per segment; defaults to `nfft` (longer pads with
    /// zeros, shorter truncates).
    pub pad_to: Option<usize>,
    /// One- or two-sided output.
    pub sides: SpectrumSides,
    /// Divide densities by the sampling frequency (density per unit
    /// frequency) rather than preserving segment power (default `true`).
    pub scale_by_freq: bool,
}

impl Default for SpectralOptions {
    fn default() -> Self {
        Self {
            nfft: 256,
            fs: 2.0,
            detrend: Detrend::None,
            window: Window::Hanning,
            noverlap: None,
            pad_to: None,
            sides: SpectrumSides::Default,
            scale_by_freq: true,
        }
    }
}

/// Parameters of the single-segment spectra ([`complex_spectrum`] and
/// friends), which transform the whole signal at once.
#[derive(Clone, Debug, PartialEq)]
pub struct SpectrumOptions {
    /// Sampling frequency (default 2).
    pub fs: f64,
    /// Window (default Hann).
    pub window: Window,
    /// FFT length; defaults to the signal length.
    pub pad_to: Option<usize>,
    /// One- or two-sided output.
    pub sides: SpectrumSides,
}

impl Default for SpectrumOptions {
    fn default() -> Self {
        Self {
            fs: 2.0,
            window: Window::Hanning,
            pad_to: None,
            sides: SpectrumSides::Default,
        }
    }
}

/// Values over frequency.
#[derive(Clone, Debug, PartialEq)]
pub struct Spectrum<T = f64> {
    /// One value per frequency.
    pub values: Vec<T>,
    /// The frequencies, ascending.
    pub freqs: Vec<f64>,
}

/// Values over frequency (rows) and time (columns).
#[derive(Clone, Debug, PartialEq)]
pub struct Specgram<T = f64> {
    /// `freqs.len() × times.len()` values.
    pub values: Grid<T>,
    /// Row frequencies.
    pub freqs: Vec<f64>,
    /// Segment midpoints in time units.
    pub times: Vec<f64>,
}

/// Power spectral density by Welch's average periodogram.
///
/// ```
/// use plt::mlab::{SpectralOptions, psd};
/// let x: Vec<f64> = (0..64).map(|i| (f64::from(i) * 0.5).sin()).collect();
/// let opts = SpectralOptions { nfft: 16, ..SpectralOptions::default() };
/// let p = psd(&x, opts).unwrap();
/// assert_eq!(p.values.len(), 9);
/// assert_eq!(p.freqs[8], 1.0);
/// ```
pub fn psd<S: SignalValue>(x: &[S], opts: SpectralOptions) -> Result<Spectrum, MlabError> {
    let raw = spectral_helper(
        &Signal::new(x),
        None,
        &opts,
        opts.noverlap.unwrap_or(0),
        Mode::Psd,
    )?;
    let Spectrum { values, freqs } = average_segments(raw);
    Ok(Spectrum {
        values: values.into_iter().map(|z| z.re).collect(),
        freqs,
    })
}

/// Cross spectral density `conj(X)·Y` by Welch's average periodogram.
///
/// When one signal has a single segment and the other several, the single
/// segment is paired with each of them.
pub fn csd<S: SignalValue, U: SignalValue>(
    x: &[S],
    y: &[U],
    opts: SpectralOptions,
) -> Result<Spectrum<Complex>, MlabError> {
    let raw = spectral_helper(
        &Signal::new(x),
        Some(&Signal::new(y)),
        &opts,
        opts.noverlap.unwrap_or(0),
        Mode::Psd,
    )?;
    Ok(average_segments(raw))
}

/// A spectrogram: the spectrum of each segment.
pub fn specgram<S: SignalValue>(
    x: &[S],
    opts: SpectralOptions,
    mode: SpecgramMode,
) -> Result<Specgram, MlabError> {
    let mode = match mode {
        SpecgramMode::Psd => Mode::Psd,
        SpecgramMode::Magnitude => Mode::Magnitude,
        SpecgramMode::Angle => Mode::Angle,
        SpecgramMode::Phase => Mode::Phase,
    };
    let raw = specgram_raw(x, &opts, mode)?;
    Ok(Specgram {
        values: raw.values.map(|z| z.re),
        freqs: raw.freqs,
        times: raw.times,
    })
}

/// A spectrogram of complex spectra `X / sum(window)` (matplotlib's
/// `mode='complex'`).
pub fn complex_specgram<S: SignalValue>(
    x: &[S],
    opts: SpectralOptions,
) -> Result<Specgram<Complex>, MlabError> {
    let raw = specgram_raw(x, &opts, Mode::Complex)?;
    Ok(Specgram {
        values: raw.values,
        freqs: raw.freqs,
        times: raw.times,
    })
}

fn specgram_raw<S: SignalValue>(
    x: &[S],
    opts: &SpectralOptions,
    mode: Mode,
) -> Result<Raw, MlabError> {
    spectral_helper(
        &Signal::new(x),
        None,
        opts,
        opts.noverlap.unwrap_or(128),
        mode,
    )
}

/// Coherence `|Pxy|² / (Pxx·Pyy)`; needs at least two segments' worth of
/// samples (`len(x) >= 2·nfft`).
pub fn cohere<S: SignalValue, U: SignalValue>(
    x: &[S],
    y: &[U],
    opts: SpectralOptions,
) -> Result<Spectrum, MlabError> {
    if x.len() < 2 * opts.nfft {
        return Err(MlabError::SignalTooShort {
            len: x.len(),
            nfft: opts.nfft,
        });
    }
    let pxx = psd(x, opts.clone())?;
    let pyy = psd(y, opts.clone())?;
    let pxy = csd(x, y, opts)?;
    let values = pxy
        .values
        .iter()
        .zip(pxx.values.iter().zip(&pyy.values))
        .map(|(z, (px, py))| {
            let m = z.abs();
            m * m / (px * py)
        })
        .collect();
    Ok(Spectrum {
        values,
        freqs: pxy.freqs,
    })
}

/// The complex spectrum `X / sum(window)` of the whole signal.
pub fn complex_spectrum<S: SignalValue>(
    x: &[S],
    opts: SpectrumOptions,
) -> Result<Spectrum<Complex>, MlabError> {
    single(x, &opts, Mode::Complex)
}

/// The magnitude spectrum `|X| / sum(window)` of the whole signal.
pub fn magnitude_spectrum<S: SignalValue>(
    x: &[S],
    opts: SpectrumOptions,
) -> Result<Spectrum, MlabError> {
    single(x, &opts, Mode::Magnitude).map(real_parts)
}

/// The wrapped phase angle spectrum of the whole signal.
pub fn angle_spectrum<S: SignalValue>(
    x: &[S],
    opts: SpectrumOptions,
) -> Result<Spectrum, MlabError> {
    single(x, &opts, Mode::Angle).map(real_parts)
}

/// The unwrapped phase spectrum of the whole signal.
pub fn phase_spectrum<S: SignalValue>(
    x: &[S],
    opts: SpectrumOptions,
) -> Result<Spectrum, MlabError> {
    single(x, &opts, Mode::Phase).map(real_parts)
}

fn single<S: SignalValue>(
    x: &[S],
    opts: &SpectrumOptions,
    mode: Mode,
) -> Result<Spectrum<Complex>, MlabError> {
    let options = SpectralOptions {
        nfft: x.len(),
        fs: opts.fs,
        detrend: Detrend::None,
        window: opts.window.clone(),
        noverlap: Some(0),
        pad_to: Some(opts.pad_to.unwrap_or(x.len())),
        sides: opts.sides,
        scale_by_freq: false,
    };
    let raw = spectral_helper(&Signal::new(x), None, &options, 0, mode)?;
    Ok(Spectrum {
        values: raw.values.column(0),
        freqs: raw.freqs,
    })
}

fn real_parts(spectrum: Spectrum<Complex>) -> Spectrum {
    Spectrum {
        values: spectrum.values.into_iter().map(|z| z.re).collect(),
        freqs: spectrum.freqs,
    }
}

/// The segment average (matplotlib `Pxy.mean(axis=1)`).
fn average_segments(raw: Raw) -> Spectrum<Complex> {
    let count = Complex::real(raw.values.cols() as f64);
    let values = raw
        .values
        .iter_rows()
        .map(|row| {
            if row.len() == 1 {
                row[0]
            } else {
                pairwise(row, Complex::ZERO) / count
            }
        })
        .collect();
    Spectrum {
        values,
        freqs: raw.freqs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                let t = i as f64;
                0.7 + (0.3 * t).sin() + 0.4 * if i % 2 == 0 { 1.0 } else { -1.0 }
            })
            .collect()
    }

    #[test]
    fn parseval_for_rectangular_two_sided_psd() {
        let x = signal(32);
        let opts = SpectralOptions {
            nfft: 32,
            fs: 1.0,
            window: Window::Rectangular,
            sides: SpectrumSides::TwoSided,
            scale_by_freq: false,
            ..SpectralOptions::default()
        };
        let p = psd(&x, opts).unwrap();
        let power: f64 = p.values.iter().sum();
        let mean_square = x.iter().map(|v| v * v).sum::<f64>() / 32.0;
        assert!((power - mean_square).abs() < 1e-12);
        assert_eq!(p.freqs[0], -0.5);
        assert!(p.freqs.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn one_sided_frequencies_and_times() {
        let x = signal(64);
        let opts = SpectralOptions {
            nfft: 16,
            noverlap: Some(8),
            ..SpectralOptions::default()
        };
        let s = specgram(&x, opts.clone(), SpecgramMode::Psd).unwrap();
        assert_eq!(s.values.shape(), (9, 7));
        assert_eq!(s.times, vec![4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 28.0]);
        assert_eq!(s.freqs[8], 1.0);
        let c = complex_specgram(&x, opts.clone()).unwrap();
        let m = specgram(&x, opts, SpecgramMode::Magnitude).unwrap();
        for (z, v) in c.values.as_slice().iter().zip(m.values.as_slice()) {
            assert!((z.abs() - v).abs() < 1e-12);
        }
    }

    #[test]
    fn coherence_of_a_signal_with_itself_is_one() {
        let x = signal(128);
        let opts = SpectralOptions {
            nfft: 32,
            ..SpectralOptions::default()
        };
        let c = cohere(&x, &x, opts.clone()).unwrap();
        assert!(c.values.iter().all(|v| (v - 1.0).abs() < 1e-12));
        assert!(matches!(
            cohere(&x[..40], &x[..40], opts),
            Err(MlabError::SignalTooShort { .. })
        ));
    }

    #[test]
    fn single_spectra_agree() {
        let x = signal(20);
        let opts = SpectrumOptions::default();
        let c = complex_spectrum(&x, opts.clone()).unwrap();
        let m = magnitude_spectrum(&x, opts.clone()).unwrap();
        let a = angle_spectrum(&x, opts.clone()).unwrap();
        assert_eq!(c.values.len(), 11);
        for ((z, mag), ang) in c.values.iter().zip(&m.values).zip(&a.values) {
            assert!((z.abs() - mag).abs() < 1e-15 && (z.arg() - ang).abs() < 1e-15);
        }
        assert!(phase_spectrum(&x, opts.clone()).is_ok());
        assert_eq!(
            complex_spectrum::<f64>(&[], opts).unwrap_err(),
            MlabError::InvalidOverlap {
                noverlap: 0,
                nfft: 0
            }
        );
    }

    #[test]
    fn complex_signals_default_to_two_sided() {
        let z: Vec<Complex> = (0..16).map(|i| Complex::new(f64::from(i), 1.0)).collect();
        let opts = SpectralOptions {
            nfft: 16,
            ..SpectralOptions::default()
        };
        let p = psd(&z, opts.clone()).unwrap();
        assert_eq!(p.values.len(), 16);
        assert_eq!(p.freqs[0], -1.0);
        let window_err = SpectralOptions {
            window: Window::Values(vec![1.0; 3]),
            ..opts
        };
        assert!(matches!(
            psd(&z, window_err),
            Err(MlabError::WindowLength { .. })
        ));
    }
}
