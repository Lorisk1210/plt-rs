//! Properties copied between artists (matplotlib's `Artist.update_from`)
//! and the sticky edges autoscaling stops at (`Artist.sticky_edges`).

use pltrs::artist::{ArtistHandle, RawHandle};

use crate::binding::prelude::*;

/// Gives `artist` the common properties of `other`; whether the
/// transform and the clip regions were copied too (they are when both
/// artists share a figure, or `artist` is loose).
fn update_from(artist: RawHandle, other: RawHandle) -> PyResult<bool> {
    Ok(artist.update_from(&other))
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
    module.add("get_sticky_edges", get_sticky_edges)?;
    module.add("set_sticky_edges", set_sticky_edges)
}
