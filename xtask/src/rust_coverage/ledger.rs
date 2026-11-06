//! The ledger, `spec/rust-coverage/ledger/<module>.txt`: for each of a
//! matplotlib module's callables, the pltrs (Rust) items that do what it
//! does, or why it does not apply to Rust.
//!
//! Each line is a rule, `PATTERN -> PATH[, PATH...]` or
//! `PATTERN n/a REASON`; `#` starts a comment line. A pattern is a
//! callable's key relative to the module, `Owner.name` for a class'
//! members and `name` at module level. One `*` in a pattern stands for any
//! run of letters, digits and underscores, and a `*` in the paths for what
//! it matched: `Axes.get_* -> pltrs::axes::Axes::*` maps `get_xlim` to
//! `xlim`.
//!
//! A callable takes the rule naming it exactly, wherever it is in the
//! file; otherwise the first wildcard rule that matches it and, for a
//! mapping, whose every path names an item of pltrs's own (not a method
//! of a standard trait such as `Clone`). A rule naming a callable exactly
//! may point at anything rustdoc documents.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use super::rustdoc::{Index, Origin, Resolution};

/// What a rule says about the callables it matches.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    /// pltrs (Rust) does it with these items.
    Maps(Vec<String>),
    /// It does not apply to Rust, for this reason.
    NotApplicable(String),
}

/// One line of the ledger.
#[derive(Debug)]
pub(super) struct Rule {
    /// The file and line it is on, for reports.
    pub(super) place: String,
    /// The key it matches, relative to the module, with at most one `*`.
    pub(super) pattern: String,
    /// What it says.
    pub(super) action: Action,
}

impl Rule {
    /// Parses `text`, the line at `place`.
    fn parse(place: String, text: &str) -> Result<Self, String> {
        let (pattern, rest) = text
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("{place}: a rule needs `-> PATH` or `n/a REASON`"))?;
        let rest = rest.trim();
        let action = if let Some(paths) = rest.strip_prefix("->") {
            let paths: Vec<String> = paths.split(',').map(|p| p.trim().to_owned()).collect();
            if paths.iter().any(String::is_empty) {
                return Err(format!("{place}: an empty path"));
            }
            Action::Maps(paths)
        } else if let Some(reason) = rest.strip_prefix("n/a") {
            let reason = reason.trim();
            if reason.is_empty() {
                return Err(format!("{place}: n/a needs a reason"));
            }
            Action::NotApplicable(reason.to_owned())
        } else {
            return Err(format!("{place}: expected `-> PATH` or `n/a REASON`"));
        };
        let stars = pattern.matches('*').count();
        let target_stars = match &action {
            Action::Maps(paths) => paths.iter().any(|p| p.contains('*')),
            Action::NotApplicable(_) => false,
        };
        if stars > 1 || (target_stars && stars == 0) {
            return Err(format!(
                "{place}: a pattern takes one `*` at most, and paths a `*` only when it does"
            ));
        }
        Ok(Self {
            place,
            pattern: pattern.to_owned(),
            action,
        })
    }

    /// Whether the pattern has a wildcard.
    pub(super) fn is_wildcard(&self) -> bool {
        self.pattern.contains('*')
    }

    /// What the `*` of the pattern matches in `key`, if it matches.
    fn capture<'k>(&self, key: &'k str) -> Option<&'k str> {
        let (before, after) = self.pattern.split_once('*')?;
        let middle = key.strip_prefix(before)?.strip_suffix(after)?;
        middle
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            .then_some(middle)
    }
}

/// The ledger of one module.
#[derive(Debug, Default)]
pub(super) struct ModuleLedger {
    /// The rules naming a callable exactly, by key.
    exact: HashMap<String, usize>,
    /// Every rule, in file order.
    pub(super) rules: Vec<Rule>,
}

/// What the ledger decides for a callable.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Decision {
    /// Mapped by the rule at this index to these paths, all public.
    Mapped(usize, Vec<String>),
    /// Mapped by the rule at this index, which names paths that are gone.
    Broken(usize, Vec<String>),
    /// Not applicable, by the rule at this index.
    NotApplicable(usize),
    /// No rule decides it.
    Unmapped,
}

