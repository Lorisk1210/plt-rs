//! The arrow of an added annotation box as a patch (matplotlib's
//! `AnnotationBbox.arrow_patch`): a loose `FancyArrowPatch` of the
//! arrow's look, which `offsetbox._AnnotationBboxArrow` hands out as a
//! mirror, as `_native.annotation` does for an annotation's arrow.

use pltrs::artist::RawHandle;
use pltrs::patches::{FancyArrowPatch, Patch};

use super::super::path::{PathArrays, path_arrays};
use super::add::annotation_bbox;
use crate::binding::prelude::*;

/// A loose patch of the look of the arrow of `annotation`, `None`
/// without an arrow.
fn arrow_patch(annotation: RawHandle) -> PyResult<Option<Patch<FancyArrowPatch>>> {
    Ok(annotation_bbox(&annotation)?.arrow_patch())
}

/// Gives the arrow of `annotation` the look of `patch`, a patch
/// `arrow_patch` made and the program changed.
fn restyle_arrow(annotation: RawHandle, patch: Patch<FancyArrowPatch>) -> PyResult<()> {
    annotation_bbox(&annotation)?.restyle_arrow(&patch);
    Ok(())
}

/// The display box `(x0, y0, x1, y1)` of the arrow as drawn, `None`
/// without an arrow or while the annotated point is hidden.
fn arrow_extent(annotation: RawHandle) -> PyResult<Option<(f64, f64, f64, f64)>> {
    Ok(annotation_bbox(&annotation)?
        .arrow_extent()
        .map(|bbox| (bbox.x0(), bbox.y0(), bbox.x1(), bbox.y1())))
}

/// The outline of the arrow as drawn, in display pixels, as the arrays of
/// a path; `None` without an arrow or while the annotated point is
/// hidden.
fn arrow_display_path(annotation: RawHandle) -> PyResult<Option<PathArrays>> {
    annotation_bbox(&annotation)?
        .arrow_display_path()
        .map(|path| path_arrays(&path, false))
        .transpose()
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("annotation_arrow_patch", arrow_patch)?;
    module.add("restyle_annotation_arrow", restyle_arrow)?;
    module.add("annotation_arrow_extent", arrow_extent)?;
    module.add("annotation_arrow_display_path", arrow_display_path)
}
