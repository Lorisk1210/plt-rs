//! `boxplot_demo`'s comparison of five distributions with bootstrap
//! resamples of themselves: shaded boxes, redrawn medians, marked means and
//! the medians written above the boxes.

use plt::axes::{
    Axes, BoxArtist, BoxplotArtists, BoxplotOptions, GridOptions, TickLabelProps, TickLevel,
};
use plt::fonts::{FontSize, FontWeight, RelativeSize};
use plt::gridspec::SubplotOverrides;
use plt::patches::Polygon;
use plt::prelude::*;
use plt::rc::{AxisBelow, ColorOrAuto};
use plt::stats::{Whis, mean};
use plt::text::HAlign;

use super::SEED;
use super::boxplots::with_sym;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `np.random.exponential(1, n)` by inversion.
fn exponential(g: &mut SpecRng, n: usize) -> Vec<f64> {
    (0..n).map(|_| -(1.0 - g.next_f64()).ln()).collect()
}

/// `np.random.gumbel(loc, scale, n)` by inversion.
fn gumbel(g: &mut SpecRng, loc: f64, scale: f64, n: usize) -> Vec<f64> {
    (0..n)
        .map(|_| loc - scale * (-(1.0 - g.next_f64()).ln()).ln())
        .collect()
}

/// `np.random.triangular(left, mode, right, n)` by inversion.
fn triangular(g: &mut SpecRng, (left, mode, right): (f64, f64, f64), n: usize) -> Vec<f64> {
    let cut = (mode - left) / (right - left);
    (0..n)
        .map(|_| {
            let u = g.next_f64();
            if u < cut {
                left + (u * (right - left) * (mode - left)).sqrt()
            } else {
                right - ((1.0 - u) * (right - left) * (right - mode)).sqrt()
            }
        })
        .collect()
}

/// `boxplot_demo`'s five distributions, each followed by a bootstrap
/// resample of itself (`bootstrap_pairs`).
fn bootstrap_pairs(g: &mut SpecRng, n: usize) -> Result<Vec<Vec<f64>>, Box<dyn std::error::Error>> {
    let dists = [
        g.normal(n, 1.0, 1.0),
        g.normal(n, 1.0, 1.0).into_iter().map(f64::exp).collect(),
        exponential(g, n),
        gumbel(g, 6.0, 4.0, n),
        triangular(g, (2.0, 9.0, 11.0), n),
    ];
    let n_max = i64::try_from(n)?;
    let indices: Vec<usize> = g
        .integers(n, 0, n_max)
        .into_iter()
        .map(usize::try_from)
        .collect::<Result<_, _>>()?;
    Ok(dists
        .into_iter()
        .flat_map(|dist| {
            let resample = indices.iter().map(|&i| dist[i]).collect();
            [dist, resample]
        })
        .collect())
}

/// `RANDOM_DISTS`: the names of `boxplot_demo`'s distributions.
const RANDOM_DISTS: [&str; 5] = [
    "Normal(1, 1)",
    "Lognormal(1, 1)",
    "Exp(1)",
    "Gumbel(6, 4)",
    "Triangular(2, 9, 11)",
];

/// The colours of the original and the resampled boxes.
const BOX_COLORS: [&str; 2] = ["darkkhaki", "royalblue"];

/// Fills the boxes of `boxplot_demo_3`, redraws their medians and marks
/// their means; returns the medians.
fn shade_boxes(
    ax: &Axes,
    bp: &BoxplotArtists,
    data: &[Vec<f64>],
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let mut medians = Vec::with_capacity(data.len());
    for (i, (part, med)) in bp.boxes.iter().zip(&bp.medians).enumerate() {
        if let BoxArtist::Line(line) = part {
            let corners = line
                .xdata()
                .into_iter()
                .zip(line.ydata())
                .take(5)
                .map(Into::into);
            ax.add_patch_with(
                Polygon::new(corners, true),
                PatchProps::default().facecolor(BOX_COLORS[i % 2]),
            );
        }
        let (median_x, median_y) = (med.xdata(), med.ydata());
        for j in 1..=2 {
            ax.plot_with(&median_x[..j], &median_y[..j], LineProps::fmt("k")?)?;
        }
        medians.push(median_y[0]);
        ax.plot_with(
            [mean(&median_x)],
            [mean(&data[i])],
            LineProps::default()
                .color("w")
                .marker(Marker::Symbol(MarkerSymbol::Star))
                .markeredgecolor(ColorOrAuto::Color("k".into())),
        )?;
    }
    Ok(medians)
}

