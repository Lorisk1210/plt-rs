//! Engineering notation and the locators and formatters of log and symlog
//! axes.

use plt::axes::{AxisName, GridOptions, TickLevel};
use plt::layout::TightLayout;
use plt::prelude::*;
use plt::scale::ScaleOptions;
use plt::ticker::{
    EngFormatter, LogFormatter, LogLabelStyle, LogLocator, LogSubs, NumTicks, OffsetMode,
};

use crate::figures::CaseResult;
use crate::rng::SpecRng;

pub(super) fn eng_formatter(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let xs = logspace(1.0, 9.0, 100, 10.0);
    let ys: Vec<f64> = rng
        .uniform(100, 0.0, 1.0)
        .iter()
        .zip(&xs)
        .map(|(u, x)| (0.8 + 0.4 * u) * x.log10().powi(2))
        .collect();
    let axs = fig.subplots(3, 1)?;
    let [ax0, ax1, ax2] = [&axs[(0, 0)], &axs[(1, 0)], &axs[(2, 0)]];
    for ax in [ax0, ax1] {
        ax.set_xscale("log")?;
        ax.plot(&xs, &ys)?;
    }
    ax0.set_title("Full unit ticklabels, w/ default precision & space separator");
    ax0.xaxis()
        .set_major_formatter(EngFormatter::new().with_unit("Hz"));
    ax0.set_xlabel("Frequency");
    ax1.set_title("SI-prefix only ticklabels, 1-digit precision & thin space separator");
    ax1.xaxis().set_major_formatter(
        EngFormatter::new()
            .with_places(Some(1))
            .with_sep("\u{2009}"),
    );
    ax1.set_xlabel("Frequency [Hz]");
    ax2.set_title("Offset and SI prefix");
    let t = linspace(0.0, 1e-3, 50);
    let f: Vec<f64> = t.iter().map(|t| 3e9 + 2e5 * (t * 6e3).sin()).collect();
    ax2.plot(&t, f)?;
    ax2.xaxis()
        .set_major_formatter(EngFormatter::new().with_unit("s"));
    ax2.yaxis().set_major_formatter(
        EngFormatter::new()
            .with_unit("Hz")
            .with_use_offset(OffsetMode::Auto),
    );
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn log_minor_labels(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    axs[(0, 0)].semilogy([0.0, 1.0], [2.0, 9.0])?;
    axs[(0, 1)].semilogy([0.0, 1.0], [1.0, 60.0])?;
    axs[(0, 2)].semilogy([0.0, 1.0], [1.0, 1e3])?;
    axs[(0, 2)].yaxis().set_minor_formatter(
        LogFormatter::new(LogLabelStyle::Plain, 10.0)
            .with_label_only_base(false)
            .with_minor_thresholds(4.0, 0.5),
    );
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn log_formatter_variants(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(4, 1)?;
    for ax in axs.as_slice() {
        ax.set_xscale("log")?;
        ax.set_xlim(1e-2, 1e4);
        ax.yaxis().set_major_locator(plt::ticker::NullLocator);
    }
    let axis = |i: usize| axs[(i, 0)].xaxis();
    axis(0).set_major_formatter(LogFormatter::new(LogLabelStyle::Exponent, 10.0));
    axis(1).set_major_formatter(LogFormatter::new(LogLabelStyle::Mathtext, 10.0));
    axis(2).set_major_formatter(
        LogFormatter::new(LogLabelStyle::Plain, 10.0).with_label_only_base(true),
    );
    let base2 = ScaleOptions {
        base: Some(2.0),
        ..ScaleOptions::default()
    };
    axs[(3, 0)].set_scale_with(AxisName::X, "log", base2)?;
    axs[(3, 0)].set_xlim(1.0 / 8.0, 512.0);
    axis(3).set_major_formatter(LogFormatter::new(LogLabelStyle::SciNotation, 2.0));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn log_wide_range_decimation(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = logspace(-12.0, 12.0, 25, 10.0);
    let y: Vec<f64> = x.iter().map(|x| x.sqrt()).collect();
    ax.set_xscale("log")?;
    ax.set_yscale("log")?;
    ax.plot_fmt(&x, y, "o-")?;
    ax.grid_with(
        true,
        GridOptions {
            which: TickLevel::Both,
            alpha: Some(0.4),
            ..GridOptions::default()
        },
    );
    Ok(())
}

pub(super) fn log_locator_subs(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let x = logspace(0.0, 3.0, 30, 10.0);
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax1.loglog(&x, &x)?;
    ax1.yaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_subs(LogSubs::Explicit(vec![1.0, 2.0, 5.0])))
        .set_major_formatter(
            LogFormatter::new(LogLabelStyle::Plain, 10.0).with_label_only_base(false),
        );
    let ln: Vec<f64> = x.iter().map(|x| x.ln()).collect();
    ax2.semilogx(&x, ln)?;
    ax2.xaxis()
        .set_major_locator(LogLocator::new(10.0)?.with_numticks(NumTicks::Fixed(3)))
        .set_minor_locator(LogLocator::new(10.0)?.with_subs(LogSubs::Auto));
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) fn symlog_subs_minor(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let x = linspace(-1000.0, 1000.0, 401);
    ax1.plot(&x, &x)?;
    ax1.set_scale_with(
        AxisName::Y,
        "symlog",
        ScaleOptions {
            linthresh: Some(1.0),
            subs: Some(vec![2.0, 5.0]),
            ..ScaleOptions::default()
        },
    )?;
    ax1.grid_with(
        true,
        GridOptions {
            which: TickLevel::Minor,
            alpha: Some(0.3),
            ..GridOptions::default()
        },
    );
    let scaled: Vec<f64> = x.iter().map(|x| x / 100.0).collect();
    ax2.plot(&x, scaled)?;
    ax2.set_scale_with(
        AxisName::X,
        "symlog",
        ScaleOptions {
            linthresh: Some(10.0),
            base: Some(2.0),
            linscale: Some(0.5),
            ..ScaleOptions::default()
        },
    )?;
    ax2.set_scale_with(
        AxisName::Y,
        "symlog",
        ScaleOptions {
            linthresh: Some(0.1),
            ..ScaleOptions::default()
        },
    )?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}
