//! Probe operations for the `axes3d` area (navigating 3-D axes with the
//! mouse).

use plt::figure::Figure;
use plt::mplot3d::{Axes3D, Axis3};
use plt::navigation::{DragKey, NavMode, Navigation, PointerButton};
use plt_core::Point;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("axes3d.rotate", rotate),
    Op::new("axes3d.zoom", zoom),
    Op::new("axes3d.pan", pan),
    Op::new("axes3d.zoom_box", zoom_box),
    Op::new("axes3d.format_coord", format_coord),
];

fn bad(what: &str) -> ProbeError {
    ProbeError::BadArgs(format!("{what} is malformed"))
}

/// A point given as `[x, y]`.
fn point(args: Args<'_>, key: &str) -> Result<Point, ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[x, y] => Ok(Point::new(x, y)),
        _ => Err(bad(key)),
    }
}

/// The reference figure: a 3-D axes in a 4 × 3 inch figure at 100 dpi with
/// the probe's `view` and `limits`, laid out.
fn scene(args: Args<'_>) -> Result<(Figure, Axes3D), ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot_3d();
    if let Some(view) = args.get("view").filter(|v| !v.is_null()) {
        match view.as_f64_vec().as_deref() {
            Some(&[elev, azim, roll]) => {
                ax.view_init(elev, azim, roll);
            }
            _ => return Err(bad("view")),
        }
    }
    if let Some(limits) = args.get("limits").filter(|v| !v.is_null()) {
        let pairs = limits.as_array().ok_or_else(|| bad("limits"))?;
        for (axis, pair) in Axis3::ALL.into_iter().zip(pairs) {
            let &[lo, hi] = pair.as_f64_vec().as_deref().ok_or_else(|| bad("limits"))? else {
                return Err(bad("limits"));
            };
            ax.set_limits(axis, lo, hi).map_err(ProbeError::rejected)?;
        }
    }
    fig.layout();
    Ok((fig, ax))
}

fn limits(ax: &Axes3D) -> Outcome {
    Ok(Json::Array(
        Axis3::ALL
            .map(|axis| {
                let (lo, hi) = ax.limits(axis);
                Json::from(vec![lo, hi])
            })
            .into(),
    ))
}

fn rotate(args: Args<'_>) -> Outcome {
    let style = args.str("style")?;
    let (start, end) = (point(args, "start")?, point(args, "end")?);
    let control = args.opt_bool("control")?.unwrap_or(false);
    plt::rc::context(|| {
        plt::rc::set("axes3d.mouserotationstyle", style).map_err(ProbeError::rejected)?;
        let (_fig, ax) = scene(args)?;
        ax.drag_rotate(start, end, control);
        let view = ax.view();
        Ok(Json::from(vec![view.elev, view.azim, view.roll]))
    })
}

fn zoom(args: Args<'_>) -> Outcome {
    let (_fig, ax) = scene(args)?;
    let (start, end) = (point(args, "start")?, point(args, "end")?);
    ax.drag_zoom(start.y, end.y).map_err(ProbeError::rejected)?;
    limits(&ax)
}

fn pan(args: Args<'_>) -> Outcome {
    let (_fig, ax) = scene(args)?;
    let (start, end) = (point(args, "start")?, point(args, "end")?);
    ax.pan_view(end.x - start.x, end.y - start.y)
        .map_err(ProbeError::rejected)?;
    limits(&ax)
}

fn zoom_box(args: Args<'_>) -> Outcome {
    let (fig, ax) = scene(args)?;
    let &[x0, y0, x1, y1] = args.f64_vec("box")?.as_slice() else {
        return Err(bad("box"));
    };
    let button = match args.str("direction")? {
        "in" => PointerButton::Left,
        "out" => PointerButton::Right,
        _ => return Err(bad("direction")),
    };
    let key = match args.opt_str("key")? {
        None => DragKey::None,
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        Some(_) => return Err(bad("key")),
    };
    let mut navigation = Navigation::new();
    navigation
        .toggle(&fig, NavMode::Zoom)
        .map_err(ProbeError::rejected)?;
    if !navigation.press(&fig, Point::new(x0, y0), button) {
        return Err(ProbeError::Rejected(
            "the box does not start on the axes".into(),
        ));
    }
    navigation.release(&fig, Point::new(x1, y1), key);
    limits(&ax)
}

fn format_coord(args: Args<'_>) -> Outcome {
    let (fig, ax) = scene(args)?;
    if args.opt_bool("rotating")?.unwrap_or(false) {
        return Ok(Json::from(ax.rotation_coords()));
    }
    Ok(Json::from(
        Navigation::new().readout(&fig, point(args, "pixel")?),
    ))
}
