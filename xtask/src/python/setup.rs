//! `cargo xtask python setup`: the two venvs of pltrs's workflow.
//!
//! The reference venv (`.venv`) holds real matplotlib 3.11 for the spec
//! runner and the reference run of matplotlib's suite; setup only adds
//! `python/requirements-reference.txt` to it. The pltrs venv
//! (`.venv-pltrs`) is made with uv, on the reference venv's Python, from
//! `python/requirements-pltrs.txt`, and must not hold matplotlib, so that
//! nothing imports the real one by accident: setup checks that.

use std::env;
use std::path::PathBuf;
use std::process::Command;

use super::{root, run_checked};

/// The Python version of both venvs (the reference venv's).
const PYTHON_VERSION: &str = "3.14";

/// The pltrs venv's directory.
fn pltrs_venv() -> PathBuf {
    root().join(".venv-pltrs")
}

/// The pltrs venv's interpreter, or why there is none.
pub(crate) fn pltrs_python() -> Result<PathBuf, String> {
    let python = pltrs_venv().join("bin/python");
    if python.exists() {
        Ok(python)
    } else {
        Err(format!(
            "{} does not exist: run `cargo xtask python setup` first",
            python.display()
        ))
    }
}

/// The uv executable: `UV`, else `~/.local/bin/uv` where the installer
/// puts it, else `uv` on `PATH`.
fn uv() -> PathBuf {
    if let Some(uv) = env::var_os("UV") {
        return uv.into();
    }
    env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/bin/uv"))
        .filter(|path| path.exists())
        .unwrap_or_else(|| "uv".into())
}

/// Makes (or completes) both venvs.
pub(super) fn run() -> Result<(), String> {
    let root = root();
    let reference = root.join(".venv/bin/python");
    if !reference.exists() {
        return Err(format!(
            "the reference venv {} is missing (see spec/mpl-reference)",
            reference.display()
        ));
    }
    install(&reference, "requirements-reference.txt")?;

    let venv = pltrs_venv();
    if !venv.join("bin/python").exists() {
        run_checked(
            Command::new(uv())
                .args(["venv", "--python", PYTHON_VERSION])
                .arg(&venv),
        )?;
    }
    let python = pltrs_python()?;
    install(&python, "requirements-pltrs.txt")?;

    let leaked = Command::new(&python)
        .args(["-c", "import matplotlib"])
        .output()
        .map_err(|err| format!("cannot run {}: {err}", python.display()))?;
    if leaked.status.success() {
        return Err(format!(
            "{} can import matplotlib; it must not (uninstall it)",
            venv.display()
        ));
    }
    println!(
        "ready: {} (pltrs) and {} (reference)",
        venv.display(),
        root.join(".venv").display()
    );
    Ok(())
}

/// Installs the pinned list `python/<requirements>` into `python`'s venv.
fn install(python: &std::path::Path, requirements: &str) -> Result<(), String> {
    run_checked(
        Command::new(uv())
            .args(["pip", "install", "--python"])
            .arg(python)
            .arg("-r")
            .arg(root().join("python").join(requirements)),
    )
}
