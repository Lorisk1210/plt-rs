//! `cargo xtask diff [--gallery] [--generated] [PATTERN...] [--failures] [--reduce]
//! [--baseline REPORT] [--jobs N] [--no-build] [--coverage | --docs [--strict]]`:
//! the differential tester of pltrs (Python) against matplotlib.
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
//!
//! `--coverage` measures the programs instead (`spec/diff/coverage.py`,
//! no extension needed): which of matplotlib's public callables and of
//! their keyword parameters the programs call under matplotlib, per module
//! and class, written to `target/diff/coverage.md` and `coverage.json`.
//!
//! `--docs` compares matplotlib's public callables themselves
//! (`spec/diff/docs.py`): their signatures, docstrings, `__module__` and
//! `__qualname__` under matplotlib and under pltrs (Python), per module,
//! written to `target/diff/docs.md` and `docs.json`. A PATTERN keeps the
//! callables whose keys (`module:Owner.name`) contain it, `--failures`
//! prints every difference and `--strict` fails while any remains.

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// The flags choosing the programs a run takes, with the driver's names
/// of their suites; without either (or with both), a run takes both.
const SUITES: [(&str, &str); 2] = [("--gallery", "gallery"), ("--generated", "generated")];

/// The flags passed on to the script as they are, each with its value.
const VALUE_FLAGS: [&str; 2] = ["--baseline", "--jobs"];

/// The flags passed on to the script as they are, without a value.
const SWITCHES: [&str; 3] = ["--failures", "--reduce", "--strict"];

/// What a run measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The programs, run under both implementations and compared.
    Programs,
    /// Which of matplotlib's callables the programs reach.
    Coverage,
    /// matplotlib's callables' signatures, docstrings and names in pltrs.
    Docs,
}

impl Mode {
    /// The mode `args` choose.
    fn of(args: &[String]) -> Result<Self, String> {
        let flag = |name: &str| args.iter().any(|arg| arg == name);
        match (flag("--coverage"), flag("--docs")) {
            (true, true) => {
                Err("--coverage and --docs measure different things; give one".to_owned())
            }
            (true, false) => Ok(Self::Coverage),
            (false, true) => Ok(Self::Docs),
            (false, false) => Ok(Self::Programs),
        }
    }

    /// The script of `spec/diff` doing the work.
    fn script(self) -> &'static str {
        match self {
            Self::Programs => "driver.py",
            Self::Coverage => "coverage.py",
            Self::Docs => "docs.py",
        }
    }

    /// Whether the mode runs programs (and takes the suites).
    fn runs_programs(self) -> bool {
        self != Self::Docs
    }

    /// Whether the mode runs pltrs (Python), which needs the extension.
    fn runs_pltrs(self) -> bool {
        self != Self::Coverage
    }
}

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
    let mode = Mode::of(args)?;
    let mut build = mode.runs_pltrs();
    let mut suites = Vec::new();
    let mut script = Command::new(root.join(".venv/bin/python"));
    script
        .arg(root.join("spec/diff").join(mode.script()))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(root.join("target/diff"));
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--no-build" => build = false,
            "--coverage" | "--docs" => {}
            flag if SUITES.iter().any(|&(name, _)| name == flag) => {
                suites.push(flag);
            }
            flag if VALUE_FLAGS.contains(&flag) => {
                let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
                script.args([flag, value]);
            }
            flag if SWITCHES.contains(&flag) => {
                script.arg(flag);
            }
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}")),
            pattern => {
                script.arg(pattern);
            }
        }
    }
    if mode.runs_programs() {
        let sdist = env::var_os("MPL_SDIST").map_or_else(
            || root.join("target/refs/matplotlib-dl/matplotlib-3.11.2"),
            PathBuf::from,
        );
        script
            .arg("--gallery")
            .arg(sdist.join("galleries"))
            .args(["--suite", suite_of(&suites)]);
    } else if !suites.is_empty() {
        return Err("--docs takes no suite: it compares callables, not programs".to_owned());
    }
    if build {
        crate::python::build(crate::python::Profile::Release)?;
    }
    if mode.runs_pltrs() {
        script
            .arg("--pltrs-python")
            .arg(crate::python::pltrs_python()?);
    }
    crate::python::run_checked(&mut script)
}

/// The driver's name of the programs the suite flags `given` choose: one
/// suite when only its flag is given, else both (`all`).
fn suite_of(given: &[&str]) -> &'static str {
    match SUITES
        .iter()
        .filter(|(flag, _)| given.contains(flag))
        .collect::<Vec<_>>()[..]
    {
        [(_, suite)] => suite,
        _ => "all",
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, suite_of};

    #[test]
    fn suite_flags_add_up() {
        assert_eq!(suite_of(&[]), "all");
        assert_eq!(suite_of(&["--gallery"]), "gallery");
        assert_eq!(suite_of(&["--generated", "--generated"]), "generated");
        assert_eq!(suite_of(&["--gallery", "--generated"]), "all");
    }

    #[test]
    fn modes_follow_their_flags() {
        let args = |flags: &[&str]| {
            flags
                .iter()
                .map(|&flag| flag.to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(Mode::of(&args(&["--failures"])), Ok(Mode::Programs));
        assert_eq!(Mode::of(&args(&["--coverage"])), Ok(Mode::Coverage));
        assert_eq!(Mode::of(&args(&["colors", "--docs"])), Ok(Mode::Docs));
        assert!(Mode::of(&args(&["--docs", "--coverage"])).is_err());
        assert!(Mode::Docs.runs_pltrs() && !Mode::Docs.runs_programs());
        assert!(Mode::Coverage.runs_programs() && !Mode::Coverage.runs_pltrs());
        assert_eq!(Mode::Docs.script(), "docs.py");
    }
}
