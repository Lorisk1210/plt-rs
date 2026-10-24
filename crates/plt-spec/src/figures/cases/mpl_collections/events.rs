//! `EventCollection`: the eight red events of
//! `generate_EventCollection_plot`, then moved, extended, turned,
//! stretched or restyled. Each case builds the collection in the state the
//! test leaves it (the limits are always set explicitly, so the collection
//! the axes autoscaled to first does not matter).

use plt::collections::{CollectionProps, CollectionSpec};
use plt::orientation::Orientation;
use plt::prelude::*;
use plt_core::Rgba;

use crate::figures::CaseResult;
use plt::axes::PlotError;

const POSITIONS: [f64; 8] = [0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0];
const EXTRA_POSITIONS: [f64; 3] = [34.0, 55.0, 89.0];

/// The events' geometry: positions, orientation, line offset and length.
struct Events {
    positions: Vec<f64>,
    orientation: Orientation,
    lineoffset: f64,
    linelength: f64,
}

impl Default for Events {
    /// The test's initial events: horizontal, centred on 1, half a unit long.
    fn default() -> Self {
        Self {
            positions: POSITIONS.to_vec(),
            orientation: Orientation::Horizontal,
            lineoffset: 1.0,
            linelength: 0.5,
        }
    }
}

impl Events {
    /// The initial positions followed by `extra`.
    fn extended(extra: &[f64]) -> Self {
        Self {
            positions: [&POSITIONS[..], extra].concat(),
            ..Self::default()
        }
    }

    /// The events turned upright.
    fn vertical() -> Self {
        Self {
            orientation: Orientation::Vertical,
            ..Self::default()
        }
    }
}

/// The test's initial properties: two points wide, solid, red.
fn initial_props() -> CollectionProps {
    CollectionProps {
        antialiased: Some(vec![true]),
        ..CollectionProps::default()
            .linewidth(2.0)
            .edgecolor(Rgba::new(1.0, 0.0, 0.0, 1.0))
            .linestyle(LineStyle::Solid)
    }
}

/// `generate_EventCollection_plot` with `events` styled by `props`, titled
/// after what the test changed.
fn plot(
    fig: &Figure,
    events: &Events,
    props: CollectionProps,
    title: &str,
) -> Result<Axes, PlotError> {
    let spec = CollectionSpec::events(
        &events.positions,
        events.orientation,
        events.lineoffset,
        events.linelength,
    )
    .with_props(props);
    let ax = fig.subplot();
    ax.add_collection(spec);
    ax.set_title(format!("EventCollection: {title}"));
    ax.set_xlim(-1.0, 22.0)?.set_ylim(0.0, 2.0)?;
    Ok(ax)
}

pub(super) fn get_props(fig: &Figure) -> CaseResult {
    plot(fig, &Events::default(), initial_props(), "default")?;
    Ok(())
}

pub(super) fn set_positions(fig: &Figure) -> CaseResult {
    let events = Events::extended(&EXTRA_POSITIONS);
    plot(fig, &events, initial_props(), "set_positions")?.set_xlim(-1.0, 90.0)?;
    Ok(())
}

pub(super) fn add_positions(fig: &Figure) -> CaseResult {
    let events = Events::extended(&EXTRA_POSITIONS[..1]);
    plot(fig, &events, initial_props(), "add_positions")?.set_xlim(-1.0, 35.0)?;
    Ok(())
}

pub(super) fn append_positions(fig: &Figure) -> CaseResult {
    let events = Events::extended(&EXTRA_POSITIONS[2..]);
    plot(fig, &events, initial_props(), "append_positions")?.set_xlim(-1.0, 90.0)?;
    Ok(())
}

pub(super) fn extend_positions(fig: &Figure) -> CaseResult {
    let events = Events::extended(&EXTRA_POSITIONS[1..]);
    plot(fig, &events, initial_props(), "extend_positions")?.set_xlim(-1.0, 90.0)?;
    Ok(())
}

pub(super) fn switch_orientation(fig: &Figure) -> CaseResult {
    plot(
        fig,
        &Events::vertical(),
        initial_props(),
        "switch_orientation",
    )?
    .set_ylim(-1.0, 22.0)?
    .set_xlim(0.0, 2.0)?;
    Ok(())
}

pub(super) fn switch_orientation_2x(fig: &Figure) -> CaseResult {
    plot(
        fig,
        &Events::default(),
        initial_props(),
        "switch_orientation 2x",
    )?;
    Ok(())
}

pub(super) fn set_orientation(fig: &Figure) -> CaseResult {
    plot(fig, &Events::vertical(), initial_props(), "set_orientation")?
        .set_ylim(-1.0, 22.0)?
        .set_xlim(0.0, 2.0)?;
    Ok(())
}

pub(super) fn set_linelength(fig: &Figure) -> CaseResult {
    let events = Events {
        linelength: 15.0,
        ..Events::default()
    };
    plot(fig, &events, initial_props(), "set_linelength")?.set_ylim(-20.0, 20.0)?;
    Ok(())
}

pub(super) fn set_lineoffset(fig: &Figure) -> CaseResult {
    let events = Events {
        lineoffset: -5.0,
        ..Events::default()
    };
    plot(fig, &events, initial_props(), "set_lineoffset")?.set_ylim(-6.0, -4.0)?;
    Ok(())
}

pub(super) fn set_prop_1(fig: &Figure) -> CaseResult {
    let props = initial_props().linestyle(LineStyle::Dashed);
    plot(fig, &Events::default(), props, "set_linestyle")?;
    Ok(())
}

pub(super) fn set_prop_2(fig: &Figure) -> CaseResult {
    // Dashes are scaled by the line width, as `"dashed"` is.
    let props = initial_props().linestyle(LineStyle::dashes(0.0, vec![3.7, 1.6]));
    plot(fig, &Events::default(), props, "set_linestyle")?;
    Ok(())
}

pub(super) fn set_prop_3(fig: &Figure) -> CaseResult {
    let props = initial_props().linewidth(5.0);
    plot(fig, &Events::default(), props, "set_linewidth")?;
    Ok(())
}

pub(super) fn set_color(fig: &Figure) -> CaseResult {
    let props = initial_props().edgecolor(Rgba::new(0.0, 1.0, 1.0, 1.0));
    plot(fig, &Events::default(), props, "set_color")?;
    Ok(())
}
