//! The census: every callable of matplotlib with the ledger's decision on
//! it, the shares per module, and the ledger's problems.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use super::api::Callable;
use super::ledger::{Action, Decision, Ledger, ModuleLedger};
use super::rustdoc::Index;

/// The counts of one module (or of all).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Counts {
    /// Callables that count (deprecated ones do not).
    pub(super) callables: usize,
    /// Mapped to public pltrs items.
    pub(super) mapped: usize,
    /// Not applicable to Rust.
    pub(super) not_applicable: usize,
    /// Neither (including those whose rule names items that are gone).
    pub(super) unmapped: usize,
    /// Deprecated by matplotlib, left out.
    pub(super) deprecated: usize,
}

impl Counts {
    /// The share of the applicable callables that are mapped, in percent.
    pub(super) fn covered(&self) -> f64 {
        let applicable = self.mapped + self.unmapped;
        if applicable == 0 {
            100.0
        } else {
            100.0 * self.mapped as f64 / applicable as f64
        }
    }

    fn add(&mut self, other: &Self) {
        self.callables += other.callables;
        self.mapped += other.mapped;
        self.not_applicable += other.not_applicable;
        self.unmapped += other.unmapped;
        self.deprecated += other.deprecated;
    }
}

/// Every callable's decision, and what is wrong with the ledger.
#[derive(Debug)]
pub(super) struct Census<'a> {
    /// The callables that count, with their decisions, in matplotlib's order.
    pub(super) decided: Vec<(&'a Callable, Decision)>,
    /// The counts per module.
    pub(super) modules: BTreeMap<&'a str, Counts>,
    /// The ledger lines naming callables or items that are gone, and rules
    /// that decide nothing.
    pub(super) stale: Vec<String>,
}

impl<'a> Census<'a> {
    /// Decides every callable of `callables` with `ledger` against `index`.
    pub(super) fn take(callables: &'a [Callable], ledger: &Ledger, index: &Index) -> Self {
        let mut decided = Vec::new();
        let mut modules: BTreeMap<&str, Counts> = BTreeMap::new();
        let mut used: HashMap<(String, usize), usize> = HashMap::new();
        let mut stale = Vec::new();
        for callable in callables {
            let counts = modules.entry(&callable.module).or_default();
            if callable.deprecated {
                counts.deprecated += 1;
                continue;
            }
            counts.callables += 1;
            let decision = ledger
                .modules
                .get(&callable.module)
                .map_or(Decision::Unmapped, |rules| {
                    rules.decide(&callable.key, index)
                });
            match &decision {
                Decision::Mapped(..) => counts.mapped += 1,
                Decision::NotApplicable(_) => counts.not_applicable += 1,
                Decision::Broken(..) | Decision::Unmapped => counts.unmapped += 1,
            }
            if let Decision::Mapped(at, _) | Decision::Broken(at, _) | Decision::NotApplicable(at) =
                &decision
            {
                *used.entry((callable.module.clone(), *at)).or_default() += 1;
            }
            if let Decision::Broken(at, gone) = &decision {
                let rule = &ledger.modules[&callable.module].rules[*at];
                for path in gone {
                    stale.push(gone_item(&rule.place, path, index));
                }
            }
            decided.push((callable, decision));
        }
        for (module, rules) in &ledger.modules {
            if !modules.contains_key(module.as_str()) {
                stale.push(format!("{module}.txt: matplotlib has no module {module}"));
                continue;
            }
            for (at, rule) in rules.rules.iter().enumerate() {
                if used.contains_key(&(module.clone(), at)) {
                    continue;
                }
                stale.push(if rule.is_wildcard() {
                    format!("{}: {} decides no callable", rule.place, rule.pattern)
                } else {
                    format!(
                        "{}: {} is no counted callable of {module} (gone, or deprecated)",
                        rule.place, rule.pattern
                    )
                });
            }
        }
        Self {
            decided,
            modules,
            stale,
        }
    }

