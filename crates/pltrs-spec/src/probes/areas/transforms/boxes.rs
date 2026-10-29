//! `Bbox` construction, properties and predicates.

use pltrs::transforms::{Bbox, Point, TrackedBbox};

use super::{bbox_arg, bbox_list_arg, points_arg};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `Bbox(points)`; any shape other than `(2, 2)` raises `ValueError`.
pub(super) fn new(args: Args<'_>) -> Outcome {
    let bbox = encode::parse_bbox(args.json("points")?).map_err(|_| {
        ProbeError::rejected("Bbox points must be of the form [[x0, y0], [x1, y1]]")
    })?;
    Ok(encode::bbox(&bbox))
}

/// `Bbox.from_bounds(x0, y0, width, height)`.
pub(super) fn from_bounds(args: Args<'_>) -> Outcome {
    let [x0, y0, width, height] = ["x0", "y0", "width", "height"].map(|key| args.f64(key));
    Ok(encode::bbox(&Bbox::from_bounds(x0?, y0?, width?, height?)))
}

/// `Bbox.from_extents(*extents, minpos=minpos)` -> `{"points", "minpos"}`;
/// a count other than four raises `ValueError`.
pub(super) fn from_extents(args: Args<'_>) -> Outcome {
    let extents = args.f64_vec("extents")?;
    let &[x0, y0, x1, y1] = extents.as_slice() else {
        return Err(ProbeError::rejected("from_extents takes four extents"));
    };
    let mut bbox = TrackedBbox::new(Bbox::from_extents(x0, y0, x1, y1));
    if let Some(minpos) = args.opt_f64("minpos")? {
        bbox.set_minpos(Point::new(minpos, minpos));
    }
    Ok(encode::object([
        ("points", encode::bbox(&bbox.bbox())),
        ("minpos", encode::point(bbox.minpos())),
    ]))
}

/// `Bbox.null()`.
pub(super) fn null(_: Args<'_>) -> Outcome {
    Ok(encode::bbox(&Bbox::NULL))
}

/// `Bbox.unit()`.
pub(super) fn unit(_: Args<'_>) -> Outcome {
    Ok(encode::bbox(&Bbox::UNIT))
}

/// Every scalar and pair property of a fresh `Bbox(bbox)`.
pub(super) fn properties(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    let (x0, y0, x1, y1) = b.extents();
    let (width, height) = (b.width(), b.height());
    let (xmin, xmax, ymin, ymax) = (b.xmin(), b.xmax(), b.ymin(), b.ymax());
    let pair = |a: f64, b: f64| Json::from(vec![a, b]);
    Ok(encode::object([
        ("x0", Json::from(x0)),
        ("y0", Json::from(y0)),
        ("x1", Json::from(x1)),
        ("y1", Json::from(y1)),
        ("xmin", Json::from(xmin)),
        ("xmax", Json::from(xmax)),
        ("ymin", Json::from(ymin)),
        ("ymax", Json::from(ymax)),
        ("width", Json::from(width)),
        ("height", Json::from(height)),
        ("size", pair(width, height)),
        ("bounds", Json::from(vec![x0, y0, width, height])),
        ("extents", Json::from(vec![x0, y0, x1, y1])),
        ("min", pair(xmin, ymin)),
        ("max", pair(xmax, ymax)),
        ("p0", encode::point(b.p0)),
        ("p1", encode::point(b.p1)),
        ("intervalx", pair(x0, x1)),
        ("intervaly", pair(y0, y1)),
        ("minpos", encode::point(TrackedBbox::new(b).minpos())),
    ]))
}

/// The private `Bbox._is_finite`.
pub(crate) fn is_finite(args: Args<'_>) -> Outcome {
    Ok(Json::from(bbox_arg(args, "bbox")?.has_finite_extent()))
}

/// `containsx(x)`.
pub(super) fn contains_x(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.contains_x(args.f64("x")?),
    ))
}

/// `containsy(y)`.
pub(super) fn contains_y(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.contains_y(args.f64("y")?),
    ))
}

/// `contains(x, y)`.
pub(super) fn contains(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.contains(point_args(args)?),
    ))
}

/// `fully_containsx(x)`.
pub(super) fn fully_contains_x(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.fully_contains_x(args.f64("x")?),
    ))
}

/// `fully_containsy(y)`.
pub(super) fn fully_contains_y(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.fully_contains_y(args.f64("y")?),
    ))
}

/// `fully_contains(x, y)`.
pub(super) fn fully_contains(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.fully_contains(point_args(args)?),
    ))
}

/// `overlaps(other)`.
pub(crate) fn overlaps(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.overlaps(&bbox_arg(args, "other")?),
    ))
}

/// `fully_overlaps(other)`.
pub(super) fn fully_overlaps(args: Args<'_>) -> Outcome {
    Ok(Json::from(
        bbox_arg(args, "bbox")?.fully_overlaps(&bbox_arg(args, "other")?),
    ))
}

/// `count_contains(vertices)`.
pub(super) fn count_contains(args: Args<'_>) -> Outcome {
    let count = bbox_arg(args, "bbox")?.count_contains(&points_arg(args, "vertices")?);
    Ok(Json::Integer(count as i64))
}

/// `count_overlaps(bboxes)`.
pub(crate) fn count_overlaps(args: Args<'_>) -> Outcome {
    let count = bbox_arg(args, "bbox")?.count_overlaps(&bbox_list_arg(args, "bboxes")?);
    Ok(Json::Integer(count as i64))
}

/// The `(x, y)` arguments as a point.
fn point_args(args: Args<'_>) -> Result<Point, ProbeError> {
    Ok(Point::new(args.f64("x")?, args.f64("y")?))
}
