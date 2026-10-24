//! View limits and shapes: `axes_margins`, `axes_box_aspect` and
//! `axis_equal_demo`.

use std::f64::consts::PI;

use plt::axes::{Adjustable, AxisMode, HistPlotOptions, ImshowOptions, Mesh, PcolorOptions};
use plt::colors::{ColorizerOptions, colormaps};
use plt::figure::Share;
use plt::gridspec::{GridSpec, SubplotOverrides};
use plt::layout::{LayoutEngine, TightLayout};
use plt::numeric::Grid;
use plt::patches::{Circle, Polygon};
use plt::prelude::*;
use plt::rc::Aspect;
use plt::stats::{HistOptions, HistOrientation};
use plt::transforms::Point;

use super::SEED;
use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};
use crate::rng::SpecRng;

/// `_damped`: `exp(-t) cos(2πt)` over `np.arange(0.0, 3.0, 0.01)`.
fn damped() -> (Vec<f64>, Vec<f64>) {
    let t = arange(0.0, 3.0, 0.01);
    let y = map(&t, |t| (-t).exp() * (2.0 * PI * t).cos());
    (t, y)
}

fn axes_margins_1(fig: &Figure) -> CaseResult {
    let (t, y) = damped();
    let ax1 = fig.add_subplot(2, 1, 2)?;
    ax1.set_margins(0.05, 0.05)?;
    ax1.plot(&t, &y)?;
    let ax2 = fig.add_subplot(2, 2, 1)?;
    ax2.set_margins(2.0, 2.0)?;
    ax2.plot(&t, &y)?;
    ax2.set_title("Zoomed out");
    let ax3 = fig.add_subplot(2, 2, 2)?;
    ax3.set_margins(0.0, -0.25)?;
    ax3.plot(&t, &y)?;
    ax3.set_title("Zoomed in");
    Ok(())
}

fn axes_margins_2(fig: &Figure) -> CaseResult {
    let x = Grid::from_fn(5, 5, |_, c| (c + 1) as f64);
    let y = Grid::from_fn(5, 5, |r, _| r as f64);
    let sums = Grid::from_fn(5, 5, |r, c| x[(r, c)] + y[(r, c)]);
    let corners = [(0.25, 2.75), (3.25, 2.75), (2.25, 0.75), (0.25, 0.75)];
    let axs = fig.subplots(1, 2)?;
    axs[(0, 1)].set_use_sticky_edges(false);
    for (ax, status) in axs.as_slice().iter().zip(["Is", "Is Not"]) {
        let options = PcolorOptions {
            colorizer: ColorizerOptions::default().cmap(colormaps::get("inferno")?),
            ..PcolorOptions::default()
        };
        ax.pcolor(
            Mesh::from_coords(x.clone(), y.clone(), sums.clone()),
            options,
        )?;
        let polygon = Polygon::new(corners.map(Point::from), true);
        ax.add_patch_with(
            polygon,
            PatchProps::default().color("forestgreen").alpha(0.5),
        );
        ax.set_margins(0.1, 0.05)?;
        ax.set_aspect(Aspect::Equal)?;
        ax.set_title(format!("{status} Sticky"));
    }
    Ok(())
}

fn axes_box_aspect_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(300.0, 400.0)?;
    ax.set_box_aspect(Some(1.0))?;
    Ok(())
}

