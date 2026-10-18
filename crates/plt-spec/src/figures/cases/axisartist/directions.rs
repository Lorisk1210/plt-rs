//! The direction and styling of axis artists in
//! `figures/cases_axisartist.py`.

use plt::axes::{AxisName, GridOptions, TickDirection, TickLevel};
use plt::axisartist::grid_finder::MaxNLocator;
use plt::axisartist::{self, Toggle};
use plt::gridspec::SubplotOverrides;
use plt::prelude::*;
use plt::rc::Aspect;
use plt::spines::SpineSide;
use plt::text::{HorizontalAlignment, VerticalAlignment};

use super::{artist, polar_grid_helper, set_text};
use crate::figures::CaseResult;

/// `_ticked(fig, pos)`: axisartist axes in cell `index` of a `rows ×
/// cols` grid with ticks at 0.2 and 0.8.
fn ticked(
    fig: &Figure,
    (rows, cols, index): (usize, usize, usize),
) -> Result<axisartist::Axes, Box<dyn std::error::Error>> {
    let ax = axisartist::Axes::new(fig.add_subplot(rows, cols, index)?);
    ax.ax().set_yticks([0.2, 0.8]).set_xticks([0.2, 0.8]);
    Ok(ax)
}

pub(super) fn simple_axis_direction01(fig: &Figure) -> CaseResult {
    let ax = axisartist::Axes::new(fig.subplot());
    fig.subplots_adjust(&SubplotOverrides {
        right: Some(0.8),
        ..SubplotOverrides::default()
    })?;
    let left = artist(&ax, "left")?;
    left.major_ticklabels().set_axis_direction(SpineSide::Top);
    left.label().set_text("Left label");
    let right = artist(&ax, "right")?;
    right.label().set_visible(true);
    right.label().set_text("Right label");
    right.label().set_axis_direction(SpineSide::Left);
    Ok(())
}

pub(super) fn simple_axis_direction03(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        wspace: Some(0.4),
        bottom: Some(0.3),
        ..SubplotOverrides::default()
    })?;
    let ax1 = ticked(fig, (1, 2, 1))?;
    ax1.ax().set_xlabel("ax1 X-label").set_ylabel("ax1 Y-label");
    for axis in ax1.axis_artists() {
        axis.invert_ticklabel_direction();
    }
    let ax2 = ticked(fig, (1, 2, 2))?;
    ax2.ax().set_xlabel("ax2 X-label").set_ylabel("ax2 Y-label");
    for axis in ax2.axis_artists() {
        axis.major_ticks().set_tick_out(false);
    }
    Ok(())
}

pub(super) fn ticklabel_alignment(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.5),
        hspace: Some(0.7),
        ..SubplotOverrides::default()
    })?;
    let mut axes = Vec::new();
    for index in 1..=3 {
        let ax = axisartist::Axes::new(fig.add_subplot(3, 1, index)?);
        ax.ax().set_yticks([0.2, 0.8]);
        ax.ax().set_yticklabels(["short", "loooong"])?;
        ax.ax().set_xticks([0.2, 0.8]);
        ax.ax().set_xticklabels([r"$\frac{1}{2}\pi$", r"$\pi$"])?;
        axes.push(ax);
    }
    axes[0]
        .ax()
        .set_ylabel("ha=right")
        .set_xlabel("va=baseline");
    for (ax, halign, valign, ylabel, xlabel) in [
        (
            &axes[1],
            HorizontalAlignment::Center,
            VerticalAlignment::Top,
            "ha=center",
            "va=top",
        ),
        (
            &axes[2],
            HorizontalAlignment::Left,
            VerticalAlignment::Bottom,
            "ha=left",
            "va=bottom",
        ),
    ] {
        artist(ax, "left")?.major_ticklabels().set_ha(halign);
        artist(ax, "bottom")?.major_ticklabels().set_va(valign);
        ax.ax().set_ylabel(ylabel).set_xlabel(xlabel);
    }
    Ok(())
}

