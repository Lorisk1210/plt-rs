//! `cargo xtask notebooks setup`: the notebooks' two venvs, and the
//! sources of the libraries whose tests `notebooks suite` runs.
//!
//! * `target/notebooks/venv`, pltrs' side: `python/requirements-notebooks.txt`
//!   (Jupyter, `IPython`, matplotlib-inline, ipympl, ipywidgets, pandas,
//!   seaborn, what the tests need, pltrs' own dependencies) installed with
//!   matplotlib excluded from the resolution, so that what requires it
//!   (ipympl) gets `python/alias`; setup checks that the venv cannot
//!   import matplotlib;
//! * `target/notebooks/reference-venv`, matplotlib's side: the same list
//!   and `python/requirements-notebooks-reference.txt` (real matplotlib
//!   3.11.2 and its dependencies), resolved together;
//! * the sdists of `IPython`, matplotlib-inline and ipympl at the versions
//!   the list pins, in `target/refs/<sdist>-dl/<sdist>-<version>`, with
//!   the files of the release's repository their tests need that the
//!   sdists leave out (matplotlib-inline's tests), which
//!   `python/ecosystem/fetch.py` downloads, checking their SHA-256.
//!   matplotlib's notebook tests come from the source tree
//!   `cargo xtask mpl-suite` reads.

use std::fs;
use std::path::Path;

use super::suite::{LIBRARIES, Library, Source};
use crate::ecosystem::fetch_command;
use crate::mpl_suite::Mode;
use crate::python;

/// The pinned list of both venvs.
const REQUIREMENTS: &str = "requirements-notebooks.txt";

/// What the reference venv adds to [`REQUIREMENTS`].
const REFERENCE_REQUIREMENTS: &str = "requirements-notebooks-reference.txt";

/// Makes (or completes) both venvs and fetches the missing sources.
pub(super) fn run() -> Result<(), String> {
    let dir = super::notebooks_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let excludes = dir.join("excludes.txt");
    fs::write(&excludes, "matplotlib\n").map_err(|err| format!("{}: {err}", excludes.display()))?;

    let venv = super::venv(Mode::Pltrs);
    let python = python::make_venv(&venv)?;
    python::run_checked(
        python::install_command(&python, &[REQUIREMENTS])
            .arg("--excludes")
            .arg(&excludes),
    )?;
    python::refuse_matplotlib(&python, &venv)?;

    let reference = super::venv(Mode::Reference);
    let reference_python = python::make_venv(&reference)?;
    python::run_checked(&mut python::install_command(
        &reference_python,
        &[REQUIREMENTS, REFERENCE_REQUIREMENTS],
    ))?;

    for library in &LIBRARIES {
        fetch(&python, library)?;
    }
    println!(
        "ready: {} (pltrs) and {} (reference)",
        venv.display(),
        reference.display()
    );
    Ok(())
}

/// Downloads and unpacks the sdist of `library` with `python`, and the
/// files it leaves out, unless they are there.
fn fetch(python: &Path, library: &Library) -> Result<(), String> {
    let tree = library.tree();
    match library.source {
        Source::Sdist(sdist) if !tree.is_dir() => {
            let refs = tree
                .parent()
                .ok_or("an sdist is unpacked in a directory of its own")?;
            python::run_checked(
                fetch_command(python, "sdist")
                    .args([sdist, library.version])
                    .arg(refs),
            )?;
        }
        Source::Matplotlib if !tree.is_dir() => {
            return Err(format!(
                "matplotlib's source tree {} is missing (set MPL_SDIST)",
                tree.display()
            ));
        }
        Source::Sdist(_) | Source::Matplotlib => {}
    }
    for (path, sha256) in library.left_out {
        let dest = tree.join(path);
        if !dest.is_file() {
            let url = format!("{}/{path}", library.repository);
            python::run_checked(
                fetch_command(python, "file")
                    .args([url.as_str(), sha256])
                    .arg(dest),
            )?;
        }
    }
    Ok(())
}