fn axes_box_aspect_2(fig: &Figure) -> CaseResult {
    let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    let (ax, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    ax.plot([1.0, 5.0], [0.0, 10.0])?;
    ax2.plot([100.0, 500.0], [10.0, 15.0])?;
    ax.set_box_aspect(Some(1.0))?;
    ax2.set_box_aspect(Some(1.0))?;
    Ok(())
}

fn axes_box_aspect_3(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let ax2 = ax.twinx()?;
    ax.plot_y([0.0, 10.0]);
    ax2.plot_y([12.0, 10.0]);
    ax.set_box_aspect(Some(1.0))?;
    Ok(())
}

fn axes_box_aspect_4(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(1, 2)?;
    let image = Grid::from_vec(16, 27, SpecRng::new(SEED).uniform(16 * 27, 0.0, 1.0))?;
    let ratio = image.rows() as f64 / image.cols() as f64;
    axs[(0, 0)].imshow(image, ImshowOptions::default())?;
    axs[(0, 1)].plot_y([23.0, 45.0]);
    axs[(0, 1)].set_box_aspect(Some(ratio))?;
    Ok(())
}

fn axes_box_aspect_5(fig: &Figure) -> CaseResult {
    let grid = GridSpec::with_ratios(
        2,
        2,
        Some(vec![3.0, 1.0]),
        Some(vec![1.0, 3.0]),
        SubplotOverrides::default(),
    )?;
    let axs = fig.subplots_on(&grid, Share::Col, Share::Row)?;
    axs[(0, 1)].set_visible(false);
    axs[(0, 0)].set_box_aspect(Some(1.0 / 3.0))?;
    axs[(1, 0)].set_box_aspect(Some(1.0))?;
    axs[(1, 1)].set_box_aspect(Some(3.0 / 1.0))?;
    let normal = SpecRng::new(SEED).normal(2 * 400, 0.0, 1.0);
    let x = map(&normal[..400], |v| v * 0.5);
    let y = map(&normal[400..], |v| v * 180.0);
    axs[(1, 0)].scatter(&x, &y)?;
    axs[(0, 0)].hist(&[&x], HistPlotOptions::default())?;
    let sideways = HistPlotOptions {
        layout: HistOptions {
            orientation: HistOrientation::Horizontal,
            ..HistOptions::default()
        },
        ..HistPlotOptions::default()
    };
    axs[(1, 1)].hist(&[&y], sideways)?;
    Ok(())
}

fn axes_box_aspect_6(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.add_patch(Circle::new((5.0, 3.0), 1.0));
    ax.set_aspect(Aspect::Equal)?
        .set_adjustable(Adjustable::Datalim)?;
    ax.set_box_aspect(Some(0.5))?;
    ax.autoscale(None, None);
    Ok(())
}

fn axes_box_aspect_7(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots_shared(2, 3, Share::All, Share::All)?;
    let hsv = colormaps::get("hsv")?;
    for (i, ax) in axs.as_slice().iter().enumerate() {
        ax.set_box_aspect(Some(1.0))?;
        let options = ScatterOptions {
            colors: ScatterColors::Single(hsv.sample(i as f64 / 6.0).into()),
            sizes: Some(vec![300.0]),
            ..ScatterOptions::default()
        };
        let row = (i / 3) as f64;
        ax.scatter_with([(i % 3) as f64], [-(row - 0.5) * 200.0], options)?;
    }
    Ok(())
}

fn axis_equal_demo(fig: &Figure) -> CaseResult {
    let an = linspace(0.0, 2.0 * PI, 100);
    let (x, y) = (map(&an, |a| 3.0 * a.cos()), map(&an, |a| 3.0 * a.sin()));
    let axs = fig.subplots(2, 2)?;
    for ax in axs.as_slice() {
        ax.plot(&x, &y)?;
    }
    let title = |ax: &Axes, text: &str| {
        ax.set_title(text).set_fontsize(10.0);
    };
    title(&axs[(0, 0)], "not equal, looks like ellipse");
    axs[(0, 1)].axis_mode(AxisMode::Equal)?;
    title(&axs[(0, 1)], "equal, looks like circle");
    axs[(1, 0)].axis_mode(AxisMode::Equal)?;
    axs[(1, 0)].set_xlim(-3.0, 3.0)?.set_ylim(-3.0, 3.0)?;
    title(&axs[(1, 0)], "still a circle, even after changing limits");
    axs[(1, 1)]
        .set_aspect(Aspect::Equal)?
        .set_adjustable(Adjustable::Box)?;
    title(&axs[(1, 1)], "still a circle, auto-adjusted data limits");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/axes_margins_1", axes_margins_1),
    FigureCase::new("gallery_subplots/axes_margins_2", axes_margins_2),
    FigureCase::new("gallery_subplots/axes_box_aspect_1", axes_box_aspect_1),
    FigureCase::new("gallery_subplots/axes_box_aspect_2", axes_box_aspect_2),
    FigureCase::new("gallery_subplots/axes_box_aspect_3", axes_box_aspect_3),
    FigureCase::new("gallery_subplots/axes_box_aspect_4", axes_box_aspect_4),
    FigureCase::new("gallery_subplots/axes_box_aspect_5", axes_box_aspect_5),
    FigureCase::new("gallery_subplots/axes_box_aspect_6", axes_box_aspect_6),
    FigureCase::new("gallery_subplots/axes_box_aspect_7", axes_box_aspect_7),
    FigureCase::new("gallery_subplots/axis_equal_demo", axis_equal_demo),
];