fn boxplot_demo_3(fig: &Figure) -> CaseResult {
    let n = 500;
    let data = bootstrap_pairs(&mut SpecRng::new(SEED), n)?;
    let ax1 = fig.subplot();
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.075),
        right: Some(0.95),
        top: Some(0.9),
        bottom: Some(0.25),
        ..SubplotOverrides::default()
    })?;
    let options = BoxplotOptions {
        whis: Some(Whis::Factor(1.5)),
        ..with_sym("+")
    };
    let bp = ax1.boxplot(&data, options)?;
    for part in &bp.boxes {
        if let BoxArtist::Line(line) = part {
            line.set_color("black");
        }
    }
    for whisker in &bp.whiskers {
        whisker.set_color("black");
    }
    for flier in &bp.fliers {
        flier
            .set_color("red")
            .set_marker(Marker::Symbol(MarkerSymbol::Plus));
    }
    ax1.yaxis().grid(
        true,
        GridOptions {
            linestyle: Some(LineStyle::Solid),
            which: TickLevel::Major,
            color: Some("lightgrey".into()),
            alpha: Some(0.5),
            ..GridOptions::default()
        },
    );
    ax1.set_axisbelow(AxisBelow::Below);
    ax1.set_title("Comparison of IID Bootstrap Resampling Across Five Distributions");
    ax1.set_xlabel("Distribution");
    ax1.set_ylabel("Value");
    let medians = shade_boxes(&ax1, &bp, &data)?;
    let num_boxes = data.len();
    ax1.set_xlim(0.5, num_boxes as f64 + 0.5)
        .set_ylim(-5.0, 40.0);
    ax1.set_xticklabels(RANDOM_DISTS.iter().flat_map(|name| [*name; 2]))?;
    ax1.xaxis().set_ticklabel_props(
        TickLevel::Major,
        TickLabelProps {
            rotation: Some(45.0),
            fontsize: Some(FontSize::Points(8.0)),
            ..TickLabelProps::default()
        },
    );
    let weights = [FontWeight::BOLD, "semibold".parse::<FontWeight>()?];
    for (tick, median) in medians.iter().enumerate() {
        let k = tick % 2;
        // `str(round(median, 2))`: numpy rounds half to even.
        let label = format!("{:?}", (median * 100.0).round_ties_even() / 100.0);
        ax1.text(tick as f64 + 1.0, 0.95, label)
            .set_transform(ax1.trans_xaxis())
            .set_halign(HAlign::Center)
            .set_fontsize(RelativeSize::XSmall)
            .set_weight(weights[k])
            .set_color(BOX_COLORS[k]);
    }
    let roman = "roman".parse::<FontWeight>()?;
    let legend_entry = |y: f64, text: String, color: &str, background: &str| {
        fig.text(0.80, y, text)
            .set_backgroundcolor(background)
            .set_color(color)
            .set_weight(roman)
            .set_fontsize(RelativeSize::XSmall);
    };
    legend_entry(0.08, format!("{n} Random Numbers"), "black", BOX_COLORS[0]);
    legend_entry(
        0.045,
        "IID Bootstrap Resample".into(),
        "white",
        BOX_COLORS[1],
    );
    fig.text(0.80, 0.015, "*")
        .set_color("white")
        .set_backgroundcolor("silver")
        .set_weight(roman)
        .set_fontsize(RelativeSize::Medium);
    fig.text(0.815, 0.013, " Average Value")
        .set_color("black")
        .set_weight(roman)
        .set_fontsize(RelativeSize::XSmall);
    Ok(())
}

/// Every case of the module.
pub(super) const CASES: &[FigureCase] =
    &[FigureCase::new("gallery_stats/boxplot_demo_3", boxplot_demo_3).sized(10.0, 6.0, 100.0)];
