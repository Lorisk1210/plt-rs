//! Probe operations of `matplotlib.bezier`.

use pltrs_core::Point;
use pltrs_core::bezier::{self, BezierSegment, Direction, Parallel};

use super::{path_arg, point_arg, point_list};
use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Outcome, ProbeError};

/// `[cx, cy, r]` as `inside_circle(cx, cy, r)`.
fn circle_arg(args: Args<'_>, key: &str) -> Result<impl Fn(Point) -> bool, ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        [cx, cy, r] => Ok(bezier::inside_circle(Point::new(*cx, *cy), *r)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not [cx, cy, r]"))),
    }
}

fn segment_arg(args: Args<'_>, key: &str) -> Result<BezierSegment, ProbeError> {
    let rows = args
        .json(key)?
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .map(Json::as_f64_vec)
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of points")))?;
    BezierSegment::from_rows(&rows).map_err(ProbeError::rejected)
}

fn quad_arg(args: Args<'_>, key: &str) -> Result<[Point; 3], ProbeError> {
    let points = encode::parse_points(args.json(key)?)?;
    points
        .try_into()
        .map_err(|_| ProbeError::BadArgs(format!("{key} needs three points")))
}

fn rows(curve: &BezierSegment) -> Json {
    Json::Array(
        curve
            .control_points()
            .map(|c| Json::from(c.to_vec()))
            .collect(),
    )
}

pub(super) fn bezier_call(args: Args<'_>) -> Outcome {
    let curve = segment_arg(args, "control_points")?;
    let t = args.json("t")?;
    match t.as_f64() {
        Some(t) => Ok(Json::from(curve.eval(t))),
        None => Ok(Json::Array(
            t.as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs("t must be a number or list".into()))?
                .into_iter()
                .map(|t| Json::from(curve.eval(t)))
                .collect(),
        )),
    }
}

pub(super) fn bezier_point_at_t(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        segment_arg(args, "control_points")?.eval(args.f64("t")?),
    ))
}

pub(super) fn bezier_properties(args: Args<'_>) -> Outcome {
    let curve = segment_arg(args, "control_points")?;
    let coefficients = curve
        .polynomial_coefficients()
        .into_iter()
        .map(Json::from)
        .collect();
    Ok(Json::Object(vec![
        ("degree".into(), Json::from(curve.degree() as f64)),
        ("dimension".into(), Json::from(curve.dimension() as f64)),
        ("control_points".into(), rows(&curve)),
        ("polynomial_coefficients".into(), Json::Array(coefficients)),
    ]))
}

pub(super) fn bezier_extrema(args: Args<'_>) -> Outcome {
    let (dims, roots) = segment_arg(args, "control_points")?.axis_aligned_extrema();
    Ok(Json::Object(vec![
        (
            "dims".into(),
            Json::from(dims.into_iter().map(|d| d as f64).collect::<Vec<_>>()),
        ),
        ("dzeros".into(), Json::from(roots)),
    ]))
}

pub(super) fn split_de_casteljau(args: Args<'_>) -> Outcome {
    let curve = segment_arg(args, "beta")?;
    if curve.degree() == 0 {
        return Err(ProbeError::rejected(
            "matplotlib loops forever on a single point",
        ));
    }
    let (left, right) = curve.split(args.f64("t")?);
    Ok(Json::Array(vec![rows(&left), rows(&right)]))
}

pub(super) fn find_t(args: Args<'_>) -> Outcome {
    let curve = segment_arg(args, "control_points")?;
    let inside = circle_arg(args, "circle")?;
    let (t0, t1) = bezier::find_bezier_t_intersecting_with_closedpath(
        |t| curve.point_at(t),
        inside,
        args.f64("t0")?,
        args.f64("t1")?,
        args.f64("tolerance")?,
    )
    .map_err(ProbeError::rejected)?;
    Ok(Json::from(vec![t0, t1]))
}

