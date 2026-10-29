//! Small, everyday figures (a line plot, a grid of panels, labelled bars,
//! scatter groups) and medium ones (thousands of points with a legend, a
//! colour-mapped scatter, grouped bars, an image with a colorbar).

use std::f64::consts::PI;

use pltrs::axes::BarLabelOptions;
use pltrs::figure::SaveFormat;
use pltrs::prelude::*;
use pltrs::stats::Bins;
use pltrs_spec::rng::SpecRng;

use super::{Build, Tier, Workload, random_walk, smooth_field};

/// The workloads of this module.
pub(super) const WORKLOADS: &[Workload] = &[
    small("small/line_png", "line", line),
    small("small/line_svg", "line", line).saved_as(SaveFormat::Svg),
    small("small/line_pdf", "line", line).saved_as(SaveFormat::Pdf),
    small("small/subplots_2x2_png", "subplots_2x2", subplots_2x2),
    small("small/bar_labels_png", "bar_labels", bar_labels),
    small("small/scatter_groups_png", "scatter_groups", scatter_groups),
    medium("medium/lines_5x1e4_png", "lines_5x1e4", lines_5x1e4),
    medium("medium/lines_5x1e4_svg", "lines_5x1e4", lines_5x1e4).saved_as(SaveFormat::Svg),
    medium("medium/lines_5x1e4_pdf", "lines_5x1e4", lines_5x1e4).saved_as(SaveFormat::Pdf),
    medium("medium/scatter_1e4_png", "scatter_1e4", scatter_1e4),
    medium("medium/grouped_bars_png", "grouped_bars", grouped_bars),
    medium(
        "medium/imshow_500_colorbar_png",
        "imshow_500_colorbar",
        imshow_colorbar,
    ),
];

/// A small workload saved as PNG.
const fn small(name: &'static str, script: &'static str, setup: fn() -> Build) -> Workload {
    Workload::new(name, script, setup).in_tier(Tier::Small)
}

/// A medium workload saved as PNG.
const fn medium(name: &'static str, script: &'static str, setup: fn() -> Build) -> Workload {
    Workload::new(name, script, setup).in_tier(Tier::Medium)
}

/// A single line of 200 points with a title and axis labels.
fn line() -> Build {
    let x = linspace(0.0, 10.0, 200);
    let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        ax.plot(&x, &y)?;
        ax.set_title("A sine wave");
        ax.set_xlabel("time (s)").set_ylabel("amplitude");
        Ok(())
    })
}

/// A 2 × 2 grid: two labelled lines with a legend, a scatter, bars and a
/// histogram, each panel titled.
fn subplots_2x2() -> Build {
    let mut rng = SpecRng::new(21);
    let t = linspace(0.0, 2.0 * PI, 100);
    let sin_t: Vec<f64> = t.iter().map(|t| t.sin()).collect();
    let cos_t: Vec<f64> = t.iter().map(|t| t.cos()).collect();
    let points_x = rng.uniform(100, 0.0, 1.0);
    let points_y = rng.uniform(100, 0.0, 1.0);
    let heights = rng.uniform(5, 1.0, 10.0);
    let sample = rng.normal(1000, 0.0, 1.0);
    Box::new(move |fig| {
        let grid = fig.subplots(2, 2)?;
        let lines = &grid[(0, 0)];
        lines.plot_with(&t, &sin_t, LineProps::default().label("sin"))?;
        lines.plot_with(&t, &cos_t, LineProps::default().label("cos"))?;
        lines.set_title("lines");
        lines.legend(LegendOptions::default())?;
        grid[(0, 1)].scatter(&points_x, &points_y)?;
        grid[(0, 1)].set_title("scatter");
        grid[(1, 0)].bar(0..5, &heights, BarOptions::default())?;
        grid[(1, 0)].set_title("bars");
        let options = HistPlotOptions {
            bins: Some(Bins::Count(20)),
            ..HistPlotOptions::default()
        };
        grid[(1, 1)].hist(&[sample.as_slice()], options)?;
        grid[(1, 1)].set_title("histogram");
        Ok(())
    })
}

