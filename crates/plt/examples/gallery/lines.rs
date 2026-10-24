//! Lines, markers, filled areas and error bars.

use plt::axes::{ErrorbarOptions, FillBetweenOptions};
use plt::collections::CollectionProps;
use plt::legend::LegendOptions;
use plt::lines::StepWhere;
use plt::prelude::*;

use super::{Built, Entry};

/// The figures of this topic.
pub(crate) const ENTRIES: &[Entry] = &[
    Entry {
        name: "line_plot",
        caption: "Line plot with a legend",
        build: line_plot,
    },
    Entry {
        name: "confidence_band",
        caption: "A mean and its confidence band (fill_between)",
        build: confidence_band,
    },
    Entry {
        name: "steps",
        caption: "Step plots: pre, mid and post",
        build: steps,
    },
    Entry {
        name: "errorbars",
        caption: "Error bars with caps",
        build: errorbars,
    },
];

/// `values` mapped through `f`.
fn map(values: &[f64], f: impl Fn(f64) -> f64) -> Vec<f64> {
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
    let x = linspace(0.0, 4.0 * std::f64::consts::PI, 200);
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
