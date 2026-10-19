//! Tickers and twins: a lone x label, fixed and linear locators with
//! printf-style and fixed formatters under twins, log scales set before
//! and after twinning, twins of axes that do not autoscale, and minor ticks
//! turned on through rcParams.

use plt::axes::AxisName;
use plt::numeric::{arange, linspace};
use plt::prelude::*;
use plt::ticker::{FixedFormatter, FixedLocator, FormatStrFormatter, LinearLocator};

use super::remove_ticks_and_titles;
use crate::figures::CaseResult;

#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
pub(super) fn formatter_ticker_001(fig: &Figure) -> CaseResult {
    fig.subplot().set_xlabel("x-label 001");
    Ok(())
}

/// `FixedFormatter` labels from string literals.
fn fixed_labels(labels: &[&str]) -> FixedFormatter {
    FixedFormatter::new(labels.iter().map(|&s| s.to_owned()).collect())
}

pub(super) fn twin_axis_locators_formatters(fig: &Figure) -> CaseResult {
    let locs = linspace(0.0, 1.0, 5)
        .into_iter()
        .map(|v| (std::f64::consts::PI * v / 2.0).sin())
        .collect();
    let ax1 = fig.subplot();
    ax1.set_margins(0.0, 0.0)?;
    ax1.plot([0.1, 100.0], [0.0, 1.0])?;
    ax1.yaxis()
        .set_major_locator(FixedLocator::new(locs))
        .set_minor_locator(FixedLocator::new(vec![0.1, 0.2, 0.3]))
        .set_major_formatter(FormatStrFormatter::new("%08.2lf")?)
        .set_minor_formatter(fixed_labels(&["tricks", "mind", "jedi"]));
    ax1.xaxis()
        .set_major_locator(LinearLocator::new(None))
        .set_minor_locator(FixedLocator::new(vec![15.0, 35.0, 55.0, 75.0]))
        .set_major_formatter(FormatStrFormatter::new("%05.2lf")?)
        .set_minor_formatter(fixed_labels(&["c", "3", "p", "o"]));
    let _ = ax1.twiny();
    let _ = ax1.twinx();
    Ok(())
}

/// `test_twin_logscale`: the parent's `axis` made logarithmic after
/// twinning (top) and before (bottom), the line drawn in the twin.
fn twin_logscale(fig: &Figure, axis: AxisName) -> CaseResult {
    let twin_of = |ax: &Axes| match axis {
        AxisName::X => ax.twinx(),
        AxisName::Y => ax.twiny(),
    };
    let set_log = |ax: &Axes| match axis {
        AxisName::X => ax.set_xscale("log").map(|_| ()),
        AxisName::Y => ax.set_yscale("log").map(|_| ()),
    };
    let x = arange(1.0, 100.0, 1.0);
    let ax = fig.add_subplot(2, 1, 1)?;
    let twin = twin_of(&ax);
    set_log(&ax)?;
    twin.plot(&x, &x)?;
    let ax = fig.add_subplot(2, 1, 2)?;
    set_log(&ax)?;
    twin_of(&ax).plot(&x, &x)?;
    remove_ticks_and_titles(fig);
    Ok(())
}

pub(super) fn twin_logscale_x(fig: &Figure) -> CaseResult {
    twin_logscale(fig, AxisName::X)
}

pub(super) fn twin_logscale_y(fig: &Figure) -> CaseResult {
    twin_logscale(fig, AxisName::Y)
}

pub(super) fn twinx_axis_scales(fig: &Figure) -> CaseResult {
    let ax = fig.add_axes([0.0, 0.0, 1.0, 1.0]);
    ax.set_autoscale_on(false);
    ax.plot_with(
        [0.0, 0.5, 1.0],
        [0.0, 0.25, 0.5],
        LineProps::default().color("blue").linewidth(10.0),
    )?;
    let ax2 = ax.twinx();
    ax2.plot_with(
        [0.0, 1.0, 2.0],
        [0.0, 2.0, 4.0],
        LineProps::fmt("r--")?.linewidth(5.0),
    )?;
    ax.set_margins(0.0, 0.0)?;
    ax2.set_margins(0.0, 0.0)?;
    Ok(())
}

pub(super) fn minorticks_on_rc_params_both(fig: &Figure) -> CaseResult {
    plt::rc::context(|| {
        plt::rc::set("xtick.minor.visible", "True")?;
        plt::rc::set("ytick.minor.visible", "True")?;
        fig.subplot().plot([0.0, 1.0], [0.0, 1.0])?;
        Ok(())
    })
}
