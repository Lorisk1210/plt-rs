//! Probe operations for the `path` area (`matplotlib.path`,
//! `matplotlib.bezier`, `matplotlib.hatch`).

mod bezier;

pub(crate) use bezier::split_bezier;

use pltrs_core::path::convert::{CleanupOptions, SimplifySettings, SnapMode};
use pltrs_core::path::{CollectionGeometry, PolygonOptions};
use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point, hatch};

use crate::json::Json;
use crate::probes::encode;
use crate::probes::{Args, Op, Outcome, ProbeError};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("path.Path", new_path),
    Op::new("path.Path.unit_rectangle", |_| {
        Ok(encode::path(&Path::unit_rectangle()))
    }),
    Op::new("path.Path.unit_circle", |_| {
        Ok(encode::path(&Path::unit_circle()))
    }),
    Op::new("path.Path.unit_circle_righthalf", |_| {
        Ok(encode::path(&Path::unit_circle_righthalf()))
    }),
    Op::new("path.Path.circle", circle),
    // pltrs paths are plain values: none is a read-only shared instance.
    Op::new("path.Path.readonly", |_| Ok(false.into())),
    Op::new("path.Path.arc", arc),
    Op::new("path.Path.wedge", wedge),
    Op::new("path.Path.unit_regular_polygon", regular_polygon),
    Op::new("path.Path.unit_regular_star", regular_star),
    Op::new("path.Path.unit_regular_asterisk", regular_asterisk),
    Op::new("path.Path.make_compound_path", compound),
    Op::new(
        "path.Path.make_compound_path_from_polys",
        compound_from_polys,
    ),
    Op::new("path.Path.hatch", hatch_path),
    Op::new("path.Path.hatch.get_extents", hatch_extents),
    Op::new("path.Path.contains_point", contains_point),
    Op::new("path.Path.contains_points", contains_points),
    Op::new("path.Path.contains_path", contains_path),
    Op::new("path.Path.get_extents", get_extents),
    Op::new("path.Path.intersects_path", intersects_path),
    Op::new("path.Path.intersects_bbox", intersects_bbox),
    Op::new("path.Path.should_simplify", should_simplify),
    Op::new("path.Path.simplify_threshold", simplify_threshold),
    Op::new("path.Path.clip_to_bbox", clip_to_bbox),
    Op::new("path.Path.interpolated", interpolated),
    Op::new("path.Path.to_polygons", to_polygons),
    Op::new("path.Path.cleaned", cleaned),
    Op::new("path.Path.iter_segments", iter_segments),
    Op::new("path.Path.iter_bezier", iter_bezier),
    Op::new("path.Path.transformed", transformed),
    Op::new("path.get_path_collection_extents", collection_extents),
    Op::new("bezier.BezierSegment.__call__", bezier::bezier_call),
    Op::new("bezier.BezierSegment.point_at_t", bezier::bezier_point_at_t),
    Op::new("bezier.BezierSegment.properties", bezier::bezier_properties),
    Op::new(
        "bezier.BezierSegment.axis_aligned_extrema",
        bezier::bezier_extrema,
    ),
    Op::new("bezier.split_de_casteljau", bezier::split_de_casteljau),
    Op::new(
        "bezier.find_bezier_t_intersecting_with_closedpath",
        bezier::find_t,
    ),
    Op::new(
        "bezier.split_bezier_intersecting_with_closedpath",
        bezier::split_bezier,
    ),
    Op::new("bezier.split_path_inout", bezier::split_path_inout),
    Op::new("bezier.get_parallels", bezier::get_parallels),
    Op::new("bezier.make_wedged_bezier2", bezier::make_wedged_bezier2),
    Op::new("bezier.get_cos_sin", bezier::get_cos_sin),
    Op::new("bezier.check_if_parallel", bezier::check_if_parallel),
    Op::new("bezier.inside_circle", bezier::inside_circle),
    Op::new("bezier.get_intersection", bezier::get_intersection),
    Op::new("bezier.get_normal_points", bezier::get_normal_points),
    Op::new("bezier.find_control_points", bezier::find_control_points),
];

// ---------------------------------------------------------------------------
// argument decoding

pub(crate) fn path_arg(args: Args<'_>, key: &str) -> Result<Path, ProbeError> {
    encode::parse_mpl_path(args.json(key)?)
}

/// `null` (no transform) or a TRANSFORM OP LIST.
fn transform_arg(args: Args<'_>, key: &str) -> Result<Option<Affine2D>, ProbeError> {
    args.get(key).map(encode::parse_affine_ops).transpose()
}

