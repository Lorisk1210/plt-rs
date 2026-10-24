//! Spectral analysis and signal helpers (matplotlib.mlab).
//!
//! * [`fft`] / [`FftPlan`] — arbitrary-length discrete Fourier transforms
//!   on the crate's own [`Complex`] type, and [`fftfreq`].
//! * [`Window`], [`hanning`], [`blackman`], [`window_hanning`], [`window_none`] —
//!   segment tapers.
//! * [`Detrend`], [`detrend_mean`], [`detrend_linear`], [`detrend_none`],
//!   [`detrend_grid`] — trend removal.
//! * [`psd`], [`csd`], [`specgram`], [`complex_specgram`], [`cohere`] —
//!   Welch estimates configured by [`SpectralOptions`]; [`complex_spectrum`],
//!   [`magnitude_spectrum`], [`angle_spectrum`], [`phase_spectrum`] —
//!   single-segment spectra configured by [`SpectrumOptions`].
//! * [`xcorr`], [`acorr`], [`correlate_full`] — the correlations behind
//!   `Axes.xcorr` / `Axes.acorr`.
//!
//! Spectral functions accept real (`f64`) or [`Complex`] samples through
//! [`SignalValue`].
//!
//! ```
//! use plt::mlab::{Detrend, SpectralOptions, psd};
//! let x: Vec<f64> = (0..256).map(|i| (f64::from(i) * 0.3).sin() + 1.0).collect();
//! let opts = SpectralOptions { nfft: 64, detrend: Detrend::Mean, ..SpectralOptions::default() };
//! let p = psd(&x, opts).unwrap();
//! let peak = (0..p.values.len()).max_by(|&a, &b| p.values[a].total_cmp(&p.values[b])).unwrap();
//! assert!((p.freqs[peak] - 0.3 / std::f64::consts::PI).abs() < 2.0 / 64.0);
//! ```
#![allow(
    clippy::float_cmp,
    reason = "numpy's unwrap and binning compare floats exactly; the ports keep them"
)]

mod complex;
mod correlate;
mod detrend;
mod fft;
mod spectral;
mod stride;
mod window;

use std::fmt;

pub use crate::stats::{BandwidthMethod, GaussianKde};
pub use complex::Complex;
pub use correlate::{Correlation, XcorrOptions, acorr, correlate_full, xcorr};
pub use detrend::{Detrend, DetrendFn, detrend_grid, detrend_linear, detrend_mean, detrend_none};
pub use fft::{FftPlan, fft, fft_n, fftfreq, ifft};
pub use spectral::{
    Sides, SignalValue, Specgram, SpecgramMode, SpectralOptions, Spectrum, SpectrumOptions,
    angle_spectrum, cohere, complex_specgram, complex_spectrum, csd, magnitude_spectrum,
    phase_spectrum, psd, specgram,
};
pub use stride::stride_windows;
pub use window::{Window, blackman, hanning, window_hanning, window_none};

/// Why a signal-processing routine rejected its input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MlabError {
    /// The overlap is not smaller than the segment length (which includes
    /// a zero segment length, e.g. an empty signal for single spectra).
    InvalidOverlap {
        /// Requested overlap.
        noverlap: usize,
        /// Segment length.
        nfft: usize,
    },
    /// A zero FFT length (`pad_to = 0`).
    ZeroLengthFft,
    /// An explicit window whose length is not the segment length.
    WindowLength {
        /// Segment length.
        expected: usize,
        /// Window length.
        actual: usize,
    },
    /// Cross spectra of signals with incompatible segment counts.
    SegmentCountMismatch {
        /// Segments of `x`.
        x: usize,
        /// Segments of `y`.
        y: usize,
    },
    /// Coherence needs at least two segments.
    SignalTooShort {
        /// Signal length.
        len: usize,
        /// Segment length.
        nfft: usize,
    },
    /// Correlated signals of different lengths.
    LengthMismatch {
        /// Length of `x`.
        x: usize,
        /// Length of `y`.
        y: usize,
    },
    /// `maxlags` outside `1..len`.
    InvalidMaxLags {
        /// Requested maximum lag.
        maxlags: usize,
        /// Signal length.
        len: usize,
    },
    /// An axis beyond the array's dimensions.
    AxisOutOfBounds {
        /// Requested axis.
        axis: usize,
        /// Number of dimensions.
        ndim: usize,
    },
    /// Linear or custom detrending of a 2-D array needs an axis.
    AxisRequired,
    /// A custom detrend function changed the number of samples.
    DetrendLength {
        /// Samples passed in.
        expected: usize,
        /// Samples returned.
        actual: usize,
    },
}

impl fmt::Display for MlabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOverlap { noverlap, nfft } => {
                write!(f, "noverlap ({noverlap}) must be less than NFFT ({nfft})")
            }
            Self::ZeroLengthFft => f.write_str("the FFT length must be positive"),
            Self::WindowLength { expected, actual } => write!(
                f,
                "the window length ({actual}) must match the segment length ({expected})"
            ),
            Self::SegmentCountMismatch { x, y } => {
                write!(f, "cannot pair {x} segments of x with {y} segments of y")
            }
            Self::SignalTooShort { len, nfft } => write!(
                f,
                "a signal of {len} samples is too short for coherence with NFFT {nfft}"
            ),
            Self::LengthMismatch { x, y } => {
                write!(f, "x and y must have equal length ({x} != {y})")
            }
            Self::InvalidMaxLags { maxlags, len } => {
                write!(
                    f,
                    "maxlags ({maxlags}) must be positive and less than {len}"
                )
            }
            Self::AxisOutOfBounds { axis, ndim } => {
                write!(f, "axis {axis} is out of bounds for {ndim} dimensions")
            }
            Self::AxisRequired => f.write_str("this detrend method needs an axis for 2-D data"),
            Self::DetrendLength { expected, actual } => write!(
                f,
                "the detrend function returned {actual} values for {expected} samples"
            ),
        }
    }
}

impl std::error::Error for MlabError {}
