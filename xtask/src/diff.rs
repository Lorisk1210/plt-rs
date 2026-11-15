//! `cargo xtask diff [--gallery] [--generated] [--corpus [NAME...]] [PATTERN...]
//! [--failures] [--reduce] [--baseline REPORT] [--jobs N] [--no-build]
//! [--coverage | --docs [--strict] | --types [--strict] | --private [--strict]]`:
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
//! Ghostscript on `PATH`, as the reference runs had them. Both run
//! offline, through a refusing proxy as the ecosystem suites do, so that
//! the gallery's timeline takes matplotlib's fallback for GitHub's list of
//! releases under both implementations.
//!
//! `--corpus` runs programs users of matplotlib wrote instead: the corpora
//! of `spec/diff/corpora.toml` (books', cheat sheets' and galleries' code,
//! each at a commit, with its licence and SHA-256), which the driver
//! fetches with git into `target/diff/corpora` and runs in a copy of their
//! trees, notebooks and Markdown pages cell by cell, offline. A NAME after
//! it (any PATTERN naming a corpus) keeps that corpus; the report then
//! counts each corpus' programs and clean ones. The corpora run in venvs of
//! their own, `target/diff/venvs/pltrs` and `reference`, holding the
//! packages their programs import (`python/requirements-corpora.txt`),
//! which the command makes or brings up to that list first.
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
//! and every public module-level name (re-exports, constants, modules as
//! attributes) by its value's type and value, written to
//! `target/diff/docs.md` and `docs.json`. A PATTERN keeps the callables
//! and names whose keys (`module:Owner.name`) contain it, `--failures`
//! prints every difference and `--strict` fails while any remains.
//!
//! `--types` holds matplotlib's type stubs, which pltrs ships
//! (`python/pltrs/*.pyi`), against pltrs (Python) at run time
//! (`spec/diff/stubs.py`): `mypy.stubtest` with matplotlib's allowlists,
//! first against real matplotlib (its own errors), then against pltrs; each
//! error of the second the first does not have is a difference from
//! matplotlib's public API as its stubs describe it, written by kind and
//! module to `target/diff/types.md` and `types.json`. Its venvs,
//! `target/diff/venvs/types-pltrs` and `types-reference`, hold mypy as
//! matplotlib's CI has it (`python/requirements-types.txt`); the command
//! makes them or brings them up to that list first. A PATTERN keeps the
//! errors whose names contain it, `--failures` prints every one and
//! `--strict` fails on any not in `spec/diff/types-allowlist.txt`.
//!
//! `--private` measures the private names of matplotlib's objects that
//! other libraries use (`spec/diff/private.py`), the rows of the census
//! `spec/diff/private_census.json` (`private_census.py` writes it from the
//! libraries' sources): whether pltrs (Python) has each where the
//! generators' scenes hold an instance of its class (built, drawn,
//! unpickled, cleared) and on objects made alone, with matplotlib's type
//! and value, and, for the names libraries write, whether an assignment
//! has matplotlib's effect, written to `target/diff/private.md` and
//! `private.json`. A PATTERN keeps the rows whose keys (`owner:name`)
//! contain it, `--failures` prints every difference and `--strict` fails
//! while any remains.

mod venv_lock;

use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// The flags choosing the programs a run takes, with the driver's names
/// of their suites; without any (or with the first two), a run takes the
/// first two. The corpora run alone.
const SUITES: [(&str, &str); 3] = [
    ("--gallery", "gallery"),
    ("--generated", "generated"),
    ("--corpus", CORPUS),
];

/// The driver's name of the corpora's suite.
const CORPUS: &str = "corpus";

/// The pinned list of the corpora's venvs, in `python/`.
const CORPUS_REQUIREMENTS: &str = "requirements-corpora.txt";

/// The pinned list of the venvs of `--types`, in `python/`.
const TYPES_REQUIREMENTS: &str = "requirements-types.txt";

/// What the reference venvs of the corpora and of `--types` add to their
/// lists: real matplotlib 3.11.2, as the ecosystem's reference venvs hold it.
const MATPLOTLIB_REQUIREMENTS: &str = "requirements-ecosystem-reference.txt";

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
    /// matplotlib's callables' signatures, docstrings and names, and its
    /// modules' public names, in pltrs.
    Docs,
    /// matplotlib's type stubs against pltrs (Python) at run time.
    Types,
    /// The private names of matplotlib's objects libraries use, in pltrs.
    Private,
}

impl Mode {
    /// The mode `args` choose.
    fn of(args: &[String]) -> Result<Self, String> {
        let modes = [
            ("--coverage", Self::Coverage),
            ("--docs", Self::Docs),
            ("--types", Self::Types),
            ("--private", Self::Private),
        ];
        let given: Vec<_> = modes
            .iter()
            .filter(|(flag, _)| args.iter().any(|arg| arg == flag))
            .collect();
        match given[..] {
            [] => Ok(Self::Programs),
            [&(_, mode)] => Ok(mode),
            _ => Err(
                "--coverage, --docs, --types and --private measure different things; give one"
                    .to_owned(),
            ),
        }
    }

    /// The script of `spec/diff` doing the work.
    fn script(self) -> &'static str {
        match self {
            Self::Programs => "driver.py",
            Self::Coverage => "coverage.py",
            Self::Docs => "docs.py",
            Self::Types => "stubs.py",
            Self::Private => "private.py",
        }
    }

    /// Whether the mode runs programs (and takes the suites).
    fn runs_programs(self) -> bool {
        matches!(self, Self::Programs | Self::Coverage)
    }

    /// Whether the mode runs pltrs (Python), which needs the extension.
    fn runs_pltrs(self) -> bool {
        self != Self::Coverage
    }
}

