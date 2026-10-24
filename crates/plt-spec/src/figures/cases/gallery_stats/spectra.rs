//! Spectral estimates and correlations: coherence, cross spectral
//! density, the power spectral densities of `psd_demo` (zero padding,
//! block size, overlap, periodogram against Welch, complex signals) and
//! cross- and auto-correlation.

use std::f64::consts::PI;

use plt::axes::{Axes, SpectralPlotOptions};
use plt::figure::{MosaicOptions, Share};
use plt::layout::LayoutEngine;
use plt::mlab::{Complex, SignalValue, SpectralOptions, Window, XcorrOptions};
use plt::prelude::*;

use super::SEED;
use crate::figures::cases::common::{map, mosaic};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Welch estimate options with `nfft` samples per segment at `fs` Hz.
fn welch(nfft: usize, fs: f64) -> SpectralOptions {
    SpectralOptions {
        nfft,
        fs,
        ..SpectralOptions::default()
    }
}

/// Plot options of the estimate `spectral`.
fn estimate(spectral: SpectralOptions) -> SpectralPlotOptions {
    SpectralPlotOptions {
        spectral,
        ..SpectralPlotOptions::default()
    }
}

/// `np.convolve(a, v)`: the full discrete convolution.
fn convolve(a: &[f64], v: &[f64]) -> Vec<f64> {
    (0..a.len() + v.len() - 1)
        .map(|k| {
            let first = k.saturating_sub(v.len() - 1);
            let last = k.min(a.len() - 1);
            (first..=last).map(|i| a[i] * v[k - i]).sum()
        })
        .collect()
}

/// `np.convolve(a, v, mode="same")`: the centre of the full convolution,
/// as long as the longer input.
fn convolve_same(a: &[f64], v: &[f64]) -> Vec<f64> {
    let offset = (a.len().min(v.len()) - 1) / 2;
    let length = a.len().max(v.len());
    convolve(a, v)[offset..offset + length].to_vec()
}

/// Two 10 Hz sines of `amplitude` plus two noises made by `noise` from
/// white noise (`noisy_sines`).
fn noisy_sines(
    g: &mut SpecRng,
    t: &[f64],
    amplitude: f64,
    noise: impl Fn(Vec<f64>) -> Vec<f64>,
) -> (Vec<f64>, Vec<f64>) {
    let sine = map(t, |t| amplitude * (2.0 * PI * 10.0 * t).sin());
    let nse1 = noise(g.normal(t.len(), 0.0, 1.0));
    let nse2 = noise(g.normal(t.len(), 0.0, 1.0));
    let add = |nse: Vec<f64>| sine.iter().zip(nse).map(|(s, n)| s + n).collect();
    (add(nse1), add(nse2))
}

/// The time-domain panel of `cohere` and `csd_demo` (`signals_panel`).
fn signals_panel(ax: &Axes, t: &[f64], (s1, s2): (&[f64], &[f64]), xmax: f64) -> CaseResult {
    ax.plot(t, s1)?;
    ax.plot(t, s2)?;
    ax.set_xlim(0.0, xmax);
    ax.set_xlabel("Time (s)");
    ax.set_ylabel("s1 and s2");
    ax.grid(true);
    Ok(())
}

/// Two constrained axes, one above the other.
fn stacked_pair(fig: &Figure) -> Result<(Axes, Axes), Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(2, 1)?;
    Ok((axs[(0, 0)].clone(), axs[(1, 0)].clone()))
}

fn cohere(fig: &Figure) -> CaseResult {
    let dt = 0.01;
    let t = arange(0.0, 30.0, dt);
    let (s1, s2) = noisy_sines(&mut SpecRng::new(SEED), &t, 1.0, |nse| nse);
    let (ax0, ax1) = stacked_pair(fig)?;
    signals_panel(&ax0, &t, (&s1, &s2), 2.0)?;
    ax1.cohere(&s1, &s2, estimate(welch(256, 1.0 / dt)))?;
    ax1.set_ylabel("Coherence");
    Ok(())
}

