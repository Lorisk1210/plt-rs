//! `cargo xtask python setup`: the two venvs of pltrs's workflow.
//!
//! The reference venv (`.venv`) holds real matplotlib 3.11 for the spec
//! runner and the reference run of matplotlib's suite; setup only adds
//! `python/requirements-reference.txt` to it. The pltrs venv
//! (`.venv-pltrs`) is made with uv, on the reference venv's Python, from
//! `python/requirements-pltrs.txt`, and must not hold matplotlib, so that
//! nothing imports the real one by accident: setup checks that. Both get
//! `python/requirements-suite.txt` too, the packages matplotlib's tests
//! skip without (pandas, tornado, the Qt bindings), resolved with the
//! venv's own list. Rerunning setup brings existing venvs to the lists.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pltrs_core::os::Os;

use super::venv::{interpreter, reference_python};
use super::{root, run_checked};

/// The Python version of both venvs (the reference venv's).
const PYTHON_VERSION: &str = "3.14";

/// The pinned list both venvs of matplotlib's suite get besides their own.
const SUITE_REQUIREMENTS: &str = "requirements-suite.txt";

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
    let python = make_venv(&venv)?;
    install(&python, "requirements-pltrs.txt")?;

    refuse_matplotlib(&python, &venv)?;
    println!(
        "ready: {} (pltrs) and {} (reference)",
        venv.display(),
        root.join(".venv").display()
    );
    Ok(())
}

/// Makes the venv `venv` with uv, on the Python of the other venvs,
/// unless it exists, and returns its interpreter.
fn make_venv(venv: &Path) -> Result<PathBuf, String> {
    let python = interpreter(venv, Os::CURRENT);
    if !python.exists() {
        run_checked(
            Command::new(uv())
                .args(["venv", "--python", PYTHON_VERSION])
                .arg(venv),
        )?;
    }
    Ok(python)
}

/// What [`refuse_matplotlib`] runs: exits 0 when `matplotlib` imports and
/// is matplotlib, 3 when it is pltrs' own `matplotlib` distribution (the
/// aliases loading pltrs, which the corpus driver installs).
const REAL_MATPLOTLIB: &str = "import pathlib, matplotlib\n\
     raise SystemExit(3 if pathlib.Path(matplotlib.__file__).parent.name == 'pltrs' else 0)";

/// Fails if `python`, the interpreter of the venv `venv`, can import
/// matplotlib, which a venv running pltrs must not hold (pltrs' own
/// `matplotlib` distribution aside).
fn refuse_matplotlib(python: &Path, venv: &Path) -> Result<(), String> {
    let leaked = Command::new(python)
        .args(["-c", REAL_MATPLOTLIB])
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

/// Installs the pinned list `python/<requirements>` into `python`'s venv,
/// with the suite's ([`SUITE_REQUIREMENTS`]).
fn install(python: &Path, requirements: &str) -> Result<(), String> {
    run_checked(&mut install_command(
        python,
        &[requirements, SUITE_REQUIREMENTS],
    ))
}

/// The uv command installing the pinned lists `python/<list>` of `lists`
/// together into `python`'s venv.
fn install_command(python: &Path, lists: &[&str]) -> Command {
    let mut command = Command::new(uv());
    command.args(["pip", "install", "--python"]).arg(python);
    for list in lists {
        command.arg("-r").arg(root().join("python").join(list));
    }
    command
}

/// Makes (or completes) two venvs of the pinned lists `python/<list>` of
/// `lists`, and returns their interpreters: `pltrs`, the lists installed
/// with matplotlib excluded from the resolution, so that what requires it
/// gets `python/alias` (checked: it cannot import matplotlib), and
/// `reference`, the lists and `reference_list` (real matplotlib) resolved
/// together.
pub(crate) fn make_venv_pair(
    pltrs: &Path,
    reference: &Path,
    lists: &[&str],
    reference_list: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let pltrs_python = make_venv(pltrs)?;
    let excludes = pltrs.join("excludes.txt");
    fs::write(&excludes, "matplotlib\n").map_err(|err| format!("{}: {err}", excludes.display()))?;
    run_checked(
        install_command(&pltrs_python, lists)
            .arg("--excludes")
            .arg(&excludes),
    )?;
    refuse_matplotlib(&pltrs_python, pltrs)?;
    let reference_python = make_venv(reference)?;
    run_checked(&mut install_command(
        &reference_python,
        &[lists, &[reference_list]].concat(),
    ))?;
    Ok((pltrs_python, reference_python))
}
