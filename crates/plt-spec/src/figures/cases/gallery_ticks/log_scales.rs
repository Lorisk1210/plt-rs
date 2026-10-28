//! Logarithmic and symmetric logarithmic scales: the semi-log and log-log
//! helpers, a base-2 axis with named ticks, error bars reaching below
//! zero, and symlog's linear region (`log_demo` and `symlog_demo`).

use plt::axes::{AxisName, ErrorSize, GridOptions, TickLevel};
use plt::layout::{LayoutEngine, TightLayout};
use plt::prelude::*;
use plt::scale::{NonPositive, ScaleOptions};

use crate::figures::cases::common::{map, stacked_pair};
use crate::figures::{CaseResult, FigureCase};

/// Grid lines at the major ticks and light grey ones at the minor ticks.
fn log_grids(ax: &Axes) {
    ax.grid(true);
    ax.grid_with(
        true,
        GridOptions {
            which: TickLevel::Minor,
            color: Some("0.9".into()),
            ..GridOptions::default()
        },
    );
}

fn log_demo_1(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 3)?;
    let (ax1, ax2, ax3) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(0, 2)]);
    let t = arange(0.01, 10.0, 0.01);
    ax1.semilogx(&t, map(&t, |t| (2.0 * std::f64::consts::PI * t).sin()))?;
    ax1.set_title("semilogx");
    log_grids(ax1);
    ax2.set_yscale("log")?;
    ax2.plot_fmt([0.0, 4.0, 8.0, 12.0], [1.0, 10.0, 100.0, 1000.0], "o--")?;
    ax2.set_title("semilogy");
    log_grids(ax2);
    ax3.set_xscale("log")?;
    ax3.set_yscale("log")?;
    ax3.plot_fmt(
        [1.0, 10.0, 100.0, 1000.0],
        [5.0, 50.0, 500.0, 5000.0],
        "o--",
    )?;
    ax3.set_title("loglog");
    log_grids(ax3);
    Ok(())
}

fn log_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.bar(
        ["L1 cache", "L2 cache", "L3 cache", "RAM", "SSD"],
        [32.0, 1_000.0, 32_000.0, 16_000_000.0, 512_000_000.0],
        BarOptions::default(),
    )?;
    ax.set_scale_with(
        AxisName::Y,
        "log",
        ScaleOptions {
            base: Some(2.0),
            ..ScaleOptions::default()
        },
    )?;
    ax.set_yticks([1.0, 2f64.powi(10), 2f64.powi(20), 2f64.powi(30)])
        .set_yticklabels(["kB", "MB", "GB", "TB"])?;
    ax.set_title("Typical memory sizes");
    ax.yaxis().grid(true, GridOptions::default());
    Ok(())
}

fn log_demo_3(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0, 10);
    let y = map(&x, |x| 10f64.powf(x));
    let yerr = map(&y, |y| 1.75 + 0.75 * y);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let axs = fig.subplots(1, 2)?;
    fig.suptitle("errorbars going negative");
    for (ax, (nonpositive, title)) in axs.as_slice().iter().zip([
        (NonPositive::Mask, "nonpositive=\"mask\""),
        (NonPositive::Clip, "nonpositive=\"clip\""),
    ]) {
        ax.set_scale_with(
            AxisName::Y,
            "log",
            ScaleOptions {
                nonpositive: Some(nonpositive),
                ..ScaleOptions::default()
            },
        )?;
        ax.set_title(title);
        ax.errorbar(
            &x,
            &y,
            ErrorbarOptions {
                yerr: Some(ErrorSize::Symmetric(yerr.clone().into())),
                fmt: "o".parse()?,
                capsize: Some(5.0),
                ..ErrorbarOptions::default()
            },
        )?;
    }
    Ok(())
}

/// Turns the minor grid lines of the x axis on.
fn minor_x_grid(ax: &Axes) {
    ax.xaxis().grid(
        true,
        GridOptions {
            which: TickLevel::Minor,
            ..GridOptions::default()
        },
    );
}

fn symlog_demo_1(fig: &Figure) -> CaseResult {
    let x = arange(-50.0, 50.0, 0.01);
    let y = arange(0.0, 100.0, 0.01);
    let axs = fig.subplots(3, 1)?;
    let (ax0, ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)], &axs[(2, 0)]);
    ax0.plot(&x, &y)?;
    ax0.set_xscale("symlog")?;
    ax0.set_ylabel("symlogx");
    ax0.grid(true);
    minor_x_grid(ax0);
    ax1.plot(&y, &x)?;
    ax1.set_yscale("symlog")?;
    ax1.set_ylabel("symlogy");
    ax2.plot(&x, map(&x, |x| (x / 3.0).sin()))?;
    ax2.set_xscale("symlog")?;
    ax2.set_scale_with(
        AxisName::Y,
        "symlog",
        ScaleOptions {
            linthresh: Some(0.015),
            ..ScaleOptions::default()
        },
    )?;
    ax2.grid(true);
    ax2.set_ylabel("symlog both");
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

/// `format_axes` of `symlog_demo` on a line over a symlog x scale with
/// `linthresh` and `linscale` (matplotlib's defaults are 2 and 1): minor
/// ticks at 2 to 9 times the powers of ten, both grids, the linear region
/// shaded, and `title` above.
fn symlog_region(ax: &Axes, linthresh: f64, linscale: f64, title: &str) -> CaseResult {
    let x = linspace(-60.0, 60.0, 201);
    let y = linspace(0.0, 100.0, 201);
    ax.plot(&x, &y)?;
    ax.set_scale_with(
        AxisName::X,
        "symlog",
        ScaleOptions {
            linthresh: Some(linthresh),
            linscale: Some(linscale),
            subs: Some((2..10).map(f64::from).collect()),
            ..ScaleOptions::default()
        },
    )?;
    ax.grid(true);
    minor_x_grid(ax);
    ax.axvspan(-linthresh, linthresh).set_color("0.9");
    ax.set_title(title);
    Ok(())
}

fn symlog_demo_2(fig: &Figure) -> CaseResult {
    let (ax1, ax2) = stacked_pair(fig)?;
    for (ax, linthresh) in [(ax1, 1.0), (ax2, 5.0)] {
        let title = format!("Linear region: linthresh={linthresh}");
        symlog_region(&ax, linthresh, 1.0, &title)?;
    }
    Ok(())
}

fn symlog_demo_3(fig: &Figure) -> CaseResult {
    let (ax1, ax2) = stacked_pair(fig)?;
    for (ax, linscale) in [(ax1, 1.0), (ax2, 0.1)] {
        let title = format!("Linear region: linthresh=1, linscale={linscale}");
        symlog_region(&ax, 1.0, linscale, &title)?;
    }
    Ok(())
}

fn symlog_demo_4(fig: &Figure) -> CaseResult {
    symlog_region(
        &fig.subplot(),
        2.0,
        0.05,
        "Discontinuous gradient at linear/log transition",
    )
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_ticks/log_demo_1", log_demo_1).sized(7.0, 7.0 / 3.0, 100.0),
    FigureCase::new("gallery_ticks/log_demo_2", log_demo_2),
    FigureCase::new("gallery_ticks/log_demo_3", log_demo_3).sized(6.0, 3.0, 100.0),
    FigureCase::new("gallery_ticks/symlog_demo_1", symlog_demo_1),
    FigureCase::new("gallery_ticks/symlog_demo_2", symlog_demo_2),
    FigureCase::new("gallery_ticks/symlog_demo_3", symlog_demo_3),
    FigureCase::new("gallery_ticks/symlog_demo_4", symlog_demo_4),
];
