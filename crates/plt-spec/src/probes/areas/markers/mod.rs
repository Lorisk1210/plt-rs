//! Probe operations for the `markers` area.

mod markevery;

use plt::lines::{DrawStyle, line_vertices, pts_to_midstep, pts_to_poststep, pts_to_prestep};
use plt::markers::{FillStyle, Marker, MarkerKey, MarkerStyle, MarkerSymbol, PolygonStyle};
use plt_core::Affine2D;
use plt_core::Path;
use plt_render::{CapStyle, JoinStyle};

use super::canon::Canon;
use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("markers.MarkerStyle", marker_style),
    Op::new("markers.MarkerStyle.styles", marker_styles),
    Op::new("markers.MarkerStyle.derived", marker_derived),
    Op::new("markers.MarkerStyle.class_attributes", class_attributes),
    Op::new("cbook.pts_to_prestep", |args| steps(args, pts_to_prestep)),
    Op::new("cbook.pts_to_poststep", |args| steps(args, pts_to_poststep)),
    Op::new("cbook.pts_to_midstep", |args| steps(args, pts_to_midstep)),
    Op::new("lines.drawstyle_names", drawstyle_names),
    Op::new("lines.drawstyle", drawstyle),
    Op::new("lines.Line2D.set_drawstyle", set_drawstyle),
    Op::new("lines.Line2D.get_path", line_path),
    Op::new("lines.markevery", markevery::marker_indices),
    Op::new("axes._process_plot_format", plot_format),
];

/// A format string's `[linestyle, marker, color]` under matplotlib's
/// defaults, colours resolved against the default property cycle.
fn plot_format(args: Args<'_>) -> Outcome {
    let rc = plt::rc::RcParams::matplotlib();
    let format = plt::lines::FormatString::parse_with(args.str("fmt")?, &rc)
        .map_err(ProbeError::rejected)?;
    let color = format
        .color
        .map(|color| color.resolve_with(None, &rc.axes_prop_cycle.colors()))
        .transpose()
        .map_err(ProbeError::rejected)?
        .map_or(Json::Null, |c| [c.r, c.g, c.b, c.a].canon());
    Ok(Json::Array(vec![
        format.linestyle.canon(),
        format.marker.canon(),
        color,
    ]))
}

/// One of plt-rs' `pts_to_*step` conversions.
pub(crate) type StepFn = fn(&[f64], &[&[f64]]) -> Result<Vec<Vec<f64>>, plt::lines::StepError>;

/// A `cbook.pts_to_*step` probe: `{x, ys}` converted by `step`.
pub(crate) fn steps(args: Args<'_>, step: StepFn) -> Outcome {
    let x = args.f64_vec("x")?;
    let ys: Vec<Vec<f64>> = args
        .json("ys")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|y| {
            y.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad y series {y}")))
        })
        .collect::<Result<_, _>>()?;
    let ys: Vec<&[f64]> = ys.iter().map(Vec::as_slice).collect();
    let rows = step(&x, &ys).map_err(ProbeError::rejected)?;
    Ok(Json::from(
        rows.into_iter().map(Json::from).collect::<Vec<_>>(),
    ))
}

/// matplotlib's legacy draw-method name for each style.
fn draw_method(style: DrawStyle) -> &'static str {
    match style {
        DrawStyle::Default => "_draw_lines",
        DrawStyle::Steps | DrawStyle::StepsPre => "_draw_steps_pre",
        DrawStyle::StepsMid => "_draw_steps_mid",
        DrawStyle::StepsPost => "_draw_steps_post",
    }
}

fn drawstyle_names(_args: Args<'_>) -> Outcome {
    let keys: Vec<Json> = DrawStyle::all().map(|(name, _)| Json::from(name)).collect();
    let methods = DrawStyle::all()
        .map(|(name, style)| (name.to_owned(), Json::from(draw_method(style))))
        .collect();
    let lookup = ["default", "steps", "steps-pre", "steps-post", "steps-mid"]
        .map(Json::from)
        .to_vec();
    Ok(Json::Object(vec![
        ("drawStyleKeys".into(), Json::Array(keys)),
        ("drawStyles".into(), Json::Object(methods)),
        ("STEP_LOOKUP_MAP".into(), Json::Array(lookup)),
    ]))
}

fn drawstyle(args: Args<'_>) -> Outcome {
    let style: DrawStyle = args
        .str("drawstyle")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let step_function = match style {
        DrawStyle::Default => "identity",
        DrawStyle::Steps | DrawStyle::StepsPre => "pts_to_prestep",
        DrawStyle::StepsMid => "pts_to_midstep",
        DrawStyle::StepsPost => "pts_to_poststep",
    };
    Ok(Json::Object(vec![
        ("draw_method".into(), draw_method(style).into()),
        ("step_function".into(), step_function.into()),
    ]))
}

