//! `cargo xtask ecosystem suite [--reference] [PATTERN...] [--failures]`:
//! the own test suites of the libraries that import matplotlib by name,
//! pandas' plotting tests and seaborn's whole suite, run unchanged from
//! their sdists against real matplotlib (`--reference`, in the
//! ecosystem's reference venv) or against pltrs as matplotlib (in the
//! ecosystem venv, with `python/alias` on `PYTHONPATH`), as `cargo xtask
//! mpl-suite` runs matplotlib's.
//!
//! Each library runs in a site of its own,
//! `target/ecosystem/suite/<mode>/<library>/site/`, laid out as its source
//! tree is: the library's package, made of links to every entry of the
//! installed package and real copies of the sdist's tests (seaborn's
//! `tests/` become `seaborn/tests/`), and the sdist's `pyproject.toml`, so
//! that pytest runs with the library's own configuration (pandas turns
//! every warning into an error). Test ids read
//! `pandas/tests/plotting/test_x.py::...` and `seaborn/tests/test_x.py::...`.
//!
//! The outcomes of both libraries go to `spec/ecosystem/<mode>.json`,
//! committed, and the report is `mpl-suite`'s: per test file, the tests
//! passing under matplotlib and the share of them passing under pltrs,
//! then what holds the others back; `--failures` lists them. A PATTERN
//! selects the test files whose path contains it, or names a test id.

use std::fs;
use std::path::{Path, PathBuf};

use pltrs_core::os::Os;

use crate::mpl_suite::outcomes::Outcomes;
use crate::mpl_suite::{self, Mode, Options, prepare, pytest, report};
use crate::python;

/// A library whose test suite runs.
#[derive(Clone, Copy, Debug)]
pub(super) struct Library {
    /// Its distribution and package name.
    pub(super) name: &'static str,
    /// What the suite is, in the sdist, and where it goes in the site.
    copies: &'static [(&'static str, &'static str)],
    /// The files of the release's source tree the suite reads that its
    /// sdist leaves out, with their SHA-256: setup adds them to the
    /// unpacked sdist from [`Library::repository`].
    pub(super) left_out: &'static [(&'static str, &'static str)],
    /// The raw files of the library's repository; a release's are under
    /// its tag, `v<version>`.
    pub(super) repository: &'static str,
    /// The directory of the tests to run, in the site.
    tests: &'static str,
    /// What the progress line calls the suite.
    title: &'static str,
}

/// pandas' iris data, which the tests of its plots of several columns
/// read (`andrews_curves`, `parallel_coordinates`, `radviz`).
const IRIS: &str = "pandas/tests/io/data/csv/iris.csv";

/// The libraries whose suites run.
pub(super) const LIBRARIES: [Library; 2] = [
    Library {
        name: "pandas",
        copies: &[
            ("pandas/conftest.py", "pandas/conftest.py"),
            ("pandas/tests/__init__.py", "pandas/tests/__init__.py"),
            ("pandas/tests/plotting", "pandas/tests/plotting"),
            (IRIS, IRIS),
        ],
        left_out: &[(
            IRIS,
            "904d791b308aa79bf4f06669f63be3f1ab331a23bd508051065e00f700f573f9",
        )],
        repository: "https://raw.githubusercontent.com/pandas-dev/pandas",
        tests: "pandas/tests/plotting",
        title: "pandas' plotting tests",
    },
    Library {
        name: "seaborn",
        copies: &[("tests", "seaborn/tests")],
        left_out: &[],
        repository: "https://raw.githubusercontent.com/mwaskom/seaborn",
        tests: "seaborn/tests",
        title: "seaborn's tests",
    },
];

/// The file of the sdist holding the library's pytest configuration.
const CONFIGURATION: &str = "pyproject.toml";

/// The version of matplotlib the reference run has.
const MATPLOTLIB: &str = "3.11.2";

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

