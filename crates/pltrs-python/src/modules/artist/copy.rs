//! Properties copied between artists (matplotlib's `Artist.update_from`),
//! artists copied apart from their figure's tree (`copy.copy`) and the
//! sticky edges autoscaling stops at (`Artist.sticky_edges`).

use pltrs::artist::{ArtistHandle, RawHandle};

use crate::binding::prelude::*;

/// Gives `artist` the common properties of `other`; whether the
/// transform and the clip regions were copied too (they are when both
/// artists share a figure, or `artist` is loose).
fn update_from(artist: RawHandle, other: RawHandle) -> PyResult<bool> {
    Ok(artist.update_from(&other))
}

/// A copy of the artist that its figure does not draw, owned by the
/// artist's owner (matplotlib's `copy.copy` of an artist).
fn detached_copy(artist: RawHandle) -> PyResult<RawHandle> {
    Ok(artist.detached_copy())
}

/// The sticky edges, `(x, y)`.
fn get_sticky_edges(artist: RawHandle) -> PyResult<(Vec<f64>, Vec<f64>)> {
    Ok(artist.sticky_edges())
}

/// Replaces the sticky edges.
fn set_sticky_edges(artist: RawHandle, x: Vec<f64>, y: Vec<f64>) -> PyResult<()> {
    artist.set_sticky_edges(x, y);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("update_from", update_from)?;
    module.add("detached_copy", detached_copy)?;
    module.add("get_sticky_edges", get_sticky_edges)?;
    module.add("set_sticky_edges", set_sticky_edges)
}
