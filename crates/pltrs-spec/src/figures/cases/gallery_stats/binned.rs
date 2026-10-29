//! Large samples binned in two dimensions or with many bins: hexagonal
//! binning, the histograms of `hist` (coloured bars, percentages,
//! `hist2d`) and a time-series histogram of a thousand random walks.

use pltrs::artist::ArtistHandle;
use pltrs::axes::{
    Axes, HexBins, HexGridSize, HexbinOptions, Hist2dOptions, HistArtists, HistPlotOptions, Mesh,
    PcolorOptions,
};
use pltrs::collections::Collection;
use pltrs::colorbar::ColorbarOptions;
use pltrs::colors::{ColorizerOptions, Colormap, LogNorm, Norm, Normalize, colormaps};
use pltrs::figure::Share;
use pltrs::layout::LayoutEngine;
use pltrs::numeric::interp;
use pltrs::prelude::*;
use pltrs::stats::{AxisBins, Bins, Histogram2dOptions, histogram2d};
use pltrs::ticker::PercentFormatter;

use super::SEED;
use crate::figures::cases::common::{cumsum, extremes};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `fig.colorbar(mappable, ax=ax, label=label)`.
fn labelled_colorbar(fig: &Figure, mappable: &Collection, ax: &Axes, label: &str) -> CaseResult {
    let options = ColorbarOptions {
        label: label.into(),
        ..ColorbarOptions::beside(ax)
    };
    fig.colorbar(mappable, options)?;
    Ok(())
}

fn hexbin_demo(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let n = 100_000;
    let x = g.normal(n, 0.0, 1.0);
    let noise = g.normal(n, 0.0, 1.0);
    let y: Vec<f64> = x
        .iter()
        .zip(noise)
        .map(|(x, e)| 2.0 + 3.0 * x + 4.0 * e)
        .collect();
    let (xlim, ylim) = (extremes(&x), extremes(&y));
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    let panels = [
        (None, "Hexagon binning"),
        (Some(HexBins::Log), "With a log color scale"),
    ];
    for (ax, (bins, title)) in axs.as_slice().iter().zip(panels) {
        let options = HexbinOptions {
            gridsize: HexGridSize::Count(50),
            bins,
            colorizer: ColorizerOptions::default().cmap(colormaps::get("inferno")?),
            ..HexbinOptions::default()
        };
        let hb = ax.hexbin(&x, &y, options)?;
        ax.set_xlim(xlim.0, xlim.1)?.set_ylim(ylim.0, ylim.1)?;
        ax.set_title(title);
        labelled_colorbar(fig, &hb, ax, "counts")?;
    }
    Ok(())
}

/// `hist`'s data: a standard normal and a narrow one around 5
/// (`two_distributions`).
fn two_distributions() -> (Vec<f64>, Vec<f64>) {
    let mut g = SpecRng::new(SEED);
    let dist1 = g.normal(100_000, 0.0, 1.0);
    let dist2 = g
        .normal(100_000, 0.0, 1.0)
        .into_iter()
        .map(|v| 0.4 * v + 5.0)
        .collect();
    (dist1, dist2)
}

/// Twenty bins and default styling, optionally as a density.
fn twenty_bins(density: bool) -> HistPlotOptions<'static> {
    HistPlotOptions {
        bins: Some(Bins::Count(20)),
        density,
        ..HistPlotOptions::default()
    }
}

fn hist_1(fig: &Figure) -> CaseResult {
    let (dist1, dist2) = two_distributions();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    axs[(0, 0)].hist(&[dist1], twenty_bins(false))?;
    axs[(0, 1)].hist(&[dist2], twenty_bins(false))?;
    Ok(())
}

fn hist_2(fig: &Figure) -> CaseResult {
    let (dist1, _) = two_distributions();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots(1, 2)?;
    let plot = axs[(0, 0)].hist(std::slice::from_ref(&dist1), twenty_bins(false))?;
    let counts = &plot.tops[0];
    let highest = counts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let fracs: Vec<f64> = counts.iter().map(|n| n / highest).collect();
    let (low, high) = extremes(&fracs);
    let normalized = Normalize::new(low, high).normalize(&fracs)?;
    let viridis = colormaps::get("viridis")?;
    if let HistArtists::Bars(bars) = &plot.artists {
        for (frac, patch) in normalized.into_iter().zip(&bars[0].patches) {
            patch.set_facecolor(viridis.lookup(frac, None));
        }
    }
    axs[(0, 1)].hist(&[dist1], twenty_bins(true))?;
    axs[(0, 1)]
        .yaxis()
        .set_major_formatter(PercentFormatter::new(1.0));
    Ok(())
}

fn hist_3(fig: &Figure) -> CaseResult {
    let (dist1, dist2) = two_distributions();
    fig.set_layout_engine(LayoutEngine::tight())?;
    fig.subplot()
        .hist2d(&dist1, &dist2, Hist2dOptions::default())?;
    Ok(())
}

