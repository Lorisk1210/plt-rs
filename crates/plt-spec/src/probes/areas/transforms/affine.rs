//! `Affine2D` construction, application, inversion and composition.

use plt::transforms::{Affine2D, Point};

use super::{affine_arg, points_arg, transformed_arg};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `get_matrix` after applying `ops`.
pub(crate) fn get_matrix(args: Args<'_>) -> Outcome {
    Ok(encode::matrix(&affine_arg(args, "ops")?))
}

/// `Affine2D.from_values(a, b, c, d, e, f)`.
pub(super) fn from_values(args: Args<'_>) -> Outcome {
    let [a, b, c, d, e, f] = ["a", "b", "c", "d", "e", "f"].map(|key| args.f64(key));
    Ok(encode::matrix(&Affine2D::from_values(
        a?, b?, c?, d?, e?, f?,
    )))
}

/// `to_values()`: `[a, b, c, d, e, f]`.
pub(super) fn to_values(args: Args<'_>) -> Outcome {
    Ok(Json::from(affine_arg(args, "ops")?.values().to_vec()))
}

/// `transform(points)`: an `(N, 2)` array, or a single `[x, y]` point
/// (1-D in, 1-D out). Rows of another length raise `ValueError`.
pub(crate) fn transform(args: Args<'_>) -> Outcome {
    let t = affine_arg(args, "ops")?;
    let raw = args.json("points")?;
    let rows = raw
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("points is not an array: {raw}")))?;
    if let Some(single) = raw.as_f64_vec().filter(|v| !v.is_empty()) {
        return Ok(encode::point(t.transform_point(point_of(&single)?)));
    }
    let mapped = rows
        .iter()
        .map(|row| {
            let xy = row
                .as_f64_vec()
                .ok_or_else(|| ProbeError::BadArgs(format!("bad point {row}")))?;
            Ok(t.transform_point(point_of(&xy)?))
        })
        .collect::<Result<Vec<_>, ProbeError>>()?;
    Ok(encode::points(&mapped))
}

/// `transform_point(point)`; `len(point) != 2` raises `ValueError`.
pub(super) fn transform_point(args: Args<'_>) -> Outcome {
    let t = affine_arg(args, "ops")?;
    Ok(encode::point(
        t.transform_point(point_of(&args.f64_vec("point")?)?),
    ))
}

/// `is_separable`: no rotation or shear, compared exactly.
pub(super) fn is_separable(args: Args<'_>) -> Outcome {
    Ok(Json::from(affine_arg(args, "ops")?.is_separable()))
}

/// `inverted().get_matrix()`; singular matrices raise `LinAlgError`.
pub(super) fn inverted(args: Args<'_>) -> Outcome {
    let inverse = affine_arg(args, "ops")?
        .inverted()
        .ok_or_else(|| ProbeError::rejected("Singular matrix"))?;
    Ok(encode::matrix(&inverse))
}

/// `T0 + T1 + ...` for the op lists in `chain` ("T0 first"), applied to
/// `points` in one affine step.
pub(crate) fn add(args: Args<'_>) -> Outcome {
    let chain = args.json("chain")?;
    let total = chain
        .as_array()
        .ok_or_else(|| ProbeError::BadArgs(format!("chain is not a list: {chain}")))?
        .iter()
        .map(encode::parse_affine_ops)
        .reduce(|total, next| Ok(total?.then(next?)))
        .ok_or_else(|| ProbeError::BadArgs("empty chain".into()))??;
    Ok(encode::object([
        ("matrix", encode::matrix(&total)),
        ("transformed", transformed_arg(&total, args)?),
        ("is_separable", Json::from(total.is_separable())),
    ]))
}

/// `frozen()` taken after `ops`, and the original after `post_ops`.
pub(super) fn frozen(args: Args<'_>) -> Outcome {
    // `Affine2D` is a value, so the frozen copy cannot see later changes.
    let frozen = affine_arg(args, "ops")?;
    let original = encode::apply_affine_ops(frozen, args.json("post_ops")?)?;
    Ok(encode::object([
        ("frozen", encode::matrix(&frozen)),
        ("original", encode::matrix(&original)),
    ]))
}

/// `IdentityTransform().transform(points)`: unlike an identity `Affine2D`
/// it performs no arithmetic, so non-finite values pass through untouched.
pub(super) fn identity(args: Args<'_>) -> Outcome {
    Ok(encode::points(&points_arg(args, "points")?))
}

/// A point from exactly two coordinates (matplotlib raises `ValueError`
/// for any other length).
fn point_of(xy: &[f64]) -> Result<Point, ProbeError> {
    match xy {
        [x, y] => Ok(Point::new(*x, *y)),
        _ => Err(ProbeError::rejected(format!(
            "expected 2 coordinates, got {}",
            xy.len()
        ))),
    }
}
