//! Where matplotlib evaluates its transform graph while a figure is
//! drawn (`pltrs::transforms::Evaluation`): the package keeps
//! matplotlib's graph over pltrs (Rust)'s leaves, and replays there the
//! evaluations of each draw whose graph has a wild leaf
//! (`_transform_replay`), so that numpy's arithmetic warns where and as
//! often as matplotlib's does.

use pltrs::artist::RawHandle;
use pltrs::figure::Figure;
use pltrs::ticker::AxisName;
use pltrs::transforms::{DrawnTick, Evaluation, EvaluationHook};

use crate::binding::prelude::*;

/// A tick `Axis.draw` draws, as Python takes it: `(gridline, (mark1,
/// mark2), (label1, label2), text, pad, loc)`.
type PyTick = (bool, (bool, bool), (bool, bool), bool, f64, f64);

/// `tick` as Python takes it.
fn py_tick(tick: &DrawnTick) -> PyTick {
    (
        tick.gridline,
        (tick.marks[0], tick.marks[1]),
        (tick.labels[0], tick.labels[1]),
        tick.text,
        tick.pad,
        tick.loc,
    )
}

/// What an evaluation of no axis hands the hook for the axis.
const NO_AXIS: Option<(String, Vec<PyTick>, bool)> = None;

/// Calls `hook(kind, artist, axis)` for each evaluation of matplotlib's
/// transform graph the draws and measurements of `figure` stand for
/// while a leaf of the graph is wild: `("draw", id, None)` where an
/// artist is drawn, `("axes", id, None)` where an axes starts drawing
/// itself, `("background", id, None)` where its background patch is,
/// `("axis", id, (name, ticks, label))` where the axis `name` (`"x"` or
/// `"y"`) of an axes is, drawing `ticks` and its label if `label`,
/// `("axis measured", id, (name, ticks, label))` where it is measured,
/// and `("measured", id, None)` where the tight box of an axes is taken
/// once its axes are measured (`"measured alone"` without its artists);
/// `None` removes the hook. What the hook raises (a warning the
/// filters turn into an exception) is raised by the native call drawing
/// the figure.
fn set_evaluation_hook(figure: Figure, hook: Option<Callback>) -> PyResult<()> {
    figure.set_evaluation_hook(hook.map(|hook| {
        EvaluationHook::new(move |evaluation| {
            let _: Option<Owned> = match evaluation {
                Evaluation::Draw(id) => hook.call(("draw", id.index(), NO_AXIS)),
                Evaluation::Axes(id) => hook.call(("axes", id.index(), NO_AXIS)),
                Evaluation::Background(id) => hook.call(("background", id.index(), NO_AXIS)),
                Evaluation::Axis {
                    axes,
                    axis,
                    ticks,
                    label,
                    measured,
                } => {
                    let kind = if *measured { "axis measured" } else { "axis" };
                    let name = match axis {
                        AxisName::X => "x",
                        AxisName::Y => "y",
                    };
                    let ticks: Vec<PyTick> = ticks.iter().map(py_tick).collect();
                    hook.call((kind, axes.index(), Some((name.to_owned(), ticks, *label))))
                }
                Evaluation::Measured { axes, children } => {
                    let kind = if *children {
                        "measured"
                    } else {
                        "measured alone"
                    };
                    hook.call((kind, axes.index(), NO_AXIS))
                }
            };
        })
    }));
    Ok(())
}

/// Whether matplotlib's evaluations of its transform graph for `artist`
/// are worth replaying where it joins an axes or is measured: a leaf of
/// its figure's graph, or its own transform, is wild.
fn replays(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.evaluates_wild())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_evaluation_hook", set_evaluation_hook)?;
    module.add("replays", replays)
}
