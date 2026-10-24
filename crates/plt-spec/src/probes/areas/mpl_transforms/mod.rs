//! Probe operations for the `mpl_transforms` area: affine transforms,
//! boxes and transform trees ported from matplotlib's
//! `tests/test_transforms.py` (see
//! `spec/reference/probes/test_mpl_transforms.py`).
//!
//! Most operations are the `transforms` area's; the transform trees of
//! `TestBasicTransform` ([`basic`]) and the data limits of artists placed
//! through transforms ([`limits`]) have modules of their own.

mod basic;
mod limits;

use std::rc::Rc;

use plt::figure::Figure;
use plt::scale::{LogTransform, NonPositive};
use plt::transforms::{
    Affine2D, AxisMap, Bbox, Point, PointMap, ResolvedTransform, Step, TrackedBbox, Transform,
};

use super::path::iter_segments;
use super::transforms;
use crate::json::Json;
use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("transforms.Affine2D.get_matrix", transforms::get_matrix),
    Op::new("transforms.Affine2D.transform", transforms::transform),
    Op::new("transforms.Affine2D.__add__", transforms::add),
    Op::new(
        "transforms.Affine2D.from_values.transform",
        from_values_transform,
    ),
    Op::new("transforms.Affine2D.transform_angles", transform_angles),
    Op::new(
        "transforms.AffineDeltaTransform.get_matrix",
        affine_delta_matrix,
    ),
    Op::new("transforms.Bbox._is_finite", transforms::is_finite),
    Op::new("transforms.Bbox.frozen", frozen),
    Op::new("transforms.Bbox.intersection", transforms::intersection),
    Op::new("transforms.Bbox.overlaps", transforms::overlaps),
    Op::new(
        "_path.count_bboxes_overlapping_bbox",
        transforms::count_overlaps,
    ),
    Op::new("transforms.TransformedBbox.contains", |args| {
        transformed_unit_box(args, Bbox::contains)
    }),
    Op::new("transforms.TransformedBbox.fully_contains", |args| {
        transformed_unit_box(args, Bbox::fully_contains)
    }),
    Op::new("transforms.nonsingular", transforms::nonsingular),
    Op::new(
        "transforms.interval_contains",
        transforms::interval_contains,
    ),
    Op::new(
        "transforms.interval_contains_open",
        transforms::interval_contains_open,
    ),
    Op::new(
        "transforms.BlendedGenericTransform.transform_path_non_affine",
        blended_log_path,
    ),
    Op::new("path.Path.iter_segments", iter_segments),
    Op::new("axes.Axes.transData.transform", trans_data_point),
    Op::new(
        "transforms.Transform.contains_branch",
        basic::contains_branch,
    ),
    Op::new(
        "transforms.Transform.contains_branch_separately",
        basic::contains_branch_separately,
    ),
    Op::new(
        "transforms.Transform.transform_non_affine",
        basic::transform_non_affine,
    ),
    Op::new("transforms.Transform.transform", basic::transform),
    Op::new("transforms.Transform.get_affine", basic::get_affine),
    Op::new("axes.Axes.plot.dataLim", limits::plot),
    Op::new("axes.Axes.add_patch.dataLim", limits::add_patch),
    Op::new("axes.Axes.contourf.dataLim", limits::contourf),
    Op::new("axes.Axes.pcolor.dataLim", limits::pcolor),
    Op::new("axes.Axes.pcolormesh.dataLim", limits::pcolormesh),
];

/// The upstream `NonAffineForTest`: a transform that does what an affine
/// map does while presenting itself as non-affine, so it stays a step of
/// its own instead of folding into the trailing affine.
#[derive(Debug)]
struct NonAffine(Affine2D);

impl PointMap for NonAffine {
    fn forward(&self, point: Point) -> Point {
        self.0.transform_point(point)
    }

    fn inverse(&self, point: Point) -> Point {
        self.0
            .inverted()
            .map_or(Point::new(f64::NAN, f64::NAN), |inverse| {
                inverse.transform_point(point)
            })
    }
}

/// Resolves a transform that refers to no figure artist.
fn resolve(transform: &Transform) -> Result<ResolvedTransform, ProbeError> {
    Figure::new()
        .resolve_transform(transform)
        .map_err(ProbeError::rejected)
}

/// The six values of `Affine2D.from_values` under `key`.
fn from_values_arg(args: Args<'_>, key: &str) -> Result<Affine2D, ProbeError> {
    match args.f64_vec(key)?[..] {
        [a, b, c, d, e, f] => Ok(Affine2D::from_values(a, b, c, d, e, f)),
        _ => Err(ProbeError::BadArgs(format!("{key} needs six values"))),
    }
}

