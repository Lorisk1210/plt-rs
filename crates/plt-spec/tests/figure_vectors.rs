//! Structural comparison of plt-rs's SVG, PDF and PostScript documents
//! with matplotlib's: the cases in `VECTOR_CASES`, the font variants in
//! `VECTOR_VARIANTS` (Type 42 fonts, SVG text as text) and the multi-page
//! PDF of `PDF_PAGES`.
//!
//! Both documents are reduced to drawing items (see
//! `plt_spec::figures::vector`): ids, object numbers, metadata, comments,
//! compression, image and font program encodings and redundant
//! graphics-state operators do not count, every drawn element, its
//! effective style and every glyph used do. Every document must agree
//! structurally with all numbers within the formatting precision, apart
//! from the plt-rs SVG group hierarchy (plt-rs opens groups only for
//! lines, patches and collections; groups carrying no style are
//! transparent to the comparison) and the cases in [`KNOWN_GAPS`].
//!
//! Prints one line per case and format (run with `--nocapture`) and
//! writes the plt-rs documents and both normalized item lists to
//! `spec/out/vector/` for inspection.
//!
//! The ignored `every_case_matches_matplotlib_structurally` compares every
//! translated case with documents matplotlib wrote into
//! `$PLT_VECTOR_REFERENCE` (see `spec/reference/figures/render_vectors.py`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use plt_spec::figures::vector::{self, VectorFormat, VectorReport};
use plt_spec::figures::{self, FigureCase};

/// Cases exempt from agreement (still reported), with the reason.
const KNOWN_GAPS: &[(&str, &str)] = &[
    (
        "afm/ps_fonts_text",
        "ps.useafm: matplotlib measures with the base-35 AFM files, whose notices forbid \
         redistribution; plt-rs uses the 14 core fonts' metrics (serif is Times, not New \
         Century Schoolbook, and the newer Helvetica kerns f-eacute)",
    ),
    (
        "contour/tripcolor_gouraud",
        "Delaunay triangles come in a different order than qhull's (same triangles)",
    ),
    (
        "hist/hist2d_lognorm_range",
        "numpy's AVX-512 log10/power round two LogNorm colorbar boundaries lying on pixel \
         centres one ulp differently from libm; with numpy limited to AVX2 matplotlib matches",
    ),
    ("mplot3d/axlim_bar3d", EQUAL_DEPTH_ORDER),
    ("mplot3d/axlim_fill_between", EQUAL_DEPTH_ORDER),
    ("mplot3d/axlim_surface", EQUAL_DEPTH_ORDER),
];

/// Why some `axlim_clip` cases order their hidden faces differently.
const EQUAL_DEPTH_ORDER: &str = "Poly3DCollection sorts faces with numpy's unstable argsort, \
     whose order for faces of equal depth (fully hidden ones, partly hidden ones sharing their \
     visible vertices) depends on the CPU's SIMD sort; only invisible or degenerate faces differ";

/// Why a report fails the acceptance policy, if it does.
fn problems(report: &VectorReport) -> Option<String> {
    if !report.is_structurally_equal() {
        return Some(format!("structural differences:\n{}", report.details(4)));
    }
    (!report.numeric.is_empty()).then(|| {
        let mut text = String::from("numbers differ:\n");
        for (item, deviation) in report.numeric.iter().take(4) {
            let short: String = item.chars().take(200).collect();
            let _ = writeln!(text, "    ({deviation:.4}) {short}");
        }
        text
    })
}

/// Writes the plt-rs document `rendered` and the normalized item lists of
/// it and of the golden at `golden` to `spec/out/vector/<stem>.*`.
fn dump(stem: &str, format: VectorFormat, golden: &Path, rendered: Option<Vec<u8>>) {
    let out = plt_spec::spec_root().join("out/vector");
    let _ = std::fs::create_dir_all(&out);
    if let Some(document) = &rendered {
        let _ = std::fs::write(out.join(format!("{stem}.{format}")), document);
    }
    for (suffix, document) in [
        ("expected", std::fs::read(golden).ok()),
        ("actual", rendered),
    ] {
        let Some(items) = document.and_then(|d| format.items(&d).ok()) else {
            continue;
        };
        write_items(&out.join(format!("{stem}.{format}.{suffix}.txt")), &items);
    }
}

