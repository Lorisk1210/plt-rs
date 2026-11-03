//! Running the examples: the Rust binaries, and the Python scripts under
//! real matplotlib and under pltrs.
//!
//! Every run draws in pltrs' modern look whatever the environment asks
//! for, so that the images agree from one machine to the next: the Rust
//! examples and pltrs get `PLTRS_LOOK=modern` and no user configuration,
//! and matplotlib applies the modern look's style sheet
//! (`run_matplotlib.py`).

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use pltrs_core::os::Os;

use super::catalogue::{self, Layout};

/// The environment variables that would change how pltrs draws.
const LOOK_VARIABLES: &[&str] = &["PLTRS_FIGURE_LOOK", "PLTRS_CHROME_LOOK", "PLTRSRC"];

/// `command` drawing in the modern look, ignoring the look and the rc
/// file the environment names.
fn in_modern_look(command: &mut Command) -> &mut Command {
    for variable in LOOK_VARIABLES {
        command.env_remove(variable);
    }
    command.env("PLTRS_LOOK", "modern")
}

/// The last line `output` wrote to stderr, for a one-line report.
fn last_error_line(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr.lines().last().unwrap_or("no message").to_owned()
}

/// Why running `program` for the `version` of the example `name` failed,
/// if it did.
fn failure(
    name: &str,
    version: &str,
    program: &Path,
    output: io::Result<Output>,
) -> Option<String> {
    match output {
        Ok(output) if output.status.success() => None,
        Ok(output) => Some(format!("{name} ({version}): {}", last_error_line(&output))),
        Err(err) => Some(format!(
            "{name} ({version}): cannot run {}: {err}",
            program.display()
        )),
    }
}

/// Builds the Rust examples in the profile this xtask was built in, and
/// returns the directory of their binaries: this program's own.
pub(super) fn build_rust() -> Result<PathBuf, String> {
    let exe = env::current_exe().map_err(|err| format!("cannot locate xtask: {err}"))?;
    let bin_dir = exe
        .parent()
        .ok_or("xtask runs from a directory")?
        .to_owned();
    let profile = match bin_dir.file_name().and_then(|name| name.to_str()) {
        Some("debug") | None => "dev",
        Some(name) => name,
    };
    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let status = Command::new(cargo)
        .args([
            "build",
            "--quiet",
            "--profile",
            profile,
            "-p",
            "pltrs-examples",
            "--bins",
        ])
        .status()
        .map_err(|err| format!("cannot run cargo: {err}"))?;
    if status.success() {
        Ok(bin_dir)
    } else {
        Err(format!("building the Rust examples failed: {status}"))
    }
}

/// Runs the binary of each example in `bin_dir` with `out` as its working
/// directory, where it writes its image; returns why each failing one
/// failed.
pub(super) fn run_rust(bin_dir: &Path, names: &[String], out: &Path) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| {
            let binary = bin_dir.join(format!("{name}{}", env::consts::EXE_SUFFIX));
            let output = in_modern_look(&mut Command::new(&binary))
                .current_dir(out)
                .output();
            failure(name, "Rust", &binary, output)
        })
        .collect()
}

/// The scripts of the examples `names`.
fn scripts(layout: &Layout, names: &[String]) -> Vec<PathBuf> {
    let dir = layout.python_sources();
    names
        .iter()
        .map(|name| dir.join(format!("{name}.py")))
        .collect()
}

/// Runs the Python examples `names` under real matplotlib in the
/// reference venv, writing their images to `out` (`run_matplotlib.py`
/// answers their `pltrs` imports with matplotlib, shows the traceback of
/// each failing script and prints its name); returns why each failing
/// one failed.
pub(super) fn run_matplotlib(layout: &Layout, names: &[String], out: &Path) -> Vec<String> {
    let python = crate::python::reference_python();
    let driver = layout.root().join("xtask/src/examples/run_matplotlib.py");
    let output = Command::new(&python)
        .arg(driver)
        .arg(out)
        .args(scripts(layout, names))
        .stderr(Stdio::inherit())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(err) => {
            return vec![format!(
                "the Python examples: cannot run the reference venv's {}: {err}",
                python.display()
            )];
        }
    };
    let failed = String::from_utf8_lossy(&output.stdout);
    let mut problems: Vec<String> = names
        .iter()
        .filter(|name| {
            failed.lines().any(|line| line == name.as_str())
                || catalogue::image(out, name).is_none()
        })
        .map(|name| format!("{name} (Python under matplotlib): see the traceback above"))
        .collect();
    if problems.is_empty() && !output.status.success() {
        problems.push(format!("run_matplotlib.py failed: {}", output.status));
    }
    problems
}

/// A Python that imports pltrs.
#[derive(Debug)]
pub(super) struct Pltrs {
    /// The interpreter.
    python: PathBuf,
    /// Its `PYTHONPATH`.
    path: OsString,
}

impl Pltrs {
    /// The interpreter that runs pltrs: `PLTRS_PYTHON`, else the pltrs
    /// venv's (`.venv-pltrs`), with `python/` (where pltrs and its native
    /// extension are built) put before the inherited `PYTHONPATH`. An error
    /// says why pltrs cannot run the examples.
    pub(super) fn find(layout: &Layout) -> Result<Self, String> {
        let python = env::var_os("PLTRS_PYTHON").map_or_else(
            || crate::python::interpreter(&layout.root().join(".venv-pltrs"), Os::CURRENT),
            PathBuf::from,
        );
        if !python.exists() {
            return Err(format!(
                "no {} (set PLTRS_PYTHON to choose one)",
                python.display()
            ));
        }
        let path = crate::python::python_path([layout.root().join("python")])?;
        let output = Command::new(&python)
            .args(["-c", "import pltrs.pyplot"])
            .env("PYTHONPATH", &path)
            .output()
            .map_err(|err| format!("cannot run {}: {err}", python.display()))?;
        if output.status.success() {
            Ok(Self { python, path })
        } else {
            Err(format!(
                "{} cannot import pltrs: {}",
                python.display(),
                last_error_line(&output)
            ))
        }
    }

    /// Runs the Python examples `names`, each as a script of its own with
    /// `out` as its working directory; returns why each failing one failed.
    pub(super) fn run(&self, layout: &Layout, names: &[String], out: &Path) -> Vec<String> {
        names
            .iter()
            .zip(scripts(layout, names))
            .filter_map(|(name, script)| {
                let output = in_modern_look(&mut Command::new(&self.python))
                    .arg(&script)
                    .env("PYTHONPATH", &self.path)
                    .current_dir(out)
                    .output();
                failure(name, "Python under pltrs", &self.python, output)
            })
            .collect()
    }
}
