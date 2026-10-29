//! Many small calls, one artist each: where the cost of a call (in pltrs,
//! a crossing from Python into Rust) adds up.

use std::f64::consts::PI;

use plt::prelude::*;
use plt::text::AnnotateOptions;
use plt_spec::rng::SpecRng;

use super::{Build, Tier, Workload};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    many("calls/plot_500", "plot_500", plot_500).calls(500),
    many("calls/line_setters_500", "line_setters_500", line_setters).calls(2000),
    many("calls/scatter_300", "scatter_300", scatter_300).calls(300),
    many("calls/bar_300", "bar_300", bar_300).calls(300),
    many("calls/annotate_1000", "annotate_1000", annotate_1000).calls(1001),
];

/// A workload of many calls, saved as PNG.
const fn many(name: &'static str, script: &'static str, setup: fn() -> Build) -> Workload {
    Workload::new(name, script, setup).in_tier(Tier::Calls)
}

/// `count` phase-shifted sine waves of `points` points over `[0, 1]`,
/// each raised above the last: the x values and the waves.
fn waves(count: u32, points: usize) -> (Vec<f64>, Vec<Vec<f64>>) {
    let x = linspace(0.0, 1.0, points);
    let waves = (0..count)
        .map(|k| {
            let k = f64::from(k);
            x.iter()
                .map(|x| (2.0 * PI * (x + k / f64::from(count))).sin() + k / 50.0)
                .collect()
        })
        .collect();
    (x, waves)
}

/// 500 lines of 50 points, one `plot` call each.
fn plot_500() -> Build {
    let (x, waves) = waves(500, 50);
    Box::new(move |fig| {
        let ax = fig.subplot();
        for y in &waves {
            ax.plot(&x, y)?;
        }
        Ok(())
    })
}

/// The colours the setters cycle through.
const COLORS: [&str; 5] = [
    "tab:blue",
    "tab:orange",
    "tab:green",
    "tab:red",
    "tab:purple",
];

/// The line style the setters give line `k` (`"-"`, `"--"`, `":"`, `"-."`
/// in turn).
fn style(k: usize) -> LineStyle {
    match k % 4 {
        0 => LineStyle::Solid,
        1 => LineStyle::Dashed,
        2 => LineStyle::Dotted,
        _ => LineStyle::DashDot,
    }
}

/// 500 lines of 50 points, one `plot` call each, then three setter calls
/// on each line: colour, width and style.
fn line_setters() -> Build {
    let (x, waves) = waves(500, 50);
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (k, y) in waves.iter().enumerate() {
            ax.plot(&x, y)?
                .set_color(COLORS[k % COLORS.len()])
                .set_linewidth(0.5 + 0.5 * (k % 4) as f64)
                .set_linestyle(style(k));
        }
        Ok(())
    })
}

/// 300 clusters of 10 points, one `scatter` call each.
fn scatter_300() -> Build {
    let mut rng = SpecRng::new(26);
    let clusters: Vec<(Vec<f64>, Vec<f64>)> = (0..300_u32)
        .map(|k| {
            let (column, row) = (f64::from(k % 20), f64::from(k / 20));
            let x = rng.uniform(10, column, column + 1.0);
            (x, rng.uniform(10, row, row + 1.0))
        })
        .collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (x, y) in &clusters {
            ax.scatter(x, y)?;
        }
        Ok(())
    })
}

/// 300 bars, one `bar` call each.
fn bar_300() -> Build {
    let heights = SpecRng::new(27).uniform(300, 1.0, 10.0);
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (i, &height) in heights.iter().enumerate() {
            ax.bar([i as f64], vec![height], BarOptions::default())?;
        }
        Ok(())
    })
}

/// 1000 points, each annotated with its own label: 1000 `annotate` calls.
fn annotate_1000() -> Build {
    let mut rng = SpecRng::new(28);
    let x = rng.uniform(1000, 0.0, 10.0);
    let y = rng.uniform(1000, 0.0, 10.0);
    Box::new(move |fig| {
        let ax = fig.subplot();
        ax.plot_with(&x, &y, LineProps::fmt("o")?.markersize(2.0))?;
        for (i, (&px, &py)) in x.iter().zip(&y).enumerate() {
            ax.annotate(format!("p{i}"), (px, py), AnnotateOptions::default())?
                .set_fontsize(6.0);
        }
        Ok(())
    })
}
