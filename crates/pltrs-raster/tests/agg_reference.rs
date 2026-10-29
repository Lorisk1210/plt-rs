//! Pixel comparison against matplotlib's Agg backend.
//!
//! `fixtures/cases.txt` describes small scenes; `fixtures/generate.py`
//! rendered them with matplotlib 3.11's `RendererAgg` into
//! `fixtures/agg_reference.bin`. Each scene is rendered here through
//! pltrs-raster the way matplotlib drives Agg and compared channel by
//! channel. All cases are expected to match exactly unless listed in
//! [`TOLERANCES`].
//!
//! To inspect a mismatch, set `DUMP=<case name>` and run with
//! `--nocapture`: every differing pixel is printed.

mod support;

use pltrs_raster::Pixmap;

/// Cases allowed to differ, with the maximum per-channel difference and
/// the reason.
const TOLERANCES: &[(&str, u8, &str)] = &[];

#[derive(Debug, Default)]
struct Diff {
    max: u8,
    pixels: usize,
}

fn compare(actual: &Pixmap, expected: &Pixmap) -> Diff {
    assert_eq!(
        (actual.width(), actual.height()),
        (expected.width(), expected.height())
    );
    let mut diff = Diff::default();
    for (a, e) in actual.data().chunks(4).zip(expected.data().chunks(4)) {
        let worst = a
            .iter()
            .zip(e)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        if worst > 0 {
            diff.pixels += 1;
            diff.max = diff.max.max(worst);
        }
    }
    diff
}

#[test]
fn matches_matplotlib_agg() {
    let references = support::references();
    let cases = support::cases();
    assert_eq!(
        cases.len(),
        references.len(),
        "cases.txt and agg_reference.bin disagree; rerun generate.py"
    );
    let mut failures = Vec::new();
    for case in &cases {
        let expected = &references[&case.name];
        let actual = support::backend::render(case);
        let diff = compare(&actual, expected);
        let allowed = TOLERANCES
            .iter()
            .find(|t| t.0 == case.name)
            .map_or(0, |t| t.1);
        println!(
            "{:<28} max diff {:>3}, {:>4} pixels differ",
            case.name, diff.max, diff.pixels
        );
        if std::env::var("DUMP").ok().as_deref() == Some(case.name.as_str()) {
            for (i, (a, e)) in actual
                .data()
                .chunks(4)
                .zip(expected.data().chunks(4))
                .enumerate()
            {
                if a != e {
                    println!(
                        "  ({}, {}) actual {a:?} expected {e:?}",
                        i as u32 % case.width,
                        i as u32 / case.width
                    );
                }
            }
        }
        if diff.max > allowed {
            failures.push(format!(
                "{}: max diff {} > {allowed} ({} pixels)",
                case.name, diff.max, diff.pixels
            ));
        }
    }
    assert!(failures.is_empty(), "mismatches:\n{}", failures.join("\n"));
}
