//! The histograms of the section: a bihistogram, cumulative
//! distributions, the histogram types, several datasets at once and
//! histograms drawn side by side as bars.

use plt::axes::{EcdfOptions, HistPlotOptions};
use plt::figure::Share;
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::prelude::*;
use plt::stats::{Bins, Cumulative, HistOptions, HistType, HistogramOptions, histogram};

use super::{SEED, columns, strings};
use crate::figures::cases::common::{colors, cumsum, map, sized};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Histogram options with `layout` and default styling.
fn laid_out(layout: HistOptions<'_>) -> HistPlotOptions<'_> {
    HistPlotOptions {
        layout,
        ..HistPlotOptions::default()
    }
}

/// `bins=n`.
fn bins(n: usize) -> HistOptions<'static> {
    HistOptions {
        bins: Some(Bins::Count(n)),
        ..HistOptions::default()
    }
}

fn histogram_bihistogram(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let dataset1 = g.normal(10_000, 0.0, 1.0);
    let dataset2 = g.normal(10_000, 1.0, 2.0);
    let both = dataset1.iter().chain(&dataset2).copied();
    let low = both.clone().fold(f64::INFINITY, f64::min);
    let high = both.fold(f64::NEG_INFINITY, f64::max);
    let bin_width = 0.25;
    let edges = arange(low, high + bin_width, bin_width);
    let ax = fig.subplot();
    let first = HistPlotOptions {
        label: strings(&["Dataset 1"]),
        ..laid_out(HistOptions {
            bins: Some(Bins::Edges(edges.clone())),
            ..HistOptions::default()
        })
    };
    ax.hist(&[dataset1], &first)?;
    let weights = [vec![-1.0; dataset2.len()]];
    let second = HistPlotOptions {
        label: strings(&["Dataset 2"]),
        ..laid_out(HistOptions {
            bins: Some(Bins::Edges(edges)),
            weights: Some(&weights),
            ..HistOptions::default()
        })
    };
    ax.hist(&[dataset2], &second)?;
    ax.axhline(0.0, LineProps::default().color("k"));
    ax.legend(LegendOptions::default())?;
    Ok(())
}

fn histogram_cumulative(fig: &Figure) -> CaseResult {
    let (mu, sigma) = (200.0, 25.0);
    let data = SpecRng::new(SEED).normal(100, mu, sigma);
    fig.set_layout_engine(LayoutEngine::constrained());
    let axs = fig.subplots_shared(1, 2, Share::All, Share::All)?;
    let (ax0, ax1) = (&axs[(0, 0)], &axs[(0, 1)]);
    let step = |cumulative: Cumulative, label: &str, bins: Bins| HistPlotOptions {
        label: strings(&[label]),
        ..laid_out(HistOptions {
            bins: Some(bins),
            density: true,
            histtype: HistType::Step,
            cumulative,
            ..HistOptions::default()
        })
    };
    ax0.ecdf(
        &data,
        EcdfOptions {
            line: LineProps::default().label("CDF"),
            ..EcdfOptions::default()
        },
    )?;
    let forward = step(Cumulative::Forward, "Cumulative histogram", Bins::Count(25));
    let edges = ax0.hist(std::slice::from_ref(&data), &forward)?.edges;
    let low = data.iter().copied().fold(f64::INFINITY, f64::min);
    let high = data.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let x = linspace(low, high, 50);
    let density = map(&x, |x| {
        (1.0 / ((2.0 * std::f64::consts::PI).sqrt() * sigma))
            * (-0.5 * (1.0 / sigma * (x - mu)).powi(2)).exp()
    });
    let mut y = cumsum(&density);
    let total = y[y.len() - 1];
    for y in &mut y {
        *y /= total;
    }
    let theory = || LineProps::fmt("k--").map(|props| props.linewidth(1.5).label("Theory"));
    ax0.plot_with(&x, &y, theory()?)?;
    ax1.ecdf(
        &data,
        EcdfOptions {
            complementary: true,
            line: LineProps::default().label("CCDF"),
            ..EcdfOptions::default()
        },
    )?;
    let reverse = step(
        Cumulative::Reverse,
        "Reversed cumulative histogram",
        Bins::Edges(edges),
    );
    ax1.hist(&[data], &reverse)?;
    ax1.plot_with(&x, map(&y, |y| 1.0 - y), theory()?)?;
    fig.suptitle("Cumulative distributions");
    for ax in axs.as_slice() {
        ax.grid(true);
        ax.legend(LegendOptions::default())?;
        ax.set_xlabel("Annual rainfall (mm)");
        ax.set_ylabel("Probability of occurrence");
        ax.label_outer();
    }
    Ok(())
}

