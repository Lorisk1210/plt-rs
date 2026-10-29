//! The `EventCollection.get` op: what the event collection of matplotlib's
//! `generate_EventCollection_plot` reads back after a list of calls.

use pltrs::artist::ArtistHandle;
use pltrs::collections::{CollectionProps, CollectionSpec, EventCollection};
use pltrs::figure::Figure;
use pltrs::orientation::Orientation;
use pltrs_core::Rgba;

use super::{colors, dashes, linestyle, made, numbers};
use crate::json::Json;
use crate::probes::encode::{object, rgba};
use crate::probes::{Args, Outcome, ProbeError};

/// The collection of `generate_EventCollection_plot`: eight events along
/// x, 0.5 long around 1, 2 points wide, red and solid.
fn fibonacci() -> Result<EventCollection, ProbeError> {
    let positions = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0];
    let props = CollectionProps::default()
        .linewidth(2.0)
        .color(Rgba::new(1.0, 0.0, 0.0, 1.0));
    let spec =
        CollectionSpec::events(&positions, Orientation::Horizontal, 1.0, 0.5).with_props(props);
    Figure::new()
        .subplot()
        .add_collection(spec)
        .raw()
        .downcast()
        .ok_or_else(|| ProbeError::rejected("not an event collection"))
}

fn number(value: Option<&Json>) -> Result<f64, ProbeError> {
    value
        .and_then(Json::as_f64)
        .ok_or_else(|| ProbeError::BadArgs("a step needs a number".into()))
}

/// Applies one `[name, argument]` step.
fn apply(events: &EventCollection, step: &Json) -> Result<(), ProbeError> {
    let parts = step
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("bad step {step}")))?;
    let name = parts
        .first()
        .and_then(Json::as_str)
        .ok_or_else(|| ProbeError::BadArgs(format!("bad step {step}")))?;
    let argument = parts.get(1);
    match name {
        "set_positions" => {
            let positions = argument
                .and_then(Json::as_f64_vec)
                .ok_or_else(|| ProbeError::BadArgs("positions".into()))?;
            events.set_positions(&positions);
        }
        "add_positions" | "append_positions" | "extend_positions" => {
            let positions = argument
                .and_then(|value| {
                    value
                        .as_f64_vec()
                        .or_else(|| value.as_f64().map(|v| vec![v]))
                })
                .ok_or_else(|| ProbeError::BadArgs("positions".into()))?;
            events.add_positions(&positions);
        }
        "switch_orientation" => {
            events.switch_orientation();
        }
        "set_orientation" => {
            let orientation: Orientation = made(
                argument
                    .and_then(Json::as_str)
                    .ok_or_else(|| ProbeError::BadArgs("orientation".into()))?
                    .parse(),
            )?;
            events.set_orientation(orientation);
        }
        "set_linelength" => {
            events.set_linelength(number(argument)?);
        }
        "set_lineoffset" => {
            events.set_lineoffset(number(argument)?);
        }
        "set_linestyle" => {
            let style = argument.ok_or_else(|| ProbeError::BadArgs("style".into()))?;
            events.set_linestyles(vec![linestyle(style)?]);
        }
        "set_linewidth" => {
            events.set_linewidths(vec![number(argument)?]);
        }
        "set_color" => {
            let rgba = argument
                .and_then(Json::as_f64_vec)
                .ok_or_else(|| ProbeError::BadArgs("colour".into()))?;
            let [r, g, b, a] = rgba[..] else {
                return Err(ProbeError::BadArgs("an RGBA colour".into()));
            };
            events.set_edgecolor(Rgba::new(r, g, b, a));
        }
        other => return Err(ProbeError::BadArgs(format!("unknown step {other:?}"))),
    }
    Ok(())
}

pub(super) fn get(args: Args<'_>) -> Outcome {
    let events = fibonacci()?;
    for step in args
        .json("steps")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("steps".into()))?
    {
        apply(&events, step)?;
    }
    let orientation = match events.orientation() {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    };
    let segments = events
        .segments()
        .into_iter()
        .map(|segment| Json::Array(segment.into_iter().map(|p| numbers([p.x, p.y])).collect()))
        .collect();
    let edges = made(events.colors())?.edges;
    let first = made(events.color())?.ok_or_else(|| ProbeError::rejected("no colour"))?;
    Ok(object([
        ("positions", numbers(events.positions())),
        ("orientation", Json::from(orientation)),
        ("horizontal", Json::Bool(events.is_horizontal())),
        ("linelength", Json::from(events.linelength())),
        ("lineoffset", Json::from(events.lineoffset())),
        ("segments", Json::Array(segments)),
        ("linestyle", dashes(events.dash_patterns())),
        ("linewidth", Json::from(events.broadcast_linewidths()[0])),
        ("color", rgba(first)),
        ("colors", colors(edges)),
    ]))
}