fn csd_demo(fig: &Figure) -> CaseResult {
    let (ax1, ax2) = stacked_pair(fig)?;
    let dt = 0.01;
    let t = arange(0.0, 30.0, dt);
    let r = map(&t, |t| (-t / 0.05).exp());
    let colored = |nse: Vec<f64>| map(&convolve_same(&nse, &r), |c| c * dt);
    let (s1, s2) = noisy_sines(&mut SpecRng::new(SEED), &t, 0.01, colored);
    signals_panel(&ax1, &t, (&s1, &s2), 5.0)?;
    ax2.csd(&s1, &s2, estimate(welch(256, 1.0 / dt)))?;
    ax2.set_ylabel("CSD (dB)");
    Ok(())
}

fn psd_demo_1(fig: &Figure) -> CaseResult {
    let dt = 0.01;
    let t = arange(0.0, 10.0, dt);
    let nse = SpecRng::new(SEED).normal(t.len(), 0.0, 1.0);
    let r = map(&t, |t| (-t / 0.05).exp());
    let cnse = map(&convolve(&nse, &r), |c| c * dt);
    let s: Vec<f64> = t
        .iter()
        .zip(cnse)
        .map(|(t, c)| 0.1 * (2.0 * PI * t).sin() + c)
        .collect();
    let (ax0, ax1) = stacked_pair(fig)?;
    ax0.plot(&t, &s)?;
    ax0.set_xlabel("Time (s)");
    ax0.set_ylabel("Signal");
    ax1.psd(&s, estimate(welch(512, 1.0 / dt)))?;
    Ok(())
}

fn psd_demo_2(fig: &Figure) -> CaseResult {
    let dt = PI / 100.0;
    let fs = 1.0 / dt;
    let t = arange(0.0, 8.0, dt);
    let noise = SpecRng::new(SEED).normal(t.len(), 0.0, 1.0);
    let y: Vec<f64> = t
        .iter()
        .zip(noise)
        .map(|(t, e)| 10.0 * (2.0 * PI * 4.0 * t).sin() + 5.0 * (2.0 * PI * 4.25 * t).sin() + e)
        .collect();
    let n = t.len();
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let layout = mosaic(&[
        &["signal", "signal", "signal"],
        &["zero padding", "block size", "overlap"],
    ])?;
    let axs = fig.subplot_mosaic(&layout, MosaicOptions::default())?;
    let panel = |label: &str| {
        axs.get(label)
            .ok_or_else(|| format!("no axes labelled {label}"))
    };
    let signal = panel("signal")?;
    signal.plot(&t, &y)?;
    signal.set_xlabel("Time (s)");
    signal.set_ylabel("Signal");
    let padded = |nfft: usize, pad_to: usize, noverlap: usize| {
        estimate(SpectralOptions {
            pad_to: Some(pad_to),
            noverlap: Some(noverlap),
            ..welch(nfft, fs)
        })
    };
    let zero_padding = panel("zero padding")?;
    for pad_to in [n, n * 2, n * 4] {
        zero_padding.psd(&y, padded(n, pad_to, 0))?;
    }
    let block_size = panel("block size")?;
    for nfft in [n, n / 2, n / 4] {
        block_size.psd(&y, padded(nfft, n, 0))?;
    }
    block_size.set_ylabel("");
    let overlap = panel("overlap")?;
    // `int(0.025 * len(t))` and `int(0.1 * len(t))`.
    let overlaps = [0.0, 0.025, 0.1].map(|fraction: f64| (fraction * n as f64) as usize);
    for noverlap in overlaps {
        overlap.psd(&y, padded(n / 2, n, noverlap))?;
    }
    overlap.set_ylabel("");
    overlap.set_title("overlap");
    for (title, ax) in axs.iter().filter(|(title, _)| *title != "signal") {
        ax.set_title(title);
        ax.share_x(zero_padding)?;
        ax.share_y(zero_padding)?;
    }
    Ok(())
}

