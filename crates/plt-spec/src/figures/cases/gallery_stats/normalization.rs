//! `histogram_normalization`: how bins, densities and weights change a
//! histogram, from nine numbers to a thousand normal draws.

use plt::axes::{Axes, HistPlotOptions};
use plt::figure::{Figure, MosaicAxes, MosaicOptions};
use plt::fonts::{FontSize, RelativeSize};
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions, HistType};

use super::{SEED, strings};
use crate::figures::cases::common::{map, mosaic};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The nine numbers binned first (`XDATA`).
const XDATA: [f64; 9] = [1.2, 2.3, 3.3, 3.1, 1.7, 3.4, 2.1, 1.25, 1.3];

/// The label of the density axes.
const DENSITY_LABEL: &str = "Probability density [$V^{-1}$]";

/// The label of the x axes binned in volts.
const VOLT_BINS: &str = "x bins [$V$]";

/// The widths of the bins compared (`BIN_WIDTHS`).
const BIN_WIDTHS: [f64; 3] = [0.1, 0.4, 1.2];

/// Thick outlines in the first colour (`OUTLINE`), binned as `layout`
/// says.
fn outlined(layout: HistOptions<'_>) -> HistPlotOptions<'_> {
    HistPlotOptions {
        layout,
        props: PatchProps::default()
            .facecolor("none")
            .edgecolor("C0")
            .linewidth(3.0),
        ..HistPlotOptions::default()
    }
}

/// [`XDATA`] binned in outline with its values as diamonds on the axis
/// (`binned_dots`).
fn binned_dots(ax: &Axes, xlabel: &str, bins: Option<Bins>) -> CaseResult {
    let layout = HistOptions {
        bins,
        ..HistOptions::default()
    };
    ax.hist(&[XDATA], &outlined(layout))?;
    ax.plot_with(XDATA, [0.0; 9], LineProps::fmt("d")?)?;
    ax.set_xlabel(xlabel);
    Ok(())
}

fn histogram_normalization_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    binned_dots(
        &ax,
        "x bins (dx=1.0)",
        Some(Bins::Edges(vec![1.0, 2.0, 3.0, 4.0])),
    )?;
    ax.set_ylabel("Number per bin");
    Ok(())
}

fn histogram_normalization_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    binned_dots(
        &ax,
        "x bins (dx=0.5)",
        Some(Bins::Edges(arange(1.0, 4.5, 0.5))),
    )?;
    ax.set_ylabel("Number per bin");
    Ok(())
}

/// A constrained figure with one row of axes labelled `labels`, sharing
/// both axes or none.
fn mosaic_row(
    fig: &Figure,
    labels: &[&str],
    shared: bool,
) -> Result<MosaicAxes, Box<dyn std::error::Error>> {
    fig.set_layout_engine(LayoutEngine::constrained());
    let options = MosaicOptions {
        sharex: shared,
        sharey: shared,
        ..MosaicOptions::default()
    };
    Ok(fig.subplot_mosaic(&mosaic(&[labels])?, &options)?)
}

/// The axes labelled `label`.
fn panel<'a>(axes: &'a MosaicAxes, label: &str) -> Result<&'a Axes, String> {
    axes.get(label)
        .ok_or_else(|| format!("no axes labelled {label}"))
}

fn histogram_normalization_3(fig: &Figure) -> CaseResult {
    let ax = mosaic_row(fig, &["auto", "n4"], true)?;
    binned_dots(panel(&ax, "auto")?, "x bins (auto)", None)?;
    panel(&ax, "auto")?.set_ylabel("Number per bin");
    binned_dots(
        panel(&ax, "n4")?,
        "x bins (\"bins=4\")",
        Some(Bins::Count(4)),
    )?;
    Ok(())
}

