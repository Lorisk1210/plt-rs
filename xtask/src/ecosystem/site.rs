//! A library's site: the directory its suite runs in, laid out as the
//! library's source tree is.
//!
//! The site holds the library's package, made of links to the entries of
//! the installed package and real copies of the source tree's tests
//! (seaborn's `tests/` become `seaborn/tests/`; the directories leading
//! to a copy deeper in the package, such as `sklearn/metrics/_plot/tests`,
//! are real directories of links in turn), and the file of the source
//! tree holding the library's pytest configuration (`Library::configuration`),
//! so that pytest runs with the library's own configuration (pandas turns
//! every warning into an error). The directory its image comparisons read
//! their baselines from (`Baselines::read_from`), if setup makes them, is
//! a link to the ones made, and so is the directory a library caches its
//! downloads in, if setup fetches its data (`Found::Link`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::libraries::{Found, Library};
use crate::mpl_suite::prepare;

/// The configuration file pandas installs in its package, which pytest
/// would take for the site's configuration, making the package's
/// directory its root.
const INSTALLED_CONFIGURATION: &str = "pyproject.toml";

/// Lays out `library`'s site in `dir` from its source tree and the
/// package installed in `installed` (a venv's `site-packages`), its
/// baselines' directory a link to `baselines`, and returns it.
pub(super) fn lay_out(
    library: &Library,
    installed: &Path,
    dir: &Path,
    baselines: &Path,
) -> Result<PathBuf, String> {
    let version = super::venvs::pinned(library)?;
    let sdist = super::setup::sdist(library.name, &version);
    if !sdist.is_dir() {
        return Err(format!(
            "{} is missing: run `cargo xtask ecosystem setup` first",
            sdist.display()
        ));
    }
    let site = dir.join("site");
    let package = site.join(library.package);
    for (directory, skip) in layout(library) {
        prepare::relink(
            &installed.join(library.package).join(&directory),
            &package.join(&directory),
            &skip,
        )?;
    }
    link_vendored(library, installed, &site)?;
    for (from, to) in library.copies {
        let (from, to) = (sdist.join(from), site.join(to));
        if from.is_dir() {
            prepare::copy_tests(&from, &to)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    let configuration = Path::new(library.configuration);
    copy_file(&sdist.join(configuration), &site.join(configuration))?;
    if let Some(read_from) = library.baselines.and_then(|made| made.read_from) {
        let link = site.join(read_from);
        prepare::remove(&link)?;
        prepare::link(baselines, &link)?;
    }
    if let Some((data, Found::Link(path))) = super::data::fetched(library)? {
        let link = site.join(path);
        prepare::remove(&link)?;
        // In a directory the tests make for themselves (scanpy's in
        // pytest's cache), made first.
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("{}: {err}", parent.display()))?;
        }
        prepare::link(&data, &link)?;
    }
    Ok(site)
}

/// Links into `site` the shared libraries the wheel of `library` vendors
/// beside its package (`scikit_learn.libs`), if it has any: its extension
/// modules find them relative to where they are loaded from
/// (`$ORIGIN/../../scikit_learn.libs`), which for the links of the site
/// is the site.
fn link_vendored(library: &Library, installed: &Path, site: &Path) -> Result<(), String> {
    let name = format!("{}.libs", library.name.replace('-', "_"));
    let link = site.join(&name);
    if link.is_symlink() {
        // (A link to a directory is a directory to Windows, which
        // `remove_file` does not remove.)
        prepare::remove(&link)?;
    }
    let vendored = installed.join(&name);
    if vendored.is_dir() {
        prepare::link(&vendored, &link)?;
    }
    Ok(())
}

/// The real directories of `library`'s package in its site (`""`, the
/// package itself, and those leading to its copies), relative to the
/// package and parents first, each with the entries of the installed
/// package its links leave out: those that are copies or real
/// directories, and in the package itself [`INSTALLED_CONFIGURATION`].
fn layout(library: &Library) -> BTreeMap<String, Vec<&'static str>> {
    let mut layout = BTreeMap::from([(String::new(), vec![INSTALLED_CONFIGURATION])]);
    let inside = library.copies.iter().filter_map(|(_, to)| {
        to.strip_prefix(library.package)
            .and_then(|path| path.strip_prefix('/'))
    });
    for path in inside {
        let mut directory = String::new();
        for name in path.split('/') {
            let skip = layout.entry(directory.clone()).or_default();
            if !skip.contains(&name) {
                skip.push(name);
            }
            if !directory.is_empty() {
                directory.push('/');
            }
            directory.push_str(name);
        }
    }
    layout
}

/// Copies the file `from` to `to`, making its directory.
fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    }
    fs::copy(from, to)
        .map(drop)
        .map_err(|err| format!("{}: {err}", from.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::libraries::named as library;

    #[test]
    fn copies_deep_in_the_package_are_reached_through_real_directories() {
        let sklearn = layout(&library("scikit-learn"));
        assert_eq!(
            sklearn[""],
            [
                "pyproject.toml",
                "conftest.py",
                "inspection",
                "metrics",
                "model_selection",
                "tests",
                "tree",
                "utils"
            ]
        );
        assert_eq!(sklearn["inspection"], ["_plot"]);
        assert_eq!(sklearn["inspection/_plot"], ["tests"]);
        assert_eq!(sklearn["metrics/_plot"], ["tests"]);
        assert_eq!(sklearn["tree"], ["tests"]);
        assert!(!sklearn.contains_key("tree/tests"));
        // Copies outside the package (xarray's own conftest) are not in it.
        let xarray = layout(&library("xarray"));
        assert_eq!(xarray.keys().collect::<Vec<_>>(), [""]);
        assert_eq!(xarray[""], ["pyproject.toml", "tests"]);
    }

    #[test]
    fn the_top_directories_of_a_source_tree_go_into_the_package() {
        let mplfinance = layout(&library("mplfinance"));
        assert_eq!(mplfinance[""], ["pyproject.toml", "tests", "examples"]);
        assert_eq!(mplfinance["examples"], ["data"]);
    }
}
