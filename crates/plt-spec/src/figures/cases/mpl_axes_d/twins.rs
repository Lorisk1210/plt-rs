//! Twin axes, tick parameters that survive tick resets and `clear`, and
//! clearing shared axes.

use std::f64::consts::TAU;

use plt::artist::BaseProps;
use plt::axes::{AxisName, FillBetweenOptions, TickDirection, TickParams};
use plt::collections::CollectionProps;
use plt::figure::{Figure, Share};
use plt::fonts::FontSize;
use plt::gridspec::SubplotOverrides;
use plt::prelude::{ArtistHandle, Color, LineStyle, arange};
use plt::spines::SpinePosition;

use super::super::mpl_tests::{classic, remove_ticks_and_titles};
use crate::figures::{CaseResult, FigureCase};

/// `test_twin_spines`: a host with two right-hand twins, the second one's
/// spine pushed out by a fifth of the axes width.
fn twin_spines(fig: &Figure) -> CaseResult {
    fig.subplots_adjust(SubplotOverrides {
        right: Some(0.75),
        ..SubplotOverrides::default()
    })?;
    let host = fig.subplot();
    let par1 = host.twinx()?;
    let par2 = host.twinx()?;
    par2.spines().right.set_position(SpinePosition::Axes(1.2));
    par2.set_frame_on(true).set_patch_visible(false);
    for spine in par2.spines().iter() {
        spine.set_visible(false);
    }
    par2.spines().right.set_visible(true);
    let lines = [
        host.plot_fmt([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], "b-")?,
        par1.plot_fmt([0.0, 1.0, 2.0], [0.0, 3.0, 2.0], "r-")?,
        par2.plot_fmt([0.0, 1.0, 2.0], [50.0, 30.0, 15.0], "g-")?,
    ];
    host.set_xlim(0.0, 2.0)?.set_ylim(0.0, 2.0)?;
    par1.set_ylim(0.0, 4.0)?;
    par2.set_ylim(1.0, 65.0)?;
    for (ax, line) in [&host, &par1, &par2].into_iter().zip(&lines) {
        let color = line.color();
        ax.yaxis().set_label_color(color.clone());
        ax.tick_params(TickParams {
            axis: Some(AxisName::Y),
            colors: Some(color),
            length: Some(4.0),
            width: Some(1.5),
            ..TickParams::default()
        });
    }
    host.tick_params(TickParams {
        axis: Some(AxisName::X),
        length: Some(4.0),
        width: Some(1.5),
        ..TickParams::default()
    });
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_twin_spines_on_top`: thick lines and fills on a host and its
/// twin, under the test's 48 pt axes and line widths.
fn twin_spines_on_top(fig: &Figure) -> CaseResult {
    let ax1 = fig.subplot();
    let x = [1000.0, 1100.0, 1200.0, 1250.0];
    let y = [310.0, 301.0, 360.0, 400.0].map(|v| v / 1e3);
    let ax2 = ax1.twinx()?;
    for (ax, color, alpha) in [(&ax1, "#BEAED4", 0.8), (&ax2, "#7FC97F", 0.5)] {
        ax.plot_with(x, y, plt::lines::LineProps::default().color(color))?;
        let fill = FillBetweenOptions {
            props: CollectionProps {
                base: BaseProps {
                    alpha: Some(alpha),
                    ..BaseProps::default()
                },
                ..CollectionProps::default().color(color)
            },
            ..FillBetweenOptions::default()
        };
        ax.fill_between(x, y, [0.0], fill)?;
    }
    remove_ticks_and_titles(fig);
    Ok(())
}

/// `test_reset_ticks`: tick parameters changing every part of the ticks.
///
/// plt-rs keeps no tick objects: the parameters live on the axis, so
/// matplotlib's `reset_ticks`, which recreates the ticks from them, has
/// nothing to do here.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn reset_ticks(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.grid(true);
    ax.tick_params(TickParams {
        direction: Some(TickDirection::In),
        length: Some(10.0),
        width: Some(5.0),
        color: Some(Color::from("C0")),
        pad: Some(12.0),
        labelsize: Some(FontSize::Points(14.0)),
        labelcolor: Some(Color::from("C1")),
        labelrotation: Some(45.0),
        grid_color: Some(Color::from("C2")),
        grid_alpha: Some(0.8),
        grid_linewidth: Some(3.0),
        grid_linestyle: Some(LineStyle::Dashed),
        ..TickParams::default()
    });
    fig.layout();
    Ok(())
}

/// `test_axes_clear_behavior`: tick parameters of one axis that
/// `clear` keeps (the tick z-order, which changes nothing drawn, is left
/// out).
#[expect(
    clippy::unnecessary_wraps,
    reason = "every case shares the fallible build signature"
)]
fn axes_clear_behavior(fig: &Figure, which: AxisName) -> CaseResult {
    let ax = fig.subplot();
    ax.tick_params(TickParams {
        axis: Some(which),
        direction: Some(TickDirection::In),
        length: Some(10.0),
        width: Some(10.0),
        color: Some(Color::from("xkcd:wine red")),
        pad: Some(0.0),
        labelfontfamily: Some("serif".into()),
        labelrotation: Some(45.0),
        labelcolor: Some(Color::from("xkcd:shocking pink")),
        grid_color: Some(Color::from("xkcd:fluorescent green")),
        grid_alpha: Some(0.5),
        grid_linewidth: Some(3.0),
        grid_linestyle: Some(LineStyle::Dotted),
        bottom: Some(false),
        top: Some(true),
        left: Some(false),
        right: Some(true),
        labelbottom: Some(true),
        labeltop: Some(true),
        labelleft: Some(true),
        labelright: Some(true),
        ..TickParams::default()
    });
    ax.clear();
    ax.grid(true);
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    classic("mpl_axes_d/twin_remove", |fig| {
        let ax = fig.subplot();
        let twinx = ax.twinx()?;
        let twiny = ax.twiny()?;
        twinx.remove();
        twiny.remove();
        Ok(())
    }),
    classic("mpl_axes_d/twin_spines", twin_spines).sized(4.0, 3.0, 100.0),
    classic("mpl_axes_d/twin_spines_on_top", twin_spines_on_top).with_rc_setup(|rc| {
        rc.set("axes.linewidth", "48")?;
        rc.set("lines.linewidth", "48")?;
        Ok(())
    }),
    classic("mpl_axes_d/reset_ticks", reset_ticks),
    classic("mpl_axes_d/axes_clear_behavior_x", |fig| {
        axes_clear_behavior(fig, AxisName::X)
    }),
    classic("mpl_axes_d/axes_clear_behavior_y", |fig| {
        axes_clear_behavior(fig, AxisName::Y)
    }),
    classic("mpl_axes_d/shared_axes_clear", |fig| {
        let x = arange(0.0, TAU, 0.01);
        let y: Vec<f64> = x.iter().map(|x| x.sin()).collect();
        for ax in fig
            .subplots_shared(2, 2, Share::All, Share::All)?
            .as_slice()
        {
            ax.clear();
            ax.plot(&x, &y)?;
        }
        Ok(())
    }),
];
