//! The venvs the libraries' suites run in (`Library::venvs`).
//!
//! Most libraries share the ecosystem's two venvs, pinned together by
//! `python/requirements-ecosystem.txt`. A library whose pins clash with
//! theirs (lifelines wants pandas 2, scanpy's pytest configuration
//! pytest 9) has two of its own, `target/ecosystem/venvs/<library>/venv`
//! and `reference-venv`, pinned by `python/requirements-ecosystem-<library>.txt`
//! and made the same way: matplotlib excluded from the pltrs side,
//! `requirements-ecosystem-reference.txt` added to the reference side.
//! A group of libraries kept apart from the shared venvs (the add-ons
//! building on matplotlib's parts) shares two the same way, named after
//! the group. uv links their packages from its cache, so they cost little disk and a
//! short setup, and every pltrs venv imports pltrs (Python) from the
//! source tree, one extension for all.

use std::fs;
use std::path::PathBuf;

use super::libraries::{Library, Venvs};
use crate::mpl_suite::Mode;
use crate::python;

/// The pinned list of the shared venvs.
pub(super) const SHARED: &str = "requirements-ecosystem.txt";

/// What a reference venv adds to its list: real matplotlib.
pub(super) const REFERENCE: &str = "requirements-ecosystem-reference.txt";

/// The pinned list of `library`'s venvs, in `python/`.
pub(super) fn requirements(library: &Library) -> String {
    match library.venvs {
        Venvs::Shared => SHARED.to_owned(),
        Venvs::Own => format!("requirements-ecosystem-{}.txt", library.name),
        Venvs::Group(group) => format!("requirements-ecosystem-{group}.txt"),
    }
}

/// The venv `library`'s suite runs in under `mode`.
pub(super) fn venv(library: &Library, mode: Mode) -> PathBuf {
    let dir = match library.venvs {
        Venvs::Shared => super::ecosystem_dir(),
        Venvs::Own => super::ecosystem_dir().join("venvs").join(library.name),
        Venvs::Group(group) => super::ecosystem_dir().join("venvs").join(group),
    };
    match mode {
        Mode::Pltrs => dir.join("venv"),
        Mode::Reference => dir.join("reference-venv"),
    }
}

/// The version `library`'s list pins it to.
pub(super) fn pinned(library: &Library) -> Result<String, String> {
    let path = python::root().join("python").join(requirements(library));
    let text = fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    pin_in(&text, library.name)
        .ok_or_else(|| format!("{} pins no {}", path.display(), library.name))
}

/// The version the requirements `text` pins `name` to (`name==version`,
/// an environment marker after it left aside).
fn pin_in(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (package, rest) = line.split_once("==")?;
        let version = rest.split_once(';').map_or(rest, |(version, _)| version);
        (package.trim() == name).then(|| version.trim().to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::libraries::{all, named};

    #[test]
    fn pins_are_read_off_their_lines() {
        let text = "# pandas==0.1 in a comment\nnumpy==2.5.4\npandas == 3.1.0\n";
        assert_eq!(pin_in(text, "pandas").as_deref(), Some("3.1.0"));
        assert_eq!(pin_in(text, "numpy").as_deref(), Some("2.5.4"));
        assert_eq!(pin_in(text, "seaborn"), None);
        let marked = "healpy==1.20.1; sys_platform != \"win32\"\n";
        assert_eq!(pin_in(marked, "healpy").as_deref(), Some("1.20.1"));
    }

    #[test]
    fn the_libraries_are_pinned() {
        for library in all() {
            assert!(pinned(&library).is_ok(), "{}", library.name);
        }
    }

    #[test]
    fn libraries_of_their_own_have_venvs_of_their_own() {
        let pandas = named("pandas");
        assert_eq!(requirements(&pandas), SHARED);
        assert_eq!(venv(&pandas, Mode::Pltrs), super::super::venv());
        let lifelines = named("lifelines");
        assert_eq!(
            requirements(&lifelines),
            "requirements-ecosystem-lifelines.txt"
        );
        assert_eq!(
            venv(&lifelines, Mode::Reference),
            super::super::ecosystem_dir().join("venvs/lifelines/reference-venv")
        );
        let mplcursors = named("mplcursors");
        assert_eq!(
            requirements(&mplcursors),
            "requirements-ecosystem-addons.txt"
        );
        assert_eq!(
            venv(&mplcursors, Mode::Pltrs),
            super::super::ecosystem_dir().join("venvs/addons/venv")
        );
    }
}