fn point_arg(args: Args<'_>, key: &str) -> Result<Point, ProbeError> {
    match args.f64_vec(key)?.as_slice() {
        [x, y] => Ok(Point::new(*x, *y)),
        _ => Err(ProbeError::BadArgs(format!("{key} is not a point"))),
    }
}

/// `np.asarray(points, float).reshape(-1, 2)`.
fn points_arg(args: Args<'_>, key: &str) -> Result<Vec<Point>, ProbeError> {
    let value = args.json(key)?;
    if value.as_array().is_some_and(<[Json]>::is_empty) {
        return Ok(Vec::new());
    }
    encode::parse_points(value)
}

/// The `path.simplify*` rcParams of a probe (the reference ones by default).
fn simplify_settings(args: Args<'_>) -> SimplifySettings {
    let rc = args.get("rc");
    let lookup = |key: &str| rc.and_then(|rc| rc.get(key));
    let defaults = SimplifySettings::default();
    SimplifySettings {
        enabled: lookup("path.simplify")
            .and_then(Json::as_bool)
            .unwrap_or(defaults.enabled),
        threshold: lookup("path.simplify_threshold")
            .and_then(Json::as_f64)
            .unwrap_or(defaults.threshold),
    }
}

pub(crate) fn point_list(points: &[Point]) -> Json {
    Json::Array(points.iter().map(|p| encode::point(*p)).collect())
}

fn flags(values: Vec<bool>) -> Json {
    Json::Array(values.into_iter().map(Json::from).collect())
}

/// A path dict with matplotlib's trailing `STOP` vertex (as `cleaned`
/// returns it).
fn path_with_stop(path: &Path) -> Json {
    let mut vertices: Vec<Json> = path.vertices().iter().map(|p| encode::point(*p)).collect();
    vertices.push(encode::point(Point::ORIGIN));
    let mut codes: Vec<Json> = path
        .resolved_codes()
        .iter()
        .map(|c| Json::from(f64::from(*c as u8)))
        .collect();
    codes.push(Json::from(0.0));
    Json::Object(vec![
        ("vertices".into(), Json::Array(vertices)),
        ("codes".into(), Json::Array(codes)),
    ])
}

// ---------------------------------------------------------------------------
// constructors

fn new_path(args: Args<'_>) -> Outcome {
    let vertices = encode::parse_points(args.json("vertices")?)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ProbeError::rejected("vertices must be an (N, 2) array"))?;
    let codes = args
        .get("codes")
        .map(|c| {
            c.as_f64_vec()
                .ok_or_else(|| ProbeError::rejected("codes must be 1-D"))
        })
        .transpose()?;
    let path = match codes {
        Some(codes) => {
            let codes = codes
                .into_iter()
                .map(|c| PathCode::try_from(c as u8).map_err(ProbeError::rejected))
                .collect::<Result<Vec<_>, _>>()?;
            Path::new(vertices, codes).map_err(ProbeError::rejected)?
        }
        None if args.bool("closed")? => Path::closed(vertices),
        None => Path::polyline(vertices),
    };
    Ok(encode::path(&path))
}

fn circle(args: Args<'_>) -> Outcome {
    let path = Path::circle(point_arg(args, "center")?, args.f64("radius")?);
    Ok(encode::path(&path))
}

/// The segment count: `None` for automatic; matplotlib rejects `n < 1`
/// and fails to size an automatic arc with a non-finite span.
fn arc_segments(args: Args<'_>, theta1: f64, theta2: f64) -> Result<Option<usize>, ProbeError> {
    match args.opt_i64("n")? {
        Some(n) if n < 1 => Err(ProbeError::rejected("n must be >= 1 or None")),
        Some(n) => Ok(Some(n as usize)),
        None if !(theta2 - theta1).is_finite() => Err(ProbeError::rejected("non-finite arc span")),
        None => Ok(None),
    }
}

pub(crate) fn arc(args: Args<'_>) -> Outcome {
    let (t1, t2) = (args.f64("theta1")?, args.f64("theta2")?);
    let n = arc_segments(args, t1, t2)?;
    let path = if args.bool("is_wedge")? {
        Path::wedge(t1, t2, n)
    } else {
        Path::arc(t1, t2, n)
    };
    Ok(encode::path(&path))
}

fn wedge(args: Args<'_>) -> Outcome {
    let (t1, t2) = (args.f64("theta1")?, args.f64("theta2")?);
    let n = arc_segments(args, t1, t2)?;
    Ok(encode::path(&Path::wedge(t1, t2, n)))
}

