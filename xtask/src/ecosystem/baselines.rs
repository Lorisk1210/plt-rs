//! The baselines of the libraries' image comparisons whose own are for
//! another version of matplotlib (plotnine's are 3.10's images, astropy's
//! hashes of 3.8's, mplfinance's older images), which `cargo xtask ecosystem setup` makes under real
//! matplotlib 3.11.2, in the ecosystem's reference venv, from the pinned
//! versions alone, so that every machine makes the same:
//! `target/ecosystem/baselines/<library>/images`, never committed.
//!
//! * pytest-mpl's (astropy's): one run of the library's tests with
//!   `--mpl-generate-path`, which saves each figure test's image instead
//!   of comparing it; the suite's runs compare with them
//!   (`--mpl-baseline-path`, or the directory the tests name, made a link
//!   to them);
//! * plotnine's and mplfinance's: their tests save every image before
//!   they compare it with its baseline, missing at first. As plotnine's
//!   `tools/update_baseline_images.py` does, the images of a run that have
//!   no baseline become baselines, and the tests that failed run again,
//!   until a run adds none: a test comparing several images stops at its
//!   first missing baseline. An image differing from the baseline made
//!   for it (which the tool would take instead) is matplotlib's own
//!   nondeterminism, and stays a failure of the reference run.
//!
//! The images are made in `images.partial`, which becomes `images` once
//! complete. `made-with.txt` beside them holds the pins they were made
//! with; setup makes them again when these change.

use std::fs;
use std::path::{Path, PathBuf};

use pltrs_core::os::Os;

use super::libraries::{Baselines, Library, Making};
use super::site;
use super::suite::Tests;
use crate::mpl_suite::{Mode, prepare};
use crate::python;

/// The file naming the pins the baselines were made with.
const STAMP: &str = "made-with.txt";

/// The requirement files whose pins decide the images.
const PINNED: [&str; 2] = [
    "requirements-ecosystem.txt",
    "requirements-ecosystem-reference.txt",
];

/// The most runs making baselines from the tests' own images, more than
/// any test has images.
const MOST_RUNS: usize = 25;

/// The suffixes of the images a comparison saves beside the result: the
/// copy of the baseline, and the difference.
const NOT_RESULTS: [&str; 2] = ["-expected.png", "-failed-diff.png"];

/// Where `library`'s baselines are made.
fn dir(library: &Library) -> PathBuf {
    super::ecosystem_dir().join("baselines").join(library.name)
}

/// The made baselines of `library`, for its site; an error if it has
/// baselines that setup has not made.
pub(super) fn made(library: &Library) -> Result<PathBuf, String> {
    let images = dir(library).join("images");
    if library.baselines.is_some() && !images.is_dir() {
        return Err(format!(
            "{} is missing: run `cargo xtask ecosystem setup` first",
            images.display()
        ));
    }
    Ok(images)
}

/// The pytest options of the runs comparing `library`'s images with the
/// made baselines.
pub(super) fn comparing(library: &Library) -> Result<Vec<String>, String> {
    let Some(Baselines {
        making: Making::PytestMpl,
        read_from,
    }) = library.baselines
    else {
        return Ok(Vec::new());
    };
    let mut options = vec!["--mpl".to_owned()];
    if read_from.is_none() {
        options.push(format!("--mpl-baseline-path={}", made(library)?.display()));
    }
    Ok(options)
}

/// Makes `library`'s baselines with the Python of the venv `venv`, unless
/// they are made with the current pins.
pub(super) fn make(library: &Library, venv: &Path) -> Result<(), String> {
    let Some(baselines) = library.baselines else {
        return Ok(());
    };
    let dir = dir(library);
    let (images, stamp) = (dir.join("images"), dir.join(STAMP));
    let pins = pins()?;
    if images.is_dir() && fs::read_to_string(&stamp).is_ok_and(|made| made == pins) {
        return Ok(());
    }
    let partial = dir.join("images.partial");
    prepare::remove(&partial)?;
    fs::create_dir_all(&partial).map_err(|err| format!("{}: {err}", partial.display()))?;
    let work = dir.join("run");
    let python = python::interpreter(venv, Os::CURRENT);
    let site = site::lay_out(library, &python::site_packages(venv)?, &work, &partial)?;
    let tests = Tests {
        library,
        mode: Mode::Reference,
        python: &python,
        site: &site,
        dir: &work,
    };
    let targets: Vec<String> = library.tests.iter().map(|&test| test.to_owned()).collect();
    match baselines.making {
        Making::PytestMpl => {
            let generate = format!("--mpl-generate-path={}", partial.display());
            tests.run(&[generate], &targets)?;
        }
        Making::Results(results) => from_results(&tests, &site.join(results), &partial, targets)?,
    }
    prepare::remove(&images)?;
    fs::rename(&partial, &images).map_err(|err| format!("{}: {err}", images.display()))?;
    fs::write(&stamp, pins).map_err(|err| format!("{}: {err}", stamp.display()))?;
    println!(
        "made {} baselines of {} in {}",
        count(&images)?,
        library.name,
        images.display()
    );
    Ok(())
}

