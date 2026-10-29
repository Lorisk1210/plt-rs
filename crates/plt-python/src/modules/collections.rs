//! `_native.collections`: collections of artists drawn together
//! (matplotlib's `collections`: what `scatter` makes).

use plt::collections::Collection;

use crate::binding::prelude::*;

artist_handle!(Collection);

/// The positions of the items, as an `(n, 2)` array.
fn get_offsets(collection: Collection) -> PyResult<NdArray> {
    let offsets = collection.offsets();
    let rows = offsets.len();
    let data = offsets
        .into_iter()
        .flat_map(|point| [point.x, point.y])
        .collect();
    NdArray::with_shape(data, vec![rows, 2])
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_offsets", get_offsets)
}
