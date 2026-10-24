//! Probe operations for the `mpl_mlab` area: matplotlib's own
//! `tests/test_mlab.py` (see `spec/reference/probes/test_mpl_mlab.py`).
//! Most are the `stats` area's operations; `TestSpectral`'s summaries of
//! the spectra of its fixture signal are in [`spectral`].

mod spectral;

use super::stats::{cohere, detrend, detrend_mean, gaussian_kde, psd, window_hanning, window_none};
use crate::probes::Op;

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("mlab.window_hanning", window_hanning),
    Op::new("mlab.window_none", window_none),
    Op::new("mlab.psd", psd),
    Op::new("mlab.cohere", cohere),
    Op::new("mlab.detrend", detrend),
    Op::new("mlab.detrend_mean", detrend_mean),
    Op::new("mlab.GaussianKDE", gaussian_kde),
    Op::new("mlab.psd.summary", spectral::psd),
    Op::new("mlab.csd.summary", spectral::csd),
    Op::new("mlab.specgram.summary", spectral::specgram),
    Op::new("mlab.complex_spectrum.summary", spectral::complex_spectrum),
    Op::new(
        "mlab.magnitude_spectrum.summary",
        spectral::magnitude_spectrum,
    ),
    Op::new("mlab.angle_spectrum.summary", spectral::angle_spectrum),
    Op::new("mlab.phase_spectrum.summary", spectral::phase_spectrum),
];
