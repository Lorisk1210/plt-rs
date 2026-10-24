//! The series plots of the section: stack plots, stairs, stems, steps and
//! a timeline of releases.

use std::f64::consts::PI;

use plt::axes::{Axes, Baseline, GridOptions, StackplotOptions, StairsOptions, StemOptions};
use plt::collections::{CollectionProps, EdgeColors};
use plt::colors::to_rgba;
use plt::dates::{DateFormatter, DateTime, YearLocator};
use plt::fonts::FontWeight;
use plt::layout::LayoutEngine;
use plt::legend::LegendOptions;
use plt::lines::StepWhere;
use plt::orientation::Orientation;
use plt::patches::{BoxStyle, StepBaseline, StepPatch};
use plt::prelude::*;
use plt::stats::{Bins, HistogramOptions, histogram};
use plt::text::{AnnotateOptions, TextBoxProps, VAlign};
use plt::ticker::MultipleLocator;
use plt::transforms::Point;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

fn stackplot_demo_1(fig: &Figure) -> CaseResult {
    let year = [
        1950.0, 1960.0, 1970.0, 1980.0, 1990.0, 2000.0, 2010.0, 2018.0,
    ];
    let population_by_continent: [(&str, [f64; 8]); 5] = [
        (
            "Africa",
            [0.228, 0.284, 0.365, 0.477, 0.631, 0.814, 1.044, 1.275],
        ),
        (
            "the Americas",
            [0.340, 0.425, 0.519, 0.619, 0.727, 0.840, 0.943, 1.006],
        ),
        (
            "Asia",
            [1.394, 1.686, 2.120, 2.625, 3.202, 3.714, 4.169, 4.560],
        ),
        (
            "Europe",
            [0.220, 0.253, 0.276, 0.295, 0.310, 0.303, 0.294, 0.293],
        ),
        (
            "Oceania",
            [0.012, 0.015, 0.019, 0.022, 0.026, 0.031, 0.036, 0.039],
        ),
    ];
    let ax = fig.subplot();
    let layers: Vec<Vec<f64>> = population_by_continent
        .iter()
        .map(|(_, values)| values.to_vec())
        .collect();
    let options = StackplotOptions {
        props: CollectionProps::default().alpha(0.8),
        ..StackplotOptions::default()
    };
    for (layer, (name, _)) in ax
        .stackplot(year, &layers, options)?
        .iter()
        .zip(population_by_continent)
    {
        layer.set_label(name);
    }
    let reversed = LegendOptions {
        reverse: true,
        ..LegendOptions::default().loc("upper left")
    };
    ax.legend(reversed)?;
    ax.set_title("World population");
    ax.set_xlabel("Year");
    ax.set_ylabel("Number of people (billions)");
    ax.yaxis().set_minor_locator(MultipleLocator::new(0.2)?);
    Ok(())
}

/// The example's `gaussian_mixture`: five random Gaussians evaluated
/// at `x`.
fn gaussian_mixture(rng: &mut SpecRng, x: &[f64]) -> Vec<f64> {
    let mut a = vec![0.0; x.len()];
    let dx = x[x.len() - 1] - x[0];
    for _ in 0..5 {
        let amplitude = 1.0 / (0.1 + rng.next_f64());
        let x0 = (2.0 * rng.next_f64() - 0.5) * dx;
        let z = 10.0 / (0.1 + rng.next_f64()) / dx;
        for (a, x) in a.iter_mut().zip(x) {
            *a += amplitude * (-(z * (x - x0)).powi(2)).exp();
        }
    }
    a
}

fn stackplot_demo_2(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(19_680_801);
    let x = linspace(0.0, 100.0, 101);
    let ys: Vec<Vec<f64>> = (0..3).map(|_| gaussian_mixture(&mut rng, &x)).collect();
    let options = StackplotOptions {
        baseline: Baseline::Wiggle,
        ..StackplotOptions::default()
    };
    fig.subplot().stackplot(&x, &ys, options)?;
    Ok(())
}

/// Stairs from `edges` (`None`: automatic) resting on `baseline`.
fn stairs_on(edges: Option<Vec<f64>>, baseline: StepBaseline) -> StairsOptions {
    StairsOptions {
        edges,
        baseline,
        ..StairsOptions::default()
    }
}

