//! `cargo xtask python [--release] | setup | test`: pltrs
//! (Python).
//!
//! * `python [--release]` builds the extension (`crates/pltrs-python`) and
//!   puts it in the package as `python/pltrs/_native.abi3.so`
//!   (`_native.pyd` on Windows), so that `python/` on `PYTHONPATH` is a
//!   working install;
//! * `python setup` makes the pltrs venv (`.venv-pltrs`, without
//!   matplotlib) and adds what the suite runner needs to the reference
//!   venv (`.venv`), from the pinned lists in `python/requirements-*.txt`;
//! * `python test` builds the extension and runs pltrs's own smoke tests
//!   (`python/tests`) in the pltrs venv.

mod setup;
mod venv;

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use pltrs_core::os::Os;

pub(crate) use setup::{install_command, pltrs_python, refuse_matplotlib, uv};
pub(crate) use venv::{interpreter, reference_python, site_packages};

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
    /// `cargo build --profile extension`: the release profile with fat
    /// LTO and one codegen unit, as the wheel ships it.
    Release,
}

impl Profile {
    /// The directory of the target dir the profile builds into.
    fn directory(self) -> &'static str {
        match self {
            Self::Dev => "debug",
            Self::Release => "extension",
        }
    }
}

/// Builds `pltrs-python` and copies the library into the package as
/// `python/pltrs/_native.abi3.so` (`_native.pyd` on Windows).
pub(crate) fn build(profile: Profile) -> Result<(), String> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.args(["build", "-p", "pltrs-python"]);
    if profile == Profile::Release {
        command.args(["--profile", "extension"]);
    }
    run_checked(&mut command)?;
    let (built, installed) = venv::extension_files(Os::CURRENT);
    let built = target_dir().join(profile.directory()).join(built);
    let installed = root().join("python/pltrs").join(installed);
    // Replacing the file (rather than writing over it) keeps a running
    // interpreter that mapped the old library from crashing.
    let mut staging = installed.clone().into_os_string();
    staging.push(".new");
    let staging = PathBuf::from(staging);
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

/// Runs pltrs's own tests (`python/tests`) in the pltrs venv, from a
/// directory under the target directory, so the images the tests write
/// (`result_images/` in the working directory) stay out of the sources.
fn smoke_test() -> Result<(), String> {
    let root = root();
    let scratch = target_dir().join("python-tests");
    fs::create_dir_all(&scratch)
        .map_err(|err| format!("cannot create {}: {err}", scratch.display()))?;
    let mut command = Command::new(pltrs_python()?);
    command
        .args([OsStr::new("-m"), OsStr::new("pytest"), OsStr::new("-q")])
        .arg(basetemp(&scratch.join("pytest-tmp")))
        .arg(root.join("python/tests"))
        .env("PYTHONPATH", python_path([root.join("python")])?)
        .current_dir(&scratch);
    run_checked(&mut command)
}

/// A `PYTHONPATH` of `dirs` followed by the inherited one, so that what
/// the environment adds (the GUI toolkits of `python/tools/toolkits.sh`)
/// stays importable behind them.
pub(crate) fn python_path<I>(dirs: I) -> Result<OsString, String>
where
    I: IntoIterator<Item = PathBuf>,
{
    joined_python_path(dirs, &env::var_os("PYTHONPATH").unwrap_or_default())
}

/// `dirs` followed by the entries of the path list `inherited`, but its
/// empty ones (which would name the working directory).
fn joined_python_path<I>(dirs: I, inherited: &OsStr) -> Result<OsString, String>
where
    I: IntoIterator<Item = PathBuf>,
{
    let inherited = env::split_paths(inherited).filter(|dir| !dir.as_os_str().is_empty());
    env::join_paths(dirs.into_iter().chain(inherited))
        .map_err(|err| format!("cannot set PYTHONPATH: {err}"))
}

/// pytest's option putting its temporary directories (`tmp_path`) in
/// `dir` rather than in the system's temporary directory.
pub(crate) fn basetemp(dir: &Path) -> OsString {
    let mut option = OsString::from("--basetemp=");
    option.push(dir);
    option
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The entries of the path list `list`.
    fn entries(list: &OsStr) -> Vec<PathBuf> {
        env::split_paths(list).collect()
    }

    #[test]
    fn python_paths_put_the_inherited_entries_last() {
        let inherited = env::join_paths(["/toolkits/site", "/other"]).expect("joinable");
        let path = joined_python_path([PathBuf::from("/site")], &inherited).expect("joinable");
        assert_eq!(
            entries(&path),
            ["/site", "/toolkits/site", "/other"].map(PathBuf::from)
        );
    }

    #[test]
    fn python_paths_leave_out_empty_inherited_entries() {
        let path = joined_python_path([PathBuf::from("/site")], OsStr::new("")).expect("joinable");
        assert_eq!(entries(&path), [PathBuf::from("/site")]);
        let separator = if cfg!(windows) { ";" } else { ":" };
        let inherited = OsString::from(format!("{separator}/kit{separator}"));
        let path = joined_python_path([PathBuf::from("/site")], &inherited).expect("joinable");
        assert_eq!(entries(&path), ["/site", "/kit"].map(PathBuf::from));
    }
}
