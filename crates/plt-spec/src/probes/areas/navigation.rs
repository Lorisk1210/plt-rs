//! Probe operations for the `navigation` area: the toolbar's
//! zoom-to-rectangle and pan modes driven by mouse drags, and what the
//! status bar shows of data points.

use plt::axes::{Adjustable, Axes, AxisName};
use plt::figure::{Figure, Share};
use plt::navigation::{DragKey, NavMode, Navigation, PointerButton};
use plt::rc::Aspect;
use plt_core::Point;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("navigation.drag", drag),
    Op::new("navigation.format_coord", format_coord),
];

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
                ax.set_adjustable(Adjustable::Datalim)
                    .map_err(ProbeError::rejected)?;
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
            line(&ax.twinx().map_err(ProbeError::rejected)?, [100.0, 0.0])?;
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

/// The points of argument `key`, `[[x, y], ...]`.
fn points(args: Args<'_>, key: &str) -> Result<Vec<Point>, ProbeError> {
    args.json(key)?
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
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is malformed")))
}

/// What the status bar shows of each of `points` in the first axes of
/// `scene`, and of their coordinates.
fn format_coord(args: Args<'_>) -> Outcome {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    build(&fig, args.str("scene")?)?;
    fig.layout();
    let ax = fig
        .axes()
        .into_iter()
        .next()
        .ok_or_else(|| ProbeError::BadArgs("the scene has no axes".into()))?;
    let text = |value: Option<String>| value.map_or(Json::Null, Json::String);
    Ok(Json::Array(
        points(args, "points")?
            .into_iter()
            .map(|p| {
                Json::Array(vec![
                    Json::String(ax.format_coord(p.x, p.y)),
                    text(ax.format_data(AxisName::X, p.x)),
                    text(ax.format_data(AxisName::Y, p.y)),
                ])
            })
            .collect(),
    ))
}

/// Drags along `path` in `mode` and returns the view limits of every axes.
fn drag(args: Args<'_>) -> Outcome {
    let fig = dragged(args, |fig| build(fig, args.str("scene")?))?;
    Ok(Json::Array(
        fig.axes()
            .iter()
            .map(|ax| Json::Array(view_limits(ax)))
            .collect(),
    ))
}

/// `[xlim, ylim]` of `ax`.
pub(super) fn view_limits(ax: &Axes) -> Vec<Json> {
    let (x0, x1) = ax.xlim();
    let (y0, y1) = ax.ylim();
    vec![
        Json::Array(vec![Json::Number(x0), Json::Number(x1)]),
        Json::Array(vec![Json::Number(y0), Json::Number(y1)]),
    ]
}

/// A figure of 4 × 3 inches at 100 dpi built by `build`, laid out, then
/// dragged along the mouse `path` in `mode` with `button` and `key` held
/// (see `drag` in `spec/reference/probes/test_navigation.py`).
pub(super) fn dragged(
    args: Args<'_>,
    build: impl FnOnce(&Figure) -> Result<(), ProbeError>,
) -> Result<Figure, ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    build(&fig)?;
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
    let path = points(args, "path")?;
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
    Ok(fig)
}