fn stairs_demo_1(fig: &Figure) -> CaseResult {
    let samples = SpecRng::new(0).normal(5000, 5.0, 3.0);
    let bins = linspace(0.0, 10.0, 20);
    let options = HistogramOptions {
        bins: Bins::Edges(bins.clone()),
        ..HistogramOptions::default()
    };
    let h = histogram(&samples, options)?.counts;
    let axs = fig.subplots(3, 1)?;
    let shifted = |offset: f64| Some(map(&bins, |e| e + offset));
    let step_histograms = [
        (
            stairs_on(shifted(0.0), StepBaseline::Value(0.0)),
            "Simple histogram",
        ),
        (
            stairs_on(shifted(5.0), StepBaseline::Value(50.0)),
            "Modified baseline",
        ),
        (stairs_on(shifted(10.0), StepBaseline::None), "No edges"),
    ];
    for (options, label) in step_histograms {
        axs[(0, 0)].stairs(h.clone(), options)?.set_label(label);
    }
    axs[(0, 0)].set_title("Step Histograms");
    let filled = StairsOptions {
        fill: true,
        ..StairsOptions::default()
    };
    axs[(1, 0)]
        .stairs(arange(1.0, 6.0, 1.0), filled)?
        .set_label("Filled histogram\nw/ automatic edges");
    let horizontal = StairsOptions {
        orientation: Orientation::Horizontal,
        ..stairs_on(Some(arange(2.0, 8.0, 1.0)), StepBaseline::Value(0.0))
    };
    let hatched = axs[(1, 0)].stairs(map(&arange(1.0, 6.0, 1.0), |v| v * 0.3), horizontal)?;
    hatched
        .set_hatch("//")
        .set_label("Hatched histogram\nw/ horizontal orientation");
    axs[(1, 0)].set_title("Filled histogram");
    let patch = StepPatch::new(vec![1.0, 2.0, 3.0, 2.0, 1.0], arange(1.0, 7.0, 1.0))?;
    axs[(2, 0)]
        .add_patch(patch)
        .set_label("Patch derived underlying object\nwith default edge/facecolor behaviour");
    axs[(2, 0)].set_xlim(0.0, 7.0)?.set_ylim(-1.0, 5.0)?;
    axs[(2, 0)].set_title("StepPatch artist");
    for ax in axs.as_slice() {
        ax.legend(LegendOptions::default())?;
    }
    Ok(())
}

fn stairs_demo_2(fig: &Figure) -> CaseResult {
    let a = [
        [0.0, 0.0, 0.0],
        [1.0, 2.0, 3.0],
        [2.0, 4.0, 6.0],
        [3.0, 6.0, 9.0],
    ];
    let ax = fig.subplot();
    for pair in a.windows(2) {
        let options = StairsOptions {
            fill: true,
            ..stairs_on(None, StepBaseline::PerStep(pair[0].to_vec()))
        };
        ax.stairs(pair[1].to_vec(), options)?;
    }
    Ok(())
}

/// The faint grey dashed line through the data points of the step demos.
fn faint(ax: &Axes, x: &[f64], y: &[f64]) -> CaseResult {
    ax.plot_with(x, y, LineProps::fmt("o--")?.color("grey").alpha(0.3))?;
    Ok(())
}

fn stairs_demo_3(fig: &Figure) -> CaseResult {
    let bins = arange(0.0, 14.0, 1.0);
    let centers: Vec<f64> = bins
        .windows(2)
        .map(|w| w[0] + (w[1] - w[0]) / 2.0)
        .collect();
    let y = map(&centers, |c| (c / 2.0).sin());
    let ax = fig.subplot();
    let post = LineProps::default().label("step(where=\"post\")");
    ax.step(&bins[..13], &y, StepWhere::Post, post)?;
    faint(&ax, &bins[..13], &y)?;
    let below = map(&y, |y| y - 1.0);
    ax.stairs(
        below.clone(),
        stairs_on(Some(bins.clone()), StepBaseline::None),
    )?
    .set_label("stairs()");
    faint(&ax, &centers, &below)?;
    // `np.repeat(bins, 2)` against the step corners.
    let corners_x: Vec<f64> = bins.iter().flat_map(|&b| [b, b]).collect();
    let mut corners_y = vec![y[0]];
    corners_y.extend(y.iter().flat_map(|&v| [v, v]));
    corners_y.push(y[12]);
    let red = LineProps::fmt("o")?.color("red").alpha(0.2);
    ax.plot_with(corners_x, map(&corners_y, |v| v - 1.0), red)?;
    ax.legend(LegendOptions::default())?;
    ax.set_title("step() vs. stairs()");
    Ok(())
}

/// `x` and `y` of `stem_plot`.
fn stem_curve() -> (Vec<f64>, Vec<f64>) {
    let x = linspace(0.1, 2.0 * PI, 41);
    let y = map(&x, |x| x.sin().exp());
    (x, y)
}

