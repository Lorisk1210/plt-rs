//! `figures/cases_spines.py`: the gallery's spine examples (bounds,
//! dropped and centred spines, a third y axis on an offset spine), spines
//! on log axes, spines moved with their ticks and spine styling.

use plt::axes::{AxisName, GridOptions, TickParams, TickPosition};
use plt::layout::TightLayout;
use plt::legend::{LegendHandle, LegendOptions};
use plt::prelude::*;
use plt::spines::{SpinePosition, SpineSide};
use plt_render::CapStyle;

use crate::figures::cases::common::map;
use crate::figures::{CaseResult, FigureCase};

/// Hides the top and right spines.
fn hide_top_right(ax: &Axes) {
    let spines = ax.spines();
    spines.top.set_visible(false);
    spines.right.set_visible(false);
}

fn gallery_bounds(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 100);
    let y = map(&x, |x| 2.0 * x.sin());
    let axs = fig.subplots(3, 1)?;
    for (row, title) in [
        "normal spines",
        "bottom-left spines",
        "spines with bounds limited to data range",
    ]
    .into_iter()
    .enumerate()
    {
        let ax = &axs[(row, 0)];
        ax.plot(&x, &y)?;
        ax.set_title(title);
    }
    hide_top_right(&axs[(1, 0)]);
    let ax2 = &axs[(2, 0)];
    let (ymin, ymax) = y
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    ax2.spines().bottom.set_bounds(Some((x[0], x[x.len() - 1])));
    ax2.spines().left.set_bounds(Some((ymin, ymax)));
    hide_top_right(ax2);
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn dropped(fig: &Figure) -> CaseResult {
    let x = linspace(0.0, 2.0 * std::f64::consts::PI, 100);
    let axs = fig.subplots(2, 2)?;
    axs[(0, 0)].plot(&x, map(&x, f64::sin))?;
    axs[(0, 1)].plot(&x, map(&x, f64::cos))?;
    axs[(1, 0)].plot(&x, map(&x, |x| -x.cos()))?;
    axs[(1, 1)].plot(&x, map(&x, |x| -x.sin()))?;
    for (cell, visible) in [
        ((0, 0), &[SpineSide::Left][..]),
        ((0, 1), &[][..]),
        ((1, 0), &[SpineSide::Left, SpineSide::Bottom][..]),
        ((1, 1), &[SpineSide::Bottom][..]),
    ] {
        let ax = &axs[cell];
        ax.label_outer_with(true);
        ax.grid_with(
            true,
            &GridOptions {
                color: Some(Color::from("0.9")),
                ..GridOptions::default()
            },
        );
        for spine in ax.spines().iter() {
            if visible.contains(&spine.side()) {
                spine.set_position(SpinePosition::Outward(10.0));
            } else {
                spine.set_visible(false);
            }
        }
    }
    Ok(())
}

fn centered_with_arrows(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.spines().left.set_position(SpinePosition::Data(0.0));
    ax.spines().bottom.set_position(SpinePosition::Data(0.0));
    hide_top_right(&ax);
    ax.plot_fmt([1.0], [0.0], ">k")?
        .set_transform(ax.trans_yaxis())
        .set_clip_on(false);
    ax.plot_fmt([0.0], [1.0], "^k")?
        .set_transform(ax.trans_xaxis())
        .set_clip_on(false);
    let x = linspace(-0.5, 1.0, 100);
    ax.plot(&x, map(&x, |x| (x * std::f64::consts::PI).sin()))?;
    Ok(())
}