fn set_drawstyle(args: Args<'_>) -> Outcome {
    let style = match args.opt_str("drawstyle")? {
        Some(name) => name.parse::<DrawStyle>().map_err(ProbeError::rejected)?,
        None => DrawStyle::Default,
    };
    Ok(style.name().into())
}

fn line_path(args: Args<'_>) -> Outcome {
    let (x, y) = (args.f64_vec("x")?, args.f64_vec("y")?);
    let style: DrawStyle = args
        .str("drawstyle")?
        .parse()
        .map_err(ProbeError::rejected)?;
    let vertices = line_vertices(&x, &y, style).map_err(ProbeError::rejected)?;
    Ok(encode::path(&Path::polyline(vertices)))
}

fn marker_style(args: Args<'_>) -> Outcome {
    build(args, args.opt_str("fillstyle")?).map(|m| describe(&m, args))
}

fn marker_styles(args: Args<'_>) -> Outcome {
    let marker = build(args, Some("full"))?;
    Ok(Json::Object(vec![
        ("joinstyle".into(), marker.joinstyle().name().into()),
        ("capstyle".into(), marker.capstyle().name().into()),
    ]))
}

fn marker_derived(args: Args<'_>) -> Outcome {
    let mut marker = build(args, args.opt_str("fillstyle")?)?;
    for step in args.json("steps")?.as_array().unwrap_or(&[]) {
        marker = apply_step(&marker, step)?;
    }
    Ok(describe(&marker, args))
}

fn class_attributes(_args: Args<'_>) -> Outcome {
    let markers = MarkerSymbol::all()
        .map(|(key, symbol)| Json::Array(vec![key_json(key), symbol.name().into()]))
        .collect();
    let filled = [
        ".", "o", "v", "^", "<", ">", "8", "s", "p", "*", "h", "H", "D", "d", "P", "X",
    ]
    .into_iter()
    .filter(|code| {
        code.parse::<Marker>()
            .ok()
            .and_then(|m| MarkerStyle::new(m, FillStyle::Full).ok())
            .is_some_and(|m| m.is_filled())
    })
    .map(Json::from)
    .collect();
    let fillstyles = [
        FillStyle::Full,
        FillStyle::Left,
        FillStyle::Right,
        FillStyle::Bottom,
        FillStyle::Top,
        FillStyle::None,
    ]
    .into_iter()
    .map(|f| Json::from(f.name()))
    .collect();
    Ok(Json::Object(vec![
        ("markers".into(), Json::Array(markers)),
        ("filled_markers".into(), Json::Array(filled)),
        ("fillstyles".into(), Json::Array(fillstyles)),
    ]))
}

/// Builds the marker described by `marker`, `fillstyle` (default: the
/// `markers.fillstyle` rc override, else `"full"`), `transform`,
/// `capstyle` and `joinstyle`.
pub(crate) fn build(args: Args<'_>, fillstyle: Option<&str>) -> Result<MarkerStyle, ProbeError> {
    let rc_fill = args
        .get("rc")
        .and_then(|rc| rc.get("markers.fillstyle"))
        .and_then(Json::as_str);
    let fill: FillStyle = fillstyle
        .or(rc_fill)
        .unwrap_or("full")
        .parse()
        .map_err(ProbeError::rejected)?;
    let cap = args
        .opt_str("capstyle")?
        .map(|name| CapStyle::from_name(name).ok_or_else(|| ProbeError::rejected(name)))
        .transpose()?;
    let join = args
        .opt_str("joinstyle")?
        .map(|name| JoinStyle::from_name(name).ok_or_else(|| ProbeError::rejected(name)))
        .transpose()?;
    let marker = decode_marker(args.get("marker"))?;
    let style = MarkerStyle::with_styles(marker, fill, cap, join).map_err(ProbeError::rejected)?;
    Ok(match args.get("transform") {
        Some(ops) => style.transformed(encode::parse_affine_ops(ops)?),
        None => style,
    })
}