/// `Affine2D.from_values(*values).transform(points)`.
fn from_values_transform(args: Args<'_>) -> Outcome {
    let affine = from_values_arg(args, "values")?;
    let points = encode::parse_points(args.json("points")?)?;
    let mapped: Vec<Point> = points.iter().map(|&p| affine.transform_point(p)).collect();
    Ok(encode::points(&mapped))
}

/// `transform_angles(angles, points)` in degrees.
fn transform_angles(args: Args<'_>) -> Outcome {
    let affine = encode::parse_affine_ops(args.json("ops")?)?;
    let resolved = ResolvedTransform::affine(affine);
    let points = encode::parse_points(args.json("points")?)?;
    let angles = args.f64_vec("angles")?;
    if angles.len() != points.len() {
        return Err(ProbeError::BadArgs("one angle per point is needed".into()));
    }
    Ok(Json::from(
        angles
            .iter()
            .zip(&points)
            .map(|(&angle, &point)| resolved.transform_angle(angle, point))
            .collect::<Vec<_>>(),
    ))
}

/// The matrix of `AffineDeltaTransform(Affine2D.from_values(*values))`.
fn affine_delta_matrix(args: Args<'_>) -> Outcome {
    let base = Transform::Affine(from_values_arg(args, "values")?);
    let delta = Transform::AffineDelta(Box::new(base));
    Ok(encode::matrix(&resolve(&delta)?.affine_part()))
}

/// `{points, minpos}` of `Bbox.from_extents(*extents,
/// minpos=minpos).frozen()`.
fn frozen(args: Args<'_>) -> Outcome {
    let [x0, y0, x1, y1] = args.f64_vec("extents")?[..] else {
        return Err(ProbeError::BadArgs("extents needs four values".into()));
    };
    let minpos = args.f64("minpos")?;
    let mut tracked = TrackedBbox::new(Bbox::from_extents(x0, y0, x1, y1));
    tracked.set_minpos(Point::new(minpos, minpos));
    let frozen = tracked.limits();
    Ok(encode::object([
        ("points", encode::bbox(&frozen.bbox)),
        ("minpos", encode::point(frozen.minpos)),
    ]))
}

/// A predicate of `TransformedBbox(Bbox.unit(), affine)` at `(x, y)`:
/// as in matplotlib, the point is mapped back by the inverse transform
/// and tested against the unit box, so a rotated box contains only the
/// points of its rotated outline.
fn transformed_unit_box(args: Args<'_>, predicate: fn(&Bbox, Point) -> bool) -> Outcome {
    let inverse = encode::parse_affine_ops(args.json("ops")?)?
        .inverted()
        .ok_or_else(|| ProbeError::rejected("the transform is singular"))?;
    let point = inverse.transform_point(Point::new(args.f64("x")?, args.f64("y")?));
    Ok(Json::from(predicate(&Bbox::UNIT, point)))
}

/// The path through `BlendedGenericTransform(Affine2D(), LogTransform(10,
/// 'clip'))`, non-affine part only.
fn blended_log_path(args: Args<'_>) -> Outcome {
    let path = encode::parse_mpl_path(args.json("path")?)?;
    let log = LogTransform::new(10.0, NonPositive::Clip).map_err(ProbeError::rejected)?;
    let y = ResolvedTransform::with_step(
        Step::Separable(None, Some(Rc::new(log) as Rc<dyn AxisMap>)),
        Affine2D::IDENTITY,
    );
    let blended = ResolvedTransform::blend(ResolvedTransform::identity(), y);
    let (mapped, _) = blended.split_path(&path);
    Ok(encode::path(&mapped))
}

/// `ax.transData.transform(point)` for `Figure().subplots()` after
/// `set_yscale(yscale)`.
fn trans_data_point(args: Args<'_>) -> Outcome {
    let figure = Figure::new();
    let ax = figure.subplot();
    ax.set_yscale(args.str("yscale")?)
        .map_err(ProbeError::rejected)?;
    let [x, y] = args.f64_vec("point")?[..] else {
        return Err(ProbeError::BadArgs("point needs two values".into()));
    };
    let data = figure
        .resolve_transform(&ax.trans_data())
        .map_err(ProbeError::rejected)?;
    Ok(encode::point(data.transform_point(Point::new(x, y))))
}
