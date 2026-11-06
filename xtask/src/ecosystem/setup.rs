//! `cargo xtask ecosystem setup`: the ecosystem's two venvs, and the
//! sdists of the libraries whose suites `ecosystem suite` runs.
//!
//! * `target/ecosystem/venv`, pltrs' side: `python/requirements-ecosystem.txt`
//!   (the libraries, what their tests use, pytest and its plugins, pltrs'
//!   own dependencies) installed with matplotlib excluded from the
//!   resolution, so that what requires it (seaborn, plotnine, pytest-mpl)
//!   gets `python/alias`; setup checks that the venv cannot import
//!   matplotlib;
//! * `target/ecosystem/reference-venv`, matplotlib's side: the same list
//!   and `python/requirements-ecosystem-reference.txt` (real matplotlib
//!   3.11.2 and its dependencies), resolved together;
//! * the source trees of the libraries at the versions the list pins, in
//!   `target/refs/<name>-dl/<name>-<version>`: their sdists, or the
//!   source archives of the releases' tags where the sdists ship no
//!   tests (mplfinance, matplotlib-venn), with the files of the release's
//!   source tree their suites read that the sdists leave out (pandas'
//!   iris data, geopandas' maps), which `python/ecosystem/fetch.py`
//!   downloads (from `PyPI`, and from the repository at the release's
//!   tag) when they are missing, checking their SHA-256;
//! * the baselines of the image comparisons whose own are for another
//!   version of matplotlib (plotnine's, astropy's, mplfinance's), made in
//!   the reference venv (`baselines`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::baselines;
use super::libraries::{LIBRARIES, Library, Source};
use crate::python::{self, make_venv};

/// The pinned list of both venvs.
const REQUIREMENTS: &str = "requirements-ecosystem.txt";

/// What the reference venv adds to [`REQUIREMENTS`].
const REFERENCE_REQUIREMENTS: &str = "requirements-ecosystem-reference.txt";

/// Makes (or completes) both venvs, fetches the missing source trees and
/// makes the missing baselines.
pub(super) fn run() -> Result<(), String> {
    let dir = super::ecosystem_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let excludes = dir.join("excludes.txt");
    fs::write(&excludes, "matplotlib\n").map_err(|err| format!("{}: {err}", excludes.display()))?;

    let venv = super::venv();
    let python = make_venv(&venv)?;
    python::run_checked(
        python::install_command(&python, &[REQUIREMENTS])
            .arg("--excludes")
            .arg(&excludes),
    )?;
    python::refuse_matplotlib(&python, &venv)?;

    let reference = make_venv(&super::reference_venv())?;
    python::run_checked(&mut python::install_command(
        &reference,
        &[REQUIREMENTS, REFERENCE_REQUIREMENTS],
    ))?;

    for library in LIBRARIES {
        fetch(&python, &library)?;
    }
    for library in LIBRARIES {
        baselines::make(&library, &super::reference_venv())?;
    }
    println!(
        "ready: {} (pltrs) and {} (reference)",
        venv.display(),
        super::reference_venv().display()
    );
    Ok(())
}

/// The version [`REQUIREMENTS`] pins `name` to.
pub(super) fn pinned(name: &str) -> Result<String, String> {
    let path = python::root().join("python").join(REQUIREMENTS);
    let text = fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
    pin_in(&text, name).ok_or_else(|| format!("{} pins no {name}", path.display()))
}

/// The version the requirements `text` pins `name` to (`name==version`).
fn pin_in(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (package, version) = line.split_once("==")?;
        (package.trim() == name).then(|| version.trim().to_owned())
    })
}

/// Where the source tree (the sdist, or the source archive) of `name`
/// `version` is unpacked.
pub(super) fn sdist(name: &str, version: &str) -> PathBuf {
    python::root()
        .join("target/refs")
        .join(format!("{name}-dl"))
        .join(format!("{name}-{version}"))
}

/// Downloads and unpacks the source tree of `library` with `python`, and
/// the files it leaves out, unless they are there.
fn fetch(python: &Path, library: &Library) -> Result<(), String> {
    let version = pinned(library.name)?;
    let sdist = sdist(library.name, &version);
    if !sdist.is_dir() {
        let mut command = match library.source {
            Source::Sdist => {
                let mut command = fetch_command(python, "sdist");
                command.args([library.name, &version]);
                command
            }
            Source::Archive { url, sha256 } => {
                let mut command = fetch_command(python, "archive");
                command.args([&url.replace("{version}", &version), sha256]);
                command
            }
        };
        python::run_checked(command.arg(&sdist))?;
    }
    let Some(left_out) = library.left_out else {
        return Ok(());
    };
    for (path, sha256) in left_out.files {
        let dest = sdist.join(path);
        if !dest.is_file() {
            let url = format!("{}/v{version}/{path}", left_out.repository);
            python::run_checked(
                fetch_command(python, "file")
                    .args([url.as_str(), sha256])
                    .arg(dest),
            )?;
        }
    }
    Ok(())
}

/// The command running `python/ecosystem/fetch.py COMMAND` with `python`.
pub(crate) fn fetch_command(python: &Path, command: &str) -> Command {
    let mut fetch = Command::new(python);
    fetch
        .arg("-I")
        .arg(python::root().join("python/ecosystem/fetch.py"))
        .arg(command);
    fetch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_are_read_off_their_lines() {
        let text = "# pandas==0.1 in a comment\nnumpy==2.5.4\npandas == 3.1.0\n";
        assert_eq!(pin_in(text, "pandas").as_deref(), Some("3.1.0"));
        assert_eq!(pin_in(text, "numpy").as_deref(), Some("2.5.4"));
        assert_eq!(pin_in(text, "seaborn"), None);
    }

    #[test]
    fn the_libraries_are_pinned() {
        for library in LIBRARIES {
            assert!(pinned(library.name).is_ok(), "{}", library.name);
        }
    }
}