fn stem_plot_1(fig: &Figure) -> CaseResult {
    let (x, y) = stem_curve();
    fig.subplot().stem(x, y, StemOptions::default())?;
    Ok(())
}

fn stem_plot_2(fig: &Figure) -> CaseResult {
    let (x, y) = stem_curve();
    let options = StemOptions {
        linefmt: "grey".parse()?,
        markerfmt: "D".parse()?,
        bottom: 1.1,
        ..StemOptions::default()
    };
    let stems = fig.subplot().stem(x, y, options)?;
    stems.markerline.set_markerfacecolor(Color::none());
    Ok(())
}

/// The example's three step curves over their data points, each drawn by
/// `steps` with its offset and label index, under a light x grid and a
/// legend titled `title`.
fn step_rows(
    ax: &Axes,
    steps: impl Fn(&[f64], &[f64], usize) -> CaseResult,
    title: &str,
) -> CaseResult {
    let x = arange(0.0, 14.0, 1.0);
    let y = map(&x, |x| (x / 2.0).sin());
    for (i, offset) in [2.0, 1.0, 0.0].into_iter().enumerate() {
        let shifted = map(&y, |y| y + offset);
        steps(&x, &shifted, i)?;
        faint(ax, &x, &shifted)?;
    }
    let grid = GridOptions {
        color: Some(Color::from("0.95")),
        ..GridOptions::default()
    };
    ax.xaxis().grid(true, grid);
    ax.legend(LegendOptions::default().title(title))?;
    Ok(())
}

fn step_demo_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let labels = [
        (StepWhere::Pre, "pre (default)"),
        (StepWhere::Mid, "mid"),
        (StepWhere::Post, "post"),
    ];
    let steps = |x: &[f64], y: &[f64], i: usize| -> CaseResult {
        let (at, label) = labels[i];
        ax.step(x, y, at, LineProps::default().label(label))?;
        Ok(())
    };
    step_rows(&ax, steps, "Parameter where:")?;
    ax.set_title("plt.step(where=...)");
    Ok(())
}

fn step_demo_2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let labels = [
        ("steps", "steps (=steps-pre)"),
        ("steps-mid", "steps-mid"),
        ("steps-post", "steps-post"),
    ];
    let steps = |x: &[f64], y: &[f64], i: usize| -> CaseResult {
        let (drawstyle, label) = labels[i];
        let props = LineProps::default()
            .drawstyle(drawstyle.parse::<DrawStyle>()?)
            .label(label);
        ax.plot_with(x, y, props)?;
        Ok(())
    };
    step_rows(&ax, steps, "Parameter drawstyle:")?;
    ax.set_title("plt.plot(drawstyle=...)");
    Ok(())
}

/// The fallback release list of `timeline`: version and date.
const RELEASES: [(&str, (i32, u32, u32)); 23] = [
    ("2.2.4", (2019, 2, 26)),
    ("3.0.3", (2019, 2, 26)),
    ("3.0.2", (2018, 11, 10)),
    ("3.0.1", (2018, 11, 10)),
    ("3.0.0", (2018, 9, 18)),
    ("2.2.3", (2018, 8, 10)),
    ("2.2.2", (2018, 3, 17)),
    ("2.2.1", (2018, 3, 16)),
    ("2.2.0", (2018, 3, 6)),
    ("2.1.2", (2018, 1, 18)),
    ("2.1.1", (2017, 12, 10)),
    ("2.1.0", (2017, 10, 7)),
    ("2.0.2", (2017, 5, 10)),
    ("2.0.1", (2017, 5, 2)),
    ("2.0.0", (2017, 1, 17)),
    ("1.5.3", (2016, 9, 9)),
    ("1.5.2", (2016, 7, 3)),
    ("1.5.1", (2016, 1, 10)),
    ("1.5.0", (2015, 10, 29)),
    ("1.4.3", (2015, 2, 16)),
    ("1.4.2", (2014, 10, 26)),
    ("1.4.1", (2014, 10, 18)),
    ("1.4.0", (2014, 8, 26)),
];

/// A release: its date and its version split into numbers.
struct Release {
    date: DateTime,
    version: [u32; 3],
}

impl Release {
    /// Whether the release is a feature release (`x.y.0`).
    fn is_feature(&self) -> bool {
        self.version[2] == 0
    }
}

