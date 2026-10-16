//! Probe operations for the `markers` area.

use plt::markers::{FillStyle, Marker, MarkerKey, MarkerStyle, MarkerSymbol, PolygonStyle};
use plt_core::Affine2D;
use plt_render::{CapStyle, JoinStyle};

use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("markers.MarkerStyle", marker_style),
    Op::new("markers.MarkerStyle.styles", marker_styles),
    Op::new("markers.MarkerStyle.derived", marker_derived),
    Op::new("markers.MarkerStyle.class_attributes", class_attributes),
];

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
fn build(args: Args<'_>, fillstyle: Option<&str>) -> Result<MarkerStyle, ProbeError> {
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
        Json::Number(_) => {
            let code = value.as_i64().ok_or_else(|| ProbeError::rejected(value))?;
            Marker::from_code(code).map_err(ProbeError::rejected)
        }
        Json::Object(_) => Ok(Marker::Path(encode::parse_path(value)?)),
        Json::Array(items)
            if items.iter().all(|i| matches!(i, Json::Number(_))) && !items.is_empty() =>
        {
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

fn apply_step(marker: &MarkerStyle, step: &Json) -> Result<MarkerStyle, ProbeError> {
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