/// Six bars on a categorical axis, each labelled with its value.
fn bar_labels() -> Build {
    Box::new(|fig| {
        let ax = fig.subplot();
        let fruits = ["apple", "blueberry", "cherry", "orange", "peach", "plum"];
        let crates = vec![40.0, 100.0, 30.0, 55.0, 72.0, 18.0];
        let bars = ax.bar(fruits, crates, BarOptions::default())?;
        ax.bar_label(&bars, BarLabelOptions::default())?;
        ax.set_ylabel("crates");
        ax.set_title("Fruit delivered this week");
        Ok(())
    })
}

/// Three labelled groups of 50 points with a legend.
fn scatter_groups() -> Build {
    let mut rng = SpecRng::new(22);
    let groups: Vec<(Vec<f64>, Vec<f64>)> = [0.0, 2.0, 4.0]
        .into_iter()
        .map(|center: f64| {
            let x = rng.normal(50, center, 0.6);
            (x, rng.normal(50, center / 2.0, 0.6))
        })
        .collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (k, (x, y)) in groups.iter().enumerate() {
            ax.scatter(x, y)?.set_label(format!("group {k}"));
        }
        ax.set_xlabel("x").set_ylabel("y");
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
}

/// Five random walks of 10⁴ points, labelled, with a legend.
fn lines_5x1e4() -> Build {
    let walks: Vec<_> = (0..5).map(|k| random_walk(10_000, 30 + k)).collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (k, (x, y)) in walks.iter().enumerate() {
            ax.plot_with(x, y, LineProps::default().label(format!("sensor {k}")))?;
        }
        ax.set_title("Five signals of 10,000 points");
        ax.set_xlabel("sample").set_ylabel("value");
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
}

/// 10⁴ normal points with per-point sizes, coloured by their radius.
fn scatter_1e4() -> Build {
    let mut rng = SpecRng::new(24);
    let x = rng.normal(10_000, 0.0, 1.0);
    let y = rng.normal(10_000, 0.0, 1.0);
    let sizes = rng.uniform(10_000, 4.0, 40.0);
    let radius: Vec<f64> = x.iter().zip(&y).map(|(x, y)| x.hypot(*y)).collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        let options = ScatterOptions {
            sizes: Some(sizes.clone()),
            colors: MappedColors::new(radius.clone()).into(),
            ..ScatterOptions::default()
        };
        ax.scatter_with(&x, &y, options)?;
        ax.set_title("10,000 points coloured by radius");
        ax.set_xlabel("x").set_ylabel("y");
        Ok(())
    })
}

/// Three series of 12 bars side by side, labelled, with a legend.
fn grouped_bars() -> Build {
    let mut rng = SpecRng::new(25);
    let sales: Vec<Vec<f64>> = (0..3).map(|_| rng.uniform(12, 5.0, 20.0)).collect();
    Box::new(move |fig| {
        let ax = fig.subplot();
        for (k, (year, heights)) in (2023..).zip(&sales).enumerate() {
            let offset = (k as f64 - 1.0) * 0.25;
            let positions: Vec<f64> = (0..12).map(|month| f64::from(month) + offset).collect();
            let options = BarOptions {
                thickness: vec![0.25].into(),
                label: Some(format!("{year}")),
                ..BarOptions::default()
            };
            ax.bar(positions, heights, options)?;
        }
        ax.set_title("Monthly sales");
        ax.set_xlabel("month").set_ylabel("units");
        ax.legend(LegendOptions::default())?;
        Ok(())
    })
}

/// A 500 × 500 image with a colorbar and a title.
fn imshow_colorbar() -> Build {
    let z = smooth_field(500);
    Box::new(move |fig| {
        let ax = fig.subplot();
        let image = ax.imshow(z.clone(), ImshowOptions::default())?;
        fig.colorbar(&image, ColorbarOptions::default())?;
        ax.set_title("A 500 x 500 field");
        Ok(())
    })
}
