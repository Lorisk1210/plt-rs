//! The `axes.*` and `xaxis`/`yaxis` label cases, tick formatting included.

use plt::prelude::*;

use super::{LABELLED, SMALL, curves, labelled, map, rc_cases, t};
use crate::figures::{CaseResult, FigureCase};

/// `_axisbelow`: bars and a thick line on a grid; the grid's z-order
/// shows.
fn axisbelow(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.bar(
        [0.0, 1.0, 2.0, 3.0],
        [3.0, 5.0, 2.0, 4.0],
        BarOptions::default(),
    )?;
    ax.plot_with(
        [-0.4, 3.4],
        [1.0, 4.0],
        LineProps::default().linewidth(4.0).color("k"),
    )?;
    Ok(())
}

/// A curve through `(x(T), y(T))`.
fn plot_of_t(fig: &Figure, x: impl Fn(f64) -> f64, y: impl Fn(f64) -> f64) -> CaseResult {
    let t = t();
    fig.subplot().plot(map(&t, x), map(&t, y))?;
    Ok(())
}

/// `_scientific`: values that need scientific notation on both axes.
fn scientific(fig: &Figure) -> CaseResult {
    plot_of_t(fig, |x| x * 1e7, |x| x.sin() * 1e-6)
}

/// `_thousands`: values in the thousands and hundredths.
fn thousands(fig: &Figure) -> CaseResult {
    plot_of_t(fig, |x| x * 1000.0, |x| x.sin() / 100.0)
}

/// `_offsets`: values close to 1000 and 100; tick offsets show.
fn offsets(fig: &Figure) -> CaseResult {
    plot_of_t(fig, |x| 1000.0 + x / 10.0, |x| 100.0 + x / 20.0)
}

/// `_logs`: a logarithmic y axis from 10^-2 to 10^4.
fn logs(fig: &Figure) -> CaseResult {
    let t = t();
    fig.subplot()
        .semilogy(&t, map(&t, |x| 10f64.powf(x - 2.0)))?;
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    curves, SMALL => {
        "axes_facecolor": [("axes.facecolor", "lightyellow")],
        "axes_edge": [("axes.edgecolor", "tab:red"), ("axes.linewidth", "3")],
        "axes_spines_top_right": [("axes.spines.top", "False"), ("axes.spines.right", "False")],
        "axes_spines_left_bottom": [("axes.spines.left", "False"), ("axes.spines.bottom", "False")],
        "axes_grid": [("axes.grid", "True")],
        "axes_grid_axis": [("axes.grid", "True"), ("axes.grid.axis", "x")],
        "axes_grid_which": [
            ("axes.grid", "True"),
            ("axes.grid.which", "minor"),
            ("xtick.minor.visible", "True"),
            ("ytick.minor.visible", "True"),
        ],
        "axes_unicode_minus": [("axes.unicode_minus", "False")],
        "axes_prop_cycle": [(
            "axes.prop_cycle",
            "cycler(color=['tab:red', 'k']) + cycler(marker=['o', 's'])"
        )],
        "axes_autolimit_mode": [("axes.autolimit_mode", "round_numbers")],
        "axes_margins": [("axes.xmargin", "0"), ("axes.ymargin", "0.3")],
    }
    labelled, LABELLED => {
        "axes_title_style": [
            ("axes.titlesize", "small"),
            ("axes.titleweight", "bold"),
            ("axes.titlecolor", "tab:green"),
        ],
        "axes_titlelocation": [("axes.titlelocation", "right")],
        "axes_titley": [("axes.titley", "0.8")],
        "axes_titlepad": [("axes.titlepad", "14")],
        "axes_label_style": [
            ("axes.labelsize", "large"),
            ("axes.labelweight", "bold"),
            ("axes.labelcolor", "tab:purple"),
        ],
        "axes_labelpad": [("axes.labelpad", "10")],
        "axis_labellocation": [("xaxis.labellocation", "right"), ("yaxis.labellocation", "top")],
    }
    axisbelow, SMALL => {
        "axes_axisbelow_true": [("axes.grid", "True"), ("axes.axisbelow", "True")],
        "axes_axisbelow_false": [("axes.grid", "True"), ("axes.axisbelow", "False")],
    }
    thousands, SMALL => {
        "axes_formatter_limits": [("axes.formatter.limits", "-2, 2")],
    }
    scientific, SMALL => {
        "axes_formatter_use_mathtext": [("axes.formatter.use_mathtext", "True")],
    }
    logs, SMALL => {
        "axes_formatter_min_exponent": [("axes.formatter.min_exponent", "2")],
    }
    offsets, SMALL => {
        "axes_formatter_useoffset": [("axes.formatter.useoffset", "False")],
        "axes_formatter_offset_threshold": [("axes.formatter.offset_threshold", "2")],
    }
};