/// Runs the suites as `args` asks.
pub(super) fn run(args: &[String]) -> Result<(), String> {
    let options = Options::parse(args);
    if options.imports {
        return Err("--imports is mpl-suite's".to_owned());
    }
    let venv = match options.mode {
        Mode::Reference => super::reference_venv(),
        Mode::Pltrs => {
            python::build(python::Profile::Release)?;
            super::venv()
        }
    };
    let python = python::interpreter(&venv, Os::CURRENT);
    if !python.exists() {
        return Err(format!(
            "{} does not exist: run `cargo xtask ecosystem setup` first",
            python.display()
        ));
    }
    let installed = python::site_packages(&venv)?;
    let mut sites = Vec::new();
    for library in LIBRARIES {
        let dir = suite_dir().join(options.mode.name()).join(library.name);
        let site = site(library, &installed, &dir)?;
        let files = test_files(&site, library.tests)?;
        sites.push((library, dir, site, files));
    }
    check_patterns(&options.patterns, &sites)?;

    let path = outcome_file(options.mode);
    let details = details_file(options.mode);
    let mut merged = Outcomes::load(&path, &details)?;
    for (library, dir, site, files) in &sites {
        let patterns = patterns_of(&options.patterns, library, files);
        if !options.patterns.is_empty() && patterns.is_empty() {
            continue;
        }
        let (targets, ran_files) =
            mpl_suite::select_from(files, &patterns, vec![library.tests.to_owned()])?;
        let mut python_path = vec![site.clone()];
        if options.mode == Mode::Pltrs {
            let sources = python::root().join("python");
            python_path.extend([sources.join("alias"), sources]);
        }
        let run = pytest::Run {
            suite: library.title,
            python: &python,
            site,
            dir,
            python_path,
            targets: &targets,
            environment: &[],
        }
        .outcomes()?;
        merged.merge(run, &ran_files);
    }
    let mut versions = Vec::new();
    for library in LIBRARIES {
        versions.push((library.name, super::setup::pinned(library.name)?));
    }
    versions.push(("matplotlib", MATPLOTLIB.to_owned()));
    let versions: Vec<(&str, &str)> = versions
        .iter()
        .map(|(name, version)| (*name, version.as_str()))
        .collect();
    merged.save(&path, &details, &versions)?;
    println!("wrote {}", path.display());

    let reference = Outcomes::load(
        &outcome_file(Mode::Reference),
        &details_file(Mode::Reference),
    )?;
    let pltrs = (options.mode == Mode::Pltrs).then_some(&merged);
    report::print(&reference, pltrs, options.failures);
    Ok(())
}

/// Lays out `library`'s site in `dir` from its sdist and the package
/// installed in `installed` (a venv's `site-packages`), and returns it.
fn site(library: Library, installed: &Path, dir: &Path) -> Result<PathBuf, String> {
    let version = super::setup::pinned(library.name)?;
    let sdist = super::setup::sdist(library.name, &version);
    if !sdist.is_dir() {
        return Err(format!(
            "{} is missing: run `cargo xtask ecosystem setup` first",
            sdist.display()
        ));
    }
    let site = dir.join("site");
    let package = site.join(library.name);
    // The package's entries the copies replace, and the `pyproject.toml`
    // pandas installs inside its package, which pytest would take for the
    // site's configuration, making the package's directory its root.
    let mut skip: Vec<&str> = library
        .copies
        .iter()
        .filter_map(|(_, to)| {
            to.strip_prefix(library.name)?
                .strip_prefix('/')?
                .split('/')
                .next()
        })
        .collect();
    skip.push(CONFIGURATION);
    prepare::relink(&installed.join(library.name), &package, &skip)?;
    for (from, to) in library.copies {
        let (from, to) = (sdist.join(from), site.join(to));
        if from.is_dir() {
            prepare::copy_tests(&from, &to)?;
        } else {
            copy_file(&from, &to)?;
        }
    }
    copy_file(&sdist.join(CONFIGURATION), &site.join(CONFIGURATION))?;
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

/// Every test file (`test_*.py`) under the directory `tests` of `site`,
/// relative to the site with `/` between the names, sorted.
fn test_files(site: &Path, tests: &str) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    let mut pending = vec![tests.to_owned()];
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
            } else if name.starts_with("test_")
                && Path::new(&name).extension().is_some_and(|ext| ext == "py")
            {
                files.push(relative);
            }
        }
    }
    files.sort();
    Ok(files)
}

/// The patterns of `patterns` that concern `library`, whose test files
/// are `files`: the test ids under its tests and the patterns matching
/// one of its files.
fn patterns_of(patterns: &[String], library: &Library, files: &[String]) -> Vec<String> {
    patterns
        .iter()
        .filter(|pattern| match pattern.split_once("::") {
            Some((file, _)) => file.starts_with(library.tests),
            None => files.iter().any(|file| file.contains(pattern.as_str())),
        })
        .cloned()
        .collect()
}

/// Fails on a pattern that concerns no library.
fn check_patterns(
    patterns: &[String],
    sites: &[(Library, PathBuf, PathBuf, Vec<String>)],
) -> Result<(), String> {
    for pattern in patterns {
        let pattern = std::slice::from_ref(pattern);
        let concerned = sites
            .iter()
            .any(|(library, _, _, files)| !patterns_of(pattern, library, files).is_empty());
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

    #[test]
    fn patterns_go_to_the_libraries_they_concern() {
        let [pandas, seaborn] = LIBRARIES;
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
        let sites = [
            (pandas, PathBuf::new(), PathBuf::new(), pandas_files),
            (seaborn, PathBuf::new(), PathBuf::new(), seaborn_files),
        ];
        assert!(check_patterns(&patterns, &sites).is_ok());
        assert!(check_patterns(&["nowhere".to_owned()], &sites).is_err());
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
            "_core/test_b.py",
            "_core/__pycache__/test_c.py",
        ] {
            fs::write(tests.join(file), "").unwrap();
        }
        assert_eq!(
            test_files(&site, "lib/tests").unwrap(),
            ["lib/tests/_core/test_b.py", "lib/tests/test_a.py"]
        );
        fs::remove_dir_all(site).unwrap();
    }
}
