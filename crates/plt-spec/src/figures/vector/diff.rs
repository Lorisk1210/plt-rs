//! Normalized drawing items and their tolerant, alignment-based comparison.

use std::fmt::{self, Write as _};

use crate::compare::Tolerance;

/// One atom of a drawing item.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// A number, compared with a tolerance.
    Num(f64),
    /// Anything else, compared exactly.
    Word(String),
}

impl Token {
    /// A word token.
    pub fn word(text: impl Into<String>) -> Self {
        Self::Word(text.into())
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num(value) => write!(f, "{value}"),
            Self::Word(word) => f.write_str(word),
        }
    }
}

/// One drawn element or painting operator with everything it refers to
/// resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// What the item is (element name or operator).
    pub kind: String,
    /// Its normalized content.
    pub tokens: Vec<Token>,
}

impl Item {
    /// An item of `kind` with no content yet.
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            tokens: Vec::new(),
        }
    }

    /// The item's structure: its kind and words, numbers left out.
    fn signature(&self) -> (&str, Vec<&str>) {
        let words = self
            .tokens
            .iter()
            .filter_map(|t| match t {
                Token::Word(w) => Some(w.as_str()),
                Token::Num(_) => None,
            })
            .collect();
        (&self.kind, words)
    }

    /// The largest difference between corresponding numbers, or `None`
    /// when the items do not have the same shape.
    fn deviation(&self, other: &Self) -> Option<f64> {
        if self.kind != other.kind || self.tokens.len() != other.tokens.len() {
            return None;
        }
        let mut worst: f64 = 0.0;
        for (a, b) in self.tokens.iter().zip(&other.tokens) {
            match (a, b) {
                (Token::Num(x), Token::Num(y)) => {
                    if !TIGHT.accepts(*x, *y) {
                        worst = worst.max(if x.is_finite() && y.is_finite() {
                            (x - y).abs()
                        } else {
                            f64::INFINITY
                        });
                    }
                }
                (Token::Word(x), Token::Word(y)) if x == y => {}
                _ => return None,
            }
        }
        Some(worst)
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.kind)?;
        for token in &self.tokens {
            write!(f, " {token}")?;
        }
        Ok(())
    }
}

/// Numbers within this are the same (formatting precision).
const TIGHT: Tolerance = Tolerance {
    rel: 1e-6,
    abs: 1e-3,
};

/// The most differences [`compare_items`] aligns; beyond it the remaining
/// items are paired in order.
const MAX_EDITS: usize = 2000;

/// How one document compares with its golden.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VectorReport {
    /// Items in the golden.
    pub expected: usize,
    /// Items in the rendering.
    pub actual: usize,
    /// Pairs whose numbers all agree.
    pub matching: usize,
    /// Pairs with the same structure but numbers further apart, with the
    /// largest difference of each.
    pub numeric: Vec<(String, f64)>,
    /// Pairs of the same kind with different structure.
    pub structural: Vec<(String, String)>,
    /// Golden items without a counterpart.
    pub missing: Vec<String>,
    /// Rendered items without a counterpart.
    pub extra: Vec<String>,
}

impl VectorReport {
    /// Compares a pair of items of the same kind.
    fn record(&mut self, expected: &Item, actual: &Item) {
        match expected.deviation(actual) {
            Some(d) if d > 0.0 => self.numeric.push((expected.to_string(), d)),
            Some(_) => self.matching += 1,
            None => self
                .structural
                .push((expected.to_string(), actual.to_string())),
        }
    }

    /// The largest numeric difference of a structurally matching pair.
    pub fn max_deviation(&self) -> f64 {
        self.numeric.iter().map(|(_, d)| *d).fold(0.0, f64::max)
    }

    /// Whether the documents agree structurally (numbers may differ).
    pub fn is_structurally_equal(&self) -> bool {
        self.structural.is_empty() && self.missing.is_empty() && self.extra.is_empty()
    }

    /// A multi-line description of the first `limit` differences.
    pub fn details(&self, limit: usize) -> String {
        let mut out = String::new();
        let short = |text: &str| {
            let cut: String = text.chars().take(220).collect();
            if cut.len() < text.len() {
                format!("{cut}…")
            } else {
                cut
            }
        };
        for (item, deviation) in self.numeric.iter().take(limit) {
            let _ = writeln!(out, "    numeric ({deviation:.4}): {}", short(item));
        }
        for (expected, actual) in self.structural.iter().take(limit) {
            let _ = writeln!(out, "    expected: {}", short(expected));
            let _ = writeln!(out, "    actual:   {}", short(actual));
        }
        for item in self.missing.iter().take(limit) {
            let _ = writeln!(out, "    missing:  {}", short(item));
        }
        for item in self.extra.iter().take(limit) {
            let _ = writeln!(out, "    extra:    {}", short(item));
        }
        out
    }
}

impl fmt::Display for VectorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "items {:5}/{:5}  match {:5}  numeric {:4} (max {:8.4})  structural {:3}  missing {:3}  extra {:3}",
            self.actual,
            self.expected,
            self.matching,
            self.numeric.len(),
            self.max_deviation(),
            self.structural.len(),
            self.missing.len(),
            self.extra.len()
        )
    }
}

/// One step of an alignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edit {
    /// Items `a[i]` and `b[j]` have the same signature.
    Keep(usize, usize),
    /// `a[i]` has no counterpart.
    Delete(usize),
    /// `b[j]` has no counterpart.
    Insert(usize),
}

