//! A shadow following its patch: matplotlib's `Shadow` reads the path and
//! the patch transform of its patch each time it draws, so the Python
//! class hands pltrs (Rust) the patch's outline again whenever it changed
//! (`_patch_state.SHADOWED`).

use pltrs::artist::RawHandle;
use pltrs::patches::{Patch, PathShape, Shadow, Wedge};

use super::super::path::{PathArrays, path_arrays};
use super::AnyPatch;
use crate::binding::prelude::*;

/// The outline `patch` draws, placed by its patch transform, as
/// `(vertices, codes)`: what a shadow of it shows.
fn placed_outline(patch: AnyPatch) -> PyResult<PathArrays> {
    path_arrays(&patch.0.placed_outline(), false)
}

/// Has `shadow` show `source` as it is now. A shadow a program made
/// takes the outline `source` draws; the shadow of a pie's wedge, a
/// wedge itself, takes the wedge's geometry (an outline of the program's
/// in its place is not shadowed).
fn set_shadow_source(shadow: RawHandle, source: RawHandle) -> PyResult<()> {
    if let Some(shadow) = shadow.downcast::<Patch<Shadow<PathShape>>>() {
        let outline = PathShape(AnyPatch::of(&source)?.0.placed_outline());
        shadow.update_shape(|shape| shape.set_source(outline));
        return Ok(());
    }
    match (
        shadow.downcast::<Patch<Shadow<Wedge>>>(),
        source.downcast::<Patch<Wedge>>(),
    ) {
        (Some(shadow), Some(wedge)) => {
            let wedge = wedge.shape();
            shadow.update_shape(|shape| shape.set_source(wedge));
            Ok(())
        }
        _ => Err(PyErr::type_error(
            "expected the native handles of a shadow and of the patch it shadows",
        )),
    }
}

/// Registers the functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("placed_outline", placed_outline)?;
    module.add("set_shadow_source", set_shadow_source)?;
    Ok(())
}
