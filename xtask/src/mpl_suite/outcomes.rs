//! Test outcomes: what the pytest plugin records, and the committed
//! outcome files they are merged into (`spec/mpl-suite/<mode>.json`,
//! `spec/ecosystem/<mode>.json`).
//!
//! The plugin (`python/mpl_suite/mpl_suite_report.py`) writes one JSON
//! object per line: `{"id", "outcome", "reason"}` per test and
//! `{"collect", "reason"}` per test file that failed to import. The
//! committed file keeps the outcome of every test id and the collection
//! errors, one entry per line so that git diffs show what changed; the
//! reasons of failures go to `details.json` beside the run's log instead.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::str::FromStr;

use pltrs_spec::json::Json;

/// How one test ended, in pytest's terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Outcome {
    /// It passed.
    Passed,
    /// Its call failed.
    Failed,
    /// Its setup or teardown failed.
    Error,
    /// It was skipped.
    Skipped,
    /// It failed as its `xfail` mark expects.
    Xfailed,
    /// It passed despite a non-strict `xfail` mark.
    Xpassed,
}

impl Outcome {
    /// Every outcome, in report order.
    pub(super) const ALL: [Self; 6] = [
        Self::Passed,
        Self::Failed,
        Self::Error,
        Self::Skipped,
        Self::Xfailed,
        Self::Xpassed,
    ];

    /// pytest's word for it.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Error => "error",
            Self::Skipped => "skipped",
            Self::Xfailed => "xfailed",
            Self::Xpassed => "xpassed",
        }
    }
}

impl FromStr for Outcome {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|outcome| outcome.name() == name)
            .ok_or_else(|| format!("unknown outcome {name:?}"))
    }
}

/// The outcomes of a run, or of all runs merged.
#[derive(Clone, Debug, Default)]
pub(crate) struct Outcomes {
    /// The outcome of each test id.
    pub(super) tests: BTreeMap<String, Outcome>,
    /// Why each test that did not pass ended as it did (not committed).
    pub(super) reasons: BTreeMap<String, String>,
    /// Why each test file that could not be imported failed.
    pub(super) collection_errors: BTreeMap<String, String>,
}

/// `reason` without the parenthesised file paths (`(/home/.../x.py)`) an
/// `ImportError` names: they differ from one checkout to the next, and the
/// collection errors are committed.
fn without_paths(reason: &str) -> String {
    let mut text = reason.to_owned();
    while let Some(start) = text.find(" (/") {
        let Some(length) = text[start..].find(')') else {
            break;
        };
        text.replace_range(start..=start + length, "");
    }
    text
}

/// The test file of a test id (`path/test_x.py` of `path/test_x.py::test`).
pub(super) fn file_of(id: &str) -> &str {
    id.split_once("::").map_or(id, |(file, _)| file)
}

impl Outcomes {
    /// Reads the plugin's JSON lines.
    pub(crate) fn from_results(text: &str) -> Result<Self, String> {
        let mut outcomes = Self::default();
        for (number, line) in text.lines().enumerate().filter(|(_, l)| !l.is_empty()) {
            let record = Json::parse(line).map_err(|err| format!("line {}: {err}", number + 1))?;
            let field = |key: &str| record.get(key).and_then(Json::as_str);
            let reason = without_paths(field("reason").unwrap_or_default());
            if let Some(file) = field("collect") {
                outcomes.collection_errors.insert(file.to_owned(), reason);
                continue;
            }
            let (Some(id), Some(outcome)) = (field("id"), field("outcome")) else {
                return Err(format!("line {}: neither a test nor a file", number + 1));
            };
            let outcome = outcome.parse()?;
            if outcome != Outcome::Passed && !reason.is_empty() {
                outcomes.reasons.insert(id.to_owned(), reason);
            }
            outcomes.tests.insert(id.to_owned(), outcome);
        }
        Ok(outcomes)
    }

    /// Reads a committed outcome file and, if it exists, the details file
    /// with the reasons; a missing outcome file reads as no outcomes.
    pub(crate) fn load(path: &Path, details: &Path) -> Result<Self, String> {
        let mut outcomes = Self::default();
        let Ok(text) = std::fs::read_to_string(path) else {
            return Ok(outcomes);
        };
        let document = Json::parse(&text).map_err(|err| format!("{}: {err}", path.display()))?;
        for (id, outcome) in object(&document, "outcomes") {
            let outcome = outcome
                .as_str()
                .ok_or_else(|| format!("{}: {id}: not a string", path.display()))?;
            outcomes.tests.insert(id.clone(), outcome.parse()?);
        }
        outcomes.collection_errors = strings(object(&document, "collection_errors"));
        if let Ok(text) = std::fs::read_to_string(details) {
            let document =
                Json::parse(&text).map_err(|err| format!("{}: {err}", details.display()))?;
            outcomes.reasons = strings(object(&document, "reasons"));
        }
        Ok(outcomes)
    }