/// The pins of [`PINNED`], one per line.
fn pins() -> Result<String, String> {
    let mut pins = String::new();
    for file in PINNED {
        let path = python::root().join("python").join(file);
        let text = fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        for line in text.lines().filter(|line| line.contains("==")) {
            pins.push_str(line.trim());
            pins.push('\n');
        }
    }
    Ok(pins)
}

/// Makes the baselines of the tests that save their images in `results`
/// (emptied before each run) into `images`, running `targets` first, then
/// the tests that failed, until a run adds no baseline.
fn from_results(
    tests: &Tests<'_>,
    results: &Path,
    images: &Path,
    mut targets: Vec<String>,
) -> Result<(), String> {
    for _ in 0..MOST_RUNS {
        prepare::remove(results)?;
        fs::create_dir_all(results).map_err(|err| format!("{}: {err}", results.display()))?;
        let outcomes = tests.run(&[], &targets)?;
        if adopt(results, images)? == 0 {
            return Ok(());
        }
        targets = outcomes.failing();
        if targets.is_empty() {
            return Ok(());
        }
    }
    Err(format!(
        "{}'s baselines are still incomplete after {MOST_RUNS} runs",
        tests.library.name
    ))
}

/// Moves the result images in `results` and its directories (not the
/// copies of baselines and the differences comparisons save beside them)
/// that have no baseline in the same place of `images` there, and
/// returns how many it moved.
fn adopt(results: &Path, images: &Path) -> Result<usize, String> {
    let Ok(entries) = fs::read_dir(results) else {
        return Ok(0);
    };
    let mut moved = 0;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let (path, baseline) = (entry.path(), images.join(&name));
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            moved += adopt(&path, &baseline)?;
            continue;
        }
        let name = name.to_string_lossy();
        if !name.ends_with(".png")
            || NOT_RESULTS.iter().any(|suffix| name.ends_with(suffix))
            || baseline.exists()
        {
            continue;
        }
        fs::create_dir_all(images).map_err(|err| format!("{}: {err}", images.display()))?;
        fs::rename(&path, &baseline).map_err(|err| format!("{}: {err}", baseline.display()))?;
        moved += 1;
    }
    Ok(moved)
}

/// The files in `dir` and its directories.
fn count(dir: &Path) -> Result<usize, String> {
    let mut count = 0;
    for entry in fs::read_dir(dir)
        .map_err(|err| format!("{}: {err}", dir.display()))?
        .filter_map(Result::ok)
    {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            count += self::count(&entry.path())?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_without_a_baseline_are_adopted() {
        let root = python::root()
            .join("target/xtask-tests")
            .join(format!("ecosystem-baselines-{}", std::process::id()));
        let (results, images) = (root.join("results"), root.join("images"));
        let module = results.join("test_geom");
        fs::create_dir_all(&module).unwrap();
        fs::create_dir_all(images.join("test_geom")).unwrap();
        for file in ["compared.png", "failing.png"] {
            fs::write(images.join("test_geom").join(file), "baseline").unwrap();
        }
        for file in [
            "new.png",
            "compared.png",
            "compared-expected.png",
            "failing.png",
            "failing-expected.png",
            "failing-failed-diff.png",
            "notes.txt",
        ] {
            fs::write(module.join(file), file).unwrap();
        }
        fs::write(results.join("flat.png"), "flat.png").unwrap();
        assert_eq!(adopt(&results, &images).unwrap(), 2);
        assert_eq!(
            fs::read_to_string(images.join("test_geom/new.png")).unwrap(),
            "new.png"
        );
        assert_eq!(
            fs::read_to_string(images.join("test_geom/failing.png")).unwrap(),
            "baseline"
        );
        assert!(images.join("flat.png").is_file());
        assert!(!module.join("new.png").exists());
        // An image whose baseline was made in an earlier run stays.
        fs::write(module.join("new.png"), "again").unwrap();
        assert_eq!(adopt(&results, &images).unwrap(), 0);
        assert_eq!(count(&images).unwrap(), 4);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pytest_mpl_compares_with_the_made_baselines() {
        let astropy = crate::ecosystem::libraries::LIBRARIES
            .into_iter()
            .find(|library| library.name == "astropy")
            .unwrap();
        let images = dir(&astropy).join("images");
        if images.is_dir() {
            assert_eq!(
                comparing(&astropy).unwrap(),
                [
                    "--mpl".to_owned(),
                    format!("--mpl-baseline-path={}", images.display())
                ]
            );
        } else {
            assert!(comparing(&astropy).is_err());
        }
    }
}
