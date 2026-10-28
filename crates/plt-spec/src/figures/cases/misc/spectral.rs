//! Spectral analysis cases of `figures/cases_axes_misc.py`.

use std::f64::consts::PI;

use plt::axes::{SpecgramOptions, SpectralPlotOptions, SpectrumPlotOptions, SpectrumScale};
use plt::gridspec::SubplotOverrides;
use plt::mlab::{SpectralOptions, SpectrumOptions, XcorrOptions};
use plt::prelude::*;

use crate::figures::FigureCase;
use crate::rng::SpecRng;

/// `_signal(n, fs=100, seed)`: two tones plus deterministic noise.
fn signal(n: usize, seed: u64) -> Vec<f64> {
    let noise = SpecRng::new(seed).normal(n, 0.0, 1.0);
    (0..n)
        .zip(noise)
        .map(|(i, noise)| {
            let t = i as f64 / 100.0;
            (2.0 * PI * 10.0 * t).sin() + 0.5 * (2.0 * PI * 23.0 * t).sin() + 0.3 * noise
        })
        .collect()
}

/// Welch estimate options with `nfft` samples per segment at 100 Hz.
fn welch(nfft: usize) -> SpectralPlotOptions {
    SpectralPlotOptions {
        spectral: SpectralOptions {
            nfft,
            fs: 100.0,
            ..SpectralOptions::default()
        },
        ..SpectralPlotOptions::default()
    }
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("misc/specgram", |fig| {
        let noise = SpecRng::new(62).normal(2048, 0.0, 1.0);
        let x: Vec<f64> = noise
            .iter()
            .enumerate()
            .map(|(i, noise)| {
                let t = i as f64 / 200.0;
                (2.0 * PI * (5.0 + 4.0 * t) * t).sin() + 0.2 * noise
            })
            .collect();
        let options = SpecgramOptions {
            spectral: SpectralOptions {
                nfft: 128,
                fs: 200.0,
                noverlap: Some(64),
                ..SpectralOptions::default()
            },
            ..SpecgramOptions::default()
        };
        fig.subplot().specgram(&x, options)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/psd", |fig| {
        fig.subplot().psd(&signal(1024, 61), welch(256))?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/csd", |fig| {
        fig.subplot()
            .csd(&signal(1024, 63), &signal(1024, 64), welch(128))?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/cohere", |fig| {
        fig.subplot()
            .cohere(&signal(1024, 65), &signal(1024, 66), welch(128))?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("misc/spectrum_magnitude_angle_phase", |fig| {
        let axs = fig.subplots(3, 1)?;
        let x = signal(256, 61);
        let options = SpectrumPlotOptions {
            spectrum: SpectrumOptions {
                fs: 100.0,
                ..SpectrumOptions::default()
            },
            ..SpectrumPlotOptions::default()
        };
        axs[(0, 0)].magnitude_spectrum(&x, SpectrumScale::Decibels, options.clone())?;
        axs[(1, 0)].angle_spectrum(&x, options.clone())?;
        axs[(2, 0)].phase_spectrum(&x, options)?;
        fig.subplots_adjust(SubplotOverrides {
            hspace: Some(0.7),
            ..SubplotOverrides::default()
        })?;
        Ok(())
    })
    .sized(5.0, 6.0, 100.0),
    FigureCase::new("misc/acorr_xcorr", |fig| {
        let axs = fig.subplots(1, 2)?;
        let x = signal(200, 67);
        let lags = |maxlags| XcorrOptions {
            maxlags: Some(maxlags),
            ..XcorrOptions::default()
        };
        axs[(0, 0)].acorr(&x, lags(30), true, LineProps::default())?;
        axs[(0, 1)].xcorr(
            &x,
            &signal(200, 68),
            lags(20),
            false,
            LineProps::default()
                .marker(Marker::Symbol(MarkerSymbol::Circle))
                .markersize(3.0),
        )?;
        Ok(())
    })
    .sized(6.0, 3.0, 100.0),
];