fn multiple_yaxis(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    fig.subplots_adjust(&plt::gridspec::SubplotOverrides {
        right: Some(0.75),
        ..Default::default()
    })?;
    let twin1 = ax.twinx();
    let twin2 = ax.twinx();
    twin2.spines().right.set_position(SpinePosition::Axes(1.2));
    let x = [0.0, 1.0, 2.0];
    let p1 = ax.plot_fmt(x, [0.0, 1.0, 2.0], "C0")?;
    p1.set_label("Density");
    let p2 = twin1.plot_fmt(x, [0.0, 3.0, 2.0], "C1")?;
    p2.set_label("Temperature");
    let p3 = twin2.plot_fmt(x, [50.0, 30.0, 15.0], "C2")?;
    p3.set_label("Velocity");
    ax.set_xlim(0.0, 2.0).set_ylim(0.0, 2.0);
    ax.set_xlabel("Distance").set_ylabel("Density");
    twin1.set_ylim(0.0, 4.0).set_ylabel("Temperature");
    twin2.set_ylim(1.0, 65.0).set_ylabel("Velocity");
    for (axes, line) in [(&ax, &p1), (&twin1, &p2), (&twin2, &p3)] {
        axes.yaxis().set_label_color(line.color());
        axes.tick_params(&TickParams {
            axis: Some(AxisName::Y),
            colors: Some(line.color()),
            ..TickParams::default()
        });
    }
    ax.legend(LegendOptions {
        handles: Some([&p1, &p2, &p3].map(LegendHandle::from).to_vec()),
        ..LegendOptions::default()
    })?;
    Ok(())
}

fn log_axes_outward_bounds(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = logspace(0.0, 3.0, 40, 10.0);
    ax.set_xscale("log")?;
    ax.set_yscale("log")?;
    ax.plot_with(
        &x,
        map(&x, |x| 1e4 / x),
        LineProps::fmt("o-")?.markersize(3.0),
    )?;
    for side in [SpineSide::Left, SpineSide::Bottom] {
        ax.spines()
            .get(side)
            .set_position(SpinePosition::Outward(8.0));
    }
    ax.spines().left.set_bounds(Some((10.0, 1e4)));
    ax.spines().bottom.set_bounds(Some((1.0, 1e3)));
    hide_top_right(&ax);
    Ok(())
}

fn moved_with_ticks_and_labels(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let (ax1, ax2) = (&axs[(0, 0)], &axs[(0, 1)]);
    let t = linspace(-2.0, 3.0, 60);
    ax1.plot(&t, map(&t, |t| t * t - 2.0))?;
    ax1.spines().top.set_position(SpinePosition::Data(0.0));
    ax1.spines().bottom.set_visible(false);
    ax1.xaxis()
        .set_ticks_position(TickPosition::Side(SpineSide::Top))?;
    ax1.spines().left.set_position(SpinePosition::Axes(0.3));
    ax1.set_xlabel("x label").set_ylabel("y label");
    ax2.plot(&t, map(&t, f64::cos))?;
    ax2.spines().bottom.set_position(SpinePosition::Axes(-0.08));
    ax2.spines()
        .left
        .set_position(SpinePosition::Outward(-10.0));
    ax2.spines().right.set_position(SpinePosition::Axes(0.5));
    ax2.yaxis().set_ticks_position(TickPosition::Both)?;
    fig.tight_layout(&TightLayout::default())?;
    Ok(())
}

fn styles_and_colors(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([0.0, 1.0], [0.0, 1.0])?;
    let spines = ax.spines();
    spines
        .left
        .set_linewidth(4.0)
        .set_color("tab:red")
        .set_capstyle(CapStyle::Round);
    spines
        .bottom
        .set_linewidth(3.0)
        .set_color("tab:blue")
        .set_linestyle(LineStyle::Dashed)
        .set_alpha(Some(0.5));
    spines
        .top
        .set_linewidth(2.0)
        .set_color("tab:green")
        .set_linestyle(LineStyle::Dotted);
    spines
        .right
        .set_linewidth(6.0)
        .set_color("0.6")
        .set_zorder(0.0);
    ax.set_facecolor("ivory");
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("spines/gallery_bounds", gallery_bounds),
    FigureCase::new("spines/dropped", dropped),
    FigureCase::new("spines/centered_with_arrows", centered_with_arrows).sized(5.0, 4.0, 100.0),
    FigureCase::new("spines/multiple_yaxis", multiple_yaxis),
    FigureCase::new("spines/log_axes_outward_bounds", log_axes_outward_bounds)
        .sized(5.0, 4.0, 100.0),
    FigureCase::new(
        "spines/moved_with_ticks_and_labels",
        moved_with_ticks_and_labels,
    )
    .sized(6.0, 3.5, 100.0),
    FigureCase::new("spines/styles_and_colors", styles_and_colors).sized(5.0, 4.0, 100.0),
];
