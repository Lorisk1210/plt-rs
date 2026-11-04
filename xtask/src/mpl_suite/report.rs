//! The suite report: of the tests that pass under real matplotlib here,
//! the share that passes under pltrs, per test file and in total, and
//! what keeps the others from passing.

use std::collections::{BTreeMap, HashMap};

use super::outcomes::{Outcome, Outcomes, file_of};

/// The longest reason the report prints.
const REASON_WIDTH: usize = 150;

/// How many blockers the report lists.
const BLOCKERS_SHOWN: usize = 60;

/// How many tests a reason holds back, in all, and per test file.
type Blocked<'a> = (usize, Vec<(&'a str, usize)>);

/// One test file's numbers.
#[derive(Clone, Debug, Default)]
struct FileRow {
    /// Tests passing in the reference run.
    reference: usize,
    /// Of those, the tests passing under pltrs.
    pltrs: usize,
    /// Why the others do not pass, with how many each reason holds back.
    blockers: HashMap<String, usize>,
}

/// Prints the outcome counts of the reference run and, per test file,
/// how many of its tests pass there and how many of those pass under
/// pltrs (`pltrs`, when it has run), then the blockers.
pub(crate) fn print(reference: &Outcomes, pltrs: Option<&Outcomes>, failures: bool) {
    print_counts("reference", reference);
    let Some(pltrs) = pltrs else {
        print_files(reference, &rows(reference, None));
        return;
    };
    print_counts("pltrs", pltrs);
    let rows = rows(reference, Some(pltrs));
    print_files(reference, &rows);
    print_skip_parity(reference, pltrs, failures);
    print_blockers(&rows);
    if failures {
        for (id, outcome) in reference
            .tests
            .iter()
            .filter(|(_, o)| **o == Outcome::Passed)
        {
            if pltrs.tests.get(id) != Some(outcome) {
                println!("FAIL {id}: {}", why(id, pltrs));
            }
        }
    }
}

/// Prints how many tests ended in each outcome.
fn print_counts(label: &str, outcomes: &Outcomes) {
    let counts = outcomes.counts();
    let parts: Vec<String> = Outcome::ALL
        .iter()
        .map(|o| format!("{} {}", counts.get(o).unwrap_or(&0), o.name()))
        .collect();
    println!(
        "{label}: {} tests: {}; {} files failed to collect",
        outcomes.tests.len(),
        parts.join(", "),
        outcomes.collection_errors.len()
    );
}

/// The rows of every file with a test passing in the reference run.
fn rows(reference: &Outcomes, pltrs: Option<&Outcomes>) -> BTreeMap<String, FileRow> {
    let mut rows: BTreeMap<String, FileRow> = BTreeMap::new();
    for (id, _) in reference
        .tests
        .iter()
        .filter(|(_, o)| **o == Outcome::Passed)
    {
        let row = rows.entry(file_of(id).to_owned()).or_default();
        row.reference += 1;
        let Some(pltrs) = pltrs else { continue };
        if pltrs.tests.get(id) == Some(&Outcome::Passed) {
            row.pltrs += 1;
        } else {
            *row.blockers.entry(why(id, pltrs)).or_default() += 1;
        }
    }
    rows
}

/// Why test `id`, passing in the reference run, does not pass under
/// pltrs: its file's collection error, its own reason, or its outcome.
fn why(id: &str, pltrs: &Outcomes) -> String {
    let reason = match (pltrs.tests.get(id), pltrs.reasons.get(id)) {
        (_, Some(reason)) => reason.clone(),
        (Some(outcome), None) => outcome.name().to_owned(),
        (None, None) => pltrs.collection_errors.get(file_of(id)).map_or_else(
            || "not collected".to_owned(),
            |error| format!("collection: {error}"),
        ),
    };
    normalize(&reason)
}

/// `reason` on one line, without the object addresses that make equal
/// reasons differ, and cut to [`REASON_WIDTH`] characters.
fn normalize(reason: &str) -> String {
    let mut text = String::with_capacity(reason.len());
    let mut words = reason.split_whitespace().peekable();
    while let Some(word) = words.next() {
        match word.find("0x") {
            Some(at) if word[at + 2..].starts_with(|c: char| c.is_ascii_hexdigit()) => {
                text.push_str(&word[..at]);
                text.push_str("0x…");
                let rest = word[at + 2..].trim_start_matches(|c: char| c.is_ascii_hexdigit());
                text.push_str(rest);
            }
            _ => text.push_str(word),
        }
        if words.peek().is_some() {
            text.push(' ');
        }
    }
    match text.char_indices().nth(REASON_WIDTH) {
        Some((at, _)) => format!("{}…", &text[..at]),
        None => text,
    }
}

/// `part` of `whole` as a percentage (0 of nothing).
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