    /// Replaces everything recorded for the test files `run` covers, and
    /// for the files `ran_files` names, by what `run` found.
    pub(crate) fn merge(&mut self, run: Self, ran_files: &[String]) {
        let replaced = |id: &str| {
            let file = file_of(id);
            ran_files.iter().any(|ran| ran == file) || run.covers_file(file)
        };
        self.tests.retain(|id, _| !replaced(id));
        self.reasons.retain(|id, _| !replaced(id));
        self.collection_errors.retain(|file, _| !replaced(file));
        self.tests.extend(run.tests);
        self.reasons.extend(run.reasons);
        self.collection_errors.extend(run.collection_errors);
    }

    /// Forgets what was recorded for the test files under `suite` (test
    /// directories and files of a suite run in full) that are not among
    /// `files`, the suite's files now: ids of files gone from it, or of
    /// files a run of another tree found (a file left in a site by hand),
    /// which no later run would replace.
    pub(crate) fn forget_files_gone(&mut self, suite: &[&str], files: &[String]) {
        let gone = |file: &str| {
            suite
                .iter()
                .any(|top| file == *top || file.starts_with(&format!("{top}/")))
                && !files.iter().any(|kept| kept == file)
        };
        self.tests.retain(|id, _| !gone(file_of(id)));
        self.reasons.retain(|id, _| !gone(file_of(id)));
        self.collection_errors.retain(|file, _| !gone(file));
    }

    /// Whether the run has a test of, or a collection error for, `file`.
    fn covers_file(&self, file: &str) -> bool {
        let prefix = format!("{file}::");
        self.collection_errors.contains_key(file)
            || self
                .tests
                .range(prefix.clone()..)
                .next()
                .is_some_and(|(id, _)| id.starts_with(&prefix))
    }

    /// The ids of the tests that failed or errored.
    pub(crate) fn failing(&self) -> Vec<String> {
        self.tests
            .iter()
            .filter(|(_, outcome)| matches!(outcome, Outcome::Failed | Outcome::Error))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// How many tests ended in each outcome.
    pub(super) fn counts(&self) -> BTreeMap<Outcome, usize> {
        let mut counts = BTreeMap::new();
        for outcome in self.tests.values() {
            *counts.entry(*outcome).or_default() += 1;
        }
        counts
    }

    /// Writes the committed outcome file, which first names the
    /// `versions` of the packages under test (`("matplotlib", "3.11.2")`),
    /// and the details file.
    pub(crate) fn save(
        &self,
        path: &Path,
        details: &Path,
        versions: &[(&str, &str)],
    ) -> Result<(), String> {
        let counts = self.counts();
        let mut text = String::from("{\n");
        for (package, version) in versions {
            let _ = writeln!(text, "  {}: {},", quoted(package), quoted(version));
        }
        text.push_str("  \"counts\": {");
        let counts: Vec<String> = Outcome::ALL
            .iter()
            .map(|o| format!("{}: {}", quoted(o.name()), counts.get(o).unwrap_or(&0)))
            .collect();
        text.push_str(&counts.join(", "));
        text.push_str("},\n");
        write_object(
            &mut text,
            "collection_errors",
            &self.collection_errors,
            |r| quoted(r),
        );
        text.push_str(",\n");
        write_object(&mut text, "outcomes", &self.tests, |o| quoted(o.name()));
        text.push_str("\n}\n");
        write(path, &text)?;

        let mut text = String::from("{\n");
        write_object(&mut text, "reasons", &self.reasons, |r| quoted(r));
        text.push_str("\n}\n");
        write(details, &text)
    }
}

/// The members of the object `document[key]` (none if it is missing).
fn object<'a>(document: &'a Json, key: &str) -> &'a [(String, Json)] {
    document.get(key).and_then(Json::as_object).unwrap_or(&[])
}

/// The string members of an object.
fn strings(members: &[(String, Json)]) -> BTreeMap<String, String> {
    members
        .iter()
        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
        .collect()
}

/// `text` as a JSON string literal.
fn quoted(text: &str) -> String {
    Json::from(text).to_string()
}

/// Appends `"key": {...}` with one member per line, indented by two.
fn write_object<V>(
    text: &mut String,
    key: &str,
    members: &BTreeMap<String, V>,
    value: impl Fn(&V) -> String,
) {
    let _ = write!(text, "  {}: {{", quoted(key));
    let mut first = true;
    for (name, member) in members {
        text.push_str(if first { "\n" } else { ",\n" });
        first = false;
        let _ = write!(text, "    {}: {}", quoted(name), value(member));
    }
    text.push_str(if first { "}" } else { "\n  }" });
}

