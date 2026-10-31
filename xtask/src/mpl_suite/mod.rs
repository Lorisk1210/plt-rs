//! `cargo xtask mpl-suite [--reference] [PATTERN...] [--failures] [--imports]`:
//! matplotlib 3.11's own test suite, run unchanged against real
//! matplotlib (`--reference`, in the reference venv) or against pltrs (in
//! the pltrs venv, with matplotlib's names resolving to pltrs).
//!
//! A run records every test's outcome (`passed`, `failed`, `error`,
//! `skipped`, `xfailed`, `xpassed`) in `spec/mpl-suite/reference.json` or
//! `spec/mpl-suite/pltrs.json`, which are committed so that git tracks
//! progress, and prints the report: per test file, how many tests pass
//! under matplotlib here and the share of those passing under pltrs, and
//! the reasons holding back the rest, the most tests first. That share is
//! the measure of pltrs.
//!
//! A PATTERN selects the test files whose path contains it
//! (`test_colors`, `mplot3d`), or names one test id (`...py::test_x`);
//! only their entries are replaced in the outcome file. `--failures`
//! lists every reference-passing test that pltrs does not pass, with its
//! reason. `--imports` runs no tests: it lists, per test file, every
//! module and name its top-level imports need that pltrs lacks
//! (`python/mpl_suite/import_gaps.py`), where a collection error names
//! only the first. The sdist comes from `MPL_SDIST`, by default
//! `target/refs/matplotlib-dl/matplotlib-3.11.2`.

mod outcomes;
mod prepare;
mod report;

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use outcomes::Outcomes;

/// The pytest workers of a run.
const WORKERS: &str = "8";

/// The seconds a single test may take before pytest-timeout fails it.
const TEST_TIMEOUT: &str = "600";

/// The variable choosing pltrs' look, set to `matplotlib` for both runs:
/// pltrs (Python) draws in its modern look by default, and matplotlib's
/// tests expect matplotlib's defaults (matplotlib itself ignores it).
const LOOK_VARIABLE: &str = "PLTRS_LOOK";

/// pltrs' own variable naming its configuration directory, which cargo
/// sets for what it runs (`.cargo/config.toml`) and the suite must not
/// pass on: pltrs (Python) then reads matplotlib's `MPLCONFIGDIR`.
const CONFIG_DIR_VARIABLE: &str = "PLTRSCONFIGDIR";

/// What runs the suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// Real matplotlib, in the reference venv.
    Reference,
    /// pltrs, in the pltrs venv.
    Pltrs,
}

impl Mode {
    /// The mode's name, in paths and file names.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Pltrs => "pltrs",
        }
    }

    /// The committed outcome file.
    fn outcome_file(self) -> PathBuf {
        crate::python::root()
            .join("spec/mpl-suite")
            .join(format!("{}.json", self.name()))
    }

    /// The details file beside the run's log (reasons of failures).
    fn details_file(self, suite: &Path) -> PathBuf {
        suite.join(self.name()).join("details.json")
    }
}

/// The parsed command line.
#[derive(Debug)]
struct Options {
    mode: Mode,
    failures: bool,
    imports: bool,
    patterns: Vec<String>,
}

impl Options {
    fn parse(args: &[String]) -> Self {
        let mut options = Self {
            mode: Mode::Pltrs,
            failures: false,
            imports: false,
            patterns: Vec::new(),
        };
        for arg in args {
            match arg.as_str() {
                "--reference" => options.mode = Mode::Reference,
                "--failures" => options.failures = true,
                "--imports" => options.imports = true,
                pattern => options.patterns.push(pattern.to_owned()),
            }
        }
        options
    }
}

