//! The violin plots of the section: customised violins with box plot
//! statistics drawn on top, and the grid of kernel density settings.

use plt::axes::{Axes, ScatterColors, ViolinDrawOptions, ViolinSide, ViolinplotOptions};
use plt::figure::Share;
use plt::gridspec::SubplotOverrides;
use plt::markers::MarkerStyle;
use plt::orientation::Orientation;
use plt::prelude::*;
use plt::stats::{BandwidthMethod, ViolinOptions, percentiles};

use super::{SEED, strings};
use crate::figures::cases::common::drawn_violins;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// The whisker ends of a sorted sample: 1.5 IQR clipped to the data
/// (`adjacent_values`).
fn adjacent_values(vals: &[f64], q1: f64, q3: f64) -> (f64, f64) {
    let upper = (q3 + (q3 - q1) * 1.5).clamp(q3, vals[vals.len() - 1]);
    let lower = (q1 - (q3 - q1) * 1.5).clamp(vals[0], q1);
    (lower, upper)
}

/// Labels the violins and leaves room on both sides (`set_axis_style`).
fn set_axis_style(ax: &Axes, labels: &[&str]) -> CaseResult {
    let n = labels.len() as f64;
    ax.set_xticks(arange(1.0, n + 1.0, 1.0));
    ax.set_xticklabels(strings(labels))?;
    ax.set_xlim(0.25, n + 0.75)?;
    ax.set_xlabel("Sample name");
    Ok(())
}

/// `color` at opacity `alpha` (matplotlib's `(color, alpha)` tuples).
fn translucent(color: &str, alpha: f64) -> Result<Color, Box<dyn std::error::Error>> {
    Ok(Color::from(Color::from(color).resolve()?.with_alpha(alpha)))
}

