//! The cases from `test_spines.py` and `test_scale.py`.

use crate::figures::cases::common::tickless;
use std::f64::consts::TAU;

use plt::axes::{AxisName, ErrorbarOptions, FillBetweenOptions, HistPlotOptions, TickParams};
use plt::axes::{TickDirection, TickPosition};
use plt::figure::Figure;
use plt::lines::LineProps;
use plt::numeric::{arange, linspace, logspace};
use plt::scale::{FuncTransform, NonPositive, ScaleOptions};
use plt::spines::{SpinePosition, SpineSide};
use plt::stats::Bins;

use super::{CLASSIC_TEST_PATCH, remove_text};
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `test_spines_axes_positions`: the right and top spines moved into the
/// axes, the others hidden.
fn axes_positions(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, TAU, 100);
    let y: Vec<f64> = x.iter().map(|x| 2.0 * x.sin()).collect();
    let ax = fig.add_subplot(1, 1, 1)?;
    ax.set_title("centered spines");
    ax.plot(&x, y)?;
    let spines = ax.spines();
    spines.right.set_position(SpinePosition::Axes(0.1));
    ax.yaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Right))?;
    spines.top.set_position(SpinePosition::Axes(0.25));
    ax.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Top))?;
    spines.left.set_color("none");
    spines.bottom.set_color("none");
    Ok(())
}

/// `test_spines_data_positions`: every spine at a data position.
fn data_positions(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let spines = ax.spines();
    spines.left.set_position(SpinePosition::Data(-1.5));
    spines.top.set_position(SpinePosition::Data(0.5));
    spines.right.set_position(SpinePosition::Data(-0.5));
    spines.bottom.set_position(SpinePosition::Data(0.0));
    ax.set_xlim(-2.0, 2.0)?.set_ylim(-2.0, 2.0)?;
    ax.xaxis().set_ticks_position(TickPosition::Both)?;
    ax.yaxis().set_ticks_position(TickPosition::Both)?;
    Ok(())
}

/// `test_spine_nonlinear_data_positions`: the left and right spines
/// swapped through data positions on a log axis.
fn nonlinear_data_positions(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xscale("log")?;
    ax.set_xlim(0.1, 1.0)?;
    let spines = ax.spines();
    spines.left.set_position(SpinePosition::Data(1.0));
    spines.left.set_linewidth(2.0);
    spines.right.set_position(SpinePosition::Data(0.1));
    ax.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelleft: Some(false),
        direction: Some(TickDirection::In),
        ..TickParams::default()
    });
    Ok(())
}

/// `test_log_scales`: lines across log and symlog axes.
fn log_scales(fig: &Figure) -> CaseResult {
    let ax = fig.add_subplot(1, 2, 2)?;
    ax.set_yscale("log")?.set_xscale("symlog")?;
    ax.axvline(24.1, LineProps::default());
    ax.axhline(24.1, LineProps::default());
    Ok(())
}

/// `test_logit_scales`: an extinction curve on a logit axis.
fn logit_scales(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = [
        0.001, 0.003, 0.01, 0.03, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.97, 0.99, 0.997,
        0.999,
    ];
    let y: Vec<f64> = x.iter().map(|x| 1.0 / x).collect();
    ax.plot(x, y)?;
    ax.set_xscale("logit")?;
    ax.grid(true);
    remove_text(fig);
    Ok(())
}

/// `test_logscale_mask`: a curve underflowing to zero on a log axis.
fn logscale_mask(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ys: Vec<f64> = linspace(0.0, 50.0, 1001)
        .iter()
        .map(|x| (-x * x).exp())
        .collect();
    ax.plot_y(ys);
    ax.set_yscale("log")?;
    let ticks: Vec<f64> = arange(-300.0, 0.0, 24.0)
        .iter()
        .map(|e| 10f64.powf(*e))
        .collect();
    ax.set_yticks(ticks);
    remove_text(fig);
    Ok(())
}

