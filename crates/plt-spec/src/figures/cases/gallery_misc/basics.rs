//! The spine and pyplot sections and the plainest miscellaneous
//! examples: spine positions and bounds, a basic plot and two stacked
//! subplots, the area under a curve, lines drawn on the figure, a filled
//! spiral, properties set on several lines, small PDF pages and drawing
//! order (`spine_placement_demo`, `spines`, `pyplot_simple`,
//! `pyplot_two_subplots`, `integral`, `fig_x`, `fill_spiral`,
//! `set_and_get`, `multipage_pdf` and `zorder_demo`).

use std::f64::consts::PI;

use plt::axes::{AxisName, Limits, ScatterOptions};
use plt::figure::{MosaicOptions, Share};
use plt::layout::{LayoutEngine, TightLayout};
use plt::legend::LegendOptions;
use plt::patches::Polygon;
use plt::prelude::*;
use plt::spines::SpinePosition;
use plt::text::HorizontalAlignment as HAlign;
use plt::transforms::Point;

use super::{named_mosaic, panel, spaced_grid};
use crate::figures::cases::common::{double_sine, hide_top_right, map, spine_bounds_demo};
use crate::figures::{CaseResult, FigureCase};

fn spine_placement_demo(fig: &Figure) -> CaseResult {
    let (x, y) = double_sine();
    let axd = fig.subplot_mosaic(
        &named_mosaic(&[&["center", "zero"], &["axes", "data"]])?,
        &MosaicOptions::default(),
    )?;
    fig.suptitle("Spine positions");
    for (name, title, left, bottom) in [
        (
            "center",
            "'center'",
            SpinePosition::Axes(0.5),
            SpinePosition::Axes(0.5),
        ),
        (
            "zero",
            "'zero'",
            SpinePosition::Data(0.0),
            SpinePosition::Data(0.0),
        ),
        (
            "axes",
            "'axes' (0.2, 0.2)",
            SpinePosition::Axes(0.2),
            SpinePosition::Axes(0.2),
        ),
        (
            "data",
            "'data' (1, 2)",
            SpinePosition::Data(1.0),
            SpinePosition::Data(2.0),
        ),
    ] {
        let ax = panel(&axd, name)?;
        ax.set_title(title);
        ax.plot(&x, &y)?;
        ax.spines().left.set_position(left);
        ax.spines().bottom.set_position(bottom);
        hide_top_right(ax);
    }
    Ok(())
}

fn spines(fig: &Figure) -> CaseResult {
    fig.set_layout_engine(LayoutEngine::constrained());
    spine_bounds_demo(fig)
}

fn pyplot_simple(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 2.0, 0.01);
    let ax = fig.subplot();
    ax.plot(&x, map(&x, |x| (2.0 * PI * x).sin()))?;
    ax.set_title("A basic plot using pyplot");
    ax.set_xlabel("Time [s]").set_ylabel("Voltage [mV]");
    Ok(())
}

fn pyplot_two_subplots(fig: &Figure) -> CaseResult {
    let f = |t: f64| (-t).exp() * (2.0 * PI * t).cos();
    let t1 = arange(0.0, 5.0, 0.1);
    let t2 = arange(0.0, 5.0, 0.02);
    let ax = fig.add_subplot(2, 1, 1)?;
    ax.plot_with(
        &t1,
        map(&t1, f),
        LineProps::default()
            .color("tab:blue")
            .marker("o".parse::<Marker>()?),
    )?;
    ax.plot_with(&t2, map(&t2, f), LineProps::default().color("black"))?;
    ax.set_title("Subplot 1");
    let ax = fig.add_subplot(2, 1, 2)?;
    ax.plot_with(
        &t2,
        map(&t2, |t| (2.0 * PI * t).cos()),
        LineProps::default()
            .color("tab:orange")
            .linestyle(LineStyle::Dashed),
    )?;
    ax.set_title("Subplot 2");
    Ok(())
}

fn integral(fig: &Figure) -> CaseResult {
    let func = |x: f64| (x - 3.0) * (x - 5.0) * (x - 7.0) + 85.0;
    let (a, b) = (2.0, 9.0);
    let x = linspace(0.0, 10.0, 50);
    let ax = fig.subplot();
    ax.plot_with(&x, map(&x, func), LineProps::fmt("r")?.linewidth(2.0))?;
    ax.set_limits_with(
        AxisName::Y,
        Limits {
            low: Some(0.0),
            ..Limits::default()
        },
    )?;
    let ix = linspace(a, b, 50);
    let verts: Vec<Point> = std::iter::once(Point::new(a, 0.0))
        .chain(ix.iter().map(|&x| Point::new(x, func(x))))
        .chain(std::iter::once(Point::new(b, 0.0)))
        .collect();
    ax.add_patch_with(
        Polygon::new(verts, true),
        PatchProps::default().facecolor("0.9").edgecolor("0.5"),
    );
    ax.text(f64::midpoint(a, b), 30.0, r"$\int_a^b f(x)\mathrm{d}x$")
        .set_halign(HAlign::Center)
        .set_fontsize(20.0);
    fig.text(0.9, 0.05, "$x$");
    fig.text(0.1, 0.9, "$y$");
    hide_top_right(&ax);
    ax.set_xticks([a, b]);
    ax.set_xticklabels(["$a$", "$b$"])?;
    ax.set_yticks(Vec::<f64>::new());
    Ok(())
}

