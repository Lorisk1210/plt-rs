//! Lines, markers, filled areas, stacked areas, stems and axis scales.

use std::f64::consts::PI;

use plt::axes::{Baseline, StackplotOptions, StemOptions};
use plt::collections::CollectionProps;
use plt::lines::{AxLineThrough, StepWhere};
use plt::offsetbox::AnchorLoc;
use plt::prelude::*;
use plt::scale::ScaleOptions;
use plt::ticker::AxisName;
use plt::transforms::Point;

use super::{Built, Entry, background};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry::figure("line_plot", "Line plot with a legend", line_plot),
    Entry::figure(
        "confidence_band",
        "A mean and its spread (fill_between)",
        confidence_band,
    ),
    Entry::figure(
        "fill_between_where",
        "fill_between where one curve exceeds another",
        fill_between_where,
    ),
    Entry::figure("steps", "Step plots: pre, mid and post", steps),
    Entry::figure("errorbars", "Error bars with caps on a log axis", errorbars),
    Entry::figure("stackplot", "Stacked areas (stackplot)", stackplot),
    Entry::figure(
        "streamgraph",
        "A streamgraph: stackplot with a wiggle baseline",
        streamgraph,
    ),
    Entry::figure("stem", "A sampled signal as a stem plot", stem),
    Entry::figure(
        "reference_lines",
        "Reference lines and spans: axhline, axvspan, axline",
        reference_lines,
    ),
    Entry::figure(
        "axis_scales",
        "Linear, log, symlog and logit scales",
        axis_scales,
    ),
];

/// `values` mapped through `f`.
pub(crate) fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
    values.iter().copied().map(f).collect()
}

/// Damped oscillations of three phases, labelled and with a legend.
fn line_plot() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let t = linspace(0.0, 10.0, 400);
    for (phase, label) in [(0.0, "phase 0"), (1.0, "phase 1"), (2.0, "phase 2")] {
        let y = map(&t, |t| (t + phase).sin() * (-t / 6.0).exp());
        ax.plot_with(&t, y, LineProps::default().label(label))?;
    }
    ax.set_title("Damped oscillations");
    ax.set_xlabel("time (s)").set_ylabel("amplitude");
    ax.grid(true);
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// A noisy-looking mean (made of sines) with a shaded band around it.
fn confidence_band() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 4.0 * PI, 200);
    let mean = map(&x, |x| x.sin() + 0.2 * (3.1 * x).sin());
    let spread = map(&x, |x| 0.25 + 0.1 * (0.7 * x).cos());
    let low: Vec<f64> = mean.iter().zip(&spread).map(|(m, s)| m - s).collect();
    let high: Vec<f64> = mean.iter().zip(&spread).map(|(m, s)| m + s).collect();
    ax.fill_between(
        &x,
        low,
        high,
        FillBetweenOptions {
            props: CollectionProps::default().alpha(0.3).label("spread"),
            ..FillBetweenOptions::default()
        },
    )?;
    ax.plot_with(&x, mean, LineProps::default().label("mean"))?;
    ax.set_title("Mean and spread");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// Income against spending: the area between them is green where the
/// first is higher and red where it is lower. `interpolate` closes each
/// region exactly where the curves cross.
fn fill_between_where() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let month = linspace(0.0, 24.0, 300);
    let income = map(&month, |m| 10.0 + 0.1 * m + 1.5 * (m * PI / 6.0).sin());
    let spending = map(&month, |m| 10.5 + 0.08 * m + (m * PI / 4.0).cos());
    for (surplus, color, label) in [
        (true, "tab:green", "surplus"),
        (false, "tab:red", "deficit"),
    ] {
        let mask = income
            .iter()
            .zip(&spending)
            .map(|(i, s)| (i > s) == surplus);
        let options = FillBetweenOptions {
            mask: Some(mask.collect()),
            interpolate: true,
            props: CollectionProps::default()
                .color(color)
                .alpha(0.25)
                .label(label),
            ..FillBetweenOptions::default()
        };
        ax.fill_between(&month, &income, &spending, options)?;
    }
    ax.plot_with(
        &month,
        income,
        LineProps::default().color("tab:green").label("income"),
    )?;
    ax.plot_with(
        &month,
        spending,
        LineProps::default().color("tab:red").label("spending"),
    )?;
    ax.set_title("Monthly balance");
    ax.set_xlabel("month").set_ylabel("k€");
    ax.legend(LegendOptions::default().ncols(2))?;
    Ok(fig)
}

/// The same data drawn as steps at the three positions.
fn steps() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x: Vec<f64> = (0..12).map(f64::from).collect();
    let y = map(&x, |x| (x * 0.6).sin() * 2.0);
    for (offset, at, label) in [
        (0.0, StepWhere::Pre, "pre"),
        (3.0, StepWhere::Mid, "mid"),
        (6.0, StepWhere::Post, "post"),
    ] {
        let shifted = map(&y, |y| y + offset);
        ax.step(x.clone(), shifted, at, LineProps::default().label(label))?;
    }
    ax.set_title("step(where=...)");
    ax.legend(LegendOptions::default())?;
    Ok(fig)
}

/// Measurements with symmetric error bars and caps.
fn errorbars() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.5, 5.0, 10);
    let y = map(&x, |x| (-x / 2.0).exp() * 3.0);
    let err = map(&x, |x| 0.1 + 0.05 * x);
    ax.errorbar(
        &x,
        &y,
        ErrorbarOptions {
            yerr: Some(err.into()),
            capsize: Some(4.0),
            fmt: "o".parse()?,
            ..ErrorbarOptions::default()
        },
    )?;
    ax.set_title("Exponential decay with error bars");
    ax.set_yscale("log")?;
    Ok(fig)
}

