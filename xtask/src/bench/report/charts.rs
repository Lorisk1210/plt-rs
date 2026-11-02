//! The report's charts, drawn by pltrs (Rust) from the raw data in its
//! own modern look: for each claim, every workload's warm and cold
//! speed-up over matplotlib with its 95% interval, on a log scale; and the
//! start-up times. The SVG files carry no metadata and ids of a fixed
//! salt, so that the same raw data draws the same files.

use std::error::Error;
use std::path::Path;

use pltrs::axes::BarOptions;
use pltrs::figure::{Figure, SaveOptions};
use pltrs::fonts::FontWeight;
use pltrs::legend::{LegendLoc, OutsideEdge};
use pltrs::look::{self, Appearance};
use pltrs::offsetbox::AnchorLoc;
use pltrs::prelude::*;
use pltrs::text::{HAlign, VAlign};
use pltrs_render::Metadata;

use super::super::implementation::Implementation;
use super::super::results::{Results, WorkloadResult};
use super::super::workloads::Tier;
use super::analysis::{Mode, ratio};

/// The file names of the charts written, by what they show.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Charts {
    /// The speed-ups of pltrs (Python).
    pub(super) python: Option<String>,
    /// The speed-ups of pltrs (Rust).
    pub(super) rust: Option<String>,
    /// The start-up times.
    pub(super) startup: Option<String>,
}

/// The colour of warm runs (the first slot of the categorical palette).
const WARM: &str = "#2a78d6";
/// The colour of cold runs (the second slot).
const COLD: &str = "#eb6834";
/// Secondary ink: tick labels and annotations.
const INK: &str = "#52514e";
/// The reference line at 1×.
const BASELINE: &str = "#898781";

/// Draws the charts of `results` into `dir`.
pub(super) fn write(results: &Results, dir: &Path) -> Result<Charts, Box<dyn Error>> {
    std::fs::create_dir_all(dir)?;
    let previous = look::appearance();
    look::set_appearance(Appearance::MODERN);
    let charts = pltrs::rc::context(|| -> Result<Charts, Box<dyn Error>> {
        pltrs::rc::set("svg.hashsalt", "pltrs-bench")?;
        pltrs::rc::set("ytick.labelsize", "7")?;
        let mut charts = Charts::default();
        for contender in Implementation::CONTENDERS {
            let file = format!("{}.svg", contender.key());
            if speedups(results, contender, &dir.join(&file))? {
                match contender {
                    Implementation::Python => charts.python = Some(file),
                    _ => charts.rust = Some(file),
                }
            }
        }
        if startup(results, &dir.join("startup.svg"))? {
            charts.startup = Some("startup.svg".to_owned());
        }
        Ok(charts)
    });
    look::set_appearance(previous);
    charts
}

/// Saves `fig` as SVG at `path`, reproducibly.
fn save(fig: &Figure, path: &Path) -> Result<(), Box<dyn Error>> {
    let options = SaveOptions {
        metadata: Some(Metadata::none()),
        ..SaveOptions::default()
    };
    Ok(fig.savefig_with(path, options)?)
}

/// A row of the speed-up chart: a tier's heading, or a workload.
enum Row<'a> {
    /// The heading of a tier.
    Tier(Tier),
    /// A workload with its warm and cold speed-ups.
    Workload(&'a WorkloadResult),
}