fn histogram_normalization_4(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let layout = HistOptions {
        bins: Some(Bins::Edges(arange(1.0, 4.5, 0.5))),
        density: true,
        ..HistOptions::default()
    };
    ax.hist(&[XDATA], &outlined(layout))?;
    ax.set_ylabel("Probability density [$V^{-1}$])");
    ax.set_xlabel("x bins (dx=0.5 $V$)");
    Ok(())
}

/// The standard normal density on `[-4, 4)` in steps of 0.1
/// (`normal_pdf`).
fn normal_pdf() -> (Vec<f64>, Vec<f64>) {
    let xpdf = arange(-4.0, 4.0, 0.1);
    let pdf = map(&xpdf, |x| {
        1.0 / (2.0 * std::f64::consts::PI).sqrt() * (-(x * x) / 2.0).exp()
    });
    (xpdf, pdf)
}

/// A step histogram of `xdata` over `xbins`, optionally a density.
fn steps(xbins: Vec<f64>, density: bool) -> HistPlotOptions<'static> {
    HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Edges(xbins)),
            density,
            histtype: HistType::Step,
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    }
}

/// Step histograms labelled `label`.
fn labelled<'a>(options: HistPlotOptions<'a>, label: &str) -> HistPlotOptions<'a> {
    HistPlotOptions {
        label: strings(&[label]),
        ..options
    }
}

/// The two panels comparing counts with a density for `xbins`
/// (`counts_and_density`).
fn counts_and_density(fig: &Figure, xbins: &[f64], scaled_label: &str) -> CaseResult {
    let xdata = [SpecRng::new(SEED).normal(1000, 0.0, 1.0)];
    let (xpdf, pdf) = normal_pdf();
    let ax = mosaic_row(fig, &["False", "True"], false)?;
    let counts = panel(&ax, "False")?;
    counts.hist(&xdata, &labelled(steps(xbins.to_vec(), false), "Counts"))?;
    counts.plot_with(
        &xpdf,
        map(&pdf, |p| p * 1000.0 * 0.1),
        LineProps::default().label(scaled_label),
    )?;
    counts.set_ylabel("Count per bin");
    counts.set_xlabel("x bins [V]");
    counts.legend(LegendOptions::default())?;
    let density = panel(&ax, "True")?;
    density.hist(&xdata, &labelled(steps(xbins.to_vec(), true), "density"))?;
    density.plot_with(&xpdf, &pdf, LineProps::default().label("$f_X(x)$"))?;
    density.set_ylabel(DENSITY_LABEL);
    density.set_xlabel(VOLT_BINS);
    density.legend(LegendOptions::default())?;
    Ok(())
}

fn histogram_normalization_5(fig: &Figure) -> CaseResult {
    counts_and_density(fig, &arange(-4.0, 4.0, 0.1), r"$N\,f_X(x)\,\delta x$")
}

fn histogram_normalization_6(fig: &Figure) -> CaseResult {
    let dx = 0.1;
    let mut xbins = arange(-4.0, -1.25, 6.0 * dx);
    xbins.extend(arange(-1.25, 4.0, dx));
    counts_and_density(fig, &xbins, r"$N\,f_X(x)\,\delta x_0$")
}

/// A small legend titled "bin width:".
fn bin_width_legend() -> LegendOptions {
    LegendOptions::default()
        .fontsize(FontSize::Relative(RelativeSize::Small))
        .title("bin width:")
}

fn histogram_normalization_7(fig: &Figure) -> CaseResult {
    let xdata = [SpecRng::new(SEED).normal(1000, 0.0, 1.0)];
    let (xpdf, pdf) = normal_pdf();
    let ax = mosaic_row(fig, &["False", "True"], false)?;
    let (counts, density) = (panel(&ax, "False")?, panel(&ax, "True")?);
    density.plot_with(
        &xpdf,
        &pdf,
        LineProps::fmt("--")?.label("$f_X(x)$").color("k"),
    )?;
    for (nn, dx) in BIN_WIDTHS.into_iter().enumerate() {
        let xbins = arange(-4.0, 4.0, dx);
        counts.plot_with(
            &xpdf,
            map(&pdf, |p| p * 1000.0 * dx),
            LineProps::fmt("--")?.color(format!("C{nn}")),
        )?;
        counts.hist(&xdata, &steps(xbins.clone(), false))?;
        density.hist(&xdata, &labelled(steps(xbins, true), &dx.to_string()))?;
    }
    counts.set_xlabel(VOLT_BINS);
    counts.set_ylabel("Count per bin");
    density.set_ylabel(DENSITY_LABEL);
    density.set_xlabel(VOLT_BINS);
    density.legend(bin_width_legend())?;
    Ok(())
}

