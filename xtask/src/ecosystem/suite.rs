//! `cargo xtask ecosystem suite [--reference] [PATTERN...] [--failures]`:
//! the own test suites of the libraries that import matplotlib by name
//! (`libraries`), the tests of each that reach matplotlib, run unchanged
//! from their source trees against real matplotlib (`--reference`, in a
//! reference venv) or against pltrs as matplotlib (in a venv without it,
//! into which a run installs pltrs and its `matplotlib` distribution as
//! users have them, `install`), as `cargo xtask mpl-suite` runs
//! matplotlib's: the ecosystem's shared venvs, or a library's own
//! (`venvs`).
//!
//! Each library runs in a site of its own,
//! `target/ecosystem/suite/<mode>/<library>/site/` (`site`), with its own
//! pytest configuration and the options the library adds
//! (`Library::options`); the image comparisons whose baselines setup
//! makes compare with those (`baselines`). Test ids name the files in the
//! package (`pandas/tests/plotting/test_x.py::...`,
//! `seaborn/tests/test_x.py::...`), which keeps each library's apart.
//!
//! The outcomes of all libraries go to `spec/ecosystem/<mode>.json`,
//! committed, and the report is `mpl-suite`'s: per test file, the tests
//! passing under matplotlib and the share of them passing under pltrs,
//! then what holds the others back; `--failures` lists them. A PATTERN
//! selects the test files whose path contains it, or names a test id.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pltrs_core::os::Os;

use super::libraries::{self, Library, Network};
use super::{baselines, data, install, site, venvs};
use crate::mpl_suite::display::VirtualDisplay;
use crate::mpl_suite::outcomes::Outcomes;
use crate::mpl_suite::pytest::Variable;
use crate::mpl_suite::{self, Mode, Options, prepare, pytest, report};
use crate::python;

/// The version of matplotlib the reference run has.
const MATPLOTLIB: &str = "3.11.2";

/// The pytest plugin running only the tests of some files that take a
/// fixture (`Library::only_taking`), in `python/ecosystem/plugin`.
const SELECT_PLUGIN: &str = "ecosystem_select";

/// Where the runs work.
fn suite_dir() -> PathBuf {
    super::ecosystem_dir().join("suite")
}

/// The committed outcome file of `mode`.
fn outcome_file(mode: Mode) -> PathBuf {
    python::root()
        .join("spec/ecosystem")
        .join(format!("{}.json", mode.name()))
}

/// The details file of `mode` (reasons of failures).
fn details_file(mode: Mode) -> PathBuf {
    suite_dir().join(mode.name()).join("details.json")
}

/// A library made ready to run: the interpreter of its venv, its run's
/// directory, its site and its test files.
struct Prepared {
    /// The library.
    library: Library,
    /// The interpreter of the venv it runs in.
    python: PathBuf,
    /// The run's own directory.
    dir: PathBuf,
    /// The site pytest runs in.
    site: PathBuf,
    /// Its test files, relative to the site.
    files: Vec<String>,
}

