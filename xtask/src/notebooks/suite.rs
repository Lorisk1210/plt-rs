//! `cargo xtask notebooks suite [--reference] [PATTERN...] [--failures]`:
//! the tests of the libraries between Jupyter and matplotlib, run
//! unchanged against real matplotlib (`--reference`, in the notebooks'
//! reference venv) or against pltrs as matplotlib (in their pltrs venv,
//! `python/alias` on `PYTHONPATH`), as `cargo xtask mpl-suite` runs
//! matplotlib's suite: `IPython`'s matplotlib tests (`test_pylabtools`, the
//! display tests, the `%matplotlib` and `%pylab` magics, the application's
//! GUI and pylab setup), matplotlib-inline's tests and notebooks (nbval),
//! ipympl's tests and its example notebook (nbval), and matplotlib's own
//! notebook tests (`test_backend_inline`, `test_backend_nbagg`, which
//! execute notebooks with `jupyter nbconvert`).
//!
//! Each library runs in a site of its own,
//! `target/notebooks/suite/<mode>/<library>/site/`, holding the library's
//! `pyproject.toml` (its pytest configuration) and copies of its tests
//! under a directory named after it, so that test ids read
//! `ipython-tests/tests/test_pylabtools.py::...`,
//! `ipympl-tests/docs/examples/...`.
//! The libraries themselves are the venv's. The outcomes go to
//! `spec/notebooks/<mode>.json`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::mpl_suite::outcomes::Outcomes;
use crate::mpl_suite::{Mode, Options, prepare, pytest, report};
use crate::python;

/// Where a library's tests come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    /// The sdist on `PyPI` of this name (its archive's), which setup
    /// downloads into `target/refs`.
    Sdist(&'static str),
    /// matplotlib's source tree, the one `cargo xtask mpl-suite` reads
    /// (`MPL_SDIST`).
    Matplotlib,
}

/// A library whose tests run.
#[derive(Clone, Copy, Debug)]
pub(super) struct Library {
    /// Its distribution's name, as the pinned list names it.
    pub(super) name: &'static str,
    /// Its version, which the pinned list pins.
    pub(super) version: &'static str,
    /// Where its tests come from.
    pub(super) source: Source,
    /// The files of the release's repository the tests need that its
    /// sdist leaves out, with their SHA-256: setup adds them to the
    /// unpacked sdist from `repository`.
    pub(super) left_out: &'static [(&'static str, &'static str)],
    /// The raw files of the library's repository at the release's tag.
    pub(super) repository: &'static str,
    /// What is copied from the source into the site's directory of the
    /// library, and where it goes there.
    copies: &'static [(&'static str, &'static str)],
    /// The tests to run, files or directories of the library's directory.
    tests: &'static [&'static str],
    /// The pytest workers: one for `IPython`'s tests, which make one profile
    /// directory per session in the working directory, and for the
    /// notebooks nbval runs, whose cells must all run in one kernel.
    workers: &'static str,
    /// What the progress line calls the tests.
    title: &'static str,
}

/// The libraries whose tests run.
pub(super) const LIBRARIES: [Library; 4] = [
    Library {
        name: "ipython",
        version: "9.18.1",
        source: Source::Sdist("ipython"),
        left_out: &[],
        repository: "",
        copies: &[("conftest.py", "conftest.py"), ("tests", "tests")],
        tests: &[
            "tests/test_display.py",
            "tests/test_display_2.py",
            "tests/test_ipapp.py",
            "tests/test_magics_pylab.py",
            "tests/test_pylabtools.py",
        ],
        workers: "1",
        title: "IPython's matplotlib tests",
    },
    Library {
        name: "matplotlib-inline",
        version: "0.2.2",
        source: Source::Sdist("matplotlib_inline"),
        left_out: &[
            (
                "tests/test_import.py",
                "2c013c8f1f7a7c4e250e00b2f8173ea00cb72be52fcb2def032cca365e0dd70d",
            ),
            (
                "tests/notebooks/config_InlineBackend.ipynb",
                "3d1655a9ded1635816de1690f082a985676ae2bdb24b961df12028c4b2da15a4",
            ),
            (
                "tests/notebooks/mpl_inline.ipynb",
                "4f8ceaef9c19fc2dc38f9a10d47e487a38f7d3322a97a75a9f8c5017199f0bee",
            ),
        ],
        repository: "https://raw.githubusercontent.com/ipython/matplotlib-inline/0.2.2",
        copies: &[("tests", "tests")],
        tests: &["tests"],
        workers: "1",
        title: "matplotlib-inline's tests",
    },
    Library {
        name: "ipympl",
        version: "0.10.0",
        source: Source::Sdist("ipympl"),
        left_out: &[],
        repository: "",
        copies: &[("tests", "tests"), ("docs/examples", "docs/examples")],
        tests: &["tests", "docs/examples"],
        workers: "1",
        title: "ipympl's tests",
    },
    Library {
        name: "matplotlib",
        version: super::MATPLOTLIB,
        source: Source::Matplotlib,
        left_out: &[],
        repository: "",
        copies: &[
            ("lib/matplotlib/tests/conftest.py", "tests/conftest.py"),
            (
                "lib/matplotlib/tests/test_backend_inline.py",
                "tests/test_backend_inline.py",
            ),
            (
                "lib/matplotlib/tests/test_backend_nbagg.py",
                "tests/test_backend_nbagg.py",
            ),
            (
                "lib/matplotlib/tests/data/test_inline_01.ipynb",
                "tests/data/test_inline_01.ipynb",
            ),
            (
                "lib/matplotlib/tests/data/test_nbagg_01.ipynb",
                "tests/data/test_nbagg_01.ipynb",
            ),
        ],
        tests: &[
            "tests/test_backend_inline.py",
            "tests/test_backend_nbagg.py",
        ],
        workers: "2",
        title: "matplotlib's notebook tests",
    },
];

