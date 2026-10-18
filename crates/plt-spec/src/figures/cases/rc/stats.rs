//! The `hist.*`, `boxplot.*`, `errorbar.*` and `scatter.*` cases.

use plt::axes::{BoxplotOptions, ErrorbarOptions, HistPlotOptions, ScatterOptions};
use plt::figure::Figure;

use super::{SMALL, rc_cases};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_hist`: a histogram of normal samples.
fn hist(fig: &Figure) -> CaseResult {
    let samples = SpecRng::new(7).normal(300, 0.0, 1.0);
    fig.subplot()
        .hist(&[samples], &HistPlotOptions::default())?;
    Ok(())
}

/// `_boxes`: three box plots with outliers.
fn boxes(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(11);
    let data: Vec<Vec<f64>> = (0..3)
        .map(|i| {
            let i = f64::from(i);
            let mut sample = g.normal(40, i, 1.0 + 0.3 * i);
            sample.extend([i + 4.5, i - 5.0]);
            sample
        })
        .collect();
    fig.subplot().boxplot(&data, &BoxplotOptions::default())?;
    Ok(())
}

/// `_errorbars`: points with x and y error bars.
fn errorbars(fig: &Figure) -> CaseResult {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0];
    let y = x.map(|v| 0.5 * v + 1.0);
    let options = ErrorbarOptions {
        yerr: Some(0.4.into()),
        xerr: Some(0.2.into()),
        fmt: "o".into(),
        ..ErrorbarOptions::default()
    };
    fig.subplot().errorbar(x, y, options)?;
    Ok(())
}

/// `_scatter`: a scatter plot with the scatter defaults.
fn scatter(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(5);
    let x = g.uniform(20, 0.0, 1.0);
    let y = g.uniform(20, 0.0, 1.0);
    let options = ScatterOptions {
        sizes: Some(vec![60.0]),
        ..ScatterOptions::default()
    };
    fig.subplot().scatter_with(x, y, options)?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    hist, SMALL => {
        "hist_bins": [("hist.bins", "25")],
        "hist_bins_auto": [("hist.bins", "auto")],
        "hist_bins_edges": [("hist.bins", "-3, -1, -0.5, 0, 0.5, 1, 3")],
    }
    boxes, SMALL => {
        "boxplot_notch": [("boxplot.notch", "True")],
        "boxplot_vertical": [("boxplot.vertical", "False")],
        "boxplot_whiskers": [("boxplot.whiskers", "0.5")],
        "boxplot_whiskers_percentiles": [("boxplot.whiskers", "5, 95")],
        "boxplot_patchartist": [("boxplot.patchartist", "True")],
        "boxplot_showmeans": [("boxplot.showmeans", "True")],
        "boxplot_meanline": [("boxplot.showmeans", "True"), ("boxplot.meanline", "True")],
        "boxplot_hidden_parts": [
            ("boxplot.showcaps", "False"),
            ("boxplot.showbox", "False"),
            ("boxplot.showfliers", "False"),
        ],
        "boxplot_flierprops": [
            ("boxplot.flierprops.color", "tab:red"),
            ("boxplot.flierprops.marker", "D"),
            ("boxplot.flierprops.markerfacecolor", "yellow"),
            ("boxplot.flierprops.markeredgecolor", "tab:green"),
            ("boxplot.flierprops.markeredgewidth", "2"),
            ("boxplot.flierprops.markersize", "8"),
            ("boxplot.flierprops.linestyle", "-"),
            ("boxplot.flierprops.linewidth", "2"),
        ],
        "boxplot_boxprops": [
            ("boxplot.boxprops.color", "tab:blue"),
            ("boxplot.boxprops.linewidth", "2.5"),
            ("boxplot.boxprops.linestyle", "--"),
        ],
        "boxplot_whiskerprops": [
            ("boxplot.whiskerprops.color", "tab:orange"),
            ("boxplot.whiskerprops.linewidth", "2.5"),
            ("boxplot.whiskerprops.linestyle", ":"),
        ],
        "boxplot_capprops": [
            ("boxplot.capprops.color", "tab:purple"),
            ("boxplot.capprops.linewidth", "3"),
            ("boxplot.capprops.linestyle", "-."),
        ],
        "boxplot_medianprops": [
            ("boxplot.medianprops.color", "black"),
            ("boxplot.medianprops.linewidth", "3"),
            ("boxplot.medianprops.linestyle", "--"),
        ],
        "boxplot_meanprops_marker": [
            ("boxplot.showmeans", "True"),
            ("boxplot.meanprops.marker", "D"),
            ("boxplot.meanprops.markerfacecolor", "white"),
            ("boxplot.meanprops.markeredgecolor", "black"),
            ("boxplot.meanprops.markersize", "9"),
        ],
        "boxplot_meanprops_line": [
            ("boxplot.showmeans", "True"),
            ("boxplot.meanline", "True"),
            ("boxplot.meanprops.color", "tab:purple"),
            ("boxplot.meanprops.linestyle", ":"),
            ("boxplot.meanprops.linewidth", "2.5"),
        ],
    }
    errorbars, SMALL => {
        "errorbar_capsize": [("errorbar.capsize", "5")],
        "errorbar_capthick": [("errorbar.capsize", "4"), ("errorbar.capthick", "3")],
        "errorbar_elinewidth": [("errorbar.elinewidth", "3")],
    }
    scatter, SMALL => {
        "scatter_marker": [("scatter.marker", "^")],
        "scatter_edgecolors": [("scatter.edgecolors", "black")],
    }
};