/// Step histograms over `xbins` with `weights`, labelled `label`.
fn weighted_steps<'a>(
    xbins: Vec<f64>,
    weights: &'a [Vec<f64>],
    label: &str,
) -> HistPlotOptions<'a> {
    let mut options = labelled(steps(xbins, false), label);
    options.layout.weights = Some(weights);
    options
}

/// Weights of `1 / n` for each of `n` values.
fn one_over_n(n: usize) -> Vec<f64> {
    vec![1.0 / n as f64; n]
}

fn histogram_normalization_8(fig: &Figure) -> CaseResult {
    let xdata = [SpecRng::new(SEED).normal(1000, 0.0, 1.0)];
    let weights = [one_over_n(1000)];
    fig.set_layout_engine(LayoutEngine::constrained());
    let ax = fig.subplot();
    for dx in BIN_WIDTHS {
        let options = weighted_steps(arange(-4.0, 4.0, dx), &weights, &dx.to_string());
        ax.hist(&xdata, &options)?;
    }
    ax.set_xlabel(VOLT_BINS);
    ax.set_ylabel("Bin count / N");
    ax.legend(bin_width_legend())?;
    Ok(())
}

fn histogram_normalization_9(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let samples = [g.normal(1000, 0.0, 1.0), g.normal(100, 0.0, 1.0)];
    let ax = mosaic_row(fig, &["no_norm", "density", "weight"], false)?;
    let xbins = arange(-4.0, 4.0, 0.25);
    let no_norm = panel(&ax, "no_norm")?;
    let density = panel(&ax, "density")?;
    let weighted = panel(&ax, "weight")?;
    for sample in &samples {
        no_norm.hist(std::slice::from_ref(sample), &steps(xbins.clone(), false))?;
    }
    no_norm.set_ylabel("Counts");
    no_norm.set_xlabel(VOLT_BINS);
    no_norm.set_title("No normalization");
    for sample in &samples {
        density.hist(std::slice::from_ref(sample), &steps(xbins.clone(), true))?;
    }
    density.set_ylabel(DENSITY_LABEL);
    density.set_title("Density=True");
    density.set_xlabel(VOLT_BINS);
    for sample in &samples {
        let weights = [one_over_n(sample.len())];
        let label = format!("N={}", sample.len());
        let options = weighted_steps(xbins.clone(), &weights, &label);
        weighted.hist(std::slice::from_ref(sample), &options)?;
    }
    weighted.set_xlabel(VOLT_BINS);
    weighted.set_ylabel("Counts / N");
    weighted.legend(LegendOptions::default().fontsize(FontSize::Relative(RelativeSize::Small)))?;
    weighted.set_title("Weight = 1/N");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new(
        "gallery_stats/histogram_normalization_1",
        histogram_normalization_1,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_2",
        histogram_normalization_2,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_3",
        histogram_normalization_3,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_4",
        histogram_normalization_4,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_5",
        histogram_normalization_5,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_6",
        histogram_normalization_6,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_7",
        histogram_normalization_7,
    ),
    FigureCase::new(
        "gallery_stats/histogram_normalization_8",
        histogram_normalization_8,
    )
    .sized(3.5, 3.0, 100.0),
    FigureCase::new(
        "gallery_stats/histogram_normalization_9",
        histogram_normalization_9,
    )
    .sized(8.0, 4.0, 100.0),
];