/// The file of a source holding the library's pytest configuration.
const CONFIGURATION: &str = "pyproject.toml";

impl Library {
    /// The directory of the site its tests go to, which starts their ids:
    /// `<name>-tests`, no name a module could import, since pytest's
    /// `importlib` import mode (matplotlib's) makes the directories above
    /// a test file packages of their names.
    fn directory(&self) -> String {
        format!("{}-tests", self.name)
    }

    /// The directory of the library's source.
    pub(super) fn tree(&self) -> PathBuf {
        match self.source {
            Source::Sdist(sdist) => python::root()
                .join("target/refs")
                .join(format!("{sdist}-dl"))
                .join(format!("{sdist}-{}", self.version)),
            Source::Matplotlib => std::env::var_os("MPL_SDIST").map_or_else(
                || {
                    python::root()
                        .join("target/refs/matplotlib-dl")
                        .join(format!("matplotlib-{}", self.version))
                },
                PathBuf::from,
            ),
        }
    }

    /// The test files of the library in `site`, relative to the site with
    /// `/` between the names: those its tests name, and the test files
    /// (`test_*.py`) and notebooks (which nbval collects) in the
    /// directories they name, sorted.
    fn test_files(&self, site: &Path) -> Result<Vec<String>, String> {
        let mut files = Vec::new();
        let mut pending: Vec<String> = self
            .tests
            .iter()
            .map(|test| format!("{}/{test}", self.directory()))
            .collect();
        while let Some(relative) = pending.pop() {
            let path = site.join(&relative);
            if !path.is_dir() {
                files.push(relative);
                continue;
            }
            let entries =
                fs::read_dir(&path).map_err(|err| format!("{}: {err}", path.display()))?;
            for entry in entries.filter_map(Result::ok) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
                let extension = Path::new(&name).extension().and_then(|ext| ext.to_str());
                let is_test = matches!(
                    (name.starts_with("test_"), extension),
                    (true, Some("py")) | (_, Some("ipynb"))
                );
                if (is_dir && !name.starts_with(['.', '_'])) || (!is_dir && is_test) {
                    pending.push(format!("{relative}/{name}"));
                }
            }
        }
        files.sort();
        Ok(files)
    }
}

/// Where the runs work.
fn suite_dir() -> PathBuf {
    super::notebooks_dir().join("suite")
}

/// Runs the tests `args` asks for.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let options = Options::parse(args);
    if options.imports {
        return Err("--imports is mpl-suite's".to_owned());
    }
    if options.mode == Mode::Pltrs {
        python::build(python::Profile::Release)?;
    }
    let python = super::interpreter(options.mode)?;
    let mut sites = Vec::new();
    for library in LIBRARIES {
        let dir = suite_dir()
            .join(options.mode.name())
            .join(library.directory());
        let site = site(&library, &dir)?;
        let files = library.test_files(&site)?;
        sites.push((library, dir, site, files));
    }
    for pattern in &options.patterns {
        if !sites
            .iter()
            .any(|(library, _, _, files)| concerns(pattern, library, files))
        {
            return Err(format!("no test of the libraries matches {pattern:?}"));
        }
    }

    let path = super::outcome_file(options.mode);
    let details = super::details_file(options.mode);
    let mut merged = Outcomes::load(&path, &details)?;
    for (library, dir, site, files) in &sites {
        let patterns: Vec<String> = options
            .patterns
            .iter()
            .filter(|pattern| concerns(pattern, library, files))
            .cloned()
            .collect();
        if !options.patterns.is_empty() && patterns.is_empty() {
            continue;
        }
        let everything = library
            .tests
            .iter()
            .map(|test| format!("{}/{test}", library.directory()))
            .collect();
        let (targets, ran_files) = crate::mpl_suite::select_from(files, &patterns, everything)?;
        let mut python_path = vec![site.clone()];
        if options.mode == Mode::Pltrs {
            let sources = python::root().join("python");
            python_path.extend([sources.join("alias"), sources]);
        }
        let variables: Vec<pytest::Variable> = super::environment(options.mode, dir)?
            .into_iter()
            .map(|(name, value)| (name.to_owned(), Some(value)))
            .collect();
        let run = pytest::Run {
            suite: library.title,
            python: &python,
            site,
            dir,
            python_path,
            targets: &targets,
            environment: &[],
            options: &[],
            cache: false,
        }
        .outcomes_with_workers(library.workers, &variables)?;
        merged.merge(run, &ran_files);
    }
    merged.save(&path, &details, &super::versions())?;
    println!("wrote {}", path.display());

    let reference = Outcomes::load(
        &super::outcome_file(Mode::Reference),
        &super::details_file(Mode::Reference),
    )?;
    let pltrs = (options.mode == Mode::Pltrs).then_some(&merged);
    report::print(&reference, pltrs, options.failures);
    Ok(())
}

