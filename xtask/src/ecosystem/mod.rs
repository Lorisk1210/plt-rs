//! `cargo xtask ecosystem [setup | suite ... | PYTEST ARGS...]`: pltrs
//! (Python) as matplotlib under the libraries that import matplotlib by
//! name: pandas, seaborn, xarray, networkx, scikit-learn, statsmodels,
//! geopandas, plotnine, shap, lifelines, yellowbrick, astropy, cartopy,
//! sympy, nilearn, arviz, scanpy, mne, mplfinance, matplotlib-venn,
//! pytest-mpl, mpl-scatter-density and missingno, and the libraries
//! building on matplotlib's parts: mpld3, matplot2tikz, mplcursors,
//! mpl-interactions, matplotlib-scalebar, matplotlib-label-lines,
//! ultraplot, mplhep, holoviews, mpltern, windrose, python-ternary,
//! pycirclize, statannotations, `UpSetPlot`, pingouin, optuna, mlxtend,
//! `MetPy`, squarify and `SciencePlots` (`libraries`).
//!
//! * `ecosystem setup` makes the ecosystem's venvs with uv (`venvs`), downloads
//!   the libraries' source trees (`setup`) and the data their tests read
//!   (`data`), and makes the baselines of the image comparisons whose own
//!   are for another matplotlib (`baselines`);
//! * `ecosystem suite [--reference] [PATTERN...] [--failures]` runs the
//!   libraries' own test suites against pltrs or matplotlib (`suite`);
//! * `ecosystem [PYTEST ARGS...]` builds the extension (release) and runs
//!   pltrs' smoke figures of the libraries (the tests in
//!   `python/ecosystem`) with `PLTRS_ECOSYSTEM_PYTHON`, else the ecosystem
//!   venv's Python, else the pltrs venv's (where they skip, without
//!   the libraries), `python/alias` and `python` first on
//!   `PYTHONPATH`, so that `import matplotlib` gives the source tree's
//!   pltrs. The figures go to `target/ecosystem/out`.

mod baselines;
mod data;
mod install;
mod libraries;
mod setup;
mod site;
mod suite;
mod venvs;

pub(crate) use setup::fetch_command;

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use pltrs_core::os::Os;

use crate::python::{self, Profile};

/// Where the venv, the figures and pytest's files go.
fn ecosystem_dir() -> PathBuf {
    python::root().join("target").join("ecosystem")
}

/// The ecosystem venv, pltrs' side (no matplotlib).
fn venv() -> PathBuf {
    ecosystem_dir().join("venv")
}

/// The ecosystem's reference venv (real matplotlib).
fn reference_venv() -> PathBuf {
    ecosystem_dir().join("reference-venv")
}

/// Runs the `ecosystem` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args.split_first() {
        Some((command, _)) if command == "setup" => setup::run(),
        Some((command, rest)) if command == "suite" => suite::run(rest),
        _ => python::build(Profile::Release).and_then(|()| test(args)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask ecosystem: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The Python the tests run with (see the module's description).
fn interpreter() -> Result<PathBuf, String> {
    if let Some(python) = env::var_os("PLTRS_ECOSYSTEM_PYTHON") {
        return Ok(python.into());
    }
    let own = python::interpreter(&venv(), Os::CURRENT);
    if own.exists() {
        return Ok(own);
    }
    println!(
        "no {} (`cargo xtask ecosystem setup` makes it): the tests skip without the \
         libraries",
        own.display()
    );
    python::pltrs_python()
}

/// Runs the tests, with the pytest arguments `args`.
fn test(args: &[String]) -> Result<(), String> {
    let root = python::root();
    let dir = ecosystem_dir();
    std::fs::create_dir_all(&dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    let sources = root.join("python");
    let path = env::join_paths([sources.join("alias"), sources.clone()])
        .map_err(|err| format!("cannot set PYTHONPATH: {err}"))?;
    let mut command = Command::new(interpreter()?);
    command
        .args(["-m", "pytest", "-q", "-p", "no:cacheprovider"])
        .arg(python::basetemp(&dir.join("pytest-tmp")))
        .args(args)
        .arg(sources.join("ecosystem"))
        .current_dir(&dir)
        .env("PYTHONPATH", path)
        .env("ECOSYSTEM_OUT", dir.join("out"))
        .env("PLTRSCONFIGDIR", dir.join("config"))
        .env("PLTRS_LOOK", "matplotlib");
    let program: OsString = command.get_program().to_owned();
    python::run_checked(&mut command)
        .map_err(|err| format!("{err} (python: {})", program.display()))
}
