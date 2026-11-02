//! What a text takes from another (matplotlib's `Text.update_from`) and
//! where its box is drawn (`Text.update_bbox_position_size`).

use pltrs::text::Text;

use super::super::transforms::matrix;
use crate::binding::prelude::*;

/// Gives `text` the common properties and the look of `other`; whether
/// the transform and the clip regions were copied too.
fn update_from(text: Text, other: Text) -> PyResult<bool> {
    Ok(text.update_from(&other))
}

/// Where a text's box is drawn: the `(width, height)` of its frame, the
/// 3×3 matrix placing it in display pixels and its mutation scale.
type BoxFrame = ((f64, f64), NdArray, f64);

/// The frame of the box of `text` where it is drawn; `None` without a
/// box.
fn bbox_frame(text: Text) -> PyResult<Option<BoxFrame>> {
    text.bbox_frame()
        .map(|(shape, transform)| {
            Ok((
                (shape.width, shape.height),
                matrix(transform)?,
                shape.mutation_scale,
            ))
        })
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("update_from", update_from)?;
    module.add("bbox_frame", bbox_frame)
}
