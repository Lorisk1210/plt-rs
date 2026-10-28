//! Densities of many points and of signals: 2-D histograms, hexagonal
//! bins with marginal histograms, and a spectrogram.

use std::error::Error;
use std::f64::consts::PI;

use plt::axes::{
    HexGridSize, HexbinOptions, Hist2dOptions, PcolorOptions, SpecgramOptions, TickParams,
};
use plt::colors::ColorizerOptions;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::mlab::SpectralOptions;
use plt::prelude::*;
use plt::stats::{AxisBins, Bins, HistOrientation, Histogram2dOptions};

use super::data::Samples;
use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("hist2d", "A 2-D histogram (hist2d) with a colorbar", hist2d),
    Entry::figure(
        "hexbin_marginals",
        "Hexagonal bins with marginal histograms",
        hexbin_marginals,
    ),
    Entry::figure(
        "spectrogram",
        "The spectrogram of a chirp (specgram)",
        spectrogram,
    ),
];

/// Points scattered around a ring: `n` angles and radii, as x and y.
fn ring(samples: &mut Samples, n: usize) -> (Vec<f64>, Vec<f64>) {
    (0..n)
        .map(|_| {
            let angle = 2.0 * PI * samples.uniform();
            let radius = samples.normal(2.0, 0.35);
            (
                radius * angle.cos(),
                0.6 * radius * angle.sin() + 0.3 * angle.cos(),
            )
        })
        .unzip()
}

/// Ten thousand points counted on a 60 × 40 grid of bins.
fn hist2d() -> Built {
    let fig = Figure::with_size(6.4, 4.5, 100.0);
    let ax = fig.subplot();
    let (x, y) = ring(&mut Samples::new(17), 10_000);
    let options = Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: [AxisBins::Count(60), AxisBins::Count(40)],
            ..Histogram2dOptions::default()
        },
        mesh: PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("magma")?),
            ..PcolorOptions::default()
        },
        ..Hist2dOptions::default()
    };
    let hist = ax.hist2d(&x, &y, options)?;
    let colorbar = ColorbarOptions {
        label: "points per bin".into(),
        ..ColorbarOptions::default()
    };
    fig.colorbar(&hist.mesh, colorbar)?;
    ax.set_title("hist2d");
    Ok(fig)
}

/// The joint density of two variables as hexagons, with the density of
/// each variable alone on the axes above and to the right. The marginal
/// axes share the hexbin's x and y axes, so they stay aligned with it.
fn hexbin_marginals() -> Built {
    let fig = Figure::with_size(6.0, 6.0, 100.0);
    let mut samples = Samples::new(29);
    let x = samples.normals(5000, 0.0, 1.0);
    let y: Vec<f64> = x
        .iter()
        .map(|x| 0.6 * x + samples.normal(0.0, 0.7))
        .collect();
    let ratios = (Some(vec![4.0, 1.0]), Some(vec![1.0, 4.0]));
    let grid = GridSpec::with_ratios(2, 2, ratios.0, ratios.1, SubplotOverrides::default())?;
    let main = fig.add_subplot_spec(&grid.cell(1, 0)?)?;
    let top = fig.add_subplot_spec(&grid.cell(0, 0)?)?;
    let right = fig.add_subplot_spec(&grid.cell(1, 1)?)?;
    let options = HexbinOptions {
        gridsize: HexGridSize::Count(30),
        mincnt: Some(1),
        ..HexbinOptions::default()
    }
    .cmap(colormaps::get("viridis")?);
    main.hexbin(&x, &y, options)?;
    main.set_xlabel("x").set_ylabel("y");
    marginal_histograms(&main, &top, &right, &x, &y, 40)?;
    top.set_title("hexbin with marginals");
    Ok(fig)
}

/// Draws the histogram of `x` on `top` and that of `y`, sideways, on
/// `right`: the marginal distributions of the joint plot on `joint`. They
/// share its axes, so their tick labels, which would repeat its own, are
/// hidden.
pub(crate) fn marginal_histograms(
    joint: &Axes,
    top: &Axes,
    right: &Axes,
    x: &[f64],
    y: &[f64],
    bins: usize,
) -> Result<(), Box<dyn Error>> {
    top.share_x(joint)?;
    right.share_y(joint)?;
    let binned = HistPlotOptions {
        bins: Some(Bins::Count(bins)),
        ..HistPlotOptions::default()
    };
    let sideways = HistPlotOptions {
        orientation: HistOrientation::Horizontal,
        ..binned.clone()
    };
    top.hist(x, binned)?;
    right.hist(y, sideways)?;
    top.tick_params(TickParams {
        labelbottom: Some(false),
        ..TickParams::default()
    });
    right.tick_params(TickParams {
        labelleft: Some(false),
        ..TickParams::default()
    });
    Ok(())
}

/// A tone whose frequency rises over time, plus a steady tone and noise:
/// the spectrogram shows the frequency content of each short window.
fn spectrogram() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let fs = 1000.0;
    let mut samples = Samples::new(3);
    let signal: Vec<f64> = (0..10_000)
        .map(|i| {
            let t = f64::from(i) / fs;
            let chirp = (2.0 * PI * (50.0 * t + 15.0 * t * t)).sin();
            chirp + 0.5 * (2.0 * PI * 400.0 * t).sin() + samples.normal(0.0, 0.3)
        })
        .collect();
    let options = SpecgramOptions {
        spectral: SpectralOptions {
            nfft: 256,
            fs,
            noverlap: Some(192),
            ..SpectralOptions::default()
        },
        image: ImshowOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("inferno")?),
            ..ImshowOptions::default()
        },
        ..SpecgramOptions::default()
    };
    let plot = ax.specgram(&signal, options)?;
    fig.colorbar(
        &plot.image,
        ColorbarOptions {
            label: "dB/Hz".into(),
            ..ColorbarOptions::default()
        },
    )?;
    ax.set_title("A chirp, a tone and noise");
    ax.set_xlabel("time (s)").set_ylabel("frequency (Hz)");
    Ok(fig)
}