/// Myers' O(ND) shortest edit script between `a` and `b` under `same`, or
/// `None` when it needs more than [`MAX_EDITS`] edits.
fn edit_script<T>(a: &[T], b: &[T], same: impl Fn(&T, &T) -> bool) -> Option<Vec<Edit>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    let max = ((n + m) as usize).min(MAX_EDITS) as isize;
    let offset = max + 1;
    let mut v = vec![0isize; (2 * max + 3) as usize];
    // Before step `d`, only diagonals -d..=d are read back.
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut end = None;
    'search: for d in 0..=max {
        trace.push(v[(offset - d) as usize..=(offset + d) as usize].to_vec());
        for k in (-d..=d).step_by(2) {
            let idx = (k + offset) as usize;
            let mut x = if k == -d || (k != d && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && same(&a[x as usize], &b[y as usize]) {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x >= n && y >= m {
                end = Some(d);
                break 'search;
            }
        }
    }
    let end = end?;
    let mut edits = Vec::new();
    let (mut x, mut y) = (n, m);
    for d in (0..=end).rev() {
        let saved = &trace[d as usize];
        let at = |k: isize| saved[(k + d) as usize];
        let k = x - y;
        let previous_k = if k == -d || (k != d && at(k - 1) < at(k + 1)) {
            k + 1
        } else {
            k - 1
        };
        let previous_x = if d == 0 { 0 } else { at(previous_k) };
        let previous_y = previous_x - previous_k;
        while x > previous_x && y > previous_y {
            x -= 1;
            y -= 1;
            edits.push(Edit::Keep(x as usize, y as usize));
        }
        if d > 0 {
            if x == previous_x {
                edits.push(Edit::Insert(previous_y as usize));
            } else {
                edits.push(Edit::Delete(previous_x as usize));
            }
        }
        x = previous_x;
        y = previous_y;
    }
    edits.reverse();
    Some(edits)
}

/// Pairs the unmatched items of one gap of the alignment by kind, in
/// order, recording structural differences, then the rest as missing or
/// extra.
fn settle_gap(expected: &[&Item], actual: &[&Item], report: &mut VectorReport) {
    let mut used = vec![false; actual.len()];
    for e in expected {
        let partner = actual
            .iter()
            .enumerate()
            .find(|(j, a)| !used[*j] && a.kind == e.kind)
            .map(|(j, _)| j);
        if let Some(j) = partner {
            used[j] = true;
            report.record(e, actual[j]);
        } else {
            report.missing.push(e.to_string());
        }
    }
    for (a, used) in actual.iter().zip(used) {
        if !used {
            report.extra.push(a.to_string());
        }
    }
}

/// Aligns `actual` with `expected` on item structure and compares the
/// pairs' numbers.
pub fn compare_items(expected: &[Item], actual: &[Item]) -> VectorReport {
    let mut report = VectorReport {
        expected: expected.len(),
        actual: actual.len(),
        ..VectorReport::default()
    };
    let signatures_a: Vec<_> = expected.iter().map(Item::signature).collect();
    let signatures_b: Vec<_> = actual.iter().map(Item::signature).collect();
    let Some(edits) = edit_script(&signatures_a, &signatures_b, |x, y| x == y) else {
        let all_a: Vec<&Item> = expected.iter().collect();
        let all_b: Vec<&Item> = actual.iter().collect();
        settle_gap(&all_a, &all_b, &mut report);
        return report;
    };
    let (mut gap_a, mut gap_b): (Vec<&Item>, Vec<&Item>) = (Vec::new(), Vec::new());
    for edit in edits {
        match edit {
            Edit::Keep(i, j) => {
                settle_gap(&gap_a, &gap_b, &mut report);
                gap_a.clear();
                gap_b.clear();
                report.record(&expected[i], &actual[j]);
            }
            Edit::Delete(i) => gap_a.push(&expected[i]),
            Edit::Insert(j) => gap_b.push(&actual[j]),
        }
    }
    settle_gap(&gap_a, &gap_b, &mut report);
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(kind: &str, tokens: &[&str]) -> Item {
        Item {
            kind: kind.to_owned(),
            tokens: tokens
                .iter()
                .map(|t| t.parse().map_or_else(|_| Token::word(*t), Token::Num))
                .collect(),
        }
    }

    #[test]
    fn identical_sequences_match() {
        let a = vec![item("path", &["M", "1", "2"]), item("use", &["#a", "3"])];
        let report = compare_items(&a, &a);
        assert_eq!(report.matching, 2);
        assert!(report.is_structurally_equal());
        assert_eq!(report.max_deviation(), 0.0);
    }

    #[test]
    fn numbers_insertions_and_structure() {
        let expected = vec![
            item("path", &["M", "1", "2"]),
            item("path", &["fill", "red"]),
            item("use", &["5"]),
            item("text", &["a"]),
        ];
        let actual = vec![
            item("path", &["M", "1", "2.5"]),
            item("extra", &[]),
            item("path", &["fill", "blue"]),
            item("use", &["5.0004"]),
        ];
        let report = compare_items(&expected, &actual);
        assert_eq!(report.matching, 1, "use within tolerance");
        assert_eq!(report.numeric.len(), 1);
        assert_eq!(report.max_deviation(), 0.5);
        assert_eq!(report.structural.len(), 1);
        assert_eq!(report.missing, ["text a"]);
        assert_eq!(report.extra, ["extra"]);
        assert!(report.to_string().contains("structural   1"));
        assert!(report.details(5).contains("missing:  text a"));
    }

    #[test]
    fn edit_scripts_are_minimal() {
        let a: Vec<char> = "abcabba".chars().collect();
        let b: Vec<char> = "cbabac".chars().collect();
        let edits = edit_script(&a, &b, |x, y| x == y).unwrap();
        let keeps = edits.iter().filter(|e| matches!(e, Edit::Keep(..))).count();
        assert_eq!(keeps, 4);
        assert_eq!(edits.len(), 4 + 5);
    }
}
