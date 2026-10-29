//! `cargo xtask python [--release] | setup | test`: pltrs
//! (Python).
//!
//! * `python [--release]` builds the extension (`crates/pltrs-python`) and
//!   puts it in the package as `python/pltrs/_native.abi3.so`, so that
//!   `python/` on `PYTHONPATH` is a working install;
//! * `python setup` makes the pltrs venv (`.venv-pltrs`, without
//!   matplotlib) and adds what the suite runner needs to the reference
//!   venv (`.venv`), from the pinned lists in `python/requirements-*.txt`;
//! * `python test` builds the extension and runs pltrs's own smoke tests
//!   (`python/tests`) in the pltrs venv.

mod setup;

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub(crate) use setup::pltrs_python;

/// The workspace root.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace root")
        .to_owned()
}

/// Runs the `python` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let outcome = match args.first().map(String::as_str) {
        Some("setup") => setup::run(),
        Some("test") => build(Profile::Dev).and_then(|()| smoke_test()),
        None => build(Profile::Dev),
        Some("--release") => build(Profile::Release),
        Some(other) => Err(format!(
            "unknown argument {other:?}; usage: cargo xtask python [--release] | setup | test"
        )),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask python: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The cargo profile the extension is built with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Profile {
    /// `cargo build` (the workspace's dev profile is optimised).
    Dev,
    /// `cargo build --release`.
    Release,
}

impl Profile {
    /// The directory of the target dir the profile builds into.
    fn directory(self) -> &'static str {
        match self {
            Self::Dev => "debug",
            Self::Release => "release",
        }
    }
}

/// Builds `pltrs-python` and copies the library into the package as
/// `python/pltrs/_native.abi3.so`.
pub(crate) fn build(profile: Profile) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.args(["build", "-p", "pltrs-python"]);
    if profile == Profile::Release {
        command.arg("--release");
    }
    run_checked(&mut command)?;
    let built = target_dir()
        .join(profile.directory())
        .join("libpltrs_python.so");
    let installed = root().join("python/pltrs/_native.abi3.so");
    // Replacing the file (rather than writing over it) keeps a running
    // interpreter that mapped the old library from crashing.
    let staging = installed.with_extension("so.new");
    std::fs::copy(&built, &staging)
        .and_then(|_| std::fs::rename(&staging, &installed))
        .map_err(|err| format!("cannot install {}: {err}", built.display()))?;
    println!("installed {}", installed.display());
    Ok(())
}

/// The target directory cargo builds into: `CARGO_TARGET_DIR`, relative
/// to the working directory, else the workspace's `target`.
fn target_dir() -> PathBuf {
    match env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => match env::current_dir() {
            Ok(cwd) => cwd.join(dir),
            Err(_) => dir.into(),
        },
        None => root().join("target"),
    }
}

/// Runs pltrs's own tests (`python/tests`) in the pltrs venv.
fn smoke_test() -> Result<(), String> {
    let root = root();
    let mut command = Command::new(pltrs_python()?);
    command
        .args([OsStr::new("-m"), OsStr::new("pytest"), OsStr::new("-q")])
        .arg(root.join("python/tests"))
        .env("PYTHONPATH", root.join("python"))
        .current_dir(&root);
    run_checked(&mut command)
}

/// Runs `command`, failing unless it exits successfully.
pub(crate) fn run_checked(command: &mut Command) -> Result<(), String> {
    let program = command.get_program().to_string_lossy().into_owned();
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("{program} failed ({status})")),
        Err(err) => Err(format!("cannot run {program}: {err}")),
    }
}