    /// The counts of all modules together.
    pub(super) fn total(&self) -> Counts {
        let mut total = Counts::default();
        for counts in self.modules.values() {
            total.add(counts);
        }
        total
    }

    /// The unmapped callables' names by group (class or module).
    pub(super) fn unmapped(&self) -> BTreeMap<String, Vec<&'a str>> {
        let mut groups: BTreeMap<String, Vec<&str>> = BTreeMap::new();
        for (callable, decision) in &self.decided {
            if matches!(decision, Decision::Unmapped | Decision::Broken(..)) {
                groups
                    .entry(callable.group())
                    .or_default()
                    .push(callable.name());
            }
        }
        groups
    }

    /// The table of counts per module, with the total, as Markdown.
    pub(super) fn table(&self) -> String {
        let mut out = String::from(
            "| module | callables | mapped | n/a | unmapped | covered | deprecated |\n\
             |---|---:|---:|---:|---:|---:|---:|\n",
        );
        let rows = self.modules.iter().map(|(m, c)| (format!("`{m}`"), *c));
        for (name, c) in rows.chain([("**total**".to_owned(), self.total())]) {
            let _ = writeln!(
                out,
                "| {name} | {} | {} | {} | {} | {} | {} |",
                c.callables,
                c.mapped,
                c.not_applicable,
                c.unmapped,
                percent(c.covered()),
                c.deprecated
            );
        }
        out
    }

    /// One line per callable: its key, `mapped`/`n/a`/`unmapped`, the
    /// paths or the reason, and the rule's place (tab-separated).
    pub(super) fn mapping(&self, ledger: &Ledger) -> String {
        let mut out = String::new();
        for (callable, decision) in &self.decided {
            let rules = ledger.modules.get(&callable.module);
            let (status, detail, place) = match decision {
                Decision::Mapped(at, paths) => ("mapped", paths.join(", "), place(rules, *at)),
                Decision::Broken(at, gone) => ("broken", gone.join(", "), place(rules, *at)),
                Decision::NotApplicable(at) => {
                    let reason = rules.map_or(String::new(), |r| match &r.rules[*at].action {
                        Action::NotApplicable(reason) => reason.clone(),
                        Action::Maps(_) => String::new(),
                    });
                    ("n/a", reason, place(rules, *at))
                }
                Decision::Unmapped => ("unmapped", String::new(), String::new()),
            };
            let _ = writeln!(
                out,
                "{}:{}\t{status}\t{detail}\t{place}",
                callable.module, callable.key
            );
        }
        out
    }
}

/// `share` (in percent) with one decimal, rounded down, so that a share
/// short of all never reads 100%.
pub(super) fn percent(share: f64) -> String {
    format!("{:.1}%", (share * 10.0).floor() / 10.0)
}

/// The place of the rule at `at`.
fn place(rules: Option<&ModuleLedger>, at: usize) -> String {
    rules.map_or(String::new(), |r| r.rules[at].place.clone())
}

/// The problem of a rule at `place` naming `path`, which is not public,
/// with the items it may have meant.
fn gone_item(place: &str, path: &str, index: &Index) -> String {
    let namesakes = index.namesakes(path);
    let mut line = format!("{place}: {path} is no public item of pltrs");
    if !namesakes.is_empty() {
        let shown: Vec<&str> = namesakes.into_iter().take(4).collect();
        let _ = write!(line, " (namesakes: {})", shown.join(", "));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_counts_only_the_applicable_callables() {
        let counts = Counts {
            callables: 10,
            mapped: 6,
            not_applicable: 2,
            unmapped: 2,
            deprecated: 3,
        };
        assert!((counts.covered() - 75.0).abs() < 1e-9);
        assert!((Counts::default().covered() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn shares_short_of_all_never_read_100() {
        assert_eq!(percent(100.0 * 3830.0 / 3831.0), "99.9%");
        assert_eq!(percent(100.0), "100.0%");
        assert_eq!(percent(96.66), "96.6%");
    }
}