fn histogram_histtypes(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let x = [g.normal(100, 200.0, 25.0)];
    let w = [g.normal(100, 200.0, 10.0)];
    let axs = fig.subplots(2, 2)?;
    for (ax, histtype) in [
        (&axs[(0, 0)], HistType::StepFilled),
        (&axs[(0, 1)], HistType::Step),
    ] {
        let options = HistPlotOptions {
            props: PatchProps::default().facecolor("g").alpha(0.75),
            ..laid_out(HistOptions {
                density: true,
                histtype,
                ..bins(20)
            })
        };
        ax.hist(&x, &options)?;
    }
    axs[(0, 0)].set_title("stepfilled");
    axs[(0, 1)].set_title("step");
    let stacked = || {
        laid_out(HistOptions {
            density: true,
            histtype: HistType::BarStacked,
            rwidth: Some(0.8),
            ..HistOptions::default()
        })
    };
    axs[(1, 0)].hist(&x, &stacked())?;
    axs[(1, 0)].hist(&w, &stacked())?;
    axs[(1, 0)].set_title("barstacked");
    let unequal = laid_out(HistOptions {
        bins: Some(Bins::Edges(vec![
            100.0, 150.0, 180.0, 195.0, 205.0, 220.0, 250.0, 300.0,
        ])),
        density: true,
        rwidth: Some(0.8),
        ..HistOptions::default()
    });
    axs[(1, 1)].hist(&x, &unequal)?;
    axs[(1, 1)].set_title("bar, unequal bins");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// The bin count of `histogram_multihist` (`n_bins`).
const N_BINS: usize = 10;

/// The colours of the stacked histograms.
const STACK_COLORS: [&str; 3] = ["green", "red", "blue"];

/// `histogram_multihist`'s data: 1000 draws in each of three columns.
fn three_normals(g: &mut SpecRng) -> Vec<Vec<f64>> {
    columns(&g.normal(3000, 0.0, 1.0), 3)
}

fn histogram_multihist_1(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let x = three_normals(&mut g);
    let axs = fig.subplots(2, 2)?;
    let names = ["red", "tan", "lime"];
    let with_legend = HistPlotOptions {
        color: colors(&names),
        label: strings(&names),
        ..laid_out(HistOptions {
            density: true,
            ..bins(N_BINS)
        })
    };
    axs[(0, 0)].hist(&x, &with_legend)?;
    axs[(0, 0)].legend(LegendOptions::default().prop(sized(10.0)))?;
    axs[(0, 0)].set_title("bars with legend");
    let stacked = laid_out(HistOptions {
        density: true,
        stacked: true,
        ..bins(N_BINS)
    });
    axs[(0, 1)].hist(&x, &stacked)?;
    axs[(0, 1)].set_title("stacked bar");
    let steps = HistPlotOptions {
        props: PatchProps::default().fill(false),
        ..laid_out(HistOptions {
            histtype: HistType::Step,
            stacked: true,
            ..bins(N_BINS)
        })
    };
    axs[(1, 0)].hist(&x, &steps)?;
    axs[(1, 0)].set_title("stack step (unfilled)");
    let x_multi: Vec<Vec<f64>> = [10_000, 5000, 2000]
        .into_iter()
        .map(|n| g.normal(n, 0.0, 1.0))
        .collect();
    axs[(1, 1)].hist(&x_multi, &laid_out(bins(N_BINS)))?;
    axs[(1, 1)].set_title("different sample sizes");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// One histogram of [`three_normals`] with a legend and a title.
fn multihist(fig: &Figure, options: &HistPlotOptions<'_>, title: &str) -> CaseResult {
    let ax = fig.subplot();
    ax.hist(&three_normals(&mut SpecRng::new(SEED)), options)?;
    ax.legend(LegendOptions::default())?;
    ax.set_title(title);
    Ok(())
}

/// Unfilled histograms of `histtype` with green, red and blue edges.
fn outlined(histtype: HistType, stacked: bool) -> HistPlotOptions<'static> {
    HistPlotOptions {
        props: PatchProps::default().fill(false),
        edgecolors: colors(&STACK_COLORS),
        ..laid_out(HistOptions {
            histtype,
            stacked,
            ..bins(N_BINS)
        })
    }
}

/// Stacked bars.
fn barstacked() -> HistPlotOptions<'static> {
    laid_out(HistOptions {
        histtype: HistType::BarStacked,
        ..bins(N_BINS)
    })
}

