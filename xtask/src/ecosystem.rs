//! `cargo xtask ecosystem [setup] [PYTEST ARGS...]`: pltrs (Python) as
//! matplotlib under the libraries that import matplotlib by name, pandas
//! and seaborn (the tests in `python/ecosystem`).
//!
//! * `ecosystem setup` makes the venv `target/ecosystem/venv` with uv:
//!   what `python/requirements-ecosystem.txt` pins (pandas, scipy, pytest
//!   and pltrs' own dependencies), then seaborn without its dependencies,
//!   so that nothing fetches the real matplotlib;
//! * `ecosystem` builds the extension (release) and runs the tests with
//!   `PLTRS_ECOSYSTEM_PYTHON`, else that venv's Python, else the pltrs
//!   venv's (where they skip, without pandas and seaborn), `python/alias`
//!   and `python` first on `PYTHONPATH`, so that `import matplotlib` gives
//!   the source tree's pltrs. The figures go to `target/ecosystem/out`.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

use pltrs_core::os::Os;

use crate::python::{self, Profile};

/// What seaborn is pinned to; installed without its dependencies.
const SEABORN: &str = "seaborn==0.13.2";

/// The Python version of the venv (the other venvs').
const PYTHON_VERSION: &str = "3.14";

/// Where the venv, the figures and pytest's files go.
fn ecosystem_dir() -> PathBuf {
    python::root().join("target").join("ecosystem")
}

/// The ecosystem venv.
fn venv() -> PathBuf {
    ecosystem_dir().join("venv")
}

/// Runs the `ecosystem` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args.first().map(String::as_str) {
        Some("setup") => setup(),
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

/// Makes (or completes) the ecosystem venv.
fn setup() -> Result<(), String> {
    let venv = venv();
    let python = python::interpreter(&venv, Os::CURRENT);
    if !python.exists() {
        python::run_checked(
            Command::new(python::uv())
                .args(["venv", "--python", PYTHON_VERSION])
                .arg(&venv),
        )?;
    }
    python::install(&python, "requirements-ecosystem.txt")?;
    python::run_checked(
        Command::new(python::uv())
            .args(["pip", "install", "--no-deps", "--python"])
            .arg(&python)
            .arg(SEABORN),
    )?;
    println!("ready: {}", venv.display());
    Ok(())
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
        "no {} (`cargo xtask ecosystem setup` makes it): the tests skip without pandas \
         and seaborn",
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