/// Runs the `mpl-suite` subcommand.
pub(crate) fn run(args: &[String]) -> ExitCode {
    match run_suite(&Options::parse(args)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cargo xtask mpl-suite: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run_suite(options: &Options) -> Result<(), String> {
    let root = crate::python::root();
    let suite = root.join("target/mpl-suite");
    let sdist = env::var_os("MPL_SDIST").map_or_else(
        || root.join("target/refs/matplotlib-dl/matplotlib-3.11.2"),
        PathBuf::from,
    );
    let python = match options.mode {
        Mode::Reference => root.join(".venv/bin/python"),
        Mode::Pltrs => {
            crate::python::build(crate::python::Profile::Release)?;
            crate::python::pltrs_python()?
        }
    };
    let site = prepare::site(options.mode, &suite, &sdist)?;
    if options.imports {
        return import_gaps(&python, &site);
    }
    let files = prepare::test_files(&site)?;
    let (targets, ran_files) = select(&files, &options.patterns)?;

    let results = suite.join(options.mode.name()).join("results.jsonl");
    let run = pytest(&python, &site, &results, &targets)?;
    let path = options.mode.outcome_file();
    let details = options.mode.details_file(&suite);
    let mut merged = Outcomes::load(&path, &details)?;
    merged.merge(run, &ran_files);
    merged.save(&path, &details)?;
    println!("wrote {}", path.display());

    let reference = Outcomes::load(
        &Mode::Reference.outcome_file(),
        &Mode::Reference.details_file(&suite),
    )?;
    let pltrs = (options.mode == Mode::Pltrs).then_some(&merged);
    report::print(&reference, pltrs, options.failures);
    Ok(())
}

/// Prints the gaps of the test files' top-level imports in `site`.
fn import_gaps(python: &Path, site: &Path) -> Result<(), String> {
    let root = crate::python::root();
    crate::python::run_checked(
        Command::new(python)
            .arg(root.join("python/mpl_suite/import_gaps.py"))
            .arg(Mode::Reference.outcome_file())
            .current_dir(site)
            .env("PYTHONPATH", site)
            .env("MPLCONFIGDIR", site.join("../mplconfig"))
            .env_remove(CONFIG_DIR_VARIABLE),
    )
}

/// The pytest arguments selecting what `patterns` asks for, and the test
/// files they cover (all of them without patterns).
fn select(files: &[String], patterns: &[String]) -> Result<(Vec<String>, Vec<String>), String> {
    if patterns.is_empty() {
        let mut targets = vec!["matplotlib/tests".to_owned()];
        targets.extend(prepare::TOOLKITS.map(|kit| format!("mpl_toolkits/{kit}/tests")));
        return Ok((targets, files.to_vec()));
    }
    let mut targets = Vec::new();
    let mut covered = Vec::new();
    for pattern in patterns {
        if pattern.contains("::") {
            targets.push(pattern.clone());
            continue;
        }
        let matching: Vec<&String> = files.iter().filter(|f| f.contains(pattern)).collect();
        if matching.is_empty() {
            return Err(format!("no test file matches {pattern:?}"));
        }
        for file in matching {
            if !covered.contains(file) {
                covered.push(file.clone());
                targets.push(file.clone());
            }
        }
    }
    Ok((targets, covered))
}

/// Runs pytest with the outcome plugin in `site` and reads what it
/// recorded. pytest's own output goes to `pytest.log` beside `results`.
fn pytest(
    python: &Path,
    site: &Path,
    results: &Path,
    targets: &[String],
) -> Result<Outcomes, String> {
    let root = crate::python::root();
    let dir = results
        .parent()
        .expect("results live in the mode's directory");
    let log_path = dir.join("pytest.log");
    let _ = std::fs::remove_file(results);
    let log =
        std::fs::File::create(&log_path).map_err(|err| format!("{}: {err}", log_path.display()))?;
    let stderr = log
        .try_clone()
        .map_err(|err| format!("{}: {err}", log_path.display()))?;
    let python_path = env::join_paths([site.to_path_buf(), root.join("python/mpl_suite")])
        .map_err(|err| err.to_string())?;
    println!(
        "running matplotlib's suite with {} (log: {})",
        python.display(),
        log_path.display()
    );
    let status = Command::new(python)
        .args([
            "-m",
            "pytest",
            "-p",
            "mpl_suite_report",
            "-p",
            "no:cacheprovider",
        ])
        .args(["-n", WORKERS, "--timeout", TEST_TIMEOUT])
        .args(["--continue-on-collection-errors", "-q", "-rN"])
        .args(targets)
        .current_dir(site)
        .env("PYTHONPATH", python_path)
        .env(LOOK_VARIABLE, "matplotlib")
        .env("MPL_SUITE_RESULTS", results)
        // The suite's own configuration directory, which pltrs reads from
        // `MPLCONFIGDIR` as matplotlib does: some of matplotlib's tests
        // make theirs unwritable, which must not reach the one cargo's
        // tests share (`target/pltrs-config`), and expect the warning to
        // name `MPLCONFIGDIR`.
        .env("MPLCONFIGDIR", dir.join("mplconfig"))
        .env_remove(CONFIG_DIR_VARIABLE)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(stderr)
        .status()
        .map_err(|err| format!("cannot run {}: {err}", python.display()))?;
    // 0: all passed, 1: some failed, 5: nothing collected; anything else
    // means pytest itself broke (internal error, bad usage, interrupted).
    if !matches!(status.code(), Some(0 | 1 | 5)) {
        return Err(format!(
            "pytest failed ({status}); see {}",
            log_path.display()
        ));
    }
    let text =
        std::fs::read_to_string(results).map_err(|err| format!("{}: {err}", results.display()))?;
    Outcomes::from_results(&text)
}