/// `hist2d` options with `bins` and optionally a log norm.
fn binned(bins: [usize; 2], log: bool) -> Hist2dOptions<'static> {
    let colors = if log {
        ColorizerOptions::default().norm(LogNorm::default())
    } else {
        ColorizerOptions::default()
    };
    Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: bins.map(AxisBins::Count),
            ..Histogram2dOptions::default()
        },
        mesh: PcolorOptions {
            colorizer: colors,
            ..PcolorOptions::default()
        },
        ..Hist2dOptions::default()
    }
}

fn hist_4(fig: &Figure) -> CaseResult {
    let (dist1, dist2) = two_distributions();
    fig.set_layout_engine(LayoutEngine::tight())?;
    let axs = fig.subplots_shared(3, 1, Share::All, Share::All)?;
    let panels = [([40, 40], false), ([40, 40], true), ([80, 10], true)];
    for (ax, (bins, log)) in axs.as_slice().iter().zip(panels) {
        ax.hist2d(&dist1, &dist2, binned(bins, log))?;
    }
    Ok(())
}

/// `time_series_histogram`'s series: random walks, the last tenth replaced
/// by noisy sines scaled like the walks.
fn walks_and_sines(g: &mut SpecRng, x: &[f64]) -> Vec<Vec<f64>> {
    let (num_series, num_points) = (1000, x.len());
    let num_signal = num_series / 10;
    let mut series: Vec<Vec<f64>> = g
        .normal(num_series * num_points, 0.0, 1.0)
        .chunks_exact(num_points)
        .map(cumsum)
        .collect();
    let phi: Vec<f64> = g
        .normal(num_signal, 0.0, 1.0)
        .into_iter()
        .map(|v| (std::f64::consts::PI / 8.0) * v)
        .collect();
    let noise = g.normal(num_signal * num_points, 0.0, 1.0);
    for (k, row) in series[num_series - num_signal..].iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            let jitter = 0.05 * noise[k * num_points + j];
            *value = (j as f64).sqrt() * ((x[j] - phi[k]).sin() + jitter);
        }
    }
    series
}

/// A 2-d histogram of `series` interpolated onto 800 points each, drawn
/// with `colors` and a colorbar.
fn series_histogram(
    fig: &Figure,
    ax: &Axes,
    (xedges, yedges, counts): (&[f64], &[f64], &pltrs::numeric::Grid<f64>),
    colorizer: ColorizerOptions,
) -> CaseResult {
    let options = PcolorOptions {
        colorizer,
        ..PcolorOptions::default()
    };
    let mesh = Mesh::from_coords(xedges, yedges, counts.transposed());
    let pcm = ax.pcolormesh(mesh, options)?;
    pcm.set_rasterized(true);
    let colorbar = ColorbarOptions {
        label: "# points".into(),
        pad: Some(0.0),
        ..ColorbarOptions::beside(ax)
    };
    fig.colorbar(&pcm, colorbar)?;
    Ok(())
}

/// `plasma` with its lowest colour for bad (masked) values.
fn plasma() -> Result<Colormap, Box<dyn std::error::Error>> {
    let cmap = colormaps::get("plasma")?;
    let lowest = cmap.sample_index(0);
    Ok(cmap.with_bad(lowest)?)
}

fn time_series_histogram(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let x = linspace(0.0, 4.0 * std::f64::consts::PI, 100);
    let series = walks_and_sines(&mut g, &x);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(3, 1)?;
    for row in &series {
        axs[(0, 0)].plot_with(&x, row, LineProps::default().color("C0").alpha(0.1))?;
    }
    axs[(0, 0)].set_title("Line plot with alpha");
    let x_fine = linspace(x[0], x[x.len() - 1], 800);
    let y_fine: Vec<f64> = series
        .iter()
        .flat_map(|row| x_fine.iter().map(|&xf| interp(xf, &x, row, None)))
        .collect();
    let x_all: Vec<f64> = x_fine.iter().copied().cycle().take(y_fine.len()).collect();
    let binning = Histogram2dOptions {
        bins: [AxisBins::Count(400), AxisBins::Count(100)],
        ..Histogram2dOptions::default()
    };
    let h = histogram2d(&x_all, &y_fine, binning)?;
    let edges_and_counts = (h.xedges.as_slice(), h.yedges.as_slice(), &h.counts);
    let mut log = LogNorm::default();
    log.set_vmax(Some(1.5e2));
    let log_colors = ColorizerOptions::default().cmap(plasma()?).norm(log);
    series_histogram(fig, &axs[(1, 0)], edges_and_counts, log_colors)?;
    axs[(1, 0)].set_title("2d histogram and log color scale");
    let linear_colors = ColorizerOptions::default()
        .cmap(plasma()?)
        .limits(None, Some(1.5e2));
    series_histogram(fig, &axs[(2, 0)], edges_and_counts, linear_colors)?;
    axs[(2, 0)].set_title("2d histogram and linear color scale");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/hexbin_demo", hexbin_demo).sized(9.0, 4.0, 100.0),
    FigureCase::new("gallery_stats/hist_1", hist_1),
    FigureCase::new("gallery_stats/hist_2", hist_2),
    FigureCase::new("gallery_stats/hist_3", hist_3),
    FigureCase::new("gallery_stats/hist_4", hist_4).sized(5.0, 15.0, 100.0),
    FigureCase::new("gallery_stats/time_series_histogram", time_series_histogram)
        .sized(6.0, 8.0, 100.0),
];