/// Runs the suites as `args` asks.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let options = Options::parse(args);
    if options.imports {
        return Err("--imports is mpl-suite's".to_owned());
    }
    if options.mode == Mode::Pltrs {
        python::build(python::Profile::Release)?;
    }
    libraries::report_left_out();
    let mut prepared = Vec::new();
    for library in libraries::here() {
        let venv = venvs::venv(&library, options.mode);
        let python = python::interpreter(&venv, Os::CURRENT);
        if !python.exists() {
            return Err(format!(
                "{} does not exist: run `cargo xtask ecosystem setup` first",
                python.display()
            ));
        }
        let installed = python::site_packages(&venv)?;
        let dir = suite_dir().join(options.mode.name()).join(library.name);
        let site = site::lay_out(&library, &installed, &dir, &baselines::made(&library)?)?;
        let files = test_files(&site, library.tests)?;
        prepared.push(Prepared {
            library,
            python,
            dir,
            site,
            files,
        });
    }
    check_patterns(&options.patterns, &prepared)?;
    if options.mode == Mode::Pltrs {
        // Each venv once, before any of its libraries runs.
        let mut venvs: Vec<&Path> = prepared.iter().map(|p| p.python.as_path()).collect();
        venvs.sort_unstable();
        venvs.dedup();
        for python in venvs {
            install::install(python)?;
        }
    }

    let path = outcome_file(options.mode);
    let details = details_file(options.mode);
    let mut merged = Outcomes::load(&path, &details)?;
    let mut times = Vec::new();
    for Prepared {
        library,
        python,
        dir,
        site,
        files,
    } in &prepared
    {
        let patterns = patterns_of(&options.patterns, library, files);
        if !options.patterns.is_empty() && patterns.is_empty() {
            continue;
        }
        let everything = library.tests.iter().map(|&test| test.to_owned()).collect();
        let (targets, ran_files) = mpl_suite::select_from(files, &patterns, everything)?;
        let started = Instant::now();
        let run = Tests {
            library,
            python,
            site,
            dir,
        }
        .run(&baselines::comparing(library)?, &targets)?;
        times.push((library.title, started.elapsed().as_secs()));
        // A run of every file of the library (by patterns or without) has
        // the whole suite: what was recorded for files no longer in it goes.
        if ran_files.len() == files.len() {
            merged.forget_files_gone(library.tests, files);
        }
        merged.merge(run, &ran_files);
    }
    let mut versions = Vec::new();
    for library in libraries::here() {
        versions.push((library.name, venvs::pinned(&library)?));
    }
    versions.push(("matplotlib", MATPLOTLIB.to_owned()));
    let versions: Vec<(&str, &str)> = versions
        .iter()
        .map(|(name, version)| (*name, version.as_str()))
        .collect();
    merged.save(&path, &details, &versions)?;
    println!("wrote {}", path.display());
    for (title, seconds) in times {
        println!("{title}: {seconds} s");
    }

    let reference = Outcomes::load(
        &outcome_file(Mode::Reference),
        &details_file(Mode::Reference),
    )?;
    let pltrs = (options.mode == Mode::Pltrs).then_some(&merged);
    report::print(&reference, pltrs, options.failures);
    Ok(())
}

/// The tests of a library, made ready to run in its site.
pub(super) struct Tests<'a> {
    /// The library.
    pub(super) library: &'a Library,
    /// The interpreter of the mode's venv.
    pub(super) python: &'a Path,
    /// The library's site.
    pub(super) site: &'a Path,
    /// The run's own directory.
    pub(super) dir: &'a Path,
}

impl Tests<'_> {
    /// Runs the tests `targets` (paths or test ids in the site) with the
    /// library's options and `extra`, and reads how they ended.
    pub(super) fn run(&self, extra: &[String], targets: &[String]) -> Result<Outcomes, String> {
        let sources = python::root().join("python");
        // pltrs is installed into the venv of a pltrs run (`install`).
        let python_path = vec![self.site.to_owned(), sources.join("ecosystem/plugin")];
        // Tests running elsewhere in the site are named by their paths.
        let (working_dir, targets) = match self.library.runs_in {
            Some(dir) => (
                self.site.join(dir),
                targets
                    .iter()
                    .map(|target| self.site.join(target).to_string_lossy().into_owned())
                    .collect(),
            ),
            None => (self.site.to_owned(), targets.to_vec()),
        };
        let mut options = pytest_options(self.library);
        options.extend_from_slice(extra);
        // The temporary files of the tests (pytest-mpl's results and
        // downloads) in the run's own directory, emptied first: what a run
        // left there (a seeded temporary name taken) reaches no other.
        let temporary = self.dir.join("tmp");
        prepare::remove(&temporary)?;
        fs::create_dir_all(&temporary).map_err(|err| format!("{}: {err}", temporary.display()))?;
        let mut variables = vec![("TMPDIR".to_owned(), Some(temporary.into_os_string()))];
        variables.extend(data::variables(self.library)?);
        if self.library.network == Network::Refused {
            variables.extend(offline());
        }
        let environment = self.library.environment;
        variables.extend(
            (environment.variables.iter())
                .map(|&(name, value)| (name.to_owned(), Some(value.into()))),
        );
        // Stopped once the run is over, when it is the run's own.
        let _display = environment
            .display
            .then(|| give_display(self.dir, &mut variables))
            .flatten();
        let run = pytest::Run {
            suite: self.library.title,
            python: self.python,
            site: &working_dir,
            dir: self.dir,
            python_path,
            options: &options,
            targets: &targets,
            environment: &[],
            cache: environment.cache,
        };
        match environment.workers {
            Some(workers) => run.outcomes_with_workers(workers, &variables),
            None => run.outcomes_with(&variables),
        }
        .map(|outcomes| {
            let site = self.site.to_string_lossy();
            let venv = self
                .python
                .parent()
                .and_then(Path::parent)
                .unwrap_or(self.python);
            outcomes.naming_paths(&[(&site, "{site}"), (&venv.to_string_lossy(), "{venv}")])
        })
    }
}

