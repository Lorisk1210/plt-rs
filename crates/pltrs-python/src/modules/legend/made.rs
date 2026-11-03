//! The handles of legend entries as Python finds and makes them: those of
//! the containers of an axes (matplotlib's containers are handles), and
//! entries whose artists a handler of the program's made (the
//! `legend_artist` of a Python handler), which cross as `("made",
//! [(artist, follows), ...])`, each artist's native object with how it
//! follows the handle box (`"box"`, `"offsets"` or `"own"`), and come back
//! as the artists the legend draws in each entry (`entry_artists`).

use pltrs::artist::RawHandle;
use pltrs::container::Container;
use pltrs::legend::{Attach, Legend, LegendHandle};

use crate::binding::prelude::*;

/// How a made artist follows the handle box, by name.
fn attach(name: &str) -> PyResult<Attach> {
    match name {
        "box" => Ok(Attach::Transform),
        "offsets" => Ok(Attach::Offsets),
        "own" => Ok(Attach::Own),
        _ => Err(PyErr::value_error(format!(
            "{name:?} is not a way to follow a handle box"
        ))),
    }
}

/// The entry drawn by `artists`, which move into the legend made with it.
pub(super) fn made_handle(artists: Vec<(RawHandle, String)>) -> PyResult<LegendHandle> {
    let artists = artists
        .into_iter()
        .map(|(artist, follows)| Ok((artist, attach(&follows)?)))
        .collect::<PyResult<Vec<_>>>()?;
    Ok(LegendHandle::made(artists)?)
}

/// The handle a legend shows `container` with (matplotlib's containers
/// are legend handles).
fn container_entry(container: Container) -> PyResult<LegendHandle> {
    Ok(LegendHandle::from(container))
}

/// Whether the legend handle `entry` shows `container`, as the handles of
/// the containers an axes lists are.
fn shows_container(entry: LegendHandle, container: Container) -> PyResult<bool> {
    Ok(entry == LegendHandle::from(container))
}

/// The artists drawing the handle of each entry of `legend`, in entry
/// order.
fn entry_artists(legend: Legend) -> PyResult<Vec<Vec<RawHandle>>> {
    Ok(legend.entry_artists())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("container_entry", container_entry)?;
    module.add("shows_container", shows_container)?;
    module.add("entry_artists", entry_artists)
}
