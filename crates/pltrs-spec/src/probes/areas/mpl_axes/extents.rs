//! The extents of axes decorations: axis texts, spines with their ticks,
//! the axes box and its tight box.

use pltrs::artist::ArtistHandle;
use pltrs::axes::{Axes, AxisName, TickDirection, TickLevel, TickParams};
use pltrs::figure::Figure;
use pltrs::rc;
use pltrs::spines::{SpinePosition, SpineSide};
use pltrs::ticker::NullFormatter;
use pltrs_core::Bbox;

use super::{made, numbers, on_axes};
use crate::json::Json;
use crate::probes::areas::mpl_ticker::styled;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// A box as JSON, `null` for none.
fn bbox(extent: Option<Bbox>) -> Json {
    extent.map_or(Json::Null, |extent| encode::bbox(&extent))
}

/// What the upstream `color_boxes` helper measures: the tight boxes of
/// both axes, the spines' extents (bottom, top, left, right), the axes box
/// and its tight box.
fn color_boxes(fig: &Figure, ax: &Axes) -> Json {
    fig.layout();
    let spines = ax.spines();
    let sides = [
        SpineSide::Bottom,
        SpineSide::Top,
        SpineSide::Left,
        SpineSide::Right,
    ];
    encode::object([
        (
            "axis",
            Json::Array(vec![
                bbox(ax.xaxis().tight_bbox()),
                bbox(ax.yaxis().tight_bbox()),
            ]),
        ),
        (
            "spines",
            Json::Array(
                sides
                    .iter()
                    .map(|&side| bbox(spines.get(side).window_extent()))
                    .collect(),
            ),
        ),
        ("ax", bbox(ax.window_extent())),
        ("tight", bbox(ax.tight_bbox(false))),
    ])
}

/// Tick parameters for both axes.
fn ticks(which: TickLevel, direction: Option<TickDirection>, length: Option<f64>) -> TickParams {
    TickParams {
        which,
        direction,
        length,
        ..TickParams::default()
    }
}

/// The decorations of one of the upstream scenes.
fn decorations(case: &str) -> Outcome {
    let fig = Figure::new();
    fig.set_dpi(200.0);
    fig.set_size_inches(6.0, 6.0)
        .map_err(ProbeError::rejected)?;
    let ax = fig.subplot();
    if matches!(case, "nodecorator" | "minor_out" | "minor_long") {
        fig.layout();
    }
    if matches!(case, "nodecorator" | "displaced_spine") {
        made(ax.set_xticklabels(Vec::<String>::new()))?;
        made(ax.set_yticklabels(Vec::<String>::new()))?;
    }
    if case == "displaced_spine" {
        ax.spines()
            .get(SpineSide::Bottom)
            .set_position(SpinePosition::Axes(-0.1));
    }
    if let Some(direction) = case.strip_prefix("tickdirs_") {
        let direction = made(direction.parse())?;
        ax.tick_params(ticks(TickLevel::Major, Some(direction), None));
    }
    if case.starts_with("minor_") {
        ax.tick_params(ticks(TickLevel::Both, Some(TickDirection::Out), None));
    }
    if case == "minor_long" {
        ax.minorticks_on();
        ax.tick_params(ticks(TickLevel::Minor, None, Some(30.0)));
    }
    Ok(color_boxes(&fig, &ax))
}

pub(super) fn decoration_boxes(args: Args<'_>) -> Outcome {
    styled(args, || {
        rc::context(|| {
            made(rc::set("_internal.classic_mode", "False"))?;
            decorations(args.str("case")?)
        })
    })
}

pub(super) fn tight_bbox_axis_off(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let tight = || {
            ax.tight_bbox(false)
                .ok_or_else(|| ProbeError::rejected("the axes have no extent"))
        };
        let before = tight()?;
        ax.set_axis_off();
        Ok(Json::Array(vec![
            encode::bbox(&before),
            encode::bbox(&tight()?),
        ]))
    })
}

pub(super) fn tight_bbox_long_label(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        let width = || {
            ax.tight_bbox(false)
                .map(|extent| extent.width())
                .ok_or_else(|| ProbeError::rejected("the axes have no extent"))
        };
        let before = width()?;
        ax.set_xlabel(format!("l{}ng", "o".repeat(70)));
        Ok(numbers([before, width()?]))
    })
}

pub(super) fn tight_bbox_nan_text(args: Args<'_>) -> Outcome {
    on_axes(args, |ax| {
        ax.set_title("");
        for name in [AxisName::X, AxisName::Y] {
            ax.axis_of(name)
                .set_major_formatter(NullFormatter)
                .set_minor_formatter(NullFormatter);
        }
        ax.text(f64::NAN, 1.0, "Boo");
        ax.tight_bbox(false)
            .map(|extent| Json::from(extent.width()))
            .ok_or_else(|| ProbeError::rejected("the axes have no extent"))
    })
}