/// Where the runs and reports go, below the repository: `target/diff`,
/// and `target/diff-<level>` for a run at another processor level
/// (`--level`), whose reference runs are others.
fn out_directory() -> String {
    crate::level::asked().map_or_else(
        || "target/diff".to_owned(),
        |level| format!("target/diff-{level}"),
    )
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
    let mut corpus = false;
    let mut script = Command::new(root.join(".venv/bin/python"));
    script
        .arg(root.join("spec/diff").join(mode.script()))
        .arg("--root")
        .arg(&root)
        .arg("--out")
        .arg(root.join(out_directory()));
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--no-build" => build = false,
            "--coverage" | "--docs" | "--types" | "--private" => {}
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
        let suite = suite_of(&suites)?;
        corpus = suite == CORPUS;
        if corpus && mode == Mode::Coverage {
            return Err(
                "--coverage measures the gallery and the generated programs, not the corpora"
                    .to_owned(),
            );
        }
        script
            .arg("--gallery")
            .arg(sdist().join("galleries"))
            .args(["--suite", suite]);
    } else if !suites.is_empty() {
        return Err(
            "--docs, --types and --private take no suite: they compare the API, not programs"
                .to_owned(),
        );
    }
    if build {
        crate::python::build(crate::python::Profile::Release)?;
    }
    // (Held until the run ends: no other run's venv step or install under
    // its programs.)
    let _venvs = (mode != Mode::Types && corpus)
        .then(|| venv_lock::hold(&crate::python::root().join("target/diff/venvs")))
        .transpose()?;
    if mode == Mode::Types {
        let (pltrs, reference) = types_venvs()?;
        script
            .arg("--sdist")
            .arg(sdist())
            .arg("--pltrs-python")
            .arg(pltrs)
            .arg("--mpl-python")
            .arg(reference);
    } else if corpus {
        let (pltrs, reference) = corpus_venvs()?;
        script
            .arg("--pltrs-python")
            .arg(pltrs)
            .arg("--mpl-python")
            .arg(reference);
    } else if mode.runs_pltrs() {
        script
            .arg("--pltrs-python")
            .arg(crate::python::pltrs_python()?);
    }
    crate::python::run_checked(&mut script)
}

/// The corpora's venvs, made or brought up to their pinned list first:
/// the interpreters of the pltrs side and of the reference side.
fn corpus_venvs() -> Result<(PathBuf, PathBuf), String> {
    let venvs = crate::python::root().join("target/diff/venvs");
    crate::python::make_venv_pair(
        &venvs.join("pltrs"),
        &venvs.join("reference"),
        &[CORPUS_REQUIREMENTS],
        MATPLOTLIB_REQUIREMENTS,
    )
}

/// matplotlib's source tree: `MPL_SDIST`, else the one `cargo xtask
/// mpl-suite` unpacks.
fn sdist() -> PathBuf {
    env::var_os("MPL_SDIST").map_or_else(
        || crate::python::root().join("target/refs/matplotlib-dl/matplotlib-3.11.2"),
        PathBuf::from,
    )
}

/// The venvs of `--types`, made or brought up to their pinned list first:
/// the interpreters of the pltrs side and of the reference side.
fn types_venvs() -> Result<(PathBuf, PathBuf), String> {
    let venvs = crate::python::root().join("target/diff/venvs");
    crate::python::make_venv_pair(
        &venvs.join("types-pltrs"),
        &venvs.join("types-reference"),
        &[TYPES_REQUIREMENTS],
        MATPLOTLIB_REQUIREMENTS,
    )
}

/// The driver's name of the programs the suite flags `given` choose: one
/// suite when only its flag is given, else the gallery and the generated
/// programs (`all`); the corpora go with no other suite.
fn suite_of(given: &[&str]) -> Result<&'static str, String> {
    match SUITES
        .iter()
        .filter(|(flag, _)| given.contains(flag))
        .collect::<Vec<_>>()[..]
    {
        [(_, suite)] => Ok(suite),
        [_, _, ..] if given.contains(&"--corpus") => Err(
            "--corpus runs the corpora alone: give it without --gallery or --generated".to_owned(),
        ),
        _ => Ok("all"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, suite_of};

    #[test]
    fn suite_flags_add_up() {
        assert_eq!(suite_of(&[]), Ok("all"));
        assert_eq!(suite_of(&["--gallery"]), Ok("gallery"));
        assert_eq!(suite_of(&["--generated", "--generated"]), Ok("generated"));
        assert_eq!(suite_of(&["--gallery", "--generated"]), Ok("all"));
        assert_eq!(suite_of(&["--corpus"]), Ok("corpus"));
        assert!(suite_of(&["--corpus", "--gallery"]).is_err());
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
        assert!(Mode::of(&args(&["--types", "--docs"])).is_err());
        assert_eq!(Mode::of(&args(&["--types", "--strict"])), Ok(Mode::Types));
        assert!(Mode::Types.runs_pltrs() && !Mode::Types.runs_programs());
        assert_eq!(Mode::Types.script(), "stubs.py");
        assert!(Mode::of(&args(&["--private", "--docs"])).is_err());
        assert_eq!(Mode::of(&args(&["--private"])), Ok(Mode::Private));
        assert!(Mode::Private.runs_pltrs() && !Mode::Private.runs_programs());
        assert_eq!(Mode::Private.script(), "private.py");
        assert!(Mode::Docs.runs_pltrs() && !Mode::Docs.runs_programs());
        assert!(Mode::Coverage.runs_programs() && !Mode::Coverage.runs_pltrs());
        assert_eq!(Mode::Docs.script(), "docs.py");
    }
}