/// Gives a run in `dir` that needs an X display one: `DISPLAY`'s, else an
/// Xvfb of its own, returned (it stops when dropped) with `DISPLAY` naming
/// it in `variables`; none, said, without Xvfb (the tests needing it skip).
fn give_display(dir: &Path, variables: &mut Vec<Variable>) -> Option<VirtualDisplay> {
    if std::env::var_os("DISPLAY").is_some_and(|display| !display.is_empty()) {
        return None;
    }
    let started = pltrs_core::os::which("Xvfb")
        .ok_or_else(|| "$DISPLAY is unset and Xvfb is not on PATH".to_owned())
        .and_then(|xvfb| VirtualDisplay::start(&xvfb, &dir.join("xvfb.log")));
    match started {
        Ok(display) => {
            variables.push(("DISPLAY".to_owned(), Some(display.name().into())));
            variables.push(("WAYLAND_DISPLAY".to_owned(), None));
            Some(display)
        }
        Err(err) => {
            println!("no display: {err}; the tests needing one skip");
            None
        }
    }
}

/// The proxy that refuses every connection: nothing listens on the
/// discard port of the loopback interface.
const REFUSING_PROXY: &str = "http://127.0.0.1:9";

/// The variables running a library's tests offline
/// ([`Network::Refused`]): HTTP and HTTPS through [`REFUSING_PROXY`],
/// which urllib, requests and pooch heed, the loopback interface spared.
fn offline() -> Vec<Variable> {
    let mut variables = Vec::new();
    for name in ["http_proxy", "https_proxy"] {
        for name in [name.to_owned(), name.to_uppercase()] {
            variables.push((name, Some(REFUSING_PROXY.into())));
        }
    }
    for name in ["no_proxy", "NO_PROXY"] {
        variables.push((name.to_owned(), Some("localhost,127.0.0.1,::1".into())));
    }
    variables
}

/// The pytest options of `library`'s runs: its own, and the selection of
/// the tests taking a fixture.
fn pytest_options(library: &Library) -> Vec<String> {
    let mut options: Vec<String> = library
        .options
        .iter()
        .map(|&option| option.to_owned())
        .collect();
    if let Some(only) = library.only_taking {
        options.extend(["-p".to_owned(), SELECT_PLUGIN.to_owned()]);
        options.extend(
            only.files
                .iter()
                .map(|file| format!("--only-taking={}:{file}", only.fixture)),
        );
    }
    options
}