fn fig_x(fig: &Figure) -> CaseResult {
    fig.subplots_on(&spaced_grid(2, 2, 0.4, 0.4)?, Share::None, Share::None)?;
    let thick = LineProps::default().linewidth(3.0);
    fig.add_line(&[0.0, 1.0], &[0.47, 0.47], thick.clone())?;
    fig.add_line(&[0.5, 0.5], &[1.0, 0.0], thick)?;
    Ok(())
}

fn fill_spiral(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let theta = arange(0.0, 8.0 * PI, 0.1);
    let (a, b) = (1.0, 0.2);
    let arm = |dt: f64, f: fn(f64) -> f64| map(&theta, |t| a * f(t + dt) * (b * t).exp());
    for dt in arange(0.0, 2.0 * PI, PI / 2.0) {
        let outer = dt + PI / 4.0;
        let x: Vec<f64> = arm(dt, f64::cos)
            .into_iter()
            .chain(arm(outer, f64::cos).into_iter().rev())
            .collect();
        let y: Vec<f64> = arm(dt, f64::sin)
            .into_iter()
            .chain(arm(outer, f64::sin).into_iter().rev())
            .collect();
        ax.fill(&x, &y, PatchProps::default())?;
    }
    Ok(())
}

fn set_and_get(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 1.0, 0.01);
    let ax = fig.subplot();
    let l1 = ax.plot(&x, map(&x, |x| (2.0 * PI * x).sin()))?;
    let l2 = ax.plot(&x, map(&x, |x| (4.0 * PI * x).sin()))?;
    for line in [&l1, &l2] {
        line.set_linestyle(LineStyle::Dashed);
    }
    l1.set_linewidth(2.0).set_color("r");
    l2.set_linewidth(1.0).set_color("g");
    ax.set_title("Hi mom");
    Ok(())
}

fn multipage_pdf_1(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot_fmt(
        [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        [3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0],
        "r-o",
    )?;
    ax.set_title("Page One");
    Ok(())
}

fn multipage_pdf_3(fig: &Figure) -> CaseResult {
    let x = arange(0.0, 5.0, 0.1);
    let ax = fig.subplot();
    ax.plot_fmt(&x, map(&x, |x| x * x), "ko")?;
    ax.set_title("Page Three");
    Ok(())
}

fn zorder_demo_1(fig: &Figure) -> CaseResult {
    let r = linspace(0.3, 1.0, 30);
    let theta = linspace(0.0, 4.0 * PI, 30);
    let x: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.sin()).collect();
    let y: Vec<f64> = r.iter().zip(&theta).map(|(r, t)| r * t.cos()).collect();
    let axs = fig.subplots(1, 2)?;
    for (col, zorder, title) in [
        (0, None, "Lines on top of dots"),
        (1, Some(2.5), "Dots on top of lines"),
    ] {
        let ax = &axs[(0, col)];
        ax.plot_with(&x, &y, LineProps::fmt("C3")?.linewidth(3.0))?;
        let dots = ax.scatter_with(
            &x,
            &y,
            ScatterOptions {
                sizes: Some(vec![120.0]),
                ..ScatterOptions::default()
            },
        )?;
        if let Some(zorder) = zorder {
            dots.set_zorder(zorder);
        }
        ax.set_title(title);
    }
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn zorder_demo_2(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 7.5, 100);
    let ax = fig.subplot();
    ax.plot_with(
        &x,
        map(&x, f64::sin),
        LineProps::default().label("zorder=2").zorder(2.0),
    )?;
    ax.plot_with(
        &x,
        map(&x, |x| (x + 0.5).sin()),
        LineProps::default().label("zorder=3").zorder(3.0),
    )?;
    ax.axhline(
        0.0,
        LineProps::default()
            .label("zorder=2.5")
            .color("lightgrey")
            .zorder(2.5),
    );
    ax.set_title("Custom order of elements");
    let legend = ax.legend(LegendOptions::default().loc("upper right"))?;
    legend.set_zorder(2.5);
    Ok(())
}

/// The cases of this module.
pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_misc/spine_placement_demo", spine_placement_demo),
    FigureCase::new("gallery_misc/spines", spines),
    FigureCase::new("gallery_misc/pyplot_simple", pyplot_simple),
    FigureCase::new("gallery_misc/pyplot_two_subplots", pyplot_two_subplots),
    FigureCase::new("gallery_misc/integral", integral),
    FigureCase::new("gallery_misc/fig_x", fig_x),
    FigureCase::new("gallery_misc/fill_spiral", fill_spiral),
    FigureCase::new("gallery_misc/set_and_get", set_and_get),
    FigureCase::new("gallery_misc/multipage_pdf_1", multipage_pdf_1).sized(3.0, 3.0, 100.0),
    FigureCase::new("gallery_misc/multipage_pdf_3", multipage_pdf_3).sized(4.0, 5.0, 100.0),
    FigureCase::new("gallery_misc/zorder_demo_1", zorder_demo_1).sized(6.0, 3.2, 100.0),
    FigureCase::new("gallery_misc/zorder_demo_2", zorder_demo_2)
        .with_rc(&[("lines.linewidth", "5")]),
];