/// `psd_demo`'s comparison of a periodogram with Welch's estimate of the
/// samples `xn` (`periodogram_and_welch`).
fn periodogram_and_welch<S: SignalValue>(fig: &Figure, xn: &[S], xticks: &[f64]) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    let yticks = arange(-50.0, 30.0, 10.0);
    let unwindowed = |nfft: usize, pad_to: usize, noverlap: usize| {
        estimate(SpectralOptions {
            window: Window::Rectangular,
            pad_to: Some(pad_to),
            noverlap: Some(noverlap),
            scale_by_freq: true,
            ..welch(nfft, 1000.0)
        })
    };
    let panels = [
        ("Periodogram", unwindowed(301, 1024, 0)),
        ("Welch", unwindowed(150, 512, 75)),
    ];
    for (ax, (title, options)) in axs.as_slice().iter().zip(panels) {
        ax.psd(xn, options)?;
        ax.set_title(title);
        ax.set_yticks(yticks.clone());
        ax.set_xticks(xticks.to_vec());
        ax.grid(true);
        ax.set_ylim(yticks[0], yticks[yticks.len() - 1]);
    }
    axs[(0, 1)].set_ylabel("");
    Ok(())
}

/// The amplitudes and frequencies of `psd_demo`'s two close tones
/// (`TONES`).
const TONES: [(f64, f64); 2] = [(2.0, 150.0), (8.0, 140.0)];

/// The 301 sample times of the two tones and their noise.
fn tone_times_and_noise() -> (Vec<f64>, Vec<f64>) {
    let t = linspace(0.0, 0.3, 301);
    let noise = SpecRng::new(SEED).normal(t.len(), 0.0, 1.0);
    (t, noise)
}

fn psd_demo_3(fig: &Figure) -> CaseResult {
    let (t, noise) = tone_times_and_noise();
    let xn: Vec<f64> = t
        .iter()
        .zip(noise)
        .map(|(t, e)| {
            let tones: f64 = TONES
                .iter()
                .map(|(a, f)| a * (2.0 * PI * f * t).sin())
                .sum();
            tones + 5.0 * e
        })
        .collect();
    periodogram_and_welch(fig, &xn, &arange(0.0, 550.0, 100.0))
}

fn psd_demo_4(fig: &Figure) -> CaseResult {
    let (t, noise) = tone_times_and_noise();
    let xn: Vec<Complex> = t
        .iter()
        .zip(noise)
        .map(|(t, e)| {
            let (re, im) = TONES.iter().fold((0.0, 0.0), |(re, im), (a, f)| {
                let phase = 2.0 * PI * f * t;
                (re + a * phase.cos(), im + a * phase.sin())
            });
            Complex::new(re + 5.0 * e, im)
        })
        .collect();
    periodogram_and_welch(fig, &xn, &arange(-500.0, 550.0, 200.0))
}

fn xcorr_acorr_demo(fig: &Figure) -> CaseResult {
    let draws = SpecRng::new(SEED).normal(200, 0.0, 1.0);
    let (x, y) = draws.split_at(100);
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let options = XcorrOptions {
        normed: true,
        maxlags: Some(50),
        ..XcorrOptions::default()
    };
    let thick = || LineProps::default().linewidth(2.0);
    axs[(0, 0)].xcorr(x, y, options.clone(), true, thick())?;
    axs[(0, 0)].grid(true);
    axs[(0, 0)].set_title("Cross-correlation (xcorr)");
    axs[(1, 0)].acorr(x, options, true, thick())?;
    axs[(1, 0)].grid(true);
    axs[(1, 0)].set_title("Auto-correlation (acorr)");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/cohere", cohere),
    FigureCase::new("gallery_stats/csd_demo", csd_demo),
    FigureCase::new("gallery_stats/psd_demo_1", psd_demo_1),
    FigureCase::new("gallery_stats/psd_demo_2", psd_demo_2),
    FigureCase::new("gallery_stats/psd_demo_3", psd_demo_3),
    FigureCase::new("gallery_stats/psd_demo_4", psd_demo_4),
    FigureCase::new("gallery_stats/xcorr_acorr_demo", xcorr_acorr_demo),
];