/// Prints the per-file table and the total.
fn print_files(reference: &Outcomes, rows: &BTreeMap<String, FileRow>) {
    println!();
    println!(
        "{:<56} {:>9} {:>7} {:>7}  main blocker",
        "file", "ref pass", "pltrs", "share"
    );
    let (mut total_reference, mut total_pltrs) = (0, 0);
    for (file, row) in rows {
        total_reference += row.reference;
        total_pltrs += row.pltrs;
        let blocker = row
            .blockers
            .iter()
            .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(reason, count)| format!("{count}: {reason}"))
            .unwrap_or_default();
        println!(
            "{file:<56} {:>9} {:>7} {:>6.1}%  {blocker}",
            row.reference,
            row.pltrs,
            percent(row.pltrs, row.reference)
        );
    }
    let uncollected = reference
        .collection_errors
        .keys()
        .filter(|file| !rows.contains_key(*file))
        .count();
    println!(
        "{:<56} {total_reference:>9} {total_pltrs:>7} {:>6.1}%",
        "TOTAL",
        percent(total_pltrs, total_reference)
    );
    if uncollected > 0 {
        println!("({uncollected} file(s) fail to collect in the reference run itself)");
    }
}

/// Prints how many tests the reference run skips that pltrs does not.
fn print_skip_parity(reference: &Outcomes, pltrs: &Outcomes, failures: bool) {
    let mismatched: Vec<&String> = reference
        .tests
        .iter()
        .filter(|(id, o)| {
            **o == Outcome::Skipped
                && pltrs
                    .tests
                    .get(*id)
                    .is_some_and(|other| *other != Outcome::Skipped)
        })
        .map(|(id, _)| id)
        .collect();
    println!(
        "\n{} test(s) skipped in the reference run are not skipped under pltrs",
        mismatched.len()
    );
    if failures {
        for id in mismatched {
            println!("SKIP-MISMATCH {id}: {}", pltrs.tests[id].name());
        }
    }
}

/// Prints the reasons holding back reference-passing tests, the most
/// tests first, with the files they hold back most.
fn print_blockers(rows: &BTreeMap<String, FileRow>) {
    let mut blockers: HashMap<&str, Blocked<'_>> = HashMap::new();
    for (file, row) in rows {
        for (reason, count) in &row.blockers {
            let entry = blockers.entry(reason).or_default();
            entry.0 += count;
            entry.1.push((file, *count));
        }
    }
    let mut blockers: Vec<_> = blockers.into_iter().collect();
    blockers.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(b.0)));
    println!(
        "\nblockers ({} distinct reasons; the {BLOCKERS_SHOWN} holding back most tests):",
        blockers.len()
    );
    for (reason, (count, mut files)) in blockers.into_iter().take(BLOCKERS_SHOWN) {
        files.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        let shown: Vec<String> = files
            .iter()
            .take(3)
            .map(|(file, n)| format!("{} {n}", file.rsplit('/').next().unwrap_or(file)))
            .collect();
        let more = files.len().saturating_sub(3);
        let more = if more > 0 {
            format!(", +{more} files")
        } else {
            String::new()
        };
        println!("{count:>6}  {reason}\n        [{}{more}]", shown.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_lose_addresses_and_line_breaks() {
        assert_eq!(
            normalize("TypeError: <Foo object at 0x7f3a9c>\n  is not callable"),
            "TypeError: <Foo object at 0x…> is not callable"
        );
        assert_eq!(normalize("x0y 0xZ"), "x0y 0xZ");
        assert_eq!(
            normalize(&"a".repeat(200)).chars().count(),
            REASON_WIDTH + 1
        );
    }

    #[test]
    fn rows_count_reference_passes_and_what_blocks_them() {
        let mut reference = Outcomes::default();
        let mut pltrs = Outcomes::default();
        for (id, outcome) in [
            ("f.py::a", Outcome::Passed),
            ("f.py::b", Outcome::Passed),
            ("f.py::c", Outcome::Skipped),
            ("g.py::d", Outcome::Passed),
        ] {
            reference.tests.insert(id.into(), outcome);
        }
        pltrs.tests.insert("f.py::a".into(), Outcome::Passed);
        pltrs.tests.insert("f.py::b".into(), Outcome::Failed);
        pltrs
            .reasons
            .insert("f.py::b".into(), "KeyError: 'x'".into());
        pltrs
            .collection_errors
            .insert("g.py".into(), "ImportError: y".into());
        let rows = rows(&reference, Some(&pltrs));
        assert_eq!((rows["f.py"].reference, rows["f.py"].pltrs), (2, 1));
        assert_eq!(rows["f.py"].blockers["KeyError: 'x'"], 1);
        assert_eq!(rows["g.py"].blockers["collection: ImportError: y"], 1);
    }
}
