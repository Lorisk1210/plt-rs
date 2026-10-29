//! Probe operations for the `axes3d` area (navigating 3-D axes with the
//! mouse, and their aspect).

use pltrs::axes::Adjustable;
use pltrs::figure::Figure;
use pltrs::mplot3d::{Aspect3D, Axes3D, Axis3, ProjType};
use pltrs::navigation::{DragKey, NavMode, Navigation, PointerButton};
use pltrs_core::Point;

use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, malformed};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("axes3d.rotate", rotate),
    Op::new("axes3d.zoom", zoom),
    Op::new("axes3d.pan", pan),
    Op::new("axes3d.zoom_box", zoom_box),
    Op::new("axes3d.format_coord", format_coord),
    Op::new("axes3d.set_aspect", set_aspect),
    Op::new("axes3d.zoom_data", zoom_data),
    Op::new("axes3d.project", project),
];

/// A point given as `[x, y]`.
fn point(args: Args<'_>, key: &str) -> Result<Point, ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        &[x, y] => Ok(Point::new(x, y)),
        _ => Err(malformed(key)),
    }
}

/// The axis a `scales` key names.
fn axis_named(name: &str) -> Result<Axis3, ProbeError> {
    match name {
        "x" => Ok(Axis3::X),
        "y" => Ok(Axis3::Y),
        "z" => Ok(Axis3::Z),
        _ => Err(malformed("scales")),
    }
}

/// The reference figure: a 3-D axes in a 4 × 3 inch figure at 100 dpi with
/// the probe's `view`, `scales` and `limits`, laid out.
fn scene(args: Args<'_>) -> Result<(Figure, Axes3D), ProbeError> {
    let fig = Figure::with_size(4.0, 3.0, 100.0);
    let ax = fig.subplot_3d();
    if let Some(view) = args.get("view").filter(|v| !v.is_null()) {
        match view.as_f64_vec().as_deref() {
            Some(&[elev, azim, roll]) => {
                ax.view_init(elev, azim, roll);
            }
            _ => return Err(malformed("view")),
        }
    }
    if let Some(name) = args.get("vertical_axis") {
        ax.set_vertical_axis(axis_named(
            name.as_str().ok_or_else(|| malformed("vertical_axis"))?,
        )?);
    }
    if let Some(focal_length) = args.get("focal_length") {
        let focal_length = focal_length
            .as_f64()
            .ok_or_else(|| malformed("focal_length"))?;
        ax.set_proj_type(ProjType::Persp { focal_length })
            .map_err(ProbeError::rejected)?;
    }
    if args.get("box_aspect").is_some() {
        let &[x, y, z] = args.f64_vec("box_aspect")?.as_slice() else {
            return Err(malformed("box_aspect"));
        };
        ax.set_box_aspect(Some([x, y, z]), 1.0)
            .map_err(ProbeError::rejected)?;
    }
    if let Some(scales) = args.get("scales") {
        for (name, scale) in scales.as_object().ok_or_else(|| malformed("scales"))? {
            let scale = scale.as_str().ok_or_else(|| malformed("scales"))?;
            let options = pltrs::scale::ScaleOptions::default();
            ax.set_scale_with(axis_named(name)?, scale, options)
                .map_err(ProbeError::rejected)?;
        }
    }
    if let Some(limits) = args.get("limits").filter(|v| !v.is_null()) {
        let pairs = limits.as_array().ok_or_else(|| malformed("limits"))?;
        for (axis, pair) in Axis3::ALL.into_iter().zip(pairs) {
            let &[lo, hi] = pair
                .as_f64_vec()
                .as_deref()
                .ok_or_else(|| malformed("limits"))?
            else {
                return Err(malformed("limits"));
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
    pltrs::rc::context(|| {
        pltrs::rc::set("axes3d.mouserotationstyle", style).map_err(ProbeError::rejected)?;
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
        return Err(malformed("box"));
    };
    let button = match args.str("direction")? {
        "in" => PointerButton::Left,
        "out" => PointerButton::Right,
        _ => return Err(malformed("direction")),
    };
    let key = match args.opt_str("key")? {
        None => DragKey::None,
        Some("x") => DragKey::X,
        Some("y") => DragKey::Y,
        Some(_) => return Err(malformed("key")),
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

/// An aspect given by its matplotlib name.
fn aspect(name: &str) -> Result<Aspect3D, ProbeError> {
    name.parse().map_err(ProbeError::rejected)
}

fn set_aspect(args: Args<'_>) -> Outcome {
    let (_fig, ax) = scene(args)?;
    let adjustable = match args.str("adjustable")? {
        "box" => Adjustable::Box,
        "datalim" => Adjustable::Datalim,
        _ => return Err(malformed("adjustable")),
    };
    ax.set_aspect(aspect(args.str("aspect")?)?, adjustable)
        .map_err(ProbeError::rejected)?;
    if let Some(then) = args.opt_str("then")? {
        ax.set_aspect(aspect(then)?, adjustable)
            .map_err(ProbeError::rejected)?;
    }
    Ok(Json::Object(vec![
        ("limits".into(), limits(&ax)?),
        ("box_aspect".into(), Json::from(ax.box_aspect().to_vec())),
    ]))
}

fn zoom_data(args: Args<'_>) -> Outcome {
    let (_fig, ax) = scene(args)?;
    if let Some(name) = args.get("aspect").and_then(Json::as_str) {
        ax.set_aspect(aspect(name)?, Adjustable::Box)
            .map_err(ProbeError::rejected)?;
    }
    let &[u, v, w] = args.f64_vec("scale")?.as_slice() else {
        return Err(malformed("scale"));
    };
    ax.zoom_data_limits(u, v, w).map_err(ProbeError::rejected)?;
    limits(&ax)
}

/// The projection matrix, and `points` projected together (alone when
/// there is one).
fn project(args: Args<'_>) -> Outcome {
    let (_fig, ax) = scene(args)?;
    let points = (args
        .json("points")?
        .as_array()
        .ok_or_else(|| malformed("points"))?
        .iter())
    .map(|p| match p.as_f64_vec().as_deref() {
        Some(&[x, y, z]) => Ok([x, y, z]),
        _ => Err(malformed("points")),
    })
    .collect::<Result<Vec<_>, _>>()?;
    let matrix = ax.proj();
    let projected = matrix.project_all(&points);
    Ok(Json::Object(vec![
        ("matrix".into(), rows(matrix.rows())),
        ("projected".into(), rows(&projected)),
    ]))
}

/// Rows of numbers as nested JSON arrays.
fn rows<const N: usize>(rows: &[[f64; N]]) -> Json {
    Json::Array(
        rows.iter()
            .map(|row| Json::Array(row.iter().copied().map(Json::Number).collect()))
            .collect(),
    )
}
