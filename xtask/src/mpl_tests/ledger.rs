//! The ledger of matplotlib tests that cannot apply to plt-rs,
//! `spec/reference/not_applicable.txt`: one `test_file.py::test_name:
//! reason` per line, blank lines and `#` comments allowed.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::scan::is_test_module;

/// The tests the ledger lists, by test file.
#[derive(Debug, Default)]
pub(super) struct Ledger {
    /// Test function names per test file name.
    pub(super) entries: BTreeMap<String, BTreeSet<String>>,
}

/// A ledger line that does not have the expected layout.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct LedgerError {
    /// The 1-based line number.
    line: usize,
    /// What is wrong with it.
    why: &'static str,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not_applicable.txt line {}: {}", self.line, self.why)
    }
}

impl std::error::Error for LedgerError {}

impl Ledger {
    /// Parses the ledger's text.
    pub(super) fn parse(text: &str) -> Result<Self, LedgerError> {
        let mut ledger = Self::default();
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let error = |why| LedgerError {
                line: index + 1,
                why,
            };
            let (file, rest) = line
                .split_once("::")
                .ok_or_else(|| error("expected `test_file.py::test_name: reason`"))?;
            let (test, reason) = rest
                .split_once(':')
                .ok_or_else(|| error("expected `: reason` after the test name"))?;
            if !is_test_module(file) {
                return Err(error("the file is not a `test_*.py` module"));
            }
            if !test.starts_with("test_") {
                return Err(error("the test name does not start with `test_`"));
            }
            if reason.trim().is_empty() {
                return Err(error("the reason is empty"));
            }
            if !ledger
                .entries
                .entry(file.to_owned())
                .or_default()
                .insert(test.to_owned())
            {
                return Err(error("the test is listed twice"));
            }
        }
        Ok(ledger)
    }

    /// Whether the ledger lists `test` of `file`.
    pub(super) fn lists(&self, file: &str, test: &str) -> bool {
        self.entries
            .get(file)
            .is_some_and(|tests| tests.contains(test))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_and_skips_comments() {
        let ledger = Ledger::parse(
            "# Pickling\n\ntest_pickle.py::test_simple: Python's pickle protocol\n\
             test_pickle.py::test_renderer: no pickling: none at all\n",
        )
        .expect("valid ledger");
        assert!(ledger.lists("test_pickle.py", "test_simple"));
        assert!(ledger.lists("test_pickle.py", "test_renderer"));
        assert!(!ledger.lists("test_pickle.py", "test_other"));
        assert!(!ledger.lists("test_axes.py", "test_simple"));
    }

    #[test]
    fn rejects_malformed_lines() {
        let line_two = |text| Ledger::parse(text).map(|_| ()).map_err(|err| err.line);
        assert_eq!(line_two("# ok\ntest_a.py test_b: why"), Err(2));
        assert_eq!(line_two("# ok\ntest_a.py::test_b"), Err(2));
        assert_eq!(line_two("# ok\na.py::test_b: why"), Err(2));
        assert_eq!(line_two("# ok\ntest_a.py::b: why"), Err(2));
        assert_eq!(line_two("# ok\ntest_a.py::test_b:   "), Err(2));
        assert_eq!(
            line_two("test_a.py::test_b: x\ntest_a.py::test_b: y"),
            Err(2)
        );
        assert_eq!(line_two(""), Ok(()));
    }
}
