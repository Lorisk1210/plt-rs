//! `cargo xtask ecosystem suite [--reference] [PATTERN...] [--failures]`:
//! the own test suites of the libraries that import matplotlib by name
//! (`libraries`), the tests of each that reach matplotlib, run unchanged
//! from their sdists against real matplotlib (`--reference`, in the
//! ecosystem's reference venv) or against pltrs as matplotlib (in the
//! ecosystem venv, with `python/alias` on `PYTHONPATH`), as `cargo xtask
//! mpl-suite` runs matplotlib's.
//!
//! Each library runs in a site of its own,
//! `target/ecosystem/suite/<mode>/<library>/site/`, laid out as its source
//! tree is: the library's package, made of links to the entries of the
//! installed package and real copies of the sdist's tests (seaborn's
//! `tests/` become `seaborn/tests/`; the directories leading to a copy
//! deeper in the package, such as `sklearn/metrics/_plot/tests`, are real
//! directories of links in turn), and the sdist's `pyproject.toml`, so
//! that pytest runs with the library's own configuration (pandas turns
//! every warning into an error) and the options the library adds
//! (`Library::options`). Test ids name the files in the package
//! (`pandas/tests/plotting/test_x.py::...`, `seaborn/tests/test_x.py::...`),
//! which keeps each library's apart.
//!
//! The outcomes of all libraries go to `spec/ecosystem/<mode>.json`,
//! committed, and the report is `mpl-suite`'s: per test file, the tests
//! passing under matplotlib and the share of them passing under pltrs,
//! then what holds the others back; `--failures` lists them. A PATTERN
//! selects the test files whose path contains it, or names a test id.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use pltrs_core::os::Os;

use super::libraries::{LIBRARIES, Library};
use crate::mpl_suite::outcomes::Outcomes;
use crate::mpl_suite::{self, Mode, Options, prepare, pytest, report};
use crate::python;

/// The file of the sdist holding the library's pytest configuration.
const CONFIGURATION: &str = "pyproject.toml";

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

/// A library made ready to run: its run's directory, its site and its
/// test files.
struct Prepared {
    /// The library.
    library: Library,
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
    let mut prepared = Vec::new();
    for library in LIBRARIES {
        let dir = suite_dir().join(options.mode.name()).join(library.name);
        let site = site(library, &installed, &dir)?;
        let files = test_files(&site, library.tests)?;
        prepared.push(Prepared {
            library,
            dir,
            site,
            files,
        });
    }
    check_patterns(&options.patterns, &prepared)?;

    let path = outcome_file(options.mode);
    let details = details_file(options.mode);
    let mut merged = Outcomes::load(&path, &details)?;
    let mut times = Vec::new();
    for Prepared {
        library,
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
        let sources = python::root().join("python");
        let mut python_path = vec![site.clone(), sources.join("ecosystem/plugin")];
        if options.mode == Mode::Pltrs {
            python_path.extend([sources.join("alias"), sources]);
        }
        let started = Instant::now();
        let run = pytest::Run {
            suite: library.title,
            python: &python,
            site,
            dir,
            python_path,
            options: &pytest_options(library),
            targets: &targets,
            environment: &[],
        }
        .outcomes()?;
        times.push((library.title, started.elapsed().as_secs()));
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
    let package = site.join(library.package);
    for (directory, skip) in layout(&library) {
        prepare::relink(
            &installed.join(library.package).join(&directory),
            &package.join(&directory),
            &skip,
        )?;
    }
    link_vendored(&library, installed, &site)?;
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

/// Links into `site` the shared libraries the wheel of `library` vendors
/// beside its package (`scikit_learn.libs`), if it has any: its extension
/// modules find them relative to where they are loaded from
/// (`$ORIGIN/../../scikit_learn.libs`), which for the links of the site
/// is the site.
fn link_vendored(library: &Library, installed: &Path, site: &Path) -> Result<(), String> {
    let name = format!("{}.libs", library.name.replace('-', "_"));
    let link = site.join(&name);
    if link.is_symlink() {
        fs::remove_file(&link).map_err(|err| format!("{}: {err}", link.display()))?;
    }
    let vendored = installed.join(&name);
    if vendored.is_dir() {
        prepare::link(&vendored, &link)?;
    }
    Ok(())
}

/// The real directories of `library`'s package in its site (`""`, the
/// package itself, and those leading to its copies), relative to the
/// package and parents first, each with the entries of the installed
/// package its links leave out: those that are copies or real
/// directories, and in the package itself the `pyproject.toml` pandas
/// installs there, which pytest would take for the site's configuration,
/// making the package's directory its root.
fn layout(library: &Library) -> BTreeMap<String, Vec<&'static str>> {
    let mut layout = BTreeMap::from([(String::new(), vec![CONFIGURATION])]);
    let inside = library.copies.iter().filter_map(|(_, to)| {
        to.strip_prefix(library.package)
            .and_then(|path| path.strip_prefix('/'))
    });
    for path in inside {
        let mut directory = String::new();
        for name in path.split('/') {
            let skip = layout.entry(directory.clone()).or_default();
            if !skip.contains(&name) {
                skip.push(name);
            }
            if !directory.is_empty() {
                directory.push('/');
            }
            directory.push_str(name);
        }
    }
    layout
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

/// Every test file (`test_*.py`) of `tests`, test files and directories
/// of `site`, relative to the site with `/` between the names, sorted.
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

    /// The library named `name`.
    fn library(name: &str) -> Library {
        LIBRARIES
            .into_iter()
            .find(|library| library.name == name)
            .unwrap()
    }

    /// `library` prepared with the test files `files`.
    fn prepared(library: Library, files: &[&str]) -> Prepared {
        Prepared {
            library,
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
            prepared(pandas, &["pandas/tests/plotting/test_misc.py"]),
            prepared(seaborn, &["seaborn/tests/test_axisgrid.py"]),
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
    fn copies_deep_in_the_package_are_reached_through_real_directories() {
        let sklearn = library("scikit-learn");
        let sklearn = layout(&sklearn);
        assert_eq!(
            sklearn[""],
            [
                "pyproject.toml",
                "conftest.py",
                "inspection",
                "metrics",
                "model_selection",
                "tests",
                "tree",
                "utils"
            ]
        );
        assert_eq!(sklearn["inspection"], ["_plot"]);
        assert_eq!(sklearn["inspection/_plot"], ["tests"]);
        assert_eq!(sklearn["metrics/_plot"], ["tests"]);
        assert_eq!(sklearn["tree"], ["tests"]);
        assert!(!sklearn.contains_key("tree/tests"));
        // Copies outside the package (xarray's own conftest) are not in it.
        let xarray = layout(&library("xarray"));
        assert_eq!(xarray.keys().collect::<Vec<_>>(), [""]);
        assert_eq!(xarray[""], ["pyproject.toml", "tests"]);
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
            "_core/test_b.py",
            "_core/__pycache__/test_c.py",
        ] {
            fs::write(tests.join(file), "").unwrap();
        }
        assert_eq!(
            test_files(&site, &["lib/tests", "other/test_d.py"]).unwrap(),
            [
                "lib/tests/_core/test_b.py",
                "lib/tests/test_a.py",
                "other/test_d.py"
            ]
        );
        fs::remove_dir_all(site).unwrap();
    }
}