fn histogram_multihist_2(fig: &Figure) -> CaseResult {
    let options = HistPlotOptions {
        label: strings(&STACK_COLORS),
        ..outlined(HistType::Step, true)
    };
    multihist(fig, &options, "Stacked Steps with Edgecolors")
}

fn histogram_multihist_3(fig: &Figure) -> CaseResult {
    let options = HistPlotOptions {
        facecolors: colors(&STACK_COLORS),
        label: strings(&STACK_COLORS),
        ..barstacked()
    };
    multihist(fig, &options, "Bars with different Facecolors")
}

fn histogram_multihist_4(fig: &Figure) -> CaseResult {
    let hatches = [".", "o", "x"];
    let options = HistPlotOptions {
        hatches: strings(&hatches),
        label: strings(&hatches),
        ..barstacked()
    };
    multihist(fig, &options, "Hatches on Stacked Bars")
}

fn histogram_multihist_5(fig: &Figure) -> CaseResult {
    let options = HistPlotOptions {
        linewidths: vec![1.0, 2.0, 3.0],
        label: strings(&["1", "2", "3"]),
        ..outlined(HistType::Bar, false)
    };
    multihist(fig, &options, "Bars with Linewidths")
}

fn histogram_multihist_6(fig: &Figure) -> CaseResult {
    let options = HistPlotOptions {
        linestyles: vec![LineStyle::Solid, LineStyle::Dotted, LineStyle::Dashed],
        label: strings(&["-", ":", "--"]),
        ..outlined(HistType::Bar, false)
    };
    multihist(fig, &options, "Bars with Linestyles")
}

fn multiple_histograms_side_by_side(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let number_of_bins = 20;
    let data_sets = [
        g.normal(387, 0.0, 1.0),
        g.normal(387, 6.0, 1.0),
        g.normal(387, -3.0, 1.0),
    ];
    let all = data_sets.iter().flatten().copied();
    let hist_range = (
        all.clone().fold(f64::INFINITY, f64::min),
        all.fold(f64::NEG_INFINITY, f64::max),
    );
    let options = HistogramOptions {
        bins: Bins::Count(number_of_bins),
        range: Some(hist_range),
        ..HistogramOptions::default()
    };
    let binned_data_sets = data_sets
        .iter()
        .map(|data| histogram(data, &options).map(|h| h.counts))
        .collect::<Result<Vec<_>, _>>()?;
    let maximums: Vec<f64> = binned_data_sets
        .iter()
        .map(|counts| counts.iter().copied().fold(0.0, f64::max))
        .collect();
    let widest = maximums.iter().copied().fold(0.0, f64::max);
    let x_locations = arange(0.0, maximums.iter().sum(), widest);
    let bin_edges = linspace(hist_range.0, hist_range.1, number_of_bins + 1);
    let heights: Vec<f64> = bin_edges.windows(2).map(|w| w[1] - w[0]).collect();
    let centers: Vec<f64> = bin_edges
        .iter()
        .zip(&heights)
        .map(|(e, h)| e + h / 2.0)
        .collect();
    let ax = fig.subplot();
    for (x_loc, binned_data) in x_locations.iter().zip(&binned_data_sets) {
        let options = BarOptions {
            thickness: heights.clone(),
            base: map(binned_data, |count| x_loc - 0.5 * count),
            ..BarOptions::default()
        };
        ax.barh(centers.clone(), binned_data, options)?;
    }
    ax.set_xticks(x_locations);
    ax.set_xticklabels(["A", "B", "C"])?;
    ax.set_ylabel("Data values");
    ax.set_xlabel("Data sets");
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/histogram_bihistogram", histogram_bihistogram),
    FigureCase::new("gallery_stats/histogram_cumulative", histogram_cumulative)
        .sized(9.0, 4.0, 100.0),
    FigureCase::new("gallery_stats/histogram_histtypes", histogram_histtypes),
    FigureCase::new("gallery_stats/histogram_multihist_1", histogram_multihist_1),
    FigureCase::new("gallery_stats/histogram_multihist_2", histogram_multihist_2),
    FigureCase::new("gallery_stats/histogram_multihist_3", histogram_multihist_3),
    FigureCase::new("gallery_stats/histogram_multihist_4", histogram_multihist_4),
    FigureCase::new("gallery_stats/histogram_multihist_5", histogram_multihist_5),
    FigureCase::new("gallery_stats/histogram_multihist_6", histogram_multihist_6),
    FigureCase::new(
        "gallery_stats/multiple_histograms_side_by_side",
        multiple_histograms_side_by_side,
    ),
];
