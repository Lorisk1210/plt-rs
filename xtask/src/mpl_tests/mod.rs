//! `cargo xtask mpl-tests [--strict]`: accounts for every test of
//! matplotlib's own suite (the `def test_*` functions of
//! `matplotlib/tests/test_*.py` in the reference venv, read without
//! running Python).
//!
//! A test is *ported* when its name, with or without the `test_` prefix,
//! is a whole word somewhere in `spec/reference` or `crates/pltrs-spec/src`
//! (the probes and figure cases are named after the tests they port, or
//! cite them); *n/a* when the ledger `spec/reference/not_applicable.txt`
//! lists it; *unaccounted* otherwise. `--strict` fails when a test is
//! unaccounted, or when the ledger lists a test that no longer exists or
//! is also ported.

mod ledger;
mod scan;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ledger::Ledger;

/// Where a matplotlib test stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    /// The spec ports it.
    Ported,
    /// The ledger explains why it cannot apply.
    NotApplicable,
    /// Neither.
    Unaccounted,
}

/// The status of every test, by test file, and the ledger's inconsistencies.
#[derive(Debug, Default)]
struct Census {
    /// Each file's tests with their status.
    files: BTreeMap<String, Vec<(String, Status)>>,
    /// Ledger entries naming a test that does not exist.
    stale: Vec<String>,
    /// Ledger entries naming a test the spec ports.
    ported_too: Vec<String>,
}

impl Census {
    /// Classifies the tests of `modules`.
    fn take(
        modules: &BTreeMap<String, BTreeSet<String>>,
        ledger: &Ledger,
        corpus: &HashSet<String>,
    ) -> Self {
        let mut census = Self::default();
        for (file, tests) in modules {
            let statuses = tests
                .iter()
                .map(|test| {
                    let listed = ledger.lists(file, test);
                    let status = if scan::mentioned(corpus, test) {
                        if listed {
                            census.ported_too.push(format!("{file}::{test}"));
                        }
                        Status::Ported
                    } else if listed {
                        Status::NotApplicable
                    } else {
                        Status::Unaccounted
                    };
                    (test.clone(), status)
                })
                .collect();
            census.files.insert(file.clone(), statuses);
        }
        for (file, tests) in &ledger.entries {
            let known = modules.get(file);
            census.stale.extend(
                tests
                    .iter()
                    .filter(|test| !known.is_some_and(|names| names.contains(*test)))
                    .map(|test| format!("{file}::{test}")),
            );
        }
        census
    }

    /// The tests nobody accounts for, as `file::test`.
    fn unaccounted(&self) -> impl Iterator<Item = String> + '_ {
        self.files.iter().flat_map(|(file, tests)| {
            tests
                .iter()
                .filter(|(_, status)| *status == Status::Unaccounted)
                .map(move |(test, _)| format!("{file}::{test}"))
        })
    }

    /// Whether `--strict` passes.
    fn is_clean(&self) -> bool {
        self.stale.is_empty() && self.ported_too.is_empty() && self.unaccounted().next().is_none()
    }

    /// Prints the table per file, the totals and every problem.
    fn print(&self) {
        println!(
            "{:<36} {:>7} {:>7} {:>7} {:>12}",
            "file", "tests", "ported", "n/a", "unaccounted"
        );
        let mut totals = [0_usize; 4];
        for (file, tests) in &self.files {
            let count = |wanted| tests.iter().filter(|(_, s)| *s == wanted).count();
            let row = [
                tests.len(),
                count(Status::Ported),
                count(Status::NotApplicable),
                count(Status::Unaccounted),
            ];
            println!(
                "{file:<36} {:>7} {:>7} {:>7} {:>12}",
                row[0], row[1], row[2], row[3]
            );
            for (total, n) in totals.iter_mut().zip(row) {
                *total += n;
            }
        }
        let [tests, ported, na, unaccounted] = totals;
        println!(
            "{:<36} {tests:>7} {ported:>7} {na:>7} {unaccounted:>12}",
            "TOTAL"
        );
        for test in self.unaccounted() {
            println!("UNACCOUNTED {test}");
        }
        for test in &self.stale {
            println!("STALE {test}: the ledger lists a test that does not exist");
        }
        for test in &self.ported_too {
            println!("PORTED {test}: the ledger lists a test the spec ports");
        }
    }
}