/// Electricity generation by source, each layer stacked on the ones
/// below so that the top edge is the total.
fn stackplot() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let years: Vec<f64> = (2000..=2024).map(f64::from).collect();
    let trend =
        |start: f64, slope: f64| map(&years, move |y| (start + slope * (y - 2000.0)).max(0.0));
    let sources = [
        ("coal", trend(40.0, -1.4)),
        ("gas", trend(20.0, 0.4)),
        ("nuclear", trend(25.0, -0.2)),
        ("wind", trend(1.0, 1.1)),
        ("solar", map(&years, |y| 0.02 * (y - 2000.0).powi(2))),
    ];
    let layers: Vec<Vec<f64>> = sources.iter().map(|(_, layer)| layer.clone()).collect();
    for (layer, (label, _)) in ax
        .stackplot(&years, &layers, StackplotOptions::default())?
        .iter()
        .zip(&sources)
    {
        layer.set_label(*label);
    }
    ax.set_title("Electricity generation by source");
    ax.set_ylabel("TWh");
    ax.set_margins(0.0, 0.05)?;
    // Outside the axes on the right, top layer first like the stack.
    let legend = LegendOptions::default()
        .loc(AnchorLoc::UpperLeft)
        .bbox_to_anchor((1.02, 1.0))
        .reverse(true);
    ax.legend(legend)?;
    Ok(fig)
}

/// Bumps rising and fading over time, stacked around a baseline that
/// keeps the layers as flat as possible (a streamgraph).
fn streamgraph() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 100.0, 200);
    let layers: Vec<Vec<f64>> = (0..7)
        .map(|i| {
            let (centre, width) = (f64::from(i) * 14.0 + 5.0, 12.0 + 3.0 * f64::from(i % 3));
            map(&x, |x| 1.0 + 6.0 * (-((x - centre) / width).powi(2)).exp())
        })
        .collect();
    let options = StackplotOptions {
        baseline: Baseline::Wiggle,
        props: CollectionProps::default()
            .edgecolor(background())
            .linewidth(0.5),
        ..StackplotOptions::default()
    };
    ax.stackplot(&x, &layers, options)?;
    ax.set_title("Streamgraph (baseline = wiggle)");
    ax.set_margins(0.0, 0.05)?;
    Ok(fig)
}

/// A damped cosine sampled at whole steps: each sample is a stem from
/// the baseline to a marker.
fn stem() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let n: Vec<f64> = (0..40).map(f64::from).collect();
    let samples = map(&n, |n| (n * 0.45).cos() * (-n / 15.0).exp());
    let stems = ax.stem(&n, &samples, StemOptions::default())?;
    stems.baseline.set_color("0.5");
    ax.set_title("x[n] = cos(0.45 n) e^(-n/15)");
    ax.set_xlabel("n");
    Ok(fig)
}

/// Lines and bands placed by the axes rather than by data extents: a
/// threshold across the whole width, a highlighted interval spanning the
/// whole height, and a trend line that extends to the edges.
fn reference_lines() -> Built {
    let fig = Figure::with_size(6.4, 4.0, 100.0);
    let ax = fig.subplot();
    let x = linspace(0.0, 10.0, 60);
    let y = map(&x, |x| 0.4 * x + 1.2 * (1.7 * x).sin());
    ax.axvspan(3.0, 5.0)
        .set_facecolor("tab:orange")
        .set_alpha(Some(0.15));
    ax.axhline(
        4.0,
        LineProps::fmt("--")?.color("tab:red").label("threshold"),
    );
    ax.axline(
        Point::new(0.0, 0.0),
        AxLineThrough::Slope(0.4),
        LineProps::fmt(":")?.color("0.5").label("trend"),
    )?;
    ax.plot_with(
        &x,
        y,
        LineProps::fmt("o-")?.markersize(4.0).label("measured"),
    )?;
    ax.set_title("axhline, axvspan and axline");
    ax.legend(LegendOptions::default().loc(AnchorLoc::LowerRight))?;
    Ok(fig)
}

/// One curve of each family on the scale that straightens it.
fn axis_scales() -> Built {
    let fig = Figure::with_size(7.0, 5.0, 100.0);
    let axs = fig.subplots(2, 2)?;
    let x = linspace(0.01, 10.0, 400);
    axs[(0, 0)].plot(&x, &x)?;
    axs[(0, 0)].set_title("linear");
    axs[(0, 1)].plot(&x, map(&x, |x| 10_f64.powf(x / 2.0)))?;
    axs[(0, 1)].set_yscale("log")?.set_title("log");
    let wide = linspace(-10.0, 10.0, 400);
    axs[(1, 0)].plot(&wide, map(&wide, |x| x.powi(3)))?;
    // Linear within ±10, logarithmic beyond.
    let linear_within = ScaleOptions {
        linthresh: Some(10.0),
        ..ScaleOptions::default()
    };
    axs[(1, 0)].set_scale_with(AxisName::Y, "symlog", linear_within)?;
    axs[(1, 0)].set_title("symlog");
    let p = linspace(0.001, 0.999, 400);
    axs[(1, 1)].plot(&p, map(&p, |p| 10.0 * (p / (1.0 - p)).ln()))?;
    axs[(1, 1)].set_xscale("logit")?.set_title("logit");
    Ok(fig)
}
