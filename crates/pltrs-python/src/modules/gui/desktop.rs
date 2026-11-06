//! pltrs's desktop entry and icons (`pltrs::gui::install_desktop_entry`),
//! for `backends/backend_pltrs.py`'s `install_desktop_entry` and
//! `remove_desktop_entry`.

use std::io;
use std::path::PathBuf;

use pltrs::gui::{
    DesktopEntry, install_desktop_entry_in, remove_desktop_entry_from, user_data_dir,
};

use crate::binding::prelude::*;

/// An `OSError` naming what failed.
fn os_error(err: &io::Error) -> PyErr {
    PyErr::new(ExceptionKind::OSError, err.to_string())
}

/// `dir`, or the user's data directory.
fn data_dir(dir: Option<String>) -> PyResult<PathBuf> {
    match dir {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => user_data_dir().map_err(|err| os_error(&err)),
    }
}

/// The paths as strings.
fn strings(paths: impl IntoIterator<Item = PathBuf>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.display().to_string())
        .collect()
}

/// Writes the desktop entry and icons into `data_dir` (`None`: the
/// user's data directory); returns the files written, the entry first.
fn install_desktop_entry(dir: Option<String>) -> PyResult<Vec<String>> {
    let DesktopEntry { entry, icons } =
        install_desktop_entry_in(&data_dir(dir)?).map_err(|err| os_error(&err))?;
    Ok(strings(std::iter::once(entry).chain(icons)))
}

/// Removes the desktop entry and icons from `data_dir` (`None`: the
/// user's data directory); returns the files removed.
fn remove_desktop_entry(dir: Option<String>) -> PyResult<Vec<String>> {
    let removed = remove_desktop_entry_from(&data_dir(dir)?).map_err(|err| os_error(&err))?;
    Ok(strings(removed))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("install_desktop_entry", install_desktop_entry)?;
    module.add("remove_desktop_entry", remove_desktop_entry)
}
