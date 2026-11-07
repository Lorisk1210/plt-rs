//! The data the libraries' tests read that is neither in their source
//! trees nor in their packages (`Library::data`): cartopy's Natural Earth
//! maps, shap's and yellowbrick's datasets. `cargo xtask ecosystem setup`
//! fetches each file once, checking its SHA-256 (`python/ecosystem/fetch.py
//! file`, or `zip` for archives unpacked beside themselves), into
//! `target/ecosystem/data/<library>`, never committed; the suites' runs
//! then find it there offline, through a variable naming the directory or
//! a link to it in the site, the same in both modes.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::libraries::{DataForm, Found, Library};
use super::setup::fetch_command;
use crate::mpl_suite::pytest::Variable;
use crate::python;

/// Where `library`'s data is fetched to.
pub(super) fn dir(library: &Library) -> PathBuf {
    super::ecosystem_dir().join("data").join(library.name)
}

/// Fetches the files of `library`'s data that are missing with `python`.
pub(super) fn fetch(python: &Path, library: &Library) -> Result<(), String> {
    let Some(data) = library.data else {
        return Ok(());
    };
    let dir = dir(library);
    let command = match data.form {
        DataForm::Plain => "file",
        DataForm::Zips => "zip",
    };
    for file in data.files {
        let dest = dir.join(file.path);
        if !dest.is_file() {
            python::run_checked(
                fetch_command(python, command)
                    .args([file.url, file.sha256])
                    .arg(dest),
            )?;
        }
    }
    Ok(())
}

/// `library`'s data directory, if it has data: an error if setup has not
/// fetched it.
pub(super) fn fetched(library: &Library) -> Result<Option<(PathBuf, Found)>, String> {
    let Some(data) = library.data else {
        return Ok(None);
    };
    let dir = dir(library);
    let missing = data
        .files
        .iter()
        .find(|file| !dir.join(file.path).is_file());
    if let Some(file) = missing {
        return Err(format!(
            "{} is missing: run `cargo xtask ecosystem setup` first",
            dir.join(file.path).display()
        ));
    }
    Ok(Some((dir, data.found)))
}

/// The variables of `library`'s runs naming its data's directory.
pub(super) fn variables(library: &Library) -> Result<Vec<Variable>, String> {
    Ok(match fetched(library)? {
        Some((dir, Found::Variable(name))) => vec![(name.to_owned(), Some(OsString::from(dir)))],
        _ => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::libraries::named;

    #[test]
    fn libraries_without_data_need_no_setup() {
        let pandas = named("pandas");
        assert!(fetched(&pandas).unwrap().is_none());
        assert_eq!(variables(&pandas).unwrap(), []);
    }

    #[test]
    fn data_is_fetched_beside_the_other_libraries() {
        let cartopy = named("cartopy");
        assert_eq!(
            dir(&cartopy),
            super::super::ecosystem_dir().join("data/cartopy")
        );
    }
}
