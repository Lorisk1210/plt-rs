//! `cargo xtask notebooks [setup | suite ... | PATTERN... [--failures]]`:
//! Jupyter and `IPython` end to end, with pltrs (Python) as matplotlib.
//!
//! * `notebooks setup` makes the notebooks' two venvs with uv and
//!   downloads the sources of the libraries whose tests `notebooks suite`
//!   runs (`setup`);
//! * `notebooks [PATTERN...] [--failures]` builds the extension (release),
//!   executes the notebooks of `spec/notebooks` (those whose names contain
//!   a pattern) in a real Jupyter kernel twice, once with real matplotlib
//!   and once with pltrs as matplotlib, and compares everything the two
//!   kernels sent (`run`, `compare`);
//! * `notebooks suite [--reference] [PATTERN...] [--failures]` runs the
//!   matplotlib tests of `IPython`, matplotlib-inline and ipympl, and
//!   matplotlib's own notebook tests, against matplotlib or pltrs
//!   (`suite`).
//!
//! The outcomes go to `spec/notebooks/reference.json` and
//! `spec/notebooks/pltrs.json`, committed, as `cargo xtask ecosystem`'s
//! go to `spec/ecosystem/`: one id per test of the libraries, and one per
//! comparison of the notebooks (`notebooks/<name>.ipynb::...`), which
//! passes under matplotlib by definition and under pltrs when pltrs sent
//! the same. The report is `mpl-suite`'s.
//!
//! Both venvs hold Jupyter's kernel and clients, `IPython`, matplotlib-inline,
//! ipympl, ipywidgets, pandas and seaborn (`python/requirements-notebooks.txt`):
//! `target/notebooks/reference-venv` with real matplotlib too, and
//! `target/notebooks/venv` without it, where `python/alias` on
//! `PYTHONPATH` makes `import matplotlib` give the source tree's pltrs,
//! in the kernels too. Every Python process of a mode keeps its
//! configuration under `target/notebooks/<mode>/`: `IPython`'s profile,
//! Jupyter's directories and matplotlib's configuration directory.

mod compare;
mod record;
mod run;
mod setup;
mod suite;

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pltrs_core::os::Os;

use crate::mpl_suite::Mode;
use crate::python;

/// The version of matplotlib the reference runs have.
const MATPLOTLIB: &str = "3.11.2";

/// Runs the `notebooks` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args.split_first() {
        Some((command, _)) if command == "setup" => setup::run(),
        Some((command, rest)) if command == "suite" => suite::run(rest),
        _ => run::run(args),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask notebooks: {err}");
            ExitCode::FAILURE
        }
    }
}

/// Where the venvs, the records and the runs' files go.
fn notebooks_dir() -> PathBuf {
    python::root().join("target").join("notebooks")
}

/// The venv of `mode`.
fn venv(mode: Mode) -> PathBuf {
    notebooks_dir().join(match mode {
        Mode::Reference => "reference-venv",
        Mode::Pltrs => "venv",
    })
}

/// The interpreter of `mode`'s venv, if setup made it.
fn interpreter(mode: Mode) -> Result<PathBuf, String> {
    let python = python::interpreter(&venv(mode), Os::CURRENT);
    if python.exists() {
        Ok(python)
    } else {
        Err(format!(
            "{} does not exist: run `cargo xtask notebooks setup` first",
            python.display()
        ))
    }
}

/// The committed outcome file of `mode`.
fn outcome_file(mode: Mode) -> PathBuf {
    python::root()
        .join("spec/notebooks")
        .join(format!("{}.json", mode.name()))
}

/// The details file of `mode` (the reasons of failures).
fn details_file(mode: Mode) -> PathBuf {
    notebooks_dir().join(mode.name()).join("details.json")
}

/// The versions the outcome files name: the libraries' and matplotlib's.
fn versions() -> Vec<(&'static str, &'static str)> {
    suite::LIBRARIES
        .iter()
        .filter(|library| library.name != "matplotlib")
        .map(|library| (library.name, library.version))
        .chain([("matplotlib", MATPLOTLIB)])
        .collect()
}

/// The variables of every Python process of `mode` whose files go to
/// `dir`: the venv's scripts first on `PATH` (`jupyter`), `IPython`'s
/// profile and Jupyter's configuration, data and runtime directories in
/// `dir`, one hash seed (so that sets of strings, which messages list,
/// iterate in one order in every run) and no warning of the debugger
/// about frozen modules.
fn environment(mode: Mode, dir: &Path) -> Result<Vec<(&'static str, OsString)>, String> {
    let scripts = interpreter(mode)?
        .parent()
        .ok_or("a venv's interpreter is in its scripts' directory")?
        .to_owned();
    let inherited = env::var_os("PATH").unwrap_or_default();
    let path = env::join_paths([scripts].into_iter().chain(env::split_paths(&inherited)))
        .map_err(|err| format!("cannot set PATH: {err}"))?;
    let jupyter = dir.join("jupyter");
    Ok(vec![
        ("PATH", path),
        ("IPYTHONDIR", dir.join("ipython").into()),
        ("JUPYTER_CONFIG_DIR", jupyter.join("config").into()),
        ("JUPYTER_DATA_DIR", jupyter.join("data").into()),
        ("JUPYTER_RUNTIME_DIR", jupyter.join("runtime").into()),
        ("JUPYTER_PLATFORM_DIRS", "1".into()),
        ("PYTHONHASHSEED", "0".into()),
        ("PYDEVD_DISABLE_FILE_VALIDATION", "1".into()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outcome_files_name_the_versions_tested() {
        assert_eq!(
            versions(),
            [
                ("ipython", "9.18.1"),
                ("matplotlib-inline", "0.2.2"),
                ("ipympl", "0.10.0"),
                ("matplotlib", "3.11.2"),
            ]
        );
    }
}
