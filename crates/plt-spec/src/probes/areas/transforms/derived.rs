//! Boxes derived from other boxes: unions, padding, anchoring, splitting
//! and transformation.

use plt::transforms::{Anchor, Bbox};

use super::{affine_arg, bbox_arg, bbox_list_arg};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `Bbox.union(bboxes)`; an empty list raises `ValueError`.
pub(super) fn union(args: Args<'_>) -> Outcome {
    let boxes = bbox_list_arg(args, "bboxes")?;
    let union =
        Bbox::union_all(&boxes).ok_or_else(|| ProbeError::rejected("'bboxes' cannot be empty"))?;
    Ok(encode::bbox(&union))
}

/// `Bbox.intersection(bbox1, bbox2)`: `null` for disjoint boxes.
pub(super) fn intersection(args: Args<'_>) -> Outcome {
    let (a, b) = (bbox_arg(args, "bbox1")?, bbox_arg(args, "bbox2")?);
    Ok(a.intersection(&b).map_or(Json::Null, |i| encode::bbox(&i)))
}

/// `padded(w_pad, h_pad)`; `h_pad` defaults to `w_pad`.
pub(super) fn padded(args: Args<'_>) -> Outcome {
    let w_pad = args.f64("w_pad")?;
    let h_pad = args.opt_f64("h_pad")?.unwrap_or(w_pad);
    Ok(encode::bbox(&bbox_arg(args, "bbox")?.padded(w_pad, h_pad)))
}

/// `expanded(sw, sh)`.
pub(super) fn expanded(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(&b.expanded(args.f64("sw")?, args.f64("sh")?)))
}

/// `shrunk(mx, my)`.
pub(super) fn shrunk(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(&b.shrunk(args.f64("mx")?, args.f64("my")?)))
}

/// `shrunk_to_aspect(box_aspect, container, fig_aspect)`; non-positive
/// aspects raise `ValueError`.
pub(super) fn shrunk_to_aspect(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    let container = args.get("container").map(encode::parse_bbox).transpose()?;
    let fig_aspect = args.opt_f64("fig_aspect")?.unwrap_or(1.0);
    let shrunk = b
        .shrunk_to_aspect(args.f64("box_aspect")?, container.as_ref(), fig_aspect)
        .ok_or_else(|| ProbeError::rejected("'box_aspect' and 'fig_aspect' must be positive"))?;
    Ok(encode::bbox(&shrunk))
}

/// `anchored(c, container)` with an anchor code or an `[cx, cy]` pair;
/// unknown codes raise `KeyError`.
pub(super) fn anchored(args: Args<'_>) -> Outcome {
    let c = args.json("c")?;
    let anchor = match (c.as_str(), c.as_f64_vec().as_deref()) {
        (Some(code), _) => {
            Anchor::from_code(code).ok_or_else(|| ProbeError::rejected(format!("{code:?}")))?
        }
        (None, Some(&[cx, cy])) => Anchor::Fraction(cx, cy),
        _ => return Err(ProbeError::BadArgs(format!("bad anchor {c}"))),
    };
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(
        &b.anchored(anchor, &bbox_arg(args, "container")?),
    ))
}

/// `splitx(*fractions)`.
pub(super) fn split_x(args: Args<'_>) -> Outcome {
    let parts = bbox_arg(args, "bbox")?.split_x(&args.f64_vec("fractions")?);
    Ok(Json::Array(parts.iter().map(encode::bbox).collect()))
}

/// `splity(*fractions)`.
pub(super) fn split_y(args: Args<'_>) -> Outcome {
    let parts = bbox_arg(args, "bbox")?.split_y(&args.f64_vec("fractions")?);
    Ok(Json::Array(parts.iter().map(encode::bbox).collect()))
}

/// `translated(tx, ty)`.
pub(super) fn translated(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(
        &b.translated(args.f64("tx")?, args.f64("ty")?),
    ))
}

/// `transformed(Affine2D(ops))`: three mapped corners, not an enclosing box.
pub(super) fn transformed(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(&b.transformed(&affine_arg(args, "ops")?)))
}

/// `rotated(radians)`.
pub(super) fn rotated(args: Args<'_>) -> Outcome {
    Ok(encode::bbox(
        &bbox_arg(args, "bbox")?.rotated(args.f64("radians")?),
    ))
}

/// `corners()`.
pub(super) fn corners(args: Args<'_>) -> Outcome {
    Ok(encode::points(&bbox_arg(args, "bbox")?.corners()))
}

/// `TransformedBbox(bbox, Affine2D(ops)).get_points()`: the box enclosing
/// all four mapped corners, in the input's orientation.
pub(super) fn transformed_bbox(args: Args<'_>) -> Outcome {
    let b = bbox_arg(args, "bbox")?;
    Ok(encode::bbox(
        &b.transformed_enclosing(&affine_arg(args, "ops")?),
    ))
}