pub(super) fn ticklabel_direction(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        bottom: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    let ax = ticked(fig, (1, 3, 1))?;
    for axis in ax.axis_artists() {
        axis.major_ticks().set_tick_out(true);
    }
    let ax = ticked(fig, (1, 3, 2))?;
    for (name, direction) in [
        ("left", SpineSide::Right),
        ("bottom", SpineSide::Top),
        ("right", SpineSide::Left),
        ("top", SpineSide::Bottom),
    ] {
        artist(&ax, name)?.set_axis_direction(direction);
    }
    let ax = ticked(fig, (1, 3, 3))?;
    artist(&ax, "left")?.set_axis_direction(SpineSide::Right);
    for axis in ax.axis_artists() {
        axis.major_ticks().set_tick_out(true);
    }
    artist(&ax, "left")?.label().set_text("Long Label Left");
    artist(&ax, "bottom")?.label().set_text("Label Bottom");
    let right = artist(&ax, "right")?;
    right.label().set_text("Long Label Right");
    right.label().set_visible(true);
    artist(&ax, "left")?.label().set_pad(0.0);
    artist(&ax, "bottom")?.label().set_pad(10.0);
    Ok(())
}

pub(super) fn axis_styling(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.15),
        right: Some(0.85),
        bottom: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    let ax = axisartist::Axes::new(fig.subplot());
    let x = linspace(0.0, 10.0, 60);
    let y: Vec<f64> = x.iter().map(|x| 40.0 * x.sin() + 50.0).collect();
    ax.ax().plot(x, y)?;
    ax.ax().minorticks_on();
    ax.ax().set_xlabel("time [s]").set_ylabel("signal");
    ax.grid(
        true,
        &GridOptions {
            which: TickLevel::Both,
            axis: Some(AxisName::X),
            color: Some("0.8".into()),
            linestyle: Some(LineStyle::Dotted),
            ..GridOptions::default()
        },
    );
    let bottom = artist(&ax, "bottom")?;
    bottom
        .major_ticks()
        .set_color(Some("tab:red".into()))
        .set_ticksize(8.0)
        .set_tick_direction(TickDirection::InOut);
    bottom
        .minor_ticks()
        .set_color(Some("tab:blue".into()))
        .set_tick_out(true);
    bottom
        .major_ticklabels()
        .set_color(Some("tab:red".into()))
        .set_rotation(30.0)
        .set_pad(6.0);
    bottom
        .label()
        .set_color(Some("tab:green".into()))
        .set_pad(12.0);
    bottom
        .line()
        .set_color(Some("tab:purple".into()))
        .set_linewidth(Some(2.0));
    let left = artist(&ax, "left")?;
    left.major_ticks()
        .set_markeredgewidth(Some(2.0))
        .set_ticksize(6.0);
    left.major_ticklabels().set_size(8.0);
    left.minor_ticks().set_visible(false);
    left.label().set_axis_direction(SpineSide::Right);
    let right = artist(&ax, "right")?;
    right.toggle(&Toggle {
        all: Some(true),
        ..Toggle::default()
    });
    right.label().set_text("right label");
    right.invert_ticklabel_direction();
    let top = artist(&ax, "top")?;
    top.toggle(&Toggle {
        all: Some(false),
        ticks: Some(true),
        ..Toggle::default()
    });
    top.major_ticks().set_tick_out(true);
    top.line().set_visible(false);
    Ok(())
}

pub(super) fn axis_direction(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(&SubplotOverrides {
        left: Some(0.04),
        right: Some(0.98),
        bottom: Some(0.06),
        top: Some(0.92),
        wspace: Some(0.35),
        hspace: Some(0.45),
    })?;
    for (row, (nth, value, name, text)) in [
        (AxisName::X, 30.0, "lat", r"$\theta = 30^{\circ}$"),
        (AxisName::Y, 6.0, "lon", r"$r = 6$"),
    ]
    .into_iter()
    .enumerate()
    {
        for (i, direction) in [
            SpineSide::Bottom,
            SpineSide::Left,
            SpineSide::Top,
            SpineSide::Right,
        ]
        .into_iter()
        .enumerate()
        {
            let helper = polar_grid_helper(Some(Box::new(MaxNLocator::new(5))));
            let ax =
                axisartist::Axes::with_grid_helper(fig.add_subplot(2, 4, 4 * row + i + 1)?, helper);
            ax.ax().set_aspect(Aspect::Equal)?;
            ax.ax().set_xlim(-5.0, 12.0).set_ylim(-5.0, 10.0);
            for axis in ax.axis_artists() {
                axis.toggle(&Toggle {
                    ticklabels: Some(false),
                    ..Toggle::default()
                });
            }
            ax.grid(
                true,
                &GridOptions {
                    color: Some(".9".into()),
                    ..GridOptions::default()
                },
            );
            let axis = ax.new_floating_axis(name, nth, value);
            set_text(&axis, text);
            axis.set_axis_direction(direction);
            ax.ax().set_title(direction.name());
        }
    }
    Ok(())
}
