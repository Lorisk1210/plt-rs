//! A triangle mesh following its triangulation: matplotlib's
//! `TriMesh.draw` reads the points and the masked triangles of its
//! triangulation each time it draws, so the Python class hands pltrs
//! (Rust) a triangulation of them again whenever they changed
//! (`_collection_state`'s `TRIANGULATION`).

use pltrs::artist::{RawHandle, TypedHandle};
use pltrs::collections::TriMesh;
use pltrs::tri::Triangulation;

use crate::binding::prelude::*;

/// Has `mesh`, a triangle mesh, draw the unmasked triangles of
/// `triangulation` over its points.
fn set_tri_mesh_triangulation(
    mesh: RawHandle,
    triangulation: Native<Triangulation>,
) -> PyResult<()> {
    let mesh = TriMesh::from_raw_handle(&mesh)
        .ok_or_else(|| PyErr::type_error("expected the native handle of a triangle mesh"))?;
    mesh.set_triangulation(&triangulation.borrow());
    Ok(())
}

/// Registers the function.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_tri_mesh_triangulation", set_tri_mesh_triangulation)
}
