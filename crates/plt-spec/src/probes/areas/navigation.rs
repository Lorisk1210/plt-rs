//! Probe operations for the `navigation` area: the toolbar's
//! zoom-to-rectangle and pan modes driven by mouse drags.

use plt::axes::{Adjustable, Axes};
use plt::figure::{Figure, Share};
use plt::navigation::{DragKey, NavMode, Navigation, PointerButton};
use plt::rc::Aspect;
use plt_core::Point;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[Op::new("navigation.drag", drag)];

/// Builds `scene` (see `spec/reference/probes/test_navigation.py`).
fn build(fig: &Figure, scene: &str) -> Result<(), ProbeError> {
    let line = |ax: &Axes, y: [f64; 2]| {
        ax.plot([0.0, 10.0], y)
            .map(|_| ())
            .map_err(ProbeError::rejected)
    };
    match scene {
        "linear" => line(&fig.subplot(), [0.0, 5.0])?,
        "logx" => {
            let ax = fig.subplot();
            ax.plot([1.0, 1000.0], [0.0, 5.0])
                .map_err(ProbeError::rejected)?;
            ax.set_xscale("log").map_err(ProbeError::rejected)?;
        }
        "symlogy" | "logity" => {
            let ax = fig.subplot();
            let (y, scale) = if scene == "symlogy" {
                ([-100.0, 100.0], "symlog")
            } else {
                ([0.01, 0.99], "logit")
            };
            line(&ax, y)?;
            ax.set_yscale(scale).map_err(ProbeError::rejected)?;
        }
        "equal" | "equal_datalim" => {
            let ax = fig.subplot();
            line(&ax, [0.0, 5.0])?;
            ax.set_aspect(Aspect::Equal).map_err(ProbeError::rejected)?;
            if scene == "equal_datalim" {
                ax.set_adjustable(Adjustable::Datalim);
            }
        }
        "inverted" => {
            let ax = fig.subplot();
            line(&ax, [0.0, 5.0])?;
            ax.xaxis().set_inverted(true);
        }
        "twinx" => {
            let ax = fig.subplot();
            line(&ax, [0.0, 5.0])?;
            line(&ax.twinx(), [100.0, 0.0])?;
        }
        "sharex" => {
            let axes = fig
                .subplots_shared(2, 1, Share::All, Share::None)
                .map_err(ProbeError::rejected)?;
            line(&axes[(0, 0)], [0.0, 5.0])?;
            line(&axes[(1, 0)], [5.0, 0.0])?;
        }
        _ => return Err(ProbeError::BadArgs(format!("unknown scene {scene:?}"))),
    }
    Ok(())
}

/// The key held, in matplotlib's naming.
fn drag_key(args: Args<'_>) -> Result<DragKey, ProbeError> {
    Ok(match args.opt_str("key")? {
        None => DragKey::None,
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        Some("control") => DragKey::Control,
        Some("shift") => DragKey::Shift,
        Some(other) => return Err(ProbeError::BadArgs(format!("unknown key {other:?}"))),
    })
}

/// Drags along `path` in `mode` and returns the view limits of every axes.
fn drag(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    build(&fig, args.str("scene")?)?;
    fig.layout();
    let mode = match args.str("mode")? {
        "zoom" => NavMode::Zoom,
        "pan" => NavMode::Pan,
        other => return Err(ProbeError::BadArgs(format!("unknown mode {other:?}"))),
    };
    let button = match args.i64("button")? {
        1 => PointerButton::Left,
        3 => PointerButton::Right,
        other => return Err(ProbeError::BadArgs(format!("unknown button {other}"))),
    };
    let key = drag_key(args)?;
    let path = args
        .json("path")?
        .as_array()
        .and_then(|points| {
            points
                .iter()
                .map(|point| match point.as_array()? {
                    [x, y] => Some(Point::new(x.as_f64()?, y.as_f64()?)),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()
        })
        .filter(|points| !points.is_empty())
        .ok_or_else(|| ProbeError::BadArgs("path is malformed".into()))?;
    let mut navigation = Navigation::new();
    navigation
        .toggle(&fig, mode)
        .map_err(ProbeError::rejected)?;
    navigation.press(&fig, path[0], button);
    for &at in &path[1..] {
        navigation.drag(at, key);
    }
    navigation.release(&fig, path[path.len() - 1], key);
    // matplotlib's toolbar redraws on release (`draw_idle`), which applies
    // the aspect; a window does the same when it repaints.
    fig.layout();
    Ok(Json::Array(
        fig.axes()
            .iter()
            .map(|ax| {
                let (x0, x1) = ax.xlim();
                let (y0, y1) = ax.ylim();
                Json::Array(vec![
                    Json::Array(vec![Json::Number(x0), Json::Number(x1)]),
                    Json::Array(vec![Json::Number(y0), Json::Number(y1)]),
                ])
            })
            .collect(),
    ))
}
