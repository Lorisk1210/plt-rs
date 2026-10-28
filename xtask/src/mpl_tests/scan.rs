//! Reading matplotlib's test files and the spec's sources without running
//! Python: the `def test_*` functions of each test module, and the set of
//! words the spec mentions.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::path::{Path, PathBuf};

/// Every test module (`test_*.py`) directly in `dir`, by file name, with
/// the names of the test functions it defines.
pub(super) fn test_modules(dir: &Path) -> io::Result<BTreeMap<String, BTreeSet<String>>> {
    let mut modules = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(file) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_test_module(file) {
            let source = std::fs::read_to_string(&path)?;
            modules.insert(file.to_owned(), test_functions(&source));
        }
    }
    Ok(modules)
}

/// Whether `file` names a test module (`test_*.py`).
pub(super) fn is_test_module(file: &str) -> bool {
    file.strip_suffix(".py")
        .is_some_and(|stem| stem.starts_with("test_"))
}

/// The names of the tests pytest collects from a Python source: the
/// module-level `def test_*` functions and the `def test_*` methods of
/// module-level classes (functions nested in either are helpers).
pub(super) fn test_functions(source: &str) -> BTreeSet<String> {
    let mut tests = BTreeSet::new();
    let mut in_class = false;
    let mut body_indent = None;
    for line in source.lines() {
        let code = line.trim_start();
        if code.is_empty() || code.starts_with('#') {
            continue;
        }
        let indent = line.len() - code.len();
        if indent == 0 {
            in_class = code.starts_with("class ");
            body_indent = None;
        } else if in_class && body_indent.is_none() {
            body_indent = Some(indent);
        }
        let collected = indent == 0 || (in_class && body_indent == Some(indent));
        if let Some(name) = defined_name(code).filter(|_| collected) {
            tests.insert(name.to_owned());
        }
    }
    tests
}

/// The name of the `test_*` function a line of code starts to define.
fn defined_name(code: &str) -> Option<&str> {
    let code = code.strip_prefix("async ").unwrap_or(code);
    let name = code.strip_prefix("def ")?.trim_start();
    let end = name.find(|c: char| !is_word_char(c)).unwrap_or(name.len());
    let name = &name[..end];
    name.starts_with("test_").then_some(name)
}

/// Whether `c` can be part of a Python or Rust identifier.
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The whole words (identifier-like runs) of a text.
pub(super) fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !is_word_char(c))
        .filter(|word| !word.is_empty())
}

/// Every word of the `.py` and `.rs` files under `dirs`, recursively,
/// except those of the file `skip`.
pub(super) fn corpus_words(dirs: &[PathBuf], skip: &Path) -> io::Result<HashSet<String>> {
    let mut found = HashSet::new();
    let mut pending: Vec<PathBuf> = dirs.to_vec();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path != skip
                && path
                    .extension()
                    .is_some_and(|ext| ext == "py" || ext == "rs")
            {
                let text = std::fs::read_to_string(&path)?;
                found.extend(words(&text).map(str::to_owned));
            }
        }
    }
    Ok(found)
}

/// Whether the spec mentions a test: its name, with or without the
/// `test_` prefix, is one of the corpus' words.
pub(super) fn mentioned(corpus: &HashSet<String>, test: &str) -> bool {
    corpus.contains(test)
        || test
            .strip_prefix("test_")
            .is_some_and(|bare| corpus.contains(bare))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_functions_and_methods_once() {
        let source = "\
import pytest

def test_alpha(x):
    def test_helper():
        pass

class TestThing:
    \"\"\"Docs.\"\"\"
# a comment
    @pytest.mark.parametrize('x', [1])
    def test_beta(self):
        def test_nested():
            pass

    def test_epsilon(self):
        pass

async def test_gamma():
    pass

def helper_test_delta():
    pass
def test_alpha(y):
    pass
";
        let names: Vec<_> = test_functions(source).into_iter().collect();
        assert_eq!(
            names,
            ["test_alpha", "test_beta", "test_epsilon", "test_gamma"]
        );
    }

    #[test]
    fn ignores_non_definitions() {
        assert!(test_functions("x = test_alpha()\n# def test_beta\n").is_empty());
        assert!(test_functions("define test_x\n").is_empty());
        assert!(test_functions("").is_empty());
    }

    #[test]
    fn matches_whole_words_with_or_without_prefix() {
        let corpus: HashSet<String> = words("probe(\"op\") # test_alpha, beta_gamma; x.delta2")
            .map(str::to_owned)
            .collect();
        assert!(mentioned(&corpus, "test_alpha"));
        assert!(mentioned(&corpus, "test_beta_gamma"));
        assert!(!mentioned(&corpus, "test_beta"));
        assert!(!mentioned(&corpus, "test_delta"));
        assert!(mentioned(&corpus, "test_delta2"));
    }
}
