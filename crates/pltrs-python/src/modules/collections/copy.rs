//! What a collection takes from another (matplotlib's
//! `Collection.update_from`), whether its faces are filled
//! (`get_fill`) and the rotation of its sized items
//! (`RegularPolyCollection.get_rotation`).

use pltrs::collections::Collection;

use crate::binding::prelude::*;

/// Gives `collection` the common properties and the look of `other`;
/// whether the transform and the clip regions were copied too.
fn update_from(collection: Collection, other: Collection) -> PyResult<bool> {
    Ok(collection.update_from(&other))
}

/// Whether the faces are coloured (`get_fill`).
fn get_fill(collection: Collection) -> PyResult<bool> {
    Ok(collection.fill())
}

/// The rotation of the sized items in radians (`get_rotation`).
fn get_rotation(collection: Collection) -> PyResult<f64> {
    Ok(collection.size_rotation())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("update_from", update_from)?;
    module.add("get_fill", get_fill)?;
    module.add("get_rotation", get_rotation)
}
