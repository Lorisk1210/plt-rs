//! Tick labels: offsets and scientific notation, rotated and multi-line
//! labels, tick parameters, label sides and minor labels.

use pltrs::axes::{
    AxisName, TickDirection, TickLabelProps, TickLevel, TickParams, TicklabelFormat,
};
use pltrs::fonts::{FontSize, RelativeSize};
use pltrs::gridspec::SubplotOverrides;
use pltrs::layout::TightLayout;
use pltrs::prelude::*;
use pltrs::spines::SpineSide;
use pltrs::text::{HAlign, RotationMode};
use pltrs::ticker::OffsetMode;

use crate::figures::CaseResult;
use crate::figures::cases::common::map;

pub(super) fn offset_threshold_and_fixed_offset(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 3)?;
    let t = linspace(0.0, 1.0, 20);
    let rising = map(&t, |t| 1000.1 + 0.8 * t);
    axs[(0, 0)].plot(&t, &rising)?;
    axs[(0, 1)].plot(&t, &rising)?;
    axs[(0, 1)].ticklabel_format(TicklabelFormat {
        axis: Some(AxisName::Y),
        use_offset: Some(OffsetMode::Fixed(1000.0)),
        ..TicklabelFormat::default()
    })?;
    axs[(0, 2)].plot(map(&t, |t| -1e5 - 3.0 * t), map(&t, |t| -2e-3 * t))?;
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

pub(super) fn mathtext_offset_and_scale(fig: &Figure) -> CaseResult {
    let axs = fig.subplots(1, 2)?;
    let t = linspace(0.0, 1.0, 20);
    axs[(0, 0)].plot(map(&t, |t| t * 5e-5), map(&t, |t| t * 4e4))?;
    axs[(0, 1)].plot(&t, map(&t, |t| 1e7 + 2e3 * t))?;
    fig.tight_layout(TightLayout::default())?;
    Ok(())
}

pub(super) fn rotated_multiline_labels(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot(arange(0.0, 5.0, 1.0), [3.0, 1.0, 4.0, 1.0, 5.0])?;
    ax.xaxis()
        .set_ticks(arange(0.0, 5.0, 1.0))
        .set_ticklabels(["one\nline", "two\nlines\nhere", "three", "four\n4", "five"])?
        .set_ticklabel_props(
            TickLevel::Major,
            TickLabelProps {
                rotation: Some(30.0),
                halign: Some(HAlign::Right),
                rotation_mode: Some(RotationMode::Anchor),
                ..TickLabelProps::default()
            },
        );
    ax.yaxis()
        .set_ticks([1.0, 3.0, 5.0])
        .set_ticklabels(["low\nvalue", "mid", "high\nvalue"])?
        .set_ticklabel_props(
            TickLevel::Major,
            TickLabelProps {
                rotation: Some(15.0),
                multialign: Some(HAlign::Center),
                ..TickLabelProps::default()
            },
        );
    ax.set_xlabel("category");
    fig.subplots_adjust(SubplotOverrides {
        bottom: Some(0.25),
        left: Some(0.2),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) fn rotation_mode_xtick_ytick(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([0.0, 10.0], [0.0, 1000.0])?;
    for (axis, rotation, mode) in [
        (AxisName::X, 40.0, RotationMode::XTick),
        (AxisName::Y, -25.0, RotationMode::YTick),
    ] {
        ax.tick_params(TickParams {
            axis: Some(axis),
            labelrotation: Some(rotation),
            labelrotation_mode: Some(mode),
            ..TickParams::default()
        });
    }
    ax.tick_params(TickParams {
        top: Some(true),
        labeltop: Some(true),
        right: Some(true),
        labelright: Some(true),
        ..TickParams::default()
    });
    fig.subplots_adjust(SubplotOverrides {
        left: Some(0.15),
        right: Some(0.85),
        bottom: Some(0.18),
        top: Some(0.82),
        ..SubplotOverrides::default()
    })?;
    Ok(())
}

pub(super) fn tick_params_grid_and_sides(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let x = linspace(0.0, 4.0, 30);
    ax.plot(&x, map(&x, |x| x * x))?;
    ax.grid(true);
    ax.tick_params(TickParams {
        grid_color: Some(Color::from("tab:orange")),
        grid_alpha: Some(0.6),
        grid_linewidth: Some(2.0),
        grid_linestyle: Some(LineStyle::Dotted),
        ..TickParams::default()
    });
    ax.tick_params(TickParams {
        axis: Some(AxisName::Y),
        which: TickLevel::Both,
        direction: Some(TickDirection::InOut),
        length: Some(8.0),
        left: Some(false),
        labelleft: Some(false),
        right: Some(true),
        labelright: Some(true),
        ..TickParams::default()
    });
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        pad: Some(10.0),
        labelcolor: Some(Color::from("tab:purple")),
        labelsize: Some(FontSize::Relative(RelativeSize::Large)),
        width: Some(2.0),
        ..TickParams::default()
    });
    Ok(())
}

pub(super) fn label_positions_top_right(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    let t = linspace(0.0, 1.0, 20);
    ax.plot(&t, map(&t, |t| 2e6 * t * t))?;
    ax.xaxis().tick_to(SpineSide::Top)?;
    ax.xaxis().set_label_position(SpineSide::Top)?;
    ax.yaxis().tick_to(SpineSide::Right)?;
    ax.yaxis().set_label_position(SpineSide::Right)?;
    ax.set_xlabel("x on top");
    ax.set_ylabel("y on the right");
    ax.set_title("title above the top tick labels");
    Ok(())
}

pub(super) fn set_ticks_labels_minor_kwargs(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.plot([0.0, 10.0], [0.0, 1.0])?;
    ax.xaxis()
        .set_ticks([0.0, 5.0, 10.0])
        .set_ticklabels(["start", "middle", "end"])?
        .set_ticklabel_props(
            TickLevel::Major,
            TickLabelProps {
                color: Some(Color::from("tab:blue")),
                fontsize: Some(FontSize::Points(12.0)),
                ..TickLabelProps::default()
            },
        );
    ax.xaxis()
        .set_minor_ticks([2.5, 7.5])
        .set_minor_ticklabels(["a", "b"])?
        .set_ticklabel_props(
            TickLevel::Minor,
            TickLabelProps {
                color: Some(Color::from("tab:red")),
                fontsize: Some(FontSize::Points(8.0)),
                ..TickLabelProps::default()
            },
        );
    ax.yaxis().set_minor_ticks([0.25, 0.75]);
    ax.tick_params(TickParams {
        axis: Some(AxisName::X),
        which: TickLevel::Minor,
        pad: Some(12.0),
        ..TickParams::default()
    });
    Ok(())
}