fn decode_marker(value: Option<&Json>) -> Result<Marker, ProbeError> {
    let Some(value) = value else {
        return Err(ProbeError::rejected("marker None"));
    };
    match value {
        Json::String(code) => code.parse().map_err(ProbeError::rejected),
        Json::Number(_) | Json::Integer(_) => {
            let code = value.as_i64().ok_or_else(|| ProbeError::rejected(value))?;
            Marker::from_code(code).map_err(ProbeError::rejected)
        }
        Json::Object(_) => Ok(Marker::Path(encode::parse_path(value)?)),
        Json::Array(items) if items.iter().all(|i| i.as_f64().is_some()) && !items.is_empty() => {
            decode_tuple(items)
        }
        Json::Array(_) => encode::parse_points(value)
            .map(Marker::Vertices)
            .map_err(|_| ProbeError::rejected("vertex markers need (N, 2) coordinates")),
        Json::Null | Json::Bool(_) => Err(ProbeError::rejected(value)),
    }
}

fn decode_tuple(items: &[Json]) -> Result<Marker, ProbeError> {
    let numbers: Vec<f64> = items.iter().filter_map(Json::as_f64).collect();
    let (sides, style, angle) = match numbers.as_slice() {
        [sides, style] => (*sides, *style, 0.0),
        [sides, style, angle] => (*sides, *style, *angle),
        _ => return Err(ProbeError::rejected("marker tuples have 2 or 3 elements")),
    };
    let style = PolygonStyle::from_code(style as i64)
        .filter(|_| style.fract() == 0.0)
        .ok_or_else(|| ProbeError::rejected("unexpected tuple marker style"))?;
    if sides < 0.0 || sides.fract() != 0.0 {
        return Err(ProbeError::rejected(
            "plt-rs requires a non-negative integer side count",
        ));
    }
    Ok(Marker::polygon(sides as usize, style, angle))
}

/// `marker` after one `["rotated", {deg | rad}]`, `["scaled", sx, sy?]`
/// or `["transformed", ops]` step.
pub(crate) fn apply_step(marker: &MarkerStyle, step: &Json) -> Result<MarkerStyle, ProbeError> {
    let parts = step.as_array().unwrap_or(&[]);
    let kind = parts.first().and_then(Json::as_str).unwrap_or_default();
    match kind {
        "rotated" => {
            let kwargs = parts.get(1).unwrap_or(&Json::Null);
            match (
                kwargs.get("deg").and_then(Json::as_f64),
                kwargs.get("rad").and_then(Json::as_f64),
            ) {
                (Some(deg), None) => Ok(marker.rotated_deg(deg)),
                (None, Some(rad)) => Ok(marker.rotated(rad)),
                _ => Err(ProbeError::rejected(
                    "exactly one of deg or rad is required",
                )),
            }
        }
        "scaled" => {
            let factors: Vec<f64> = parts[1..].iter().filter_map(Json::as_f64).collect();
            match factors.as_slice() {
                [s] => Ok(marker.scaled(*s, *s)),
                [sx, sy] => Ok(marker.scaled(*sx, *sy)),
                _ => Err(ProbeError::BadArgs(format!("bad scaled step {step}"))),
            }
        }
        "transformed" => {
            let ops = parts.get(1).unwrap_or(&Json::Null);
            Ok(marker.transformed(encode::parse_affine_ops(ops)?))
        }
        _ => Err(ProbeError::BadArgs(format!("unknown step {step}"))),
    }
}

/// The MARKER DESCRIPTION of the spec. `get_marker()` returns matplotlib's
/// input object verbatim, so the recorded `marker` argument is echoed.
fn describe(marker: &MarkerStyle, args: Args<'_>) -> Json {
    let optional_matrix = |t: Option<Affine2D>| t.as_ref().map_or(Json::Null, encode::matrix);
    let snap = marker.snap_threshold().map_or(Json::Null, Json::from);
    Json::Object(vec![
        ("path".into(), encode::path(marker.path())),
        ("transform".into(), encode::matrix(&marker.transform())),
        (
            "alt_path".into(),
            marker.alt_path().map_or(Json::Null, encode::path),
        ),
        (
            "alt_transform".into(),
            optional_matrix(marker.alt_transform()),
        ),
        (
            "user_transform".into(),
            optional_matrix(marker.user_transform()),
        ),
        ("joinstyle".into(), marker.joinstyle().name().into()),
        ("capstyle".into(), marker.capstyle().name().into()),
        ("filled".into(), marker.is_filled().into()),
        ("fillstyle".into(), marker.fillstyle().name().into()),
        ("snap_threshold".into(), snap),
        (
            "marker".into(),
            args.get("marker").cloned().unwrap_or(Json::Null),
        ),
    ])
}

fn key_json(key: MarkerKey) -> Json {
    match key {
        MarkerKey::Char(c) => c.into(),
        MarkerKey::Code(code) => Json::from(code as f64),
    }
}