/// `matplotlib/tests` in the reference venv under `root`.
fn matplotlib_tests_dir(root: &Path) -> Result<PathBuf, String> {
    let lib = root.join(".venv/lib");
    let entries = std::fs::read_dir(&lib).map_err(|err| format!("{}: {err}", lib.display()))?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("site-packages/matplotlib/tests"))
        .find(|dir| dir.is_dir())
        .ok_or_else(|| format!("no matplotlib/tests under {}", lib.display()))
}

/// Scans the tests, the ledger and the spec, and classifies the tests.
fn take_census(root: &Path) -> Result<Census, String> {
    let tests_dir = matplotlib_tests_dir(root)?;
    let modules =
        scan::test_modules(&tests_dir).map_err(|err| format!("{}: {err}", tests_dir.display()))?;
    let ledger_path = root.join("spec/reference/not_applicable.txt");
    let ledger_text = std::fs::read_to_string(&ledger_path)
        .map_err(|err| format!("{}: {err}", ledger_path.display()))?;
    let ledger = Ledger::parse(&ledger_text).map_err(|err| err.to_string())?;
    let spec_dirs = [
        root.join("spec/reference"),
        root.join("crates/pltrs-spec/src"),
    ];
    let corpus = scan::corpus_words(&spec_dirs, &ledger_path)
        .map_err(|err| format!("reading the spec: {err}"))?;
    Ok(Census::take(&modules, &ledger, &corpus))
}

/// Runs the census and prints it; with `--strict`, fails unless every
/// test is accounted for and the ledger is consistent.
pub(crate) fn run(args: &[String]) -> ExitCode {
    let mut strict = false;
    for arg in args {
        match arg.as_str() {
            "--strict" => strict = true,
            other => {
                eprintln!("unknown argument {other}");
                eprintln!("usage: cargo xtask mpl-tests [--strict]");
                return ExitCode::FAILURE;
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    match take_census(&root) {
        Ok(census) => {
            census.print();
            if strict && !census.is_clean() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn census(ledger: &str, corpus: &str) -> Census {
        let modules = BTreeMap::from([
            (
                "test_a.py".to_owned(),
                BTreeSet::from(["test_one".to_owned(), "test_two".to_owned()]),
            ),
            (
                "test_b.py".to_owned(),
                BTreeSet::from(["test_three".to_owned()]),
            ),
        ]);
        let corpus = scan::words(corpus).map(str::to_owned).collect();
        Census::take(
            &modules,
            &Ledger::parse(ledger).expect("valid ledger"),
            &corpus,
        )
    }

    #[test]
    fn classifies_ported_listed_and_unaccounted_tests() {
        let census = census("test_a.py::test_two: no such thing\n", "fn one() {}");
        assert_eq!(
            census.files["test_a.py"],
            [
                ("test_one".to_owned(), Status::Ported),
                ("test_two".to_owned(), Status::NotApplicable),
            ]
        );
        assert_eq!(
            census.unaccounted().collect::<Vec<_>>(),
            ["test_b.py::test_three"]
        );
        assert!(!census.is_clean());
    }

    #[test]
    fn clean_when_everything_is_accounted_for() {
        let census = census("test_a.py::test_two: no such thing\n", "test_one three");
        assert!(census.is_clean());
    }

    #[test]
    fn flags_stale_and_ported_ledger_entries() {
        let census = census(
            "test_a.py::test_one: x\ntest_a.py::test_gone: y\ntest_c.py::test_two: z\n",
            "one two three",
        );
        assert_eq!(census.ported_too, ["test_a.py::test_one"]);
        assert_eq!(
            census.stale,
            ["test_a.py::test_gone", "test_c.py::test_two"]
        );
        assert!(!census.is_clean());
    }
}