/// `test_logscale_nonpos_values`: histograms, a band and error bars
/// reaching non-positive values on log axes.
fn nonpos_values(fig: &Figure) -> CaseResult {
    let xs = SpecRng::new(19_680_801).normal(1000, 0.0, 1.0);
    let axs = fig.subplots(2, 2)?;
    let hist = HistPlotOptions {
        bins: Some(Bins::Count(10)),
        range: Some((-5.0, 5.0)),
        ..HistPlotOptions::default()
    };
    axs[(0, 0)].hist(&[&xs], hist.clone())?;
    axs[(0, 0)].set_yscale("log")?;
    axs[(0, 1)].hist(&[&xs], hist)?;
    let masked = ScaleOptions {
        nonpositive: Some(NonPositive::Mask),
        ..ScaleOptions::default()
    };
    axs[(0, 1)].set_scale_with(AxisName::Y, "log", masked)?;
    let xdata = arange(0.0, 10.0, 0.01);
    let ydata: Vec<f64> = xdata.iter().map(|x| (-x).exp()).collect();
    let edata: Vec<f64> = xdata
        .iter()
        .map(|x| 0.2 * (10.0 - x) * (5.0 * x).cos() * (-x).exp())
        .collect();
    let lower: Vec<f64> = ydata.iter().zip(&edata).map(|(y, e)| y - e).collect();
    let upper: Vec<f64> = ydata.iter().zip(&edata).map(|(y, e)| y + e).collect();
    axs[(1, 0)].fill_between(&xdata, lower, upper, FillBetweenOptions::default())?;
    axs[(1, 0)].set_yscale("log")?;
    let x = logspace(-1.0, 1.0, 50, 10.0);
    let y: Vec<f64> = x.iter().map(|x| x.powi(3)).collect();
    let yerr: Vec<f64> = x.iter().map(|x| x.powi(2)).collect();
    axs[(1, 1)].errorbar(
        &x,
        y,
        ErrorbarOptions {
            yerr: Some(yerr.into()),
            ..ErrorbarOptions::default()
        },
    )?;
    axs[(1, 1)].set_yscale("log")?.set_xscale("log")?;
    axs[(1, 1)].set_yticks(vec![1e-2, 1.0, 1e2]);
    remove_text(fig);
    Ok(())
}

/// `test_function_scale`: a square-root function scale.
fn function_scales(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = arange(1.0, 1000.0, 1.0);
    ax.plot(&x, &x)?;
    let sqrt = ScaleOptions {
        functions: Some(FuncTransform::new(f64::sqrt, |x| x * x)),
        ..ScaleOptions::default()
    };
    ax.set_scale_with(AxisName::X, "function", sqrt)?;
    ax.set_xlim(1.0, 1000.0)?;
    remove_text(fig);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_small/spines_axes_positions", axes_positions),
    FigureCase::new("mpl_small/spines_data_positions", data_positions),
    FigureCase::new(
        "mpl_small/spine_nonlinear_data_positions",
        nonlinear_data_positions,
    ),
    FigureCase::new("mpl_small/spines_capstyle", |fig| {
        tickless(&fig.subplot());
        Ok(())
    })
    .with_style("classic")
    .with_rc(&[
        ("ytick.alignment", "center_baseline"),
        ("hatch.color", "edge"),
        ("text.hinting", "default"),
        ("axes.linewidth", "20"),
    ]),
    FigureCase::new("mpl_small/spines_black_axes", |fig| {
        let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0])?;
        ax.set_xticklabels(Vec::<String>::new())?
            .set_yticklabels(Vec::<String>::new())?;
        tickless(&ax);
        ax.set_facecolor("black");
        Ok(())
    })
    .sized(4.0, 4.0, 100.0)
    .with_style("classic")
    .with_rc(&[
        ("ytick.alignment", "center_baseline"),
        ("hatch.color", "edge"),
        ("text.hinting", "default"),
        ("savefig.pad_inches", "0"),
        ("savefig.bbox", "tight"),
    ]),
    FigureCase::new("mpl_small/log_scales", log_scales),
    FigureCase::new("mpl_small/logit_scales", logit_scales)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH),
    FigureCase::new("mpl_small/logscale_mask", logscale_mask)
        .with_style("classic")
        .with_rc(CLASSIC_TEST_PATCH),
    FigureCase::new("mpl_small/logscale_nonpos_values", nonpos_values),
    FigureCase::new("mpl_small/function_scales", function_scales),
];