impl ModuleLedger {
    /// The decision for the callable `key` (relative to the module).
    pub(super) fn decide(&self, key: &str, index: &Index) -> Decision {
        if let Some(&at) = self.exact.get(key) {
            return match &self.rules[at].action {
                Action::NotApplicable(_) => Decision::NotApplicable(at),
                Action::Maps(paths) => {
                    let gone: Vec<String> = paths
                        .iter()
                        .filter(|path| index.resolve(path) == Resolution::Missing)
                        .cloned()
                        .collect();
                    if gone.is_empty() {
                        Decision::Mapped(at, paths.clone())
                    } else {
                        Decision::Broken(at, gone)
                    }
                }
            };
        }
        for (at, rule) in self.rules.iter().enumerate() {
            let Some(capture) = rule.capture(key) else {
                continue;
            };
            match &rule.action {
                Action::NotApplicable(_) => return Decision::NotApplicable(at),
                Action::Maps(paths) => {
                    let paths: Vec<String> =
                        paths.iter().map(|p| p.replace('*', capture)).collect();
                    if paths.iter().all(|path| own(index.resolve(path))) {
                        return Decision::Mapped(at, paths);
                    }
                }
            }
        }
        Decision::Unmapped
    }
}

/// Whether a wildcard rule may map to what `resolution` found.
fn own(resolution: Resolution) -> bool {
    matches!(
        resolution,
        Resolution::Item | Resolution::Member(Origin::Workspace)
    )
}

/// The ledger: every module's rules.
#[derive(Debug, Default)]
pub(super) struct Ledger {
    /// The rules by matplotlib module.
    pub(super) modules: BTreeMap<String, ModuleLedger>,
}

impl Ledger {
    /// Reads every `<module>.txt` of `dir`.
    pub(super) fn read(dir: &Path) -> Result<Self, String> {
        let entries =
            fs::read_dir(dir).map_err(|err| format!("cannot list {}: {err}", dir.display()))?;
        let mut files: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "txt"))
            .collect();
        files.sort();
        let mut ledger = Self::default();
        for file in files {
            let module = file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| format!("{} has no UTF-8 name", file.display()))?
                .to_owned();
            let text = fs::read_to_string(&file)
                .map_err(|err| format!("cannot read {}: {err}", file.display()))?;
            let name = file
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
            ledger.modules.insert(module, parse_module(&name, &text)?);
        }
        Ok(ledger)
    }
}

/// The rules of one file, `name` holding `text`.
fn parse_module(name: &str, text: &str) -> Result<ModuleLedger, String> {
    let mut ledger = ModuleLedger::default();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let rule = Rule::parse(format!("{name}:{}", number + 1), line)?;
        if !rule.is_wildcard() {
            let at = ledger.rules.len();
            if let Some(&first) = ledger.exact.get(&rule.pattern) {
                return Err(format!(
                    "{}: {} already has a rule, at {}",
                    rule.place, rule.pattern, ledger.rules[first].place
                ));
            }
            ledger.exact.insert(rule.pattern.clone(), at);
        }
        ledger.rules.push(rule);
    }
    Ok(ledger)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(text: &str) -> Rule {
        Rule::parse("t.txt:1".into(), text).expect("a valid rule")
    }

    #[test]
    fn rules_map_to_paths_or_give_a_reason() {
        let mapping = rule("Axes.hist -> pltrs::axes::Axes::hist, pltrs::stats::Bins");
        assert_eq!(mapping.pattern, "Axes.hist");
        assert_eq!(
            mapping.action,
            Action::Maps(vec![
                "pltrs::axes::Axes::hist".into(),
                "pltrs::stats::Bins".into()
            ])
        );
        let skipped = rule("_api.*   n/a  Python's deprecation machinery");
        assert_eq!(
            skipped.action,
            Action::NotApplicable("Python's deprecation machinery".into())
        );
        assert!(skipped.is_wildcard());
    }

    #[test]
    fn malformed_rules_are_refused() {
        for text in [
            "Axes.hist",
            "Axes.hist pltrs::axes::Axes::hist",
            "Axes.hist n/a",
            "Axes.hist -> a::B,,c::D",
            "*.get_* -> a::B::*",
            "Axes.hist -> pltrs::*",
        ] {
            assert!(Rule::parse("t.txt:1".into(), text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_wildcard_captures_one_name_part() {
        let getters = rule("Axes.get_* -> pltrs::axes::Axes::*");
        assert_eq!(getters.capture("Axes.get_xlim"), Some("xlim"));
        assert_eq!(getters.capture("Axes.set_xlim"), None);
        assert_eq!(getters.capture("Axes.get_x.y"), None);
        let module_level = rule("* -> pltrs::cm::*");
        assert_eq!(module_level.capture("Axes.hist"), None);
        assert_eq!(module_level.capture("colormaps"), Some("colormaps"));
    }

    #[test]
    fn a_name_has_one_exact_rule() {
        let text = "Axes.hist -> a::B\n# comment\n\nAxes.hist n/a twice\n";
        let err = parse_module("m.txt", text).expect_err("a duplicate");
        assert!(err.contains("m.txt:4") && err.contains("m.txt:1"), "{err}");
        assert_eq!(
            parse_module("m.txt", "Axes.* -> a::B::*\nAxes.* n/a rest\n")
                .expect("wildcards may repeat")
                .rules
                .len(),
            2
        );
    }
}
