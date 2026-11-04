//! `cargo xtask python setup`: the two venvs of pltrs's workflow.
//!
//! The reference venv (`.venv`) holds real matplotlib 3.11 for the spec
//! runner and the reference run of matplotlib's suite; setup only adds
//! `python/requirements-reference.txt` to it. The pltrs venv
//! (`.venv-pltrs`) is made with uv, on the reference venv's Python, from
//! `python/requirements-pltrs.txt`, and must not hold matplotlib, so that
//! nothing imports the real one by accident: setup checks that.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use pltrs_core::os::Os;

use super::venv::{interpreter, reference_python};
use super::{root, run_checked};

/// The Python version of both venvs (the reference venv's).
const PYTHON_VERSION: &str = "3.14";

/// The pltrs venv's directory.
fn pltrs_venv() -> PathBuf {
    root().join(".venv-pltrs")
}

/// The pltrs venv's interpreter, or why there is none.
pub(crate) fn pltrs_python() -> Result<PathBuf, String> {
    let python = interpreter(&pltrs_venv(), Os::CURRENT);
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
pub(crate) fn uv() -> PathBuf {
    if let Some(uv) = env::var_os("UV") {
        return uv.into();
    }
    let program = format!("uv{}", env::consts::EXE_SUFFIX);
    env::home_dir()
        .map(|home| home.join(".local").join("bin").join(&program))
        .filter(|path| path.exists())
        .unwrap_or_else(|| program.into())
}

/// Makes (or completes) both venvs.
pub(super) fn run() -> Result<(), String> {
    let root = root();
    let reference = reference_python();
    if !reference.exists() {
        return Err(format!(
            "the reference venv {} is missing (see spec/mpl-reference)",
            reference.display()
        ));
    }
    install(&reference, "requirements-reference.txt")?;

    let venv = pltrs_venv();
    if !interpreter(&venv, Os::CURRENT).exists() {
        run_checked(
            Command::new(uv())
                .args(["venv", "--python", PYTHON_VERSION])
                .arg(&venv),
        )?;
    }
    let python = pltrs_python()?;
    install(&python, "requirements-pltrs.txt")?;

    refuse_matplotlib(&python, &venv)?;
    println!(
        "ready: {} (pltrs) and {} (reference)",
        venv.display(),
        root.join(".venv").display()
    );
    Ok(())
}

/// Fails if `python`, the interpreter of the venv `venv`, can import
/// matplotlib, which a venv running pltrs must not hold.
pub(crate) fn refuse_matplotlib(python: &Path, venv: &Path) -> Result<(), String> {
    let leaked = Command::new(python)
        .args(["-c", "import matplotlib"])
        .output()
        .map_err(|err| format!("cannot run {}: {err}", python.display()))?;
    if leaked.status.success() {
        return Err(format!(
            "{} can import matplotlib; it must not (uninstall it)",
            venv.display()
        ));
    }
    Ok(())
}

/// Installs the pinned list `python/<requirements>` into `python`'s venv.
fn install(python: &Path, requirements: &str) -> Result<(), String> {
    run_checked(&mut install_command(python, &[requirements]))
}

/// The uv command installing the pinned lists `python/<list>` of `lists`
/// together into `python`'s venv.
pub(crate) fn install_command(python: &Path, lists: &[&str]) -> Command {
    let mut command = Command::new(uv());
    command.args(["pip", "install", "--python"]).arg(python);
    for list in lists {
        command.arg("-r").arg(root().join("python").join(list));
    }
    command
}