/// Draws the quartiles, whiskers and medians of each sorted sample over
/// its violin.
fn box_statistics(ax: &Axes, data: &[Vec<f64>]) -> CaseResult {
    let mut quartile1 = Vec::new();
    let mut medians = Vec::new();
    let mut quartile3 = Vec::new();
    let mut whiskers_min = Vec::new();
    let mut whiskers_max = Vec::new();
    for sorted in data {
        let quartiles = percentiles(sorted, &[25.0, 50.0, 75.0])?;
        let (low, high) = adjacent_values(sorted, quartiles[0], quartiles[2]);
        quartile1.push(quartiles[0]);
        medians.push(quartiles[1]);
        quartile3.push(quartiles[2]);
        whiskers_min.push(low);
        whiskers_max.push(high);
    }
    let inds = arange(1.0, medians.len() as f64 + 1.0, 1.0);
    let white_dots = ScatterOptions {
        marker: Some("o".parse::<MarkerStyle>()?),
        colors: ScatterColors::Single("white".into()),
        sizes: Some(vec![30.0]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(&inds, medians, white_dots)?.set_zorder(3.0);
    for (low, high, width) in [
        (quartile1, quartile3, 5.0),
        (whiskers_min, whiskers_max, 1.0),
    ] {
        ax.vlines(&inds, low, high)?
            .set_edgecolor("k")
            .set_linestyles(vec![LineStyle::Solid])
            .set_linewidths(vec![width]);
    }
    Ok(())
}

fn customized_violin(fig: &Figure) -> CaseResult {
    let mut g = SpecRng::new(SEED);
    let data: Vec<Vec<f64>> = (1..5)
        .map(|std| {
            let mut sample = g.normal(100, 0.0, f64::from(std));
            sample.sort_by(f64::total_cmp);
            sample
        })
        .collect();
    let axs = fig.subplots_shared(1, 3, Share::None, Share::All)?;
    let [ax1, ax2, ax3] = [0, 1, 2].map(|col| axs[(0, col)].clone());
    ax1.set_title("Default violin plot");
    ax1.set_ylabel("Observed values");
    ax1.violinplot(&data, ViolinplotOptions::default())?;
    ax2.set_title("Set colors of violins");
    ax2.set_ylabel("Observed values");
    let facecolors = ["yellow", "blue", "red", "green"]
        .into_iter()
        .map(|color| translucent(color, 0.3))
        .collect::<Result<_, _>>()?;
    let colored = drawn_violins(ViolinDrawOptions {
        facecolors,
        linecolors: vec!["black".into()],
        ..ViolinDrawOptions::default()
    });
    ax2.violinplot(&data, colored)?;
    ax3.set_title("Customized violin plot");
    let bare = drawn_violins(ViolinDrawOptions {
        showmeans: false,
        showmedians: false,
        showextrema: false,
        facecolors: vec!["#D43F3A".into()],
        linecolors: vec!["black".into()],
        ..ViolinDrawOptions::default()
    });
    let parts = ax3.violinplot(&data, bare)?;
    for pc in &parts.bodies {
        pc.set_edgecolor("black").set_linewidths(vec![1.0]);
        pc.set_alpha(Some(1.0));
    }
    box_statistics(&ax3, &data)?;
    for ax in [&ax1, &ax2, &ax3] {
        set_axis_style(ax, &["A", "B", "C", "D"])?;
    }
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.15),
        wspace: Some(0.05),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// One panel of `violinplot`: the density settings, the drawing settings
/// and whether only the last dataset is drawn.
struct Panel {
    points: usize,
    widths: f64,
    bw_method: BandwidthMethod,
    quantiles: Quantiles,
    sides: &'static [ViolinSide],
}

/// The quantiles a panel marks.
#[derive(Clone, Copy)]
enum Quantiles {
    /// None, on every dataset.
    None,
    /// Some on some datasets (`[[0.1], [], [], [0.175, 0.954], [0.75], [0.25]]`).
    Mixed,
    /// The tails of the last dataset only (`[0.05, 0.1, 0.8, 0.9]`).
    Tails,
}

/// Both halves at once.
const BOTH: &[ViolinSide] = &[ViolinSide::Both];
/// The two halves drawn one after the other.
const HALVES: &[ViolinSide] = &[ViolinSide::Low, ViolinSide::High];

impl Panel {
    const fn new(points: usize, widths: f64, bw_method: BandwidthMethod) -> Self {
        Self {
            points,
            widths,
            bw_method,
            quantiles: Quantiles::None,
            sides: BOTH,
        }
    }

    const fn with(self, quantiles: Quantiles, sides: &'static [ViolinSide]) -> Self {
        Self {
            quantiles,
            sides,
            ..self
        }
    }

    /// Draws the panel's violins of `data` at `pos` into `ax`.
    fn draw(
        &self,
        ax: &Axes,
        data: &[Vec<f64>],
        pos: &[f64],
        orientation: Orientation,
    ) -> CaseResult {
        let (data, pos, quantiles) = match self.quantiles {
            Quantiles::None => (data, pos, None),
            Quantiles::Mixed => (
                data,
                pos,
                Some(vec![
                    vec![0.1],
                    vec![],
                    vec![],
                    vec![0.175, 0.954],
                    vec![0.75],
                    vec![0.25],
                ]),
            ),
            Quantiles::Tails => (
                &data[data.len() - 1..],
                &pos[pos.len() - 1..],
                Some(vec![vec![0.05, 0.1, 0.8, 0.9]]),
            ),
        };
        for &side in self.sides {
            let options = ViolinplotOptions {
                stats: ViolinOptions {
                    bw_method: self.bw_method,
                    points: self.points,
                    quantiles: quantiles.clone(),
                },
                draw: ViolinDrawOptions {
                    positions: Some(pos.to_vec()),
                    orientation,
                    widths: vec![self.widths],
                    showmeans: true,
                    showextrema: true,
                    showmedians: true,
                    side,
                    ..ViolinDrawOptions::default()
                },
            };
            ax.violinplot(data, options)?;
        }
        Ok(())
    }
}

fn violinplot(fig: &Figure) -> CaseResult {
    use BandwidthMethod::{Factor, Scott, Silverman};
    let mut g = SpecRng::new(SEED);
    let pos = [1.0, 2.0, 4.0, 5.0, 7.0, 8.0];
    let data: Vec<Vec<f64>> = pos.iter().map(|&std| g.normal(100, 0.0, std)).collect();
    let vertical = [
        Panel::new(20, 0.3, Scott),
        Panel::new(40, 0.5, Silverman),
        Panel::new(60, 0.7, Factor(0.5)),
        Panel::new(60, 0.7, Factor(0.5)).with(Quantiles::Mixed, BOTH),
        Panel::new(60, 0.7, Factor(0.5)).with(Quantiles::Tails, BOTH),
        Panel::new(60, 0.7, Factor(0.5)).with(Quantiles::Tails, HALVES),
    ];
    let horizontal = [
        Panel::new(80, 0.7, Scott),
        Panel::new(100, 0.9, Silverman),
        Panel::new(200, 1.1, Factor(0.5)),
        Panel::new(200, 1.1, Factor(0.5)).with(Quantiles::Mixed, BOTH),
        Panel::new(200, 1.1, Factor(0.5)).with(Quantiles::Tails, BOTH),
        Panel::new(200, 1.1, Factor(0.5)).with(Quantiles::Tails, HALVES),
    ];
    let axs = fig.subplots(2, 6)?;
    let panels = vertical
        .iter()
        .map(|panel| (panel, Orientation::Vertical))
        .chain(
            horizontal
                .iter()
                .map(|panel| (panel, Orientation::Horizontal)),
        );
    for (i, (ax, (panel, orientation))) in axs.as_slice().iter().zip(panels).enumerate() {
        panel.draw(ax, &data, &pos, orientation)?;
        ax.set_title(format!("Custom violin {}", i + 1))
            .set_fontsize(10.0);
        ax.set_yticklabels(Vec::<String>::new())?;
    }
    fig.suptitle("Violin Plotting Examples");
    fig.subplots_adjust(&SubplotOverrides {
        hspace: Some(0.4),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_stats/customized_violin", customized_violin).sized(9.0, 3.0, 100.0),
    FigureCase::new("gallery_stats/violinplot", violinplot).sized(10.0, 4.0, 100.0),
];