pub(crate) fn split_bezier(args: Args<'_>) -> Outcome {
    let points = encode::parse_points(args.json("bezier")?)?;
    let (left, right) = bezier::split_bezier_intersecting_with_closedpath(
        &points,
        circle_arg(args, "circle")?,
        args.f64("tolerance")?,
    )
    .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![point_list(&left), point_list(&right)]))
}

pub(super) fn split_path_inout(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let (inner, outer) = bezier::split_path_inout(
        &path,
        circle_arg(args, "circle")?,
        args.f64("tolerance")?,
        args.bool("reorder_inout")?,
    )
    .map_err(ProbeError::rejected)?;
    Ok(Json::Array(vec![
        encode::path(&inner),
        encode::path(&outer),
    ]))
}

fn quad_pair((left, right): ([Point; 3], [Point; 3])) -> Json {
    Json::Array(vec![point_list(&left), point_list(&right)])
}

pub(super) fn get_parallels(args: Args<'_>) -> Outcome {
    Ok(quad_pair(bezier::get_parallels(
        quad_arg(args, "bezier2")?,
        args.f64("width")?,
    )))
}

pub(super) fn make_wedged_bezier2(args: Args<'_>) -> Outcome {
    Ok(quad_pair(bezier::make_wedged_bezier2(
        quad_arg(args, "bezier2")?,
        args.f64("width")?,
        args.f64("w1")?,
        args.f64("wm")?,
        args.f64("w2")?,
    )))
}

pub(super) fn get_cos_sin(args: Args<'_>) -> Outcome {
    let d = bezier::get_cos_sin(
        Point::new(args.f64("x0")?, args.f64("y0")?),
        Point::new(args.f64("x1")?, args.f64("y1")?),
    );
    Ok(Json::from(vec![d.cos, d.sin]))
}

pub(super) fn check_if_parallel(args: Args<'_>) -> Outcome {
    let result = bezier::check_if_parallel(
        Point::new(args.f64("dx1")?, args.f64("dy1")?),
        Point::new(args.f64("dx2")?, args.f64("dy2")?),
        args.f64("tolerance")?,
    );
    Ok(match result {
        Some(Parallel::Same) => Json::from(1.0),
        Some(Parallel::Opposite) => Json::from(-1.0),
        None => Json::from(false),
    })
}

pub(super) fn inside_circle(args: Args<'_>) -> Outcome {
    let inside =
        bezier::inside_circle(Point::new(args.f64("cx")?, args.f64("cy")?), args.f64("r")?);
    Ok(inside(point_arg(args, "xy")?).into())
}

fn direction(args: Args<'_>, cos: &str, sin: &str) -> Result<Direction, ProbeError> {
    Ok(Direction {
        cos: args.f64(cos)?,
        sin: args.f64(sin)?,
    })
}

pub(super) fn get_intersection(args: Args<'_>) -> Outcome {
    let p = bezier::get_intersection(
        Point::new(args.f64("cx1")?, args.f64("cy1")?),
        direction(args, "cos_t1", "sin_t1")?,
        Point::new(args.f64("cx2")?, args.f64("cy2")?),
        direction(args, "cos_t2", "sin_t2")?,
    )
    .map_err(ProbeError::rejected)?;
    Ok(encode::point(p))
}

pub(super) fn get_normal_points(args: Args<'_>) -> Outcome {
    let (a, b) = bezier::get_normal_points(
        Point::new(args.f64("cx")?, args.f64("cy")?),
        direction(args, "cos_t", "sin_t")?,
        args.f64("length")?,
    );
    Ok(Json::from(vec![a.x, a.y, b.x, b.y]))
}

pub(super) fn find_control_points(args: Args<'_>) -> Outcome {
    let points = bezier::find_control_points(
        Point::new(args.f64("c1x")?, args.f64("c1y")?),
        Point::new(args.f64("mmx")?, args.f64("mmy")?),
        Point::new(args.f64("c2x")?, args.f64("c2y")?),
    );
    Ok(point_list(&points))
}