/// Every test file (`test_*.py` or `*_test.py`, pytest's defaults) of
/// `tests`, test files and directories of `site`, relative to the site
/// with `/` between the names, sorted.
fn test_files(site: &Path, tests: &[&str]) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut pending = Vec::new();
    for &test in tests {
        if site.join(test).is_dir() {
            pending.push(test.to_owned());
        } else {
            files.push(test.to_owned());
        }
    }
    while let Some(dir) = pending.pop() {
        let path = site.join(&dir);
        let entries = fs::read_dir(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = format!("{dir}/{name}");
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                if name != "__pycache__" {
                    pending.push(relative);
                }
            } else if name
                .strip_suffix(".py")
                .is_some_and(|stem| stem.starts_with("test_") || stem.ends_with("_test"))
            {
                files.push(relative);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The patterns of `patterns` that concern `library`, whose test files
/// are `files`: the test ids of its test files and the patterns matching
/// one of them.
fn patterns_of(patterns: &[String], library: &Library, files: &[String]) -> Vec<String> {
    patterns
        .iter()
        .filter(|pattern| match pattern.split_once("::") {
            Some((file, _)) => library.tests.iter().any(|test| file.starts_with(test)),
            None => files.iter().any(|file| file.contains(pattern.as_str())),
        })
        .cloned()
        .collect()
}

/// Fails on a pattern that concerns no library.
fn check_patterns(patterns: &[String], prepared: &[Prepared]) -> Result<(), String> {
    for pattern in patterns {
        let pattern = std::slice::from_ref(pattern);
        let concerned = prepared
            .iter()
            .any(|ready| !patterns_of(pattern, &ready.library, &ready.files).is_empty());
        if !concerned {
            return Err(format!(
                "no test file of the libraries matches {:?}",
                pattern[0]
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecosystem::libraries::named as library;

    /// `library` prepared with the test files `files`.
    fn prepared(library: &Library, files: &[&str]) -> Prepared {
        Prepared {
            library: *library,
            python: PathBuf::new(),
            dir: PathBuf::new(),
            site: PathBuf::new(),
            files: files.iter().map(|&file| file.to_owned()).collect(),
        }
    }

    #[test]
    fn patterns_go_to_the_libraries_they_concern() {
        let (pandas, seaborn) = (library("pandas"), library("seaborn"));
        let pandas_files = vec!["pandas/tests/plotting/test_misc.py".to_owned()];
        let seaborn_files = vec!["seaborn/tests/test_axisgrid.py".to_owned()];
        let patterns = [
            "test_misc".to_owned(),
            "seaborn/tests/test_axisgrid.py::TestFacetGrid::test_x".to_owned(),
            "axisgrid".to_owned(),
        ];
        assert_eq!(
            patterns_of(&patterns, &pandas, &pandas_files),
            ["test_misc"]
        );
        assert_eq!(
            patterns_of(&patterns, &seaborn, &seaborn_files),
            [&patterns[1], &patterns[2]].map(String::as_str)
        );
        let prepared = [
            prepared(&pandas, &["pandas/tests/plotting/test_misc.py"]),
            prepared(&seaborn, &["seaborn/tests/test_axisgrid.py"]),
        ];
        assert!(check_patterns(&patterns, &prepared).is_ok());
        assert!(check_patterns(&["nowhere".to_owned()], &prepared).is_err());
    }

    #[test]
    fn test_ids_of_a_test_file_go_to_its_library() {
        let networkx = library("networkx");
        let id = "networkx/drawing/tests/test_pylab.py::test_draw".to_owned();
        assert_eq!(
            patterns_of(std::slice::from_ref(&id), &networkx, &[]),
            [id.as_str()]
        );
        let elsewhere = "networkx/drawing/tests/test_layout.py::test_x".to_owned();
        assert_eq!(
            patterns_of(&[elsewhere], &networkx, &[]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn libraries_select_their_fixtures_with_the_plugin() {
        let options = pytest_options(&library("scikit-learn"));
        assert_eq!(options[..2], ["-p", SELECT_PLUGIN]);
        assert!(
            options[2..]
                .iter()
                .all(|option| option.starts_with("--only-taking=pyplot:sklearn/"))
        );
        assert_eq!(
            pytest_options(&library("networkx")),
            ["--mpl", "--mpl-results-path=mpl-results"]
        );
    }

    #[test]
    fn test_files_are_found_in_subdirectories() {
        let site = python::root()
            .join("target/xtask-tests")
            .join(format!("ecosystem-suite-{}", std::process::id()));
        let tests = site.join("lib/tests");
        fs::create_dir_all(tests.join("_core/__pycache__")).unwrap();
        for file in [
            "test_a.py",
            "conftest.py",
            "utils.py",
            "_core/test_b.py",
            "_core/b_test.py",
            "_core/__pycache__/test_c.py",
        ] {
            fs::write(tests.join(file), "").unwrap();
        }
        assert_eq!(
            test_files(&site, &["lib/tests", "other/test_d.py"]).unwrap(),
            [
                "lib/tests/_core/b_test.py",
                "lib/tests/_core/test_b.py",
                "lib/tests/test_a.py",
                "other/test_d.py"
            ]
        );
        fs::remove_dir_all(site).unwrap();
    }
}