/// The speed-up chart of `contender`, written to `path` unless it has no
/// speed-up to show.
fn speedups(
    results: &Results,
    contender: Implementation,
    path: &Path,
) -> Result<bool, Box<dyn Error>> {
    let mut rows = Vec::new();
    for tier in Tier::ALL {
        let workloads: Vec<&WorkloadResult> = results
            .workloads
            .iter()
            .filter(|w| w.tier == tier && ratio(w, contender, Mode::Warm).is_some())
            .collect();
        if !workloads.is_empty() {
            rows.push(Row::Tier(tier));
            rows.extend(workloads.into_iter().map(Row::Workload));
        }
    }
    if rows.is_empty() {
        return Ok(false);
    }
    let height = 1.2 + 0.19 * rows.len() as f64;
    let fig = Figure::with_size(9.0, height, 100.0);
    fig.set_layout_engine(pltrs::layout::LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let (mut least, mut most) = (1.0_f64, 1.0_f64);
    for (row, entry) in rows.iter().enumerate() {
        if let Row::Workload(w) = entry {
            let (low, high) = speedup_row(&ax, row, w, contender)?;
            least = least.min(low);
            most = most.max(high);
        }
    }
    let (left, right) = (least.min(0.8) / 1.15, most * 1.4);
    ax.set_xscale("log")?;
    ax.set_xlim(left, right)?;
    ax.set_ylim(rows.len() as f64 - 0.4, -0.6)?;
    label_rows(&ax, &rows, left)?;
    let factors: Vec<f64> = [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0]
        .into_iter()
        .filter(|f| (left..=right).contains(f))
        .collect();
    let names: Vec<String> = factors.iter().map(|f| format!("{f}×")).collect();
    ax.set_xticks(factors).set_xticklabels(names)?;
    ax.axvline(
        1.0,
        LineProps::default()
            .color(BASELINE)
            .linewidth(1.0)
            .linestyle(LineStyle::Dashed),
    );
    ax.set_xlabel(
        "speed-up over matplotlib (log scale, right of 1× is faster), with 95% intervals",
    );
    ax.set_title(match contender {
        Implementation::Python => {
            "pltrs (Python) against matplotlib: the same scripts, only the imports changed"
        }
        _ => "pltrs (Rust) against matplotlib: the same figures, written in Rust",
    });
    let legend = LegendOptions {
        loc: Some(LegendLoc::Outside {
            edge: OutsideEdge::Lower,
            loc: AnchorLoc::LowerCenter,
        }),
        ..LegendOptions::default().ncols(2)
    };
    fig.legend(legend)?;
    save(&fig, path)?;
    Ok(true)
}

/// Draws the warm and cold speed-ups of `w` at `row`: a dot (warm) or a
/// square (cold) and its interval. Returns the lowest and highest
/// speed-up drawn. The first row's marks carry the legend's labels.
fn speedup_row(
    ax: &Axes,
    row: usize,
    w: &WorkloadResult,
    contender: Implementation,
) -> Result<(f64, f64), Box<dyn Error>> {
    let (mut least, mut most) = (f64::INFINITY, 0.0_f64);
    let y = row as f64;
    for (mode, offset, color, marker, label) in [
        (Mode::Warm, -0.17, WARM, "o", "warm: the figure alone"),
        (
            Mode::Cold,
            0.17,
            COLD,
            "s",
            "cold: a fresh process, start-up included",
        ),
    ] {
        let Some(r) = ratio(w, contender, mode) else {
            continue;
        };
        let (speedup, low, high) = (1.0 / r.value, 1.0 / r.high, 1.0 / r.low);
        least = least.min(low);
        most = most.max(high);
        ax.hlines([y + offset], [low], [high])?
            .set_edgecolor(color)
            .set_linewidths(vec![1.5]);
        let props = LineProps::fmt(marker)?
            .color(color)
            .markersize(5.0)
            .label(if row == 1 { label } else { "" });
        ax.plot_with([speedup], [y + offset], props)?;
    }
    Ok((least, most))
}

/// Names the workload rows on the y axis, and writes each tier's heading
/// in its own row at the left edge `left`.
fn label_rows(ax: &Axes, rows: &[Row<'_>], left: f64) -> Result<(), Box<dyn Error>> {
    let mut ticks = Vec::new();
    let mut labels = Vec::new();
    for (row, entry) in rows.iter().enumerate() {
        match entry {
            Row::Tier(tier) => {
                ax.text(left, row as f64, tier.title())
                    .set_weight(FontWeight::BOLD)
                    .set_fontsize(9.0)
                    .set_halign(HAlign::Left)
                    .set_valign(VAlign::Center);
            }
            Row::Workload(w) => {
                ticks.push(row as f64);
                labels.push(w.name.clone());
            }
        }
    }
    ax.set_yticks(ticks).set_yticklabels(labels)?;
    Ok(())
}

/// The start-up chart, written to `path` unless the raw data has no
/// start-up times.
fn startup(results: &Results, path: &Path) -> Result<bool, Box<dyn Error>> {
    let processes = [
        ("python", "python -c pass"),
        ("numpy", "python -c \"import numpy\""),
        ("matplotlib", "python -c \"import matplotlib.pyplot\""),
        ("pltrs-python", "python -c \"import pltrs.pyplot\""),
        ("pltrs-rust", "the executable of pltrs (Rust)"),
    ];
    let shown: Vec<(&str, f64)> = processes
        .iter()
        .filter_map(|(key, name)| Some((*name, results.startup(key)?.seconds.median() * 1e3)))
        .collect();
    if shown.is_empty() {
        return Ok(false);
    }
    let fig = Figure::with_size(7.0, 2.8, 100.0);
    fig.set_layout_engine(pltrs::layout::LayoutEngine::constrained())?;
    let ax = fig.subplot();
    let rows: Vec<f64> = (0..shown.len()).map(|i| i as f64).collect();
    let times: Vec<f64> = shown.iter().map(|(_, ms)| *ms).collect();
    let options = BarOptions {
        thickness: vec![0.6].into(),
        color: vec![WARM.into()],
        ..BarOptions::default()
    };
    ax.barh(rows.clone(), times.clone(), options)?;
    let longest = times.iter().copied().fold(0.0, f64::max);
    for (y, ms) in rows.iter().zip(&times) {
        ax.text(ms + longest * 0.01, *y, format!("{ms:.0} ms"))
            .set_color(INK)
            .set_fontsize(8.0)
            .set_valign(VAlign::Center);
    }
    ax.set_yticks(rows)
        .set_yticklabels(shown.iter().map(|(name, _)| *name))?;
    ax.set_ylim(shown.len() as f64 - 0.4, -0.6)?;
    ax.set_xlim(0.0, longest * 1.15)?;
    ax.set_xlabel("median ms from spawn to exit");
    ax.set_title("Start-up: what every cold run pays first");
    save(&fig, path)?;
    Ok(true)
}
