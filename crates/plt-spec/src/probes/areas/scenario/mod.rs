//! Replaying scenarios ported from matplotlib's own test suite: a list of
//! method calls on a fresh figure, its axes and the artists they return,
//! recording the results of the queries (see
//! `spec/reference/probes/_scenario.py` for the step language).
//!
//! A [`Scene`](scene::Scene) holds the figure, its axes numbered in
//! creation order and the objects earlier steps returned; each step is a
//! [`Call`](call::Call) dispatched on its target: the figure, an axes (by
//! topic: limits, layout, labels, ticks, plotting, legends) or a returned
//! artist.

mod axes_fills;
mod axes_images;
mod axes_labels;
mod axes_layout;
mod axes_limits;
mod axes_plot;
mod axes_steps;
mod axes_ticks;
mod call;
mod colorbar_steps;
mod figure_steps;
mod legend_steps;
mod object_steps;
mod scene;

use plt::rc;
use plt::style;

use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError};
use scene::Scene;

/// Replays `steps` in a fresh figure under the spec's reference
/// configuration, inside the scenario's style and rcParams.
pub(crate) fn replay(args: Args<'_>) -> Outcome {
    let steps = args
        .json("steps")?
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs("steps is not a list".into()))?;
    rc::context(|| {
        // `matplotlib.testing.setup` names the font family and turns
        // hinting off.
        rc::reset();
        rc::set("font.family", "DejaVu Sans").map_err(ProbeError::rejected)?;
        rc::set("text.hinting", "none").map_err(ProbeError::rejected)?;
        if let Some(name) = args.opt_str("style")? {
            style::use_style(name).map_err(ProbeError::rejected)?;
        }
        if let Some(params) = args.get("rc") {
            apply_rc(params)?;
        }
        Scene::new(args.get("figure"))?.run(steps)
    })
}

/// `rc_context(params)`: every value is given in rc-file syntax.
fn apply_rc(params: &Json) -> Result<(), ProbeError> {
    let entries = params
        .as_object()
        .ok_or_else(|| ProbeError::BadArgs(format!("rc is not an object: {params}")))?;
    for (key, value) in entries {
        let text = match value {
            Json::String(text) => text.clone(),
            Json::Bool(on) => on.to_string(),
            other => other.to_string(),
        };
        rc::set(key, &text).map_err(ProbeError::rejected)?;
    }
    Ok(())
}
