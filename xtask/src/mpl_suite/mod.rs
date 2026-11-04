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

pub(crate) mod outcomes;
pub(crate) mod prepare;
pub(crate) mod pytest;
pub(crate) mod report;

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use outcomes::Outcomes;
use pytest::CONFIG_DIR_VARIABLE;

/// The versions the outcome files name.
const VERSIONS: [(&str, &str); 1] = [("matplotlib", "3.11.2")];

/// What runs a suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Real matplotlib, in a reference venv.
    Reference,
    /// pltrs, in a venv without matplotlib.
    Pltrs,
}

impl Mode {
    /// The mode's name, in paths and file names.
    pub(crate) fn name(self) -> &'static str {
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

/// The parsed command line: `[--reference] [PATTERN...] [--failures]
/// [--imports]`.
#[derive(Debug)]
pub(crate) struct Options {
    /// What runs the suite (`--reference`: matplotlib).
    pub(crate) mode: Mode,
    /// Whether to list every failure (`--failures`).
    pub(crate) failures: bool,
    /// Whether to list the import gaps instead of running (`--imports`).
    pub(crate) imports: bool,
    /// The test files or ids to run.
    pub(crate) patterns: Vec<String>,
}

impl Options {
    /// Reads the command line `args`.
    pub(crate) fn parse(args: &[String]) -> Self {
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
        Mode::Reference => crate::python::reference_python(),
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

    let run = pytest::Run {
        suite: "matplotlib's suite",
        python: &python,
        site: &site,
        dir: &suite.join(options.mode.name()),
        python_path: vec![site.clone()],
        targets: &targets,
    }
    .outcomes()?;
    let path = options.mode.outcome_file();
    let details = options.mode.details_file(&suite);
    let mut merged = Outcomes::load(&path, &details)?;
    merged.merge(run, &ran_files);
    merged.save(&path, &details, &VERSIONS)?;
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

/// The pytest arguments selecting what `patterns` asks for of matplotlib's
/// suite, and the test files they cover (all of them without patterns).
fn select(files: &[String], patterns: &[String]) -> Result<(Vec<String>, Vec<String>), String> {
    let mut everything = vec!["matplotlib/tests".to_owned()];
    everything.extend(prepare::TOOLKITS.map(|kit| format!("mpl_toolkits/{kit}/tests")));
    select_from(files, patterns, everything)
}

/// The pytest arguments selecting what `patterns` asks for of a suite of
/// the test files `files`, and the files they cover: the test files whose
/// path contains a pattern, the test ids of the patterns naming one
/// (`...py::test_x`), and without patterns `everything` and all `files`.
pub(crate) fn select_from(
    files: &[String],
    patterns: &[String],
    everything: Vec<String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    if patterns.is_empty() {
        return Ok((everything, files.to_vec()));
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