#[test]
fn vector_documents_match_matplotlib_structurally() {
    let cases = figures::translated_cases();
    let mut failures = Vec::new();
    for name in vector::VECTOR_CASES {
        let Some(case) = cases.iter().find(|c| c.name == *name) else {
            failures.push(format!("{name}: no Rust translation"));
            continue;
        };
        let gap = KNOWN_GAPS.iter().find(|(gap, _)| gap == name);
        for format in VectorFormat::ALL {
            dump(
                &name.replace('/', "__"),
                format,
                &case.vector_path(format),
                vector::render_vector(case, format).ok(),
            );
            match vector::vector_report(case, format) {
                Ok(report) => {
                    let note =
                        gap.map_or(String::new(), |(_, why)| format!("  [known gap: {why}]"));
                    println!("{name:<40} {format:<4} {report}{note}");
                    if let (Some(problem), None) = (problems(&report), gap) {
                        failures.push(format!("{name} {format}: {problem}"));
                    }
                }
                Err(err) => failures.push(format!("{name} {format}: {err}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "vector comparison failures:\n{}",
        failures.join("\n")
    );
}

/// The translated case called `name`.
fn translated(name: &str) -> FigureCase {
    figures::translated_cases()
        .into_iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("{name} has a Rust translation"))
}

#[test]
fn vector_variants_match_matplotlib_structurally() {
    let mut failures = Vec::new();
    for variant in vector::VECTOR_VARIANTS {
        let case = translated(variant.case);
        for &format in variant.formats {
            let rendered = vector::render_vector_with(&case, format, variant.rc);
            let golden = case.variant_path(variant.name, format);
            let stem = format!("{}.{}", case.name.replace('/', "__"), variant.name);
            dump(&stem, format, &golden, rendered.as_ref().ok().cloned());
            match vector::compare_documents(format, &golden, rendered) {
                Ok(report) => {
                    println!("{:<30} {:<8} {format:<4} {report}", case.name, variant.name);
                    if let Some(problem) = problems(&report) {
                        failures.push(format!(
                            "{} {} {format}: {problem}",
                            case.name, variant.name
                        ));
                    }
                }
                Err(err) => {
                    failures.push(format!("{} {} {format}: {err}", case.name, variant.name));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "vector variant failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn pdf_pages_match_matplotlib_structurally() {
    let cases: Vec<FigureCase> = vector::PDF_PAGES
        .iter()
        .map(|name| translated(name))
        .collect();
    let golden = plt_spec::spec_root().join("goldens/figures/vectors/pages.pdf");
    let rendered = vector::render_pdf_pages(&cases);
    dump(
        "vectors__pages",
        VectorFormat::Pdf,
        &golden,
        rendered.as_ref().ok().cloned(),
    );
    let report = vector::compare_documents(VectorFormat::Pdf, &golden, rendered)
        .unwrap_or_else(|err| panic!("{err}"));
    println!("vectors/pages pdf {report}");
    if let Some(problem) = problems(&report) {
        panic!("multi-page PDF: {problem}");
    }
}

/// How one case compares with matplotlib's document in the wide
/// comparison.
enum WideOutcome {
    /// Same structure, every number within the formatting precision.
    Exact,
    /// Same structure; the largest difference between numbers.
    Numeric(f64),
    /// A different structure (or no rendering), described.
    Different(String),
}

/// Writes a normalized item list, one item per line.
fn write_items(path: &Path, items: &[vector::Item]) {
    let text = items.iter().fold(String::new(), |mut text, item| {
        let _ = writeln!(text, "{item}");
        text
    });
    let _ = std::fs::write(path, text);
}

/// Compares one case in one format with matplotlib's document in
/// `reference` (`None` when there is none). Both item lists of a case
/// that is not exact are written to `spec/out/vector-wide/`.
fn wide_outcome(case: &FigureCase, format: VectorFormat, reference: &Path) -> Option<WideOutcome> {
    let name = case.name.replace('/', "__");
    let golden = std::fs::read(reference.join(format!("{name}.{format}"))).ok()?;
    let compared = vector::render_vector(case, format)
        .map_err(|e| format!("render failed: {e}"))
        .and_then(|rendered| {
            let items = (format.items(&golden)?, format.items(&rendered)?);
            Ok((rendered, items))
        });
    let (rendered, (expected, actual)) = match compared {
        Ok(compared) => compared,
        Err(err) => return Some(WideOutcome::Different(err)),
    };
    let report = vector::compare_items(&expected, &actual);
    let outcome = if !report.is_structurally_equal() {
        WideOutcome::Different(format!("{report}\n{}", report.details(3)))
    } else if report.numeric.is_empty() {
        return Some(WideOutcome::Exact);
    } else {
        WideOutcome::Numeric(report.max_deviation())
    };
    let out = plt_spec::spec_root().join("out/vector-wide");
    let _ = std::fs::create_dir_all(&out);
    write_items(
        &out.join(format!("{name}.{format}.expected.txt")),
        &expected,
    );
    write_items(&out.join(format!("{name}.{format}.actual.txt")), &actual);
    let _ = std::fs::write(out.join(format!("{name}.{format}")), rendered);
    Some(outcome)
}

/// Compares every translated case with the matplotlib documents in the
/// directory `$PLT_VECTOR_REFERENCE` (`<group>__<name>.<svg|pdf|ps>`, as
/// `spec/reference/figures/render_vectors.py` writes them) and reports per
/// format how many agree structurally and how many of those exactly.
#[test]
#[ignore = "needs matplotlib's documents of every case in $PLT_VECTOR_REFERENCE"]
fn every_case_matches_matplotlib_structurally() {
    let reference = std::env::var_os("PLT_VECTOR_REFERENCE")
        .map(PathBuf::from)
        .expect("PLT_VECTOR_REFERENCE names the directory of matplotlib's documents");
    let cases = figures::translated_cases();
    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                while let Some(case) = cases.get(next.fetch_add(1, Ordering::Relaxed)) {
                    for format in VectorFormat::ALL {
                        if let Some(outcome) = wide_outcome(case, format, &reference) {
                            results
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .push((case.name, format, outcome));
                        }
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap_or_else(PoisonError::into_inner);
    results.sort_by_key(|(name, format, _)| (*name, format.extension()));
    for (name, format, outcome) in &results {
        match outcome {
            WideOutcome::Exact => {}
            WideOutcome::Numeric(deviation) => {
                println!("{name} {format}: numbers differ by up to {deviation:.4}");
            }
            WideOutcome::Different(problem) => println!("{name} {format}: {problem}"),
        }
    }
    for format in VectorFormat::ALL {
        let outcomes: Vec<&WideOutcome> = results
            .iter()
            .filter(|(_, f, _)| *f == format)
            .map(|(_, _, outcome)| outcome)
            .collect();
        let exact = outcomes
            .iter()
            .filter(|o| matches!(o, WideOutcome::Exact))
            .count();
        let numeric = outcomes
            .iter()
            .filter(|o| matches!(o, WideOutcome::Numeric(_)))
            .count();
        println!(
            "{format}: {}/{} structurally equal, {exact} with every number equal",
            exact + numeric,
            outcomes.len()
        );
    }
}