/// Writes `text` to `path`, creating its directory.
fn write(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| format!("{}: {err}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|err| format!("{}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_lines_become_outcomes_reasons_and_collection_errors() {
        let text = concat!(
            r#"{"id": "a/test_x.py::t1", "outcome": "passed", "reason": ""}"#,
            "\n",
            r#"{"id": "a/test_x.py::t2", "outcome": "failed", "reason": "ValueError: no"}"#,
            "\n\n",
            r#"{"collect": "a/test_y.py", "reason": "ImportError: gone"}"#,
            "\n",
        );
        let outcomes = Outcomes::from_results(text).unwrap();
        assert_eq!(outcomes.tests["a/test_x.py::t1"], Outcome::Passed);
        assert_eq!(outcomes.tests["a/test_x.py::t2"], Outcome::Failed);
        assert_eq!(outcomes.reasons["a/test_x.py::t2"], "ValueError: no");
        assert!(!outcomes.reasons.contains_key("a/test_x.py::t1"));
        assert_eq!(
            outcomes.collection_errors["a/test_y.py"],
            "ImportError: gone"
        );
        assert!(Outcomes::from_results(r#"{"id": "x", "outcome": "odd"}"#).is_err());
        assert_eq!(outcomes.failing(), ["a/test_x.py::t2"]);
    }

    #[test]
    fn reasons_lose_checkout_paths() {
        assert_eq!(
            without_paths("ImportError: cannot import name 'cm' from 'm' (/a/b/__init__.py)"),
            "ImportError: cannot import name 'cm' from 'm'"
        );
        assert_eq!(without_paths("f(x) (/unclosed"), "f(x) (/unclosed");
    }

    #[test]
    fn merging_replaces_the_files_a_run_covers() {
        let mut all = Outcomes::default();
        all.tests.insert("f.py::a".into(), Outcome::Failed);
        all.tests.insert("f.py::gone".into(), Outcome::Failed);
        all.tests.insert("g.py::b".into(), Outcome::Passed);
        all.collection_errors
            .insert("h.py".into(), "ImportError".into());
        let mut run = Outcomes::default();
        run.tests.insert("f.py::a".into(), Outcome::Passed);
        all.merge(run, &["h.py".to_owned()]);
        assert_eq!(all.tests.len(), 2);
        assert_eq!(all.tests["f.py::a"], Outcome::Passed);
        assert_eq!(all.tests["g.py::b"], Outcome::Passed);
        assert!(all.collection_errors.is_empty(), "h.py ran and collected");
    }

    #[test]
    fn a_full_run_forgets_the_files_gone_from_its_suite() {
        let mut all = Outcomes::default();
        for id in [
            "lib/tests/test_a.py::t",
            "lib/tests/plots/test_left_by_hand.py::t",
            "lib/testsuite/test_c.py::t",
            "other/tests/test_d.py::t",
        ] {
            all.tests.insert(id.into(), Outcome::Passed);
        }
        all.collection_errors
            .insert("lib/tests/test_gone.py".into(), "ImportError".into());
        all.forget_files_gone(&["lib/tests"], &["lib/tests/test_a.py".to_owned()]);
        assert_eq!(
            all.tests.keys().collect::<Vec<_>>(),
            [
                "lib/tests/test_a.py::t",
                "lib/testsuite/test_c.py::t",
                "other/tests/test_d.py::t"
            ]
        );
        assert!(all.collection_errors.is_empty());
    }

    #[test]
    fn saved_files_load_back() {
        let dir = std::env::temp_dir().join(format!("mpl-suite-{}", std::process::id()));
        let mut outcomes = Outcomes::default();
        outcomes.tests.insert("f.py::a\"q".into(), Outcome::Xfailed);
        outcomes
            .reasons
            .insert("f.py::a\"q".into(), "why\nnot".into());
        outcomes.collection_errors.insert("g.py".into(), "E".into());
        let (path, details) = (dir.join("o.json"), dir.join("d.json"));
        outcomes
            .save(&path, &details, &[("matplotlib", "3.11.2")])
            .unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("{\n  \"matplotlib\": \"3.11.2\",\n  \"counts\": {"));
        let loaded = Outcomes::load(&path, &details).unwrap();
        assert_eq!(loaded.tests, outcomes.tests);
        assert_eq!(loaded.reasons, outcomes.reasons);
        assert_eq!(loaded.collection_errors, outcomes.collection_errors);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