/// Whether `pattern` concerns `library`, whose test files are `files`: a
/// test id under its directory, or a part of the path of one of its files.
fn concerns(pattern: &str, library: &Library, files: &[String]) -> bool {
    match pattern.split_once("::") {
        Some((file, _)) => file.starts_with(&format!("{}/", library.directory())),
        None => files.iter().any(|file| file.contains(pattern)),
    }
}

/// Lays out `library`'s site in `dir` from its source, and returns it.
fn site(library: &Library, dir: &Path) -> Result<PathBuf, String> {
    let tree = library.tree();
    if !tree.is_dir() {
        return Err(format!(
            "{} is missing: run `cargo xtask notebooks setup` first",
            tree.display()
        ));
    }
    let site = dir.join("site");
    if site.exists() {
        fs::remove_dir_all(&site).map_err(|err| format!("{}: {err}", site.display()))?;
    }
    for (from, to) in library.copies {
        let (from, to) = (tree.join(from), site.join(library.directory()).join(to));
        if from.is_dir() {
            prepare::copy_tests(&from, &to)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    copy_file(&tree.join(CONFIGURATION), &site.join(CONFIGURATION))?;
    Ok(site)
}

/// Copies the file `from` to `to`, making its directory.
fn copy_file(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(dir) = to.parent() {
        fs::create_dir_all(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    }
    fs::copy(from, to)
        .map(drop)
        .map_err(|err| format!("{}: {err}", from.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_requirements_pin_the_libraries_tested() {
        let path = python::root().join("python/requirements-notebooks.txt");
        let pins = fs::read_to_string(&path).unwrap();
        for library in LIBRARIES
            .iter()
            .filter(|library| library.name != "matplotlib")
        {
            let pin = format!("{}=={}", library.name, library.version);
            assert!(pins.lines().any(|line| line == pin), "{pin}");
            assert!(
                library.repository.is_empty() || library.repository.ends_with(library.version),
                "{}",
                library.repository
            );
        }
        let reference = python::root().join("python/requirements-notebooks-reference.txt");
        let pins = fs::read_to_string(reference).unwrap();
        assert!(
            pins.lines()
                .any(|line| line == format!("matplotlib=={}", super::super::MATPLOTLIB))
        );
    }

    #[test]
    fn patterns_go_to_the_libraries_they_concern() {
        let [ipython, _, ipympl, _] = LIBRARIES;
        let files = ["ipython-tests/tests/test_pylabtools.py".to_owned()];
        assert!(concerns("pylabtools", &ipython, &files));
        assert!(concerns(
            "ipython-tests/tests/test_display.py::test_x",
            &ipython,
            &files
        ));
        assert!(!concerns(
            "ipympl-tests/tests/test_download.py::test_x",
            &ipython,
            &files
        ));
        assert!(!concerns("download", &ipympl, &files));
    }

    #[test]
    fn test_files_are_those_named_and_found() {
        let site = python::root()
            .join("target/xtask-tests")
            .join(format!("notebooks-suite-{}", std::process::id()));
        let ipympl = LIBRARIES[2];
        for file in [
            "ipympl-tests/tests/test_download.py",
            "ipympl-tests/tests/helpers.py",
            "ipympl-tests/tests/manual.ipynb",
            "ipympl-tests/tests/.ipynb_checkpoints/manual.ipynb",
            "ipympl-tests/docs/examples/full-example.ipynb",
        ] {
            let path = site.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        assert_eq!(
            ipympl.test_files(&site).unwrap(),
            [
                "ipympl-tests/docs/examples/full-example.ipynb",
                "ipympl-tests/tests/manual.ipynb",
                "ipympl-tests/tests/test_download.py",
            ]
        );
        fs::remove_dir_all(site).unwrap();
    }
}
