//! Shared, inverted and twinned axes: `shared_axis_demo`, `invert_axes`,
//! `two_scales`, `fahrenheit_celsius_scales` and `twin_axes_zorder`.

use std::f64::consts::PI;

use plt::axes::{AxisName, FillBetweenOptions, TickParams};
use plt::collections::{CollectionProps, EdgeColors};
use plt::figure::Share;
use plt::layout::{LayoutEngine, TightLayout};
use plt::prelude::*;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

fn shared_axis_demo(fig: &Figure) -> CaseResult {
    let t1 = linspace(0.0, 8.0, 201);
    let y1 = map(&t1, |t| (2.0 * PI * t).sin());
    let t2 = linspace(2.0, 10.0, 201);
    let y2 = map(&t2, |t| {
        20.0 * (2.0 * PI * t).cos().powi(2) * (-0.3 * t).exp()
    });
    let axs = fig.subplots_shared(2, 1, Share::All, Share::None)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(1, 0)]);
    ax1.plot(&t1, &y1)?;
    ax1.set_ylabel("Signal 1");
    ax2.plot(&t2, &y2)?;
    ax2.set_ylabel("Signal 2").set_xlabel("Time (s)");
    Ok(())
}

fn invert_axes(fig: &Figure) -> CaseResult {
    let x = arange(0.01, 4.0, 0.01);
    let y = map(&x, |x| (-x).exp());
    fig.set_layout_engine(Some(LayoutEngine::constrained()))?;
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    fig.suptitle("Inverted axis with ...");
    ax1.plot(&x, &y)?;
    ax1.set_xlim(4.0, 0.0)?;
    ax1.set_title("fixed limits: set_xlim(4, 0)");
    ax1.set_xlabel("decreasing x ⟶");
    ax1.grid(true);
    ax2.plot(&x, &y)?;
    ax2.xaxis().set_inverted(true);
    ax2.set_title("autoscaling: set_inverted(True)");
    ax2.set_xlabel("decreasing x ⟶");
    ax2.grid(true);
    Ok(())
}

/// Colours the y label and the y tick labels of `ax`.
fn color_y(ax: &Axes, color: &str) {
    ax.yaxis().set_label_color(color);
    ax.tick_params(&TickParams {
        axis: Some(AxisName::Y),
        labelcolor: Some(color.into()),
        ..TickParams::default()
    });
}

fn two_scales(fig: &Figure) -> CaseResult {
    let t = arange(0.01, 10.0, 0.01);
    let ax1 = fig.subplot();
    let color = "tab:red";
    ax1.set_xlabel("time (s)").set_ylabel("exp");
    ax1.plot_with(&t, map(&t, f64::exp), LineProps::default().color(color))?;
    color_y(&ax1, color);
    let ax2 = ax1.twinx()?;
    let color = "tab:blue";
    ax2.set_ylabel("sin");
    ax2.plot_with(
        &t,
        map(&t, |t| (2.0 * PI * t).sin()),
        LineProps::default().color(color),
    )?;
    color_y(&ax2, color);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

/// Degrees Fahrenheit in degrees Celsius.
fn fahrenheit2celsius(temp: f64) -> f64 {
    (5.0 / 9.0) * (temp - 32.0)
}

fn fahrenheit_celsius_scales(fig: &Figure) -> CaseResult {
    let ax_f = fig.subplot();
    let ax_c = ax_f.twinx()?;
    ax_f.plot_y(linspace(-40.0, 120.0, 100));
    ax_f.set_xlim(0.0, 100.0)?;
    ax_f.set_title("Two scales: Fahrenheit and Celsius");
    ax_f.set_ylabel("Fahrenheit");
    ax_c.set_ylabel("Celsius");
    // The example's `ylim_changed` callback, once the limits are final.
    let (y1, y2) = ax_f.ylim();
    ax_c.set_ylim(fahrenheit2celsius(y1), fahrenheit2celsius(y2))?;
    Ok(())
}

fn twin_axes_zorder(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 10.0, 400);
    let y_main = map(&x, f64::sin);
    let y_twin = map(&x, |x| 0.4 * x.cos() + 0.6);
    let ax = fig.subplot();
    let ax2 = ax.twin_with(AxisName::X, -1.0)?;
    let fill = FillBetweenOptions {
        props: CollectionProps::default()
            .color("C1")
            .alpha(0.35)
            .label("twin fill"),
        ..FillBetweenOptions::default()
    };
    ax2.fill_between(&x, [0.0], &y_twin, fill)?;
    ax2.plot_with(
        &x,
        &y_twin,
        LineProps::default().color("C1").linewidth(6.0).alpha(0.8),
    )?;
    let every_eighth = |values: &[f64]| values.iter().step_by(8).copied().collect::<Vec<_>>();
    let markers = ScatterOptions {
        sizes: Some(vec![35.0]),
        colors: ScatterColors::Single("C0".into()),
        edgecolors: Some(EdgeColors::Colors(vec!["k".into()])),
        linewidths: Some(vec![0.5]),
        ..ScatterOptions::default()
    };
    ax.scatter_with(every_eighth(&x), every_eighth(&y_main), markers)?
        .set_zorder(3.0)
        .set_label("main scatter");
    ax.plot_with(&x, &y_main, LineProps::default().color("C0").linewidth(4.0))?;
    ax.set_xlabel("x").set_ylabel("main y");
    ax2.set_ylabel("twin y");
    ax.set_title("Twin Axes drawn behind the main Axes using delta_zorder");
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("gallery_subplots/shared_axis_demo", shared_axis_demo),
    FigureCase::new("gallery_subplots/invert_axes", invert_axes).sized(6.4, 4.0, 100.0),
    FigureCase::new("gallery_subplots/two_scales", two_scales),
    FigureCase::new(
        "gallery_subplots/fahrenheit_celsius_scales",
        fahrenheit_celsius_scales,
    ),
    FigureCase::new("gallery_subplots/twin_axes_zorder", twin_axes_zorder),
];
