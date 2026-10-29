//! Transforms built from other objects: boxes (`BboxTransform*`), two
//! children (`blended_transform_factory`), physical offsets
//! (`ScaledTranslation`, `offset_copy`).

use pltrs::artist::ArtistId;
use pltrs::transforms::{
    self, Bbox, OffsetUnits, ResolvedTransform, Transform, TransformContext, blended_affine,
};

use super::{affine_arg, bbox_arg, matrix_and_points, transformed_arg};
use crate::json::Json;
use crate::probes::{Args, Outcome, ProbeError, encode};

/// `BboxTransform(boxin, boxout)`.
pub(super) fn bbox_transform(args: Args<'_>) -> Outcome {
    let t = transforms::bbox_transform(&bbox_arg(args, "boxin")?, &bbox_arg(args, "boxout")?);
    matrix_and_points(&t, args)
}

/// `BboxTransformTo(boxout)`.
pub(super) fn bbox_transform_to(args: Args<'_>) -> Outcome {
    matrix_and_points(&transforms::unit_to_bbox(&bbox_arg(args, "boxout")?), args)
}

/// `BboxTransformFrom(boxin)`.
pub(super) fn bbox_transform_from(args: Args<'_>) -> Outcome {
    matrix_and_points(
        &transforms::bbox_transform_from(&bbox_arg(args, "boxin")?),
        args,
    )
}

/// `blended_transform_factory(Affine2D(x_ops), Affine2D(y_ops))`; a child
/// with rotation or shear raises `ValueError`.
pub(super) fn blended(args: Args<'_>) -> Outcome {
    let (x, y) = (affine_arg(args, "x_ops")?, affine_arg(args, "y_ops")?);
    let t = blended_affine(&x, &y).map_err(ProbeError::rejected)?;
    Ok(encode::object([
        ("matrix", encode::matrix(&t)),
        ("transformed", transformed_arg(&t, args)?),
        ("is_separable", Json::from(t.is_separable())),
    ]))
}

/// `ScaledTranslation(xt, yt, Affine2D(scale_ops))`.
pub(super) fn scaled_translation(args: Args<'_>) -> Outcome {
    let scale = ResolvedTransform::affine(affine_arg(args, "scale_ops")?);
    let t = transforms::scaled_translation(args.f64("xt")?, args.f64("yt")?, &scale);
    matrix_and_points(&t, args)
}

/// `offset_copy(Affine2D(base_ops), Figure(dpi=dpi), x, y, units)`; a
/// `null` dpi stands for `fig=None`.
///
/// pltrs always resolves transforms against a figure, so matplotlib's
/// "a fig kwarg is needed" check for dpi-dependent units happens here.
pub(super) fn offset_copy(args: Args<'_>) -> Outcome {
    let name = args.opt_str("units")?.unwrap_or("inches");
    let units = OffsetUnits::from_name(name)
        .ok_or_else(|| ProbeError::rejected(format!("unknown offset units {name:?}")))?;
    let dpi = match (args.opt_f64("dpi")?, units.needs_figure()) {
        (Some(dpi), _) => dpi,
        // Dots never consult the figure's resolution.
        (None, false) => f64::NAN,
        (None, true) => {
            return Err(ProbeError::rejected(
                "for units of inches or points a figure is needed",
            ));
        }
    };
    let offset = Transform::Affine(affine_arg(args, "base_ops")?).offset_copy(
        args.f64("x")?,
        args.f64("y")?,
        units,
    );
    let resolved = offset.resolve(&Dpi(dpi)).map_err(ProbeError::rejected)?;
    matrix_and_points(&resolved.affine_part(), args)
}

/// A figure reduced to its resolution: the only geometry offsets need.
struct Dpi(f64);

impl TransformContext for Dpi {
    fn dpi(&self) -> f64 {
        self.0
    }

    fn frame_bbox(&self, _: ArtistId) -> Option<Bbox> {
        None
    }

    fn axes_bbox(&self, _: ArtistId) -> Option<Bbox> {
        None
    }

    fn data_transform(&self, _: ArtistId) -> Option<ResolvedTransform> {
        None
    }
}
