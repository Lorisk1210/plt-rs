//! The style sheet examples: `bmh`, `dark_background`,
//! `fivethirtyeight`, `ggplot`, `grayscale`, `petroff10` and
//! `plot_solarizedlight2`, each under its sheet.

use std::f64::consts::PI;

use plt::axes::{AxisMode, HistPlotOptions, ImshowOptions};
use plt::figure::SubfigureOptions;
use plt::fonts::FontSize;
use plt::image::Interpolation;
use plt::layout::LayoutEngine;
use plt::numeric::Grid;
use plt::patches::Circle;
use plt::prelude::*;
use plt::stats::{Bins, HistOptions, HistType};

use super::{SEED, beta};
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `len(plt.rcParams['axes.prop_cycle'])`.
fn cycle_length() -> usize {
    plt::rc::params().axes_prop_cycle.len()
}

/// One sine per colour of the cycle over `[0, length]`, shifted evenly
/// over the same range, drawn with `format`.
fn shifted_sines(ax: &Axes, format: &str, length: f64) -> CaseResult {
    let x = linspace(0.0, length, 50);
    let n = cycle_length();
    // `np.linspace(0, length, n, endpoint=False)`.
    let shifts = linspace(0.0, length, n + 1);
    for &s in &shifts[..n] {
        ax.plot_fmt(x.clone(), map(&x, |x| (x + s).sin()), format)?;
    }
    Ok(())
}

/// `image_and_patch_example`: a 20 × 20 random image under a circle.
pub(super) fn image_and_patch(ax: &Axes, rng: &mut SpecRng) -> CaseResult {
    let values = Grid::from_vec(20, 20, rng.uniform(20 * 20, 0.0, 1.0))?;
    ax.imshow(
        values,
        ImshowOptions::default().interpolation(Interpolation::None),
    )?;
    ax.add_patch_with(
        Circle::new((5.0, 5.0), 5.0),
        PatchProps::default().label("patch"),
    );
    Ok(())
}

/// Four translucent, filled step histograms of beta distributions in
/// `bins` bins.
pub(super) fn beta_histograms(ax: &Axes, rng: &mut SpecRng, bins: usize) -> CaseResult {
    for (a, b) in [(10, 10), (4, 12), (50, 12), (6, 55)] {
        let options = HistPlotOptions {
            layout: HistOptions {
                bins: Some(Bins::Count(bins)),
                density: true,
                histtype: HistType::StepFilled,
                ..HistOptions::default()
            },
            props: PatchProps::default().alpha(0.8),
            ..HistPlotOptions::default()
        };
        ax.hist(&[beta(rng, a, b, 10_000)], options)?;
    }
    Ok(())
}

/// One sigmoid per colour of the cycle, optionally marked at 20 points in
/// the line's colour (`colored_lines_example`).
pub(super) fn sigmoids(ax: &Axes, markers: bool) -> CaseResult {
    let t = linspace(-10.0, 10.0, 100);
    let n = cycle_length();
    for (t0, a) in linspace(-5.0, 5.0, n)
        .into_iter()
        .zip(linspace(1.0, 1.5, n))
    {
        let y = map(&t, |t| a / (1.0 + (-(t - t0)).exp()));
        let line = ax.plot_fmt(t.clone(), y.clone(), "-")?;
        if markers {
            // `np.linspace(0, len(t) - 1, 20, dtype=int)`.
            let indices = linspace(0.0, 99.0, 20).into_iter().map(|i| i as usize);
            let (tp, yp): (Vec<f64>, Vec<f64>) = indices.map(|i| (t[i], y[i])).unzip();
            ax.plot_with(tp, yp, LineProps::fmt("o")?.color(line.color()))?;
        }
    }
    ax.set_xlim(-10.0, 10.0)?;
    Ok(())
}

fn bmh(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    beta_histograms(&ax, &mut SpecRng::new(SEED), 25)?;
    ax.set_title("'bmh' style sheet");
    Ok(())
}

fn dark_background(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    shifted_sines(&ax, "o-", 6.0)?;
    ax.set_xlabel("x-axis").set_ylabel("y-axis");
    ax.set_title("'dark_background' style sheet");
    Ok(())
}

/// `sin(x) + slope * x` plus normal noise, for each slope.
fn noisy_lines(ax: &Axes, rng: &mut SpecRng, slopes: &[f64]) -> CaseResult {
    let x = linspace(0.0, 10.0, 50);
    for &slope in slopes {
        let noise = rng.normal(50, 0.0, 1.0);
        let y = x
            .iter()
            .zip(&noise)
            .map(|(x, n)| x.sin() + slope * x + n)
            .collect::<Vec<_>>();
        ax.plot(x.clone(), y)?;
    }
    Ok(())
}

