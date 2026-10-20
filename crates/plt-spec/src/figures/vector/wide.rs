//! The wide comparison: every translated case in every vector format
//! against the documents matplotlib wrote into a reference directory
//! (`spec/reference/figures/render_vectors.py`), as opposed to the
//! goldens of the selected `VECTOR_CASES`.

use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use super::{Item, VectorFormat, compare_items, render_vector};
use crate::figures::FigureCase;

/// How one case compares with matplotlib's document.
#[derive(Clone, Debug, PartialEq)]
pub enum WideOutcome {
    /// Same structure, every number within the formatting precision.
    Exact,
    /// Same structure; the largest difference between numbers.
    Numeric(f64),
    /// A different structure (or no rendering), described.
    Different(String),
}

/// One compared case and format.
#[derive(Clone, Debug)]
pub struct WideResult {
    /// The case.
    pub case: &'static str,
    /// The format.
    pub format: VectorFormat,
    /// How it compares.
    pub outcome: WideOutcome,
}

/// Writes a normalized item list, one item per line (for inspection).
pub fn write_items(path: &Path, items: &[Item]) {
    let text = items.iter().fold(String::new(), |mut text, item| {
        let _ = writeln!(text, "{item}");
        text
    });
    let _ = std::fs::write(path, text);
}

/// Compares one case in one format with matplotlib's document in
/// `reference` (`None` when there is none). Both item lists of a case that
/// is not exact are written to `spec/out/vector-wide/`.
pub fn wide_outcome(
    case: &FigureCase,
    format: VectorFormat,
    reference: &Path,
) -> Option<WideOutcome> {
    let name = case.name.replace('/', "__");
    let golden = std::fs::read(reference.join(format!("{name}.{format}"))).ok()?;
    let compared = render_vector(case, format)
        .map_err(|e| format!("render failed: {e}"))
        .and_then(|rendered| {
            let items = (format.items(&golden)?, format.items(&rendered)?);
            Ok((rendered, items))
        });
    let (rendered, (expected, actual)) = match compared {
        Ok(compared) => compared,
        Err(err) => return Some(WideOutcome::Different(err)),
    };
    let report = compare_items(&expected, &actual);
    let outcome = if !report.is_structurally_equal() {
        WideOutcome::Different(format!("{report}\n{}", report.details(3)))
    } else if report.numeric.is_empty() {
        return Some(WideOutcome::Exact);
    } else {
        WideOutcome::Numeric(report.max_deviation())
    };
    let out = crate::spec_root().join("out/vector-wide");
    let _ = std::fs::create_dir_all(&out);
    write_items(
        &out.join(format!("{name}.{format}.expected.txt")),
        &expected,
    );
    write_items(&out.join(format!("{name}.{format}.actual.txt")), &actual);
    let _ = std::fs::write(out.join(format!("{name}.{format}")), rendered);
    Some(outcome)
}

/// Compares `cases` in every format of the wide comparison with the
/// documents in `reference`, on all cores; sorted by case and format.
pub fn wide_comparison(cases: &[FigureCase], reference: &Path) -> Vec<WideResult> {
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(case) = cases.get(next.fetch_add(1, Ordering::Relaxed)) {
                    for format in VectorFormat::WIDE {
                        if let Some(outcome) = wide_outcome(case, format, reference) {
                            results.lock().unwrap_or_else(PoisonError::into_inner).push(
                                WideResult {
                                    case: case.name,
                                    format,
                                    outcome,
                                },
                            );
                        }
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
    results.sort_by_key(|result| (result.case, result.format.extension()));
    results
}

/// The report of a wide comparison: a line per case and format that is not
/// exact, then per format how many cases agree structurally and how many
/// of those exactly.
pub fn wide_summary(results: &[WideResult]) -> String {
    let mut text = String::new();
    for result in results {
        match &result.outcome {
            WideOutcome::Exact => {}
            WideOutcome::Numeric(deviation) => {
                let _ = writeln!(
                    text,
                    "{} {}: numbers differ by up to {deviation:.4}",
                    result.case, result.format
                );
            }
            WideOutcome::Different(problem) => {
                let _ = writeln!(text, "{} {}: {problem}", result.case, result.format);
            }
        }
    }
    for format in VectorFormat::WIDE {
        let outcomes: Vec<&WideOutcome> = results
            .iter()
            .filter(|result| result.format == format)
            .map(|result| &result.outcome)
            .collect();
        let exact = outcomes
            .iter()
            .filter(|o| matches!(o, WideOutcome::Exact))
            .count();
        let numeric = outcomes
            .iter()
            .filter(|o| matches!(o, WideOutcome::Numeric(_)))
            .count();
        let _ = writeln!(
            text,
            "{format}: {}/{} structurally equal, {exact} with every number equal",
            exact + numeric,
            outcomes.len()
        );
    }
    text
}
