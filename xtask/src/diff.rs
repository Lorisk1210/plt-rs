//! `cargo xtask diff [--gallery | --generated] [PATTERN...] [--failures] [--reduce]
//! [--baseline REPORT] [--jobs N] [--no-build]`: the differential tester of
//! pltrs (Python) against matplotlib.
//!
//! Programs run under real matplotlib (the reference venv) and under pltrs
//! (Python) (the pltrs venv, the import changed, in matplotlib's look), and
//! everything they compute is compared, call by call: the types of the
//! values (with matplotlib's names), the values, the exceptions, the
//! warnings, the printed output and, at the end, every figure as a PNG
//! (RMS ≤ 1, the spec's tolerance). The programs are matplotlib's gallery
//! (the `galleries/` of the sdist `MPL_SDIST` names, as for `mpl-suite`,
//! run whole) and programs generated from matplotlib's API
//! (`spec/diff/generators`, run statement by statement). Put TeX and
//! Ghostscript on `PATH`, as the reference runs had them.
//!
//! The work is done by `spec/diff/driver.py` in the reference venv; this
//! command builds the extension first (release, as `mpl-suite` does;
//! `--no-build` skips it) and hands the driver the paths. The report goes
//! to `target/diff/report.md` (the differences grouped by cause, the
//! biggest groups first, each with a reproduction; the gallery examples
//! with differences; the programs matplotlib cannot run here) and
//! `target/diff/report.json`, and is printed. `--failures` lists every
//! difference of each group, `--reduce` reduces the first of each group to
//! the fewest statements still showing it, `--baseline` adds the counts of
//! an earlier `report.json`. A PATTERN keeps the programs whose names
//! contain it (`gallery/examples/color`, `generated/artists/line`).
//! Differences proven intended are listed in `spec/diff/allowlist.txt`.

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// The programs a run takes, by default both suites.
const SUITES: [&str; 2] = ["--gallery", "--generated"];

/// The flags passed on to the driver as they are, each with its value.
const VALUE_FLAGS: [&str; 2] = ["--baseline", "--jobs"];

/// Runs the `diff` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    match run_diff(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask diff: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_diff(args: &[String]) -> Result<(), String> {
    let root = crate::python::root();
    let sdist = env::var_os("MPL_SDIST").map_or_else(
        || root.join("target/refs/matplotlib-dl/matplotlib-3.11.2"),
        PathBuf::from,
    );
    let mut build = true;
    let mut driver = Command::new(root.join(".venv/bin/python"));
    driver
        .arg(root.join("spec/diff/driver.py"))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(root.join("target/diff"))
        .arg("--gallery")
        .arg(sdist.join("galleries"));
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--no-build" => build = false,
            suite if SUITES.contains(&suite) => {
                driver.args(["--suite", &suite[2..]]);
            }
            flag if VALUE_FLAGS.contains(&flag) => {
                let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
                driver.args([flag, value]);
            }
            "--failures" | "--reduce" => {
                driver.arg(arg);
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            pattern => {
                driver.arg(pattern);
            }
        }
    }
    if build {
        crate::python::build(crate::python::Profile::Release)?;
    }
    driver
        .arg("--pltrs-python")
        .arg(crate::python::pltrs_python()?);
    crate::python::run_checked(&mut driver)
}
