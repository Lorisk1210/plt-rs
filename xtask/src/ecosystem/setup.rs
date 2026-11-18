//! `cargo xtask ecosystem setup`: the ecosystem's venvs, and the source
//! trees of the libraries whose suites `ecosystem suite` runs.
//!
//! * the venvs (`venvs`): the shared pair, `target/ecosystem/venv` and
//!   `reference-venv`, and the pairs of the libraries whose pins clash with
//!   theirs. The pltrs side gets its list (the libraries, what their tests
//!   use, pytest and its plugins, pltrs' own dependencies) installed with
//!   matplotlib excluded from the resolution, so that what requires it
//!   (seaborn, plotnine, pytest-mpl) gets `python/alias`; setup checks that
//!   it cannot import matplotlib. The reference side gets the same list and
//!   `python/requirements-ecosystem-reference.txt` (real matplotlib 3.11.2
//!   and its dependencies), resolved together;
//! * the source trees of the libraries at the versions their lists pin, in
//!   `target/refs/<name>-dl/<name>-<version>`: their sdists, or the
//!   source archives of the releases' tags where the sdists ship no
//!   tests (mplfinance, matplotlib-venn, arviz-plots, mne, windrose,
//!   `UpSetPlot`, mlxtend), with the files
//!   of the release's source tree their suites read that the sdists leave
//!   out (pandas' iris data, geopandas' maps, lifelines' conftest), which
//!   `python/ecosystem/fetch.py`
//!   downloads (from `PyPI`, and from the repository at the release's
//!   tag) when they are missing, checking their SHA-256;
//! * the data the libraries' tests read that is in neither (cartopy's
//!   Natural Earth maps, shap's, yellowbrick's, scanpy's and `MetPy`'s
//!   datasets),
//!   checked by SHA-256 (`data`);
//! * the baselines of the image comparisons whose own are for another
//!   version of matplotlib (plotnine's, astropy's, mplfinance's,
//!   nilearn's, mpl-scatter-density's, yellowbrick's,
//!   matplotlib-label-lines' and `MetPy`'s) or missing (missingno's,
//!   ultraplot's and mplhep's), made in the reference venv (`baselines`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pltrs_core::os::Os;

use super::libraries::{self, Library, Source};
use super::{baselines, data, venvs};
use crate::mpl_suite::Mode;
use crate::python;

/// Makes (or completes) the venvs, fetches the missing source trees and
/// makes the missing baselines.
pub(super) fn run() -> Result<(), String> {
    let dir = super::ecosystem_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    libraries::report_left_out();

    let mut made = Vec::new();
    for library in libraries::here() {
        let list = venvs::requirements(&library);
        if made.contains(&list) {
            continue;
        }
        python::make_venv_pair(
            &venvs::venv(&library, Mode::Pltrs),
            &venvs::venv(&library, Mode::Reference),
            &[list.as_str()],
            venvs::REFERENCE,
        )?;
        made.push(list);
    }

    let python = python::interpreter(&super::venv(), Os::CURRENT);
    for library in libraries::here() {
        fetch(&python, &library)?;
        data::fetch(&python, &library)?;
    }
    for library in libraries::here() {
        baselines::make(&library, &venvs::venv(&library, Mode::Reference))?;
    }
    println!(
        "ready: {} (pltrs) and {} (reference), and the venvs of {}",
        super::venv().display(),
        super::reference_venv().display(),
        super::ecosystem_dir().join("venvs").display()
    );
    Ok(())
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
    let version = venvs::pinned(library)?;
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
            let tag = left_out.tag.replace("{version}", &version);
            let url = format!("{}/{tag}/{path}", left_out.repository);
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
