//! Whether an artist changed since it was last drawn (matplotlib's
//! `Artist.stale`), as pltrs (Rust) tracks it for every artist of a
//! figure, the figure itself included.

use pltrs::artist::{ArtistHandle, RawHandle};

use crate::binding::prelude::*;

/// Whether the artist changed since it was last drawn (`stale`).
fn get_stale(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.stale())
}

/// Marks the artist as changed, with what contains it unless it is
/// animated, or as drawn (`stale = value`).
fn set_stale(artist: RawHandle, stale: bool) -> PyResult<()> {
    artist.set_stale(stale);
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_stale", get_stale)?;
    module.add("set_stale", set_stale)
}
