//! `_native.artist`: what every artist has (matplotlib's `Artist`), on
//! the native object of any artist handle.

use pltrs::artist::{ArtistHandle, RawHandle};

use crate::binding::prelude::*;

/// The artist's id in its figure: how pltrs's Python classes find the
/// one object they keep per artist.
fn id(artist: RawHandle) -> PyResult<u32> {
    Ok(artist.id().index())
}

/// Whether the artist is drawn.
fn get_visible(artist: RawHandle) -> PyResult<bool> {
    Ok(artist.visible())
}

/// Shows or hides the artist.
fn set_visible(artist: RawHandle, visible: bool) -> PyResult<()> {
    artist.set_visible(visible);
    Ok(())
}

/// The drawing order.
fn get_zorder(artist: RawHandle) -> PyResult<f64> {
    Ok(artist.zorder())
}

/// Sets the drawing order.
fn set_zorder(artist: RawHandle, zorder: f64) -> PyResult<()> {
    artist.set_zorder(zorder);
    Ok(())
}

/// The opacity override, `None` when unset.
fn get_alpha(artist: RawHandle) -> PyResult<Option<f64>> {
    Ok(artist.alpha())
}

/// Sets the opacity override (`None` removes it).
fn set_alpha(artist: RawHandle, alpha: Option<f64>) -> PyResult<()> {
    artist.set_alpha(alpha);
    Ok(())
}

/// The legend label.
fn get_label(artist: RawHandle) -> PyResult<String> {
    Ok(artist.label())
}

/// Sets the legend label.
fn set_label(artist: RawHandle, label: String) -> PyResult<()> {
    artist.set_label(label);
    Ok(())
}

/// Removes the artist from its figure.
fn remove(artist: RawHandle) -> PyResult<()> {
    artist.remove();
    Ok(())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("id", id)?;
    module.add("get_visible", get_visible)?;
    module.add("set_visible", set_visible)?;
    module.add("get_zorder", get_zorder)?;
    module.add("set_zorder", set_zorder)?;
    module.add("get_alpha", get_alpha)?;
    module.add("set_alpha", set_alpha)?;
    module.add("get_label", get_label)?;
    module.add("set_label", set_label)?;
    module.add("remove", remove)
}
