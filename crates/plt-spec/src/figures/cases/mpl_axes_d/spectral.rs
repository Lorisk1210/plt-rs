//! Spectrograms, power and cross spectral densities and single-segment
//! spectra (`test_specgram*`, `test_psd_csd`, `test_spectrum`).
//!
//! The tests draw one figure per signal and option set; the cases are the
//! product of the tests' signals ("freqs": summed sines, "noise": the
//! seeded generator) with their option sets, in the tests' order.

use std::f64::consts::TAU;

use plt::axes::{Axes, SpecgramOptions, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale};
use plt::colors::{Colorize, ColorizerOptions, LogNorm};
use plt::figure::Figure;
use plt::mlab::{Sides, SpecgramMode, SpectralOptions, SpectrumOptions};
use plt::prelude::arange;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The three `sides` of each figure's stacked axes.
const SIDES: [Sides; 3] = [Sides::Default, Sides::OneSided, Sides::TwoSided];

/// One sum of sines of `x` per row of frequencies (`np.sin(2 * np.pi *
/// np.multiply.outer(fstims, x)).sum(axis=1)`).
fn summed_sines(fstims: &[[f64; 3]], x: &[f64]) -> Vec<Vec<f64>> {
    fstims
        .iter()
        .map(|row| {
            x.iter()
                .map(|&x| row.iter().map(|f| (TAU * (f * x)).sin()).sum())
                .collect()
        })
        .collect()
}

/// The tests' `standard_normal(n)` and `rand(n)` after seeding with 0.
fn noise(n: usize) -> [Vec<f64>; 2] {
    let mut stream = SpecRng::new(0);
    let normal = stream.normal(n, 0.0, 1.0);
    [normal, stream.uniform(n, 0.0, 1.0)]
}

/// Python's `int(value)` for the tests' positive segment lengths.
fn int(value: f64) -> usize {
    value as usize
}

/// The next power of two of `nfft` (`2 ** ceil(log2(nfft))`).
fn pad_to(nfft: usize) -> usize {
    nfft.next_power_of_two()
}

/// Three axes stacked in a column.
fn stacked(fig: &Figure) -> Result<Vec<Axes>, Box<dyn std::error::Error>> {
    Ok(fig.subplots(3, 1)?.into_vec())
}

/// Welch estimate options with half-overlapping `nfft`-sample segments.
fn welch(fs: f64, nfft: usize, sides: Sides) -> SpectralOptions {
    SpectralOptions {
        nfft,
        fs,
        noverlap: Some(nfft / 2),
        pad_to: Some(pad_to(nfft)),
        sides,
        ..SpectralOptions::default()
    }
}

const SPECGRAM_N: f64 = 1000.0;
const SPECGRAM_FS: f64 = 10.0;
const SPECGRAM_FSTIMS: [[f64; 3]; 2] = [
    [SPECGRAM_FS / 4.0, SPECGRAM_FS / 5.0, SPECGRAM_FS / 11.0],
    [SPECGRAM_FS / 4.7, SPECGRAM_FS / 5.6, SPECGRAM_FS / 11.9],
];

/// The specgram tests' summed sines end to end, each row's last sample
/// optionally replaced, with `NFFT = nfft_scale * Fs / min(fstims)`.
fn specgram_freqs(nfft_scale: f64, last_sample: Option<f64>) -> (Vec<f64>, usize) {
    let x = arange(0.0, SPECGRAM_N, 1.0 / SPECGRAM_FS);
    let mut rows = summed_sines(&SPECGRAM_FSTIMS, &x);
    if let Some(last) = last_sample {
        for row in &mut rows {
            if let Some(sample) = row.last_mut() {
                *sample = last;
            }
        }
    }
    let lowest = SPECGRAM_FS / 11.9;
    (rows.concat(), int(nfft_scale * SPECGRAM_FS / lowest))
}

/// The specgram tests' noise and its `NFFT`.
fn specgram_noise() -> (Vec<f64>, usize) {
    (noise(1000).concat(), int(10.0 * SPECGRAM_FS / 11.0))
}

/// How one specgram figure draws its signal.
#[derive(Clone, Copy)]
struct SpecgramKind {
    mode: SpecgramMode,
    /// `scale="linear", norm=LogNorm()`.
    linear_log: bool,
}

const PSD: SpecgramKind = SpecgramKind {
    mode: SpecgramMode::Psd,
    linear_log: false,
};