/// The releases sorted by date, then version.
fn releases() -> Result<Vec<Release>, Box<dyn std::error::Error>> {
    let mut releases = RELEASES
        .iter()
        .map(|&(version, (year, month, day))| {
            let numbers = version
                .split('.')
                .map(str::parse::<u32>)
                .collect::<Result<Vec<_>, _>>()?;
            let version: [u32; 3] = numbers.try_into().map_err(|_| "three version numbers")?;
            Ok(Release {
                date: DateTime::new(year, month, day, 0, 0, 0, 0)?,
                version,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    releases.sort_by_key(|release| (release.date.timestamp_micros(), release.version));
    Ok(releases)
}

/// The stem heights: shorter for later micro releases, alternating sides
/// by macro-meso release.
fn levels(releases: &[Release]) -> Vec<f64> {
    let mut macro_meso: Vec<[u32; 2]> = releases
        .iter()
        .map(|r| [r.version[0], r.version[1]])
        .collect();
    macro_meso.sort_unstable();
    macro_meso.dedup();
    releases
        .iter()
        .map(|release| {
            let h = 1.0 + 0.8 * (5.0 - f64::from(release.version[2]));
            let rank = macro_meso
                .iter()
                .position(|mm| *mm == [release.version[0], release.version[1]])
                .unwrap_or_default();
            if rank % 2 == 0 { h } else { -h }
        })
        .collect()
}

fn timeline(fig: &Figure) -> CaseResult {
    let releases = releases()?;
    let levels = levels(&releases);
    fig.set_layout_engine(LayoutEngine::constrained())?;
    let ax = fig.subplot();
    ax.set_title("Matplotlib release dates");
    let dates: Vec<DateTime> = releases.iter().map(|r| r.date).collect();
    let days = ax.xaxis().convert_units(dates);
    let colors = releases
        .iter()
        .map(|r| to_rgba("tab:red", Some(if r.is_feature() { 1.0 } else { 0.5 })).map(Color::from))
        .collect::<Result<_, _>>()?;
    ax.vlines(&days, [0.0], &levels)?
        .set_edgecolors(EdgeColors::Colors(colors));
    ax.axhline(0.0, LineProps::default().color("black"));
    let (meso, micro): (Vec<_>, Vec<_>) = releases
        .iter()
        .zip(&days)
        .partition(|(r, _)| r.is_feature());
    for (group, face) in [(micro, "white"), (meso, "tab:red")] {
        let x: Vec<f64> = group.iter().map(|&(_, day)| *day).collect();
        let zeros = vec![0.0; x.len()];
        ax.plot_with(x, zeros, LineProps::fmt("ko")?.markerfacecolor(face))?;
    }
    for ((release, &day), &level) in releases.iter().zip(&days).zip(&levels) {
        let version = release.version.map(|n| n.to_string()).join(".");
        let options = AnnotateOptions {
            xytext: Some(Point::new(-3.0, level.signum() * 3.0)),
            textcoords: Some("offset points".parse()?),
            ..AnnotateOptions::default()
        };
        let label = ax.annotate(version, Point::new(day, level), options)?;
        label.set_valign(if level > 0.0 {
            VAlign::Bottom
        } else {
            VAlign::Top
        });
        label.set_weight(if release.is_feature() {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        });
        let backdrop = PatchProps::default()
            .linewidth(0.0)
            .facecolor(Color::from((1.0, 1.0, 1.0, 0.7)));
        label.set_bbox(Some(
            TextBoxProps::new("square, pad=0".parse::<BoxStyle>()?).with_patch(backdrop),
        ));
    }
    ax.xaxis()
        .set_major_locator(YearLocator::new(1, 1, 1)?)
        .set_major_formatter(DateFormatter::new("%Y"));
    ax.yaxis().set_visible(false);
    let spines = ax.spines();
    for spine in [&spines.left, &spines.top, &spines.right] {
        spine.set_visible(false);
    }
    ax.set_margins(None, 0.1)?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_lines/stackplot_demo_1", stackplot_demo_1),
    FigureCase::new("gallery_lines/stackplot_demo_2", stackplot_demo_2),
    FigureCase::new("gallery_lines/stairs_demo_1", stairs_demo_1).sized(7.0, 15.0, 100.0),
    FigureCase::new("gallery_lines/stairs_demo_2", stairs_demo_2),
    FigureCase::new("gallery_lines/stairs_demo_3", stairs_demo_3),
    FigureCase::new("gallery_lines/stem_plot_1", stem_plot_1),
    FigureCase::new("gallery_lines/stem_plot_2", stem_plot_2),
    FigureCase::new("gallery_lines/step_demo_1", step_demo_1),
    FigureCase::new("gallery_lines/step_demo_2", step_demo_2),
    FigureCase::new("gallery_lines/timeline", timeline).sized(8.8, 4.0, 100.0),
];
