//! The statistics plot types: box and violin plots, error bars, event
//! lines, histograms in one and two dimensions, hexagonal bins, a pie
//! and an empirical distribution.

use plt::axes::{
    BoxStyle, BoxplotOptions, BxpOptions, EcdfOptions, ErrorSize, EventplotOptions, HexGridSize,
    HexbinOptions, Hist2dOptions, HistPlotOptions, PieOptions, ViolinDrawOptions,
    ViolinplotOptions,
};
use plt::colors::colormaps;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::stats::{AxisBins, Bins, HistOptions, Histogram2dOptions};

use super::{gridded, normal_columns, plain, unit_square};
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// Where the box, violin, error bar and event plots stand.
const POSITIONS: [f64; 3] = [2.0, 4.0, 6.0];

/// 200 normal draws around 4 (`ecdf` and `hist`).
fn spread_sample() -> Vec<f64> {
    map(&SpecRng::new(1).normal(200, 0.0, 1.5), |v| 4.0 + v)
}

/// 5000 correlated points plus noise (`hexbin` and `hist2d`).
fn correlated() -> (Vec<f64>, Vec<f64>) {
    let mut rng = SpecRng::new(1);
    let x = rng.normal(5000, 0.0, 1.0);
    let noise = rng.normal(5000, 0.0, 1.0);
    let y = x
        .iter()
        .zip(noise)
        .map(|(x, e)| 1.2 * x + e / 3.0)
        .collect();
    (x, y)
}

/// The view of the two-dimensional histograms.
fn correlated_view(ax: &Axes) {
    ax.set_xlim(-2.0, 2.0).set_ylim(-3.0, 3.0);
}

/// `color` lines `width` points wide.
fn stroke(color: &str, width: f64) -> LineProps {
    LineProps::default().color(color).linewidth(width)
}

fn boxplot_plot(fig: &Figure) -> CaseResult {
    let data = normal_columns(
        &mut SpecRng::new(10),
        100,
        [3.0, 5.0, 4.0],
        [1.25, 1.0, 1.25],
    );
    let ax = fig.subplot();
    let boxes = PatchProps::default()
        .facecolor("C0")
        .edgecolor("white")
        .linewidth(0.5);
    let options = BoxplotOptions {
        draw: BxpOptions {
            // `[2, 4, 6]`: integers.
            positions: Some(POSITIONS.to_vec()),
            integer_positions: true,
            widths: vec![1.5],
            boxes: Some(BoxStyle::Patch(boxes)),
            showmeans: Some(false),
            showfliers: Some(false),
            medianprops: stroke("white", 0.5),
            whiskerprops: stroke("C0", 1.5),
            capprops: stroke("C0", 1.5),
            ..BxpOptions::default()
        },
        ..BoxplotOptions::default()
    };
    ax.boxplot(&data, options)?;
    unit_square(&ax);
    Ok(())
}

fn ecdf(fig: &Figure) -> CaseResult {
    fig.subplot()
        .ecdf(&spread_sample(), EcdfOptions::default())?;
    Ok(())
}

fn errorbar_plot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = ErrorbarOptions {
        yerr: Some(ErrorSize::Symmetric(vec![0.9, 1.2, 0.5])),
        fmt: "o".parse()?,
        line: LineProps::default().linewidth(2.0),
        capsize: Some(6.0),
        ..ErrorbarOptions::default()
    };
    ax.errorbar(POSITIONS, [3.6, 5.0, 4.2], options)?;
    unit_square(&ax);
    Ok(())
}

fn eventplot(fig: &Figure) -> CaseResult {
    let draws = SpecRng::new(1).gamma(3 * 50, 4);
    let data: Vec<Vec<f64>> = draws.chunks(50).map(<[f64]>::to_vec).collect();
    let ax = fig.subplot();
    let options = EventplotOptions {
        orientation: Orientation::Vertical,
        lineoffsets: POSITIONS.to_vec(),
        linewidths: vec![0.75],
        ..EventplotOptions::default()
    };
    ax.eventplot(&data, options)?;
    unit_square(&ax);
    Ok(())
}

fn hexbin(fig: &Figure) -> CaseResult {
    let (x, y) = correlated();
    let ax = fig.subplot();
    let options = HexbinOptions {
        gridsize: HexGridSize::Count(20),
        ..HexbinOptions::default()
    };
    ax.hexbin(&x, &y, options)?;
    correlated_view(&ax);
    Ok(())
}

fn hist2d(fig: &Figure) -> CaseResult {
    let (x, y) = correlated();
    let ax = fig.subplot();
    let edges = || AxisBins::Edges(arange(-3.0, 3.0, 0.1));
    let options = Hist2dOptions {
        histogram: Histogram2dOptions {
            bins: [edges(), edges()],
            ..Histogram2dOptions::default()
        },
        ..Hist2dOptions::default()
    };
    ax.hist2d(&x, &y, options)?;
    correlated_view(&ax);
    Ok(())
}

fn hist_plot(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let options = HistPlotOptions {
        layout: HistOptions {
            bins: Some(Bins::Count(8)),
            ..HistOptions::default()
        },
        props: PatchProps::default().linewidth(0.5).edgecolor("white"),
        ..HistPlotOptions::default()
    };
    ax.hist(&[spread_sample()], options)?;
    ax.set_xlim(0.0, 8.0).set_xticks(arange(1.0, 8.0, 1.0));
    ax.set_ylim(0.0, 56.0).set_yticks(linspace(0.0, 56.0, 9));
    Ok(())
}

fn pie(fig: &Figure) -> CaseResult {
    let x = [1.0, 2.0, 3.0, 4.0];
    let blues = colormaps::get("Blues")?;
    let ax = fig.subplot();
    let options = PieOptions {
        colors: linspace(0.2, 0.7, x.len())
            .into_iter()
            .map(|v| blues.sample(v).into())
            .collect(),
        radius: 3.0,
        center: (4.0, 4.0),
        wedgeprops: PatchProps::default().linewidth(1.0).edgecolor("white"),
        frame: true,
        ..PieOptions::default()
    };
    ax.pie(&x, options)?;
    unit_square(&ax);
    Ok(())
}

fn violin(fig: &Figure) -> CaseResult {
    let data = normal_columns(
        &mut SpecRng::new(10),
        200,
        [3.0, 5.0, 4.0],
        [0.75, 1.0, 0.75],
    );
    let ax = fig.subplot();
    let options = ViolinplotOptions {
        draw: ViolinDrawOptions {
            positions: Some(POSITIONS.to_vec()),
            widths: vec![2.0],
            showmeans: false,
            showmedians: false,
            showextrema: false,
            ..ViolinDrawOptions::default()
        },
        ..ViolinplotOptions::default()
    };
    for body in &ax.violinplot(&data, options)?.bodies {
        body.set_alpha(Some(0.9));
    }
    unit_square(&ax);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    gridded("gallery_plot_types/boxplot_plot", boxplot_plot),
    gridded("gallery_plot_types/ecdf", ecdf),
    gridded("gallery_plot_types/errorbar_plot", errorbar_plot),
    gridded("gallery_plot_types/eventplot", eventplot),
    plain("gallery_plot_types/hexbin", hexbin),
    plain("gallery_plot_types/hist2d", hist2d),
    gridded("gallery_plot_types/hist_plot", hist_plot),
    plain("gallery_plot_types/pie", pie),
    gridded("gallery_plot_types/violin", violin),
];