fn fivethirtyeight(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    noisy_lines(
        &ax,
        &mut SpecRng::new(SEED),
        &[1.0, 0.5, 2.0, -0.5, -2.0, 0.0],
    )?;
    ax.set_title("'fivethirtyeight' style sheet");
    Ok(())
}

fn ggplot(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    let axs = fig.subplots(2, 2)?;
    let (ax1, ax2, ax3, ax4) = (&axs[(0, 0)], &axs[(0, 1)], &axs[(1, 0)], &axs[(1, 1)]);
    let xy = rng.normal(2 * 200, 0.0, 1.0);
    ax1.plot_fmt(xy[..200].to_vec(), xy[200..].to_vec(), "o")?;
    shifted_sines(ax2, "-", 2.0 * PI)?;
    ax2.set_margins(0.0, 0.0)?;
    let x = arange(0.0, 5.0, 1.0);
    let heights: Vec<f64> = rng
        .integers(2 * 5, 1, 25)
        .into_iter()
        .map(|v| v as f64)
        .collect();
    let width = 0.25;
    let narrow = || BarOptions {
        thickness: vec![width],
        ..BarOptions::default()
    };
    ax3.bar(x.clone(), &heights[..5], narrow())?;
    let third = plt::rc::params().axes_prop_cycle.colors()[2];
    let shifted = map(&x, |x| x + width);
    ax3.bar(
        shifted.clone(),
        &heights[5..],
        BarOptions {
            color: vec![third.into()],
            ..narrow()
        },
    )?;
    ax3.set_xticks(shifted)
        .set_xticklabels(["a", "b", "c", "d", "e"])?;
    for color in plt::rc::params().axes_prop_cycle.colors() {
        let xy = rng.normal(2, 0.0, 1.0);
        ax4.add_patch_with(
            Circle::new((xy[0], xy[1]), 0.3),
            PatchProps::default().color(color),
        );
    }
    ax4.axis_mode(AxisMode::Equal);
    ax4.set_margins(0.0, 0.0)?;
    Ok(())
}

fn grayscale(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    fig.suptitle("'grayscale' style sheet");
    shifted_sines(&axs[(0, 0)], "o-", 6.0)?;
    image_and_patch(&axs[(0, 1)], &mut SpecRng::new(SEED))?;
    Ok(())
}

fn petroff10(fig: &Figure) -> CaseResult {
    let mut rng = SpecRng::new(SEED);
    fig.set_layout_engine(Some(LayoutEngine::compressed()))?;
    let sfigs = fig.subfigures(3, 1, SubfigureOptions::default())?;
    for (style, sfig) in ["petroff6", "petroff8", "petroff10"]
        .into_iter()
        .zip(sfigs.as_slice())
    {
        sfig.suptitle(format!("'{style}' style sheet"));
        plt::style::context(style, || -> CaseResult {
            let axs = sfig.subplots(1, 2)?;
            sigmoids(&axs[(0, 0)], true)?;
            image_and_patch(&axs[(0, 1)], &mut rng)
        })??;
    }
    Ok(())
}

fn plot_solarizedlight2(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let slopes: Vec<f64> = (1..9).map(f64::from).collect();
    noisy_lines(&ax, &mut SpecRng::new(SEED), &slopes)?;
    ax.set_title("8 Random Lines - Line");
    ax.set_xlabel("x label")
        .xaxis()
        .set_label_fontsize(FontSize::Points(14.0));
    ax.set_ylabel("y label")
        .yaxis()
        .set_label_fontsize(FontSize::Points(14.0));
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_shapes/bmh", bmh).with_style("bmh"),
    FigureCase::new("gallery_shapes/dark_background", dark_background)
        .with_style("dark_background"),
    FigureCase::new("gallery_shapes/fivethirtyeight", fivethirtyeight)
        .with_style("fivethirtyeight"),
    FigureCase::new("gallery_shapes/ggplot", ggplot).with_style("ggplot"),
    FigureCase::new("gallery_shapes/grayscale", grayscale).with_style("grayscale"),
    FigureCase::new("gallery_shapes/petroff10", petroff10).sized(6.4, 9.6, 100.0),
    FigureCase::new("gallery_shapes/plot_solarizedlight2", plot_solarizedlight2)
        .with_style("Solarize_Light2"),
];
