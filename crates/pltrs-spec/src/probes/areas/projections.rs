//! Probe operations for the `projections` area: the pan/zoom and
//! zoom-to-rectangle tools on axes of a projection the program registered
//! that pans and zooms them in its own way.

use pltrs::axes::{Axes, AxisName};
use pltrs::figure::{Figure, Share};

use super::navigation::{dragged, view_limits};
use crate::figures::{RangeFrame, RangeFrameOptions, register_range_frame};
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("projections.drag", drag)];

/// Builds `scene` (see `spec/reference/probes/test_projections.py`).
fn build(fig: &Figure, scene: &str) -> Result<(), ProbeError> {
    let range_frame = register_range_frame().map_err(ProbeError::rejected)?;
    let padded = |pad| range_frame.with_options(RangeFrameOptions { pad });
    let line = |ax: &Axes, y: [f64; 2]| {
        ax.plot([0.0, 10.0], y)
            .map(drop)
            .map_err(ProbeError::rejected)
    };
    let single = || {
        let ax = fig
            .subplot_with_projection(padded(0.1))
            .map_err(ProbeError::rejected)?;
        line(&ax, [0.0, 5.0])?;
        Ok::<_, ProbeError>(ax)
    };
    match scene {
        "single" => {
            single()?;
        }
        "twinx" => {
            let twin = single()?
                .twin_with_projection(AxisName::X, 0.0, padded(0.2))
                .map_err(ProbeError::rejected)?;
            line(&twin, [100.0, 0.0])?;
        }
        "sharey" => {
            let axs = fig
                .subplots_with_projection(1, 2, Share::None, Share::All, range_frame)
                .map_err(ProbeError::rejected)?;
            line(&axs[(0, 0)], [0.0, 5.0])?;
            line(&axs[(0, 1)], [5.0, 0.0])?;
        }
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {scene:?}"))),
    }
    Ok(())
}

/// Drags along `path` in `mode` and returns the view limits and the pad
/// of every axes.
fn drag(args: Args<'_>) -> Outcome {
    let fig = dragged(args, |fig| build(fig, args.str("scene")?))?;
    fig.axes()
        .iter()
        .map(|ax| {
            let pad = ax
                .with_projection(|frame: &RangeFrame| frame.pad)
                .ok_or_else(|| ProbeError::BadArgs("an axes is not a range frame".into()))?;
            let mut entry = view_limits(ax);
            entry.push(Json::Number(pad));
            Ok(Json::Array(entry))
        })
        .collect::<Result<_, _>>()
        .map(Json::Array)
}