fn side_count(args: Args<'_>) -> Result<usize, ProbeError> {
    usize::try_from(args.i64("numVertices")?)
        .ok()
        .filter(|n| *n > 0)
        .ok_or_else(|| ProbeError::rejected("pltrs requires a positive vertex count"))
}

fn regular_polygon(args: Args<'_>) -> Outcome {
    Ok(encode::path(&Path::unit_regular_polygon(side_count(args)?)))
}

fn regular_star(args: Args<'_>) -> Outcome {
    let inner = args.f64("innerCircle")?;
    Ok(encode::path(&Path::unit_regular_star(
        side_count(args)?,
        inner,
    )))
}

fn regular_asterisk(args: Args<'_>) -> Outcome {
    Ok(encode::path(&Path::unit_regular_asterisk(side_count(
        args,
    )?)))
}

pub(crate) fn compound(args: Args<'_>) -> Outcome {
    let paths = args
        .json("paths")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(encode::parse_mpl_path)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(encode::path(&Path::compound(&paths)))
}

fn compound_from_polys(args: Args<'_>) -> Outcome {
    let polygons = args
        .json("XY")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|poly| {
            encode::parse_points(poly).map_err(|_| ProbeError::rejected("XY must be (N, M, 2)"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(encode::path(&Path::compound_from_polygons(&polygons)))
}

fn hatch_of(args: Args<'_>) -> Result<Option<Path>, ProbeError> {
    let Some(pattern) = args.opt_str("hatchpattern")? else {
        return Ok(None);
    };
    let density = args.f64("density")?.trunc() as i32;
    hatch::pattern_path(pattern, density)
        .map(Some)
        .map_err(ProbeError::rejected)
}

fn hatch_path(args: Args<'_>) -> Outcome {
    Ok(hatch_of(args)?.map_or(Json::Null, |p| encode::path(&p)))
}

fn hatch_extents(args: Args<'_>) -> Outcome {
    let path = hatch_of(args)?.unwrap_or_default();
    Ok(encode::bbox(&path.extents()))
}

// ---------------------------------------------------------------------------
// queries

fn contains_point(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let t = transform_arg(args, "transform")?;
    let inside = path.contains_point(point_arg(args, "point")?, t.as_ref(), args.f64("radius")?);
    Ok(inside.into())
}

pub(crate) fn contains_points(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let t = transform_arg(args, "transform")?;
    let points = points_arg(args, "points")?;
    Ok(flags(path.contains_points(
        &points,
        t.as_ref(),
        args.f64("radius")?,
    )))
}

pub(crate) fn contains_path(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let t = transform_arg(args, "transform")?;
    Ok(path
        .contains_path(&path_arg(args, "other")?, t.as_ref())
        .into())
}

pub(crate) fn get_extents(args: Args<'_>) -> Outcome {
    let mut path = path_arg(args, "path")?;
    if let Some(t) = transform_arg(args, "transform")? {
        path.transform(&t);
    }
    Ok(encode::bbox(&path.extents()))
}

pub(crate) fn intersects_path(args: Args<'_>) -> Outcome {
    let (a, b) = (path_arg(args, "path")?, path_arg(args, "other")?);
    Ok(a.intersects_path(&b, args.bool("filled")?).into())
}

fn intersects_bbox(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let bbox = encode::parse_bbox(args.json("bbox")?)?;
    Ok(path.intersects_bbox(&bbox, args.bool("filled")?).into())
}

fn should_simplify(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    Ok(simplify_settings(args).applies_to(&path).into())
}

fn simplify_threshold(args: Args<'_>) -> Outcome {
    Ok(simplify_settings(args).threshold.into())
}

// ---------------------------------------------------------------------------
// conversions

fn clip_to_bbox(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let bbox = encode::parse_bbox(args.json("bbox")?)?;
    Ok(encode::path(
        &path.clip_to_bbox(&bbox, args.bool("inside")?),
    ))
}

pub(crate) fn interpolated(args: Args<'_>) -> Outcome {
    let steps = usize::try_from(args.i64("steps")?)
        .ok()
        .filter(|s| *s > 0)
        .ok_or_else(|| ProbeError::rejected("steps must be positive"))?;
    Ok(encode::path(&path_arg(args, "path")?.interpolated(steps)))
}

pub(crate) fn to_polygons(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let options = PolygonOptions {
        transform: transform_arg(args, "transform")?,
        width: args.f64("width")?,
        height: args.f64("height")?,
        closed_only: args.bool("closed_only")?,
        simplify: simplify_settings(args),
    };
    let polygons = path.to_polygons(&options);
    Ok(Json::Array(
        polygons.iter().map(|p| point_list(p)).collect(),
    ))
}

/// The common arguments of `cleaned` and `iter_segments`.
fn cleanup_options(
    args: Args<'_>,
    path: &Path,
    simplify: Option<bool>,
) -> Result<CleanupOptions, ProbeError> {
    let settings = simplify_settings(args);
    let clip = args
        .get("clip")
        .map(|c| match c.as_f64_vec().as_deref() {
            Some([x0, y0, x1, y1]) => Ok(Bbox::from_extents(*x0, *y0, *x1, *y1)),
            _ => Err(ProbeError::BadArgs(format!("bad clip {c}"))),
        })
        .transpose()?;
    Ok(CleanupOptions {
        transform: transform_arg(args, "transform")?.unwrap_or(Affine2D::IDENTITY),
        remove_nans: args.bool("remove_nans")?,
        clip,
        snap: SnapMode::from_option(args.opt_bool("snap")?),
        stroke_width: args.f64("stroke_width")?,
        simplify: match simplify {
            Some(true) => Some(settings.threshold),
            Some(false) => None,
            None => settings.for_path(path),
        },
        curves: args.bool("curves")?,
        sketch: None,
    })
}

pub(crate) fn cleaned(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let options = cleanup_options(args, &path, Some(args.bool("simplify")?))?;
    Ok(path_with_stop(&path.cleaned(&options)))
}

/// The cleaned path's commands as `[flat_vertices, code]` (closures keep
/// their stored point, as in matplotlib).
pub(crate) fn iter_segments(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let options = cleanup_options(args, &path, args.opt_bool("simplify")?)?;
    let cleaned = path.cleaned(&options);
    let mut items = Vec::new();
    let mut vertices = cleaned.vertices().iter().zip(cleaned.resolved_codes());
    while let Some((&p, code)) = vertices.next() {
        let mut flat = vec![p.x, p.y];
        for (q, _) in vertices.by_ref().take(code.vertex_count() - 1) {
            flat.extend([q.x, q.y]);
        }
        items.push(Json::Array(vec![
            Json::from(flat),
            Json::from(f64::from(code as u8)),
        ]));
    }
    Ok(Json::Array(items))
}

fn iter_bezier(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let options = CleanupOptions::segment_defaults(&path, &SimplifySettings::default());
    let pieces = path
        .iter_bezier(&options)
        .into_iter()
        .map(|(points, code)| {
            Json::Array(vec![point_list(&points), Json::from(f64::from(code as u8))])
        })
        .collect();
    Ok(Json::Array(pieces))
}

fn transformed(args: Args<'_>) -> Outcome {
    let path = path_arg(args, "path")?;
    let t = transform_arg(args, "transform")?.unwrap_or(Affine2D::IDENTITY);
    Ok(encode::path(&path.transformed(&t)))
}

/// A list of 3×3 matrices `[[a, c, e], [b, d, f], [0, 0, 1]]`.
fn matrices_arg(args: Args<'_>, key: &str) -> Result<Vec<Affine2D>, ProbeError> {
    let matrix = |m: &Json| -> Option<Affine2D> {
        let rows: Vec<Vec<f64>> = m
            .as_array()?
            .iter()
            .map(Json::as_f64_vec)
            .collect::<Option<_>>()?;
        match rows.as_slice() {
            [r0, r1, _] if r0.len() == 3 && r1.len() == 3 => Some(Affine2D::from_values(
                r0[0], r1[0], r0[1], r1[1], r0[2], r1[2],
            )),
            _ => None,
        }
    };
    args.json(key)?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(|m| matrix(m).ok_or_else(|| ProbeError::BadArgs(format!("bad matrix {m}"))))
        .collect()
}

fn collection_extents(args: Args<'_>) -> Outcome {
    let paths = args
        .json("paths")?
        .as_array()
        .unwrap_or(&[])
        .iter()
        .map(encode::parse_mpl_path)
        .collect::<Result<Vec<_>, _>>()?;
    let transforms = matrices_arg(args, "transforms")?;
    let offsets = points_arg(args, "offsets")?;
    let geometry = CollectionGeometry {
        master_transform: transform_arg(args, "master_transform")?.unwrap_or(Affine2D::IDENTITY),
        paths: &paths,
        transforms: &transforms,
        offsets: &offsets,
        offset_transform: transform_arg(args, "offset_transform")?.unwrap_or(Affine2D::IDENTITY),
    };
    let limits = geometry.extents().map_err(ProbeError::rejected)?;
    Ok(Json::Object(vec![
        ("bbox".into(), encode::bbox(&limits.bbox)),
        ("minpos".into(), encode::point(limits.minpos)),
    ]))
}
