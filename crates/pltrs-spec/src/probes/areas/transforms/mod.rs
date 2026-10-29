//! Probe operations for the `transforms` area.
//!
//! Affine transforms are built from the spec's TRANSFORM OP LISTs (see
//! [`encode::apply_affine_ops`]), optionally starting from an explicit
//! `matrix`; boxes are `[[x0, y0], [x1, y1]]` and are never reordered.
//! Transform trees name the coordinate systems of a figure with two axes.
//! The arithmetic itself lives in `pltrs_core` (`Affine2D`, `Bbox`) and
//! `pltrs::transforms`; this module only decodes arguments and encodes
//! results.

mod affine;
mod boxes;
mod derived;
mod maps;
mod scalar;
mod trees;
mod update;

use pltrs::transforms::{Affine2D, Bbox, Point};

pub(crate) use affine::{add, get_matrix, transform};
pub(crate) use boxes::{count_overlaps, is_finite, overlaps};
pub(crate) use derived::intersection;
pub(crate) use scalar::{interval_contains, interval_contains_open, nonsingular};

use crate::probes::{Args, Op, Outcome, ProbeError, encode};

/// Operations implemented for this area.
pub(crate) const OPS: &[Op] = &[
    Op::new("transforms.Affine2D.get_matrix", affine::get_matrix),
    Op::new("transforms.Affine2D.from_values", affine::from_values),
    Op::new("transforms.Affine2D.to_values", affine::to_values),
    Op::new("transforms.Affine2D.transform", affine::transform),
    Op::new(
        "transforms.Affine2D.transform_point",
        affine::transform_point,
    ),
    Op::new("transforms.Affine2D.is_separable", affine::is_separable),
    Op::new("transforms.Affine2D.inverted", affine::inverted),
    Op::new("transforms.Affine2D.__add__", affine::add),
    Op::new("transforms.Affine2D.frozen", affine::frozen),
    Op::new("transforms.IdentityTransform.transform", affine::identity),
    Op::new("transforms.Bbox.new", boxes::new),
    Op::new("transforms.Bbox.from_bounds", boxes::from_bounds),
    Op::new("transforms.Bbox.from_extents", boxes::from_extents),
    Op::new("transforms.Bbox.null", boxes::null),
    Op::new("transforms.Bbox.unit", boxes::unit),
    Op::new("transforms.Bbox.properties", boxes::properties),
    Op::new("transforms.Bbox._is_finite", boxes::is_finite),
    Op::new("transforms.Bbox.containsx", boxes::contains_x),
    Op::new("transforms.Bbox.containsy", boxes::contains_y),
    Op::new("transforms.Bbox.contains", boxes::contains),
    Op::new("transforms.Bbox.fully_containsx", boxes::fully_contains_x),
    Op::new("transforms.Bbox.fully_containsy", boxes::fully_contains_y),
    Op::new("transforms.Bbox.fully_contains", boxes::fully_contains),
    Op::new("transforms.Bbox.overlaps", boxes::overlaps),
    Op::new("transforms.Bbox.fully_overlaps", boxes::fully_overlaps),
    Op::new("transforms.Bbox.count_contains", boxes::count_contains),
    Op::new("transforms.Bbox.count_overlaps", boxes::count_overlaps),
    Op::new("transforms.Bbox.union", derived::union),
    Op::new("transforms.Bbox.intersection", derived::intersection),
    Op::new("transforms.Bbox.padded", derived::padded),
    Op::new("transforms.Bbox.expanded", derived::expanded),
    Op::new("transforms.Bbox.shrunk", derived::shrunk),
    Op::new(
        "transforms.Bbox.shrunk_to_aspect",
        derived::shrunk_to_aspect,
    ),
    Op::new("transforms.Bbox.anchored", derived::anchored),
    Op::new("transforms.Bbox.splitx", derived::split_x),
    Op::new("transforms.Bbox.splity", derived::split_y),
    Op::new("transforms.Bbox.translated", derived::translated),
    Op::new("transforms.Bbox.transformed", derived::transformed),
    Op::new("transforms.Bbox.rotated", derived::rotated),
    Op::new("transforms.Bbox.corners", derived::corners),
    Op::new("transforms.TransformedBbox", derived::transformed_bbox),
    Op::new("transforms.Bbox.update", update::update),
    Op::new("transforms.Bbox.frozen", update::frozen),
    Op::new("transforms.BboxTransform", maps::bbox_transform),
    Op::new("transforms.BboxTransformTo", maps::bbox_transform_to),
    Op::new("transforms.BboxTransformFrom", maps::bbox_transform_from),
    Op::new("transforms.blended_transform_factory", maps::blended),
    Op::new("transforms.ScaledTranslation", maps::scaled_translation),
    Op::new("transforms.offset_copy", maps::offset_copy),
    Op::new(
        "transforms.Transform.contains_branch",
        trees::contains_branch,
    ),
    Op::new(
        "transforms.Transform.contains_branch_separately",
        trees::contains_branch_separately,
    ),
    Op::new("transforms.Transform.transform_bbox", trees::transform_bbox),
    Op::new("transforms.nonsingular", scalar::nonsingular),
    Op::new("transforms.interval_contains", scalar::interval_contains),
    Op::new(
        "transforms.interval_contains_open",
        scalar::interval_contains_open,
    ),
    Op::new(
        "transforms.interval_contains_close",
        scalar::interval_contains_close,
    ),
];

/// The affine transform described by the op list under `key`, applied to
/// the optional `matrix` argument (identity when absent).
fn affine_arg(args: Args<'_>, key: &str) -> Result<Affine2D, ProbeError> {
    let start = args
        .get("matrix")
        .map_or(Ok(Affine2D::IDENTITY), encode::parse_matrix)?;
    encode::apply_affine_ops(start, args.json(key)?)
}

/// A box argument.
fn bbox_arg(args: Args<'_>, key: &str) -> Result<Bbox, ProbeError> {
    encode::parse_bbox(args.json(key)?)
}

/// A list of boxes.
fn bbox_list_arg(args: Args<'_>, key: &str) -> Result<Vec<Bbox>, ProbeError> {
    let value = args.json(key)?;
    value
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("{key} is not a list of boxes: {value}")))?
        .iter()
        .map(encode::parse_bbox)
        .collect()
}

/// An `[[x, y], ...]` argument.
fn points_arg(args: Args<'_>, key: &str) -> Result<Vec<Point>, ProbeError> {
    encode::parse_points(args.json(key)?)
}

/// `{"matrix", "transformed"}`: the matrix of `t` and the `points`
/// argument mapped through it.
fn matrix_and_points(t: &Affine2D, args: Args<'_>) -> Outcome {
    Ok(encode::object([
        ("matrix", encode::matrix(t)),
        ("transformed", transformed_arg(t, args)?),
    ]))
}

/// The `points` argument mapped through `t`.
fn transformed_arg(t: &Affine2D, args: Args<'_>) -> Outcome {
    let mapped: Vec<Point> = points_arg(args, "points")?
        .into_iter()
        .map(|p| t.transform_point(p))
        .collect();
    Ok(encode::points(&mapped))
}