/// Three stacked specgrams of `signal`, one per `sides`.
fn specgrams(fig: &Figure, (y, nfft): (Vec<f64>, usize), kind: SpecgramKind) -> CaseResult {
    for (ax, sides) in stacked(fig)?.iter().zip(SIDES) {
        let mut options = SpecgramOptions {
            spectral: welch(SPECGRAM_FS, nfft, sides),
            mode: kind.mode,
            ..SpecgramOptions::default()
        };
        if kind.linear_log {
            options.scale = Some(SpectrumScale::Linear);
            options.image.colorizer = ColorizerOptions::default().norm(LogNorm::default());
        }
        ax.specgram(&y, options)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// A specgram case of the default style with nearest interpolation.
const fn specgram_case(name: &'static str, build: fn(&Figure) -> CaseResult) -> FigureCase {
    FigureCase::new(name, build).with_rc(&[("image.interpolation", "nearest")])
}

/// `test_specgram`, `test_specgram_magnitude` and `test_specgram_angle`.
const SPECGRAMS: &[FigureCase] = &[
    specgram_case("mpl_axes_d/specgram_1", |fig| {
        specgrams(fig, specgram_freqs(10.0, None), PSD)
    }),
    specgram_case("mpl_axes_d/specgram_2", |fig| {
        specgrams(fig, specgram_freqs(10.0, None), LINEAR_LOG)
    }),
    specgram_case("mpl_axes_d/specgram_3", |fig| {
        specgrams(fig, specgram_noise(), PSD)
    }),
    specgram_case("mpl_axes_d/specgram_4", |fig| {
        specgrams(fig, specgram_noise(), LINEAR_LOG)
    }),
    specgram_case("mpl_axes_d/specgram_magnitude_1", |fig| {
        specgrams(fig, specgram_freqs(100.0, Some(1.0)), MAGNITUDE)
    }),
    specgram_case("mpl_axes_d/specgram_magnitude_2", |fig| {
        specgrams(fig, specgram_freqs(100.0, Some(1.0)), MAGNITUDE_LINEAR_LOG)
    }),
    specgram_case("mpl_axes_d/specgram_magnitude_3", |fig| {
        specgrams(fig, specgram_noise(), MAGNITUDE)
    }),
    specgram_case("mpl_axes_d/specgram_magnitude_4", |fig| {
        specgrams(fig, specgram_noise(), MAGNITUDE_LINEAR_LOG)
    }),
    specgram_case("mpl_axes_d/specgram_angle_1", |fig| {
        specgrams(fig, specgram_freqs(10.0, Some(1.0)), ANGLE)
    }),
    specgram_case("mpl_axes_d/specgram_angle_2", |fig| {
        specgrams(fig, specgram_freqs(10.0, Some(1.0)), PHASE)
    }),
    specgram_case("mpl_axes_d/specgram_angle_3", |fig| {
        specgrams(fig, specgram_noise(), ANGLE)
    }),
    specgram_case("mpl_axes_d/specgram_angle_4", |fig| {
        specgrams(fig, specgram_noise(), PHASE)
    }),
    classic("mpl_axes_d/specgram_origin_rcparam", |fig| {
        let signal: Vec<f64> = (0..500).map(|t| f64::from(t).sin()).collect();
        fig.subplot()
            .specgram(&signal, SpecgramOptions::default())?;
        Ok(())
    })
    .with_rc_setup(|rc| Ok(rc.set("image.origin", "lower")?)),
];

const LINEAR_LOG: SpecgramKind = SpecgramKind {
    linear_log: true,
    ..PSD
};
const MAGNITUDE: SpecgramKind = SpecgramKind {
    mode: SpecgramMode::Magnitude,
    linear_log: false,
};
const MAGNITUDE_LINEAR_LOG: SpecgramKind = SpecgramKind {
    linear_log: true,
    ..MAGNITUDE
};
const ANGLE: SpecgramKind = SpecgramKind {
    mode: SpecgramMode::Angle,
    linear_log: false,
};
const PHASE: SpecgramKind = SpecgramKind {
    mode: SpecgramMode::Phase,
    linear_log: false,
};

const PSD_N: f64 = 10000.0;
const PSD_FS: f64 = 100.0;

/// The psd/csd tests' two signals and their `NFFT`.
fn psd_signals(freqs: bool) -> ([Vec<f64>; 2], usize) {
    if freqs {
        let fstims = [
            [PSD_FS / 4.0, PSD_FS / 5.0, PSD_FS / 11.0],
            [PSD_FS / 4.7, PSD_FS / 5.6, PSD_FS / 11.9],
        ];
        let x = arange(0.0, PSD_N, 1.0 / PSD_FS);
        let [a, b]: [Vec<f64>; 2] = summed_sines(&fstims, &x)
            .try_into()
            .expect("two rows of frequencies");
        ([a, b], int(1000.0 * PSD_FS / (PSD_FS / 11.9)))
    } else {
        (noise(10000), int(1000.0 * PSD_FS / 11.0))
    }
}

/// `psd` of both signals end to end, or their `csd`, once per `sides`.
fn psd_csd(fig: &Figure, freqs: bool, cross: bool) -> CaseResult {
    let ([a, b], nfft) = psd_signals(freqs);
    for (ax, sides) in stacked(fig)?.iter().zip(SIDES) {
        let options = SpectralPlotOptions {
            spectral: welch(PSD_FS, nfft, sides),
            ..SpectralPlotOptions::default()
        };
        if cross {
            ax.csd(&a, &b, options)?;
        } else {
            ax.psd(&[a.as_slice(), b.as_slice()].concat(), options)?;
        }
        ax.set_xlabel("").set_ylabel("");
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// The spectrum test's three weighted sines, or its noise minus one half.
fn spectrum_signal(freqs: bool) -> Vec<f64> {
    if freqs {
        let fstims = [PSD_FS / 4.0, PSD_FS / 5.0, PSD_FS / 11.0];
        arange(0.0, PSD_N, 1.0 / PSD_FS)
            .iter()
            .map(|&x| {
                fstims
                    .iter()
                    .zip([1.0, 10.0, 100.0])
                    .map(|(f, weight)| (TAU * (x * f)).sin() * weight)
                    .sum()
            })
            .collect()
    } else {
        noise(10000).concat().iter().map(|v| v - 0.5).collect()
    }
}

/// Which single-segment spectrum a figure plots.
#[derive(Clone, Copy)]
enum SpectrumKind {
    Magnitude(SpectrumScale),
    Angle,
    Phase,
}

/// One spectrum of the test's signal per `sides`.
fn spectrum(fig: &Figure, freqs: bool, kind: SpectrumKind) -> CaseResult {
    let y = spectrum_signal(freqs);
    let nfft = int(1000.0 * PSD_FS / (PSD_FS / 11.0));
    for (ax, sides) in stacked(fig)?.iter().zip(SIDES) {
        let options = SpectrumPlotOptions {
            spectrum: SpectrumOptions {
                fs: PSD_FS,
                pad_to: Some(pad_to(nfft)),
                sides,
                ..SpectrumOptions::default()
            },
            ..SpectrumPlotOptions::default()
        };
        match kind {
            SpectrumKind::Magnitude(scale) => ax.magnitude_spectrum(&y, scale, options)?,
            SpectrumKind::Angle => ax.angle_spectrum(&y, options)?,
            SpectrumKind::Phase => ax.phase_spectrum(&y, options)?,
        };
        ax.set_xlabel("").set_ylabel("");
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

const LINEAR: SpectrumKind = SpectrumKind::Magnitude(SpectrumScale::Linear);
const DECIBELS: SpectrumKind = SpectrumKind::Magnitude(SpectrumScale::Decibels);

/// `test_psd_csd` and `test_spectrum`.
const SPECTRA: &[FigureCase] = &[
    classic("mpl_axes_d/psd_csd_1", |fig| psd_csd(fig, true, false)),
    classic("mpl_axes_d/psd_csd_2", |fig| psd_csd(fig, true, true)),
    classic("mpl_axes_d/psd_csd_3", |fig| psd_csd(fig, false, false)),
    classic("mpl_axes_d/psd_csd_4", |fig| psd_csd(fig, false, true)),
    classic("mpl_axes_d/spectrum_1", |fig| spectrum(fig, true, LINEAR)),
    classic("mpl_axes_d/spectrum_2", |fig| spectrum(fig, true, DECIBELS)),
    classic("mpl_axes_d/spectrum_3", |fig| {
        spectrum(fig, true, SpectrumKind::Angle)
    }),
    classic("mpl_axes_d/spectrum_4", |fig| {
        spectrum(fig, true, SpectrumKind::Phase)
    }),
    classic("mpl_axes_d/spectrum_5", |fig| spectrum(fig, false, LINEAR)),
    classic("mpl_axes_d/spectrum_6", |fig| {
        spectrum(fig, false, DECIBELS)
    }),
    classic("mpl_axes_d/spectrum_7", |fig| {
        spectrum(fig, false, SpectrumKind::Angle)
    }),
    classic("mpl_axes_d/spectrum_8", |fig| {
        spectrum(fig, false, SpectrumKind::Phase)
    }),
];

/// Every spectral case.
pub(super) fn cases() -> Vec<FigureCase> {
    [SPECGRAMS, SPECTRA].concat()
}
