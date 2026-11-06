//! Structural comparison of pltrs's SVG, PDF and PostScript documents
//! with matplotlib's: the cases in `VECTOR_CASES`, the usetex cases in
//! `TEX_VECTOR_CASES` (skipped without TeX, their PostScript without
//! `psfrag`, `dvips` and Ghostscript), the variants in `VECTOR_VARIANTS`
//! (Type 42 fonts, SVG text as text, the distillers, usetex EPS and paper
//! sizes; skipped without the programs they need) and the multi-page PDF
//! of `PDF_PAGES`.
//!
//! Both documents are reduced to drawing items (see
//! `pltrs_spec::figures::vector`): ids, object numbers, metadata, comments,
//! compression, image and font program encodings and redundant
//! graphics-state operators do not count, every drawn element, its
//! effective style and every glyph used do. Every document must agree
//! structurally with all numbers within the formatting precision, apart
//! from the pltrs SVG group hierarchy (pltrs opens groups only for
//! lines, patches and collections; groups carrying no style are
//! transparent to the comparison) and the cases in [`KNOWN_GAPS`].
//!
//! Prints one line per case and format (run with `--nocapture`) and
//! writes the pltrs documents and both normalized item lists to
//! `spec/out/vector/` for inspection.
//!
//! The ignored `every_case_matches_matplotlib_structurally` compares every
//! translated case with documents matplotlib wrote into
//! `$PLTRS_VECTOR_REFERENCE` (see `spec/reference/figures/render_vectors.py`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pltrs_spec::figures::vector::{self, VectorFormat, VectorReport};
use pltrs_spec::figures::{self, FigureCase};

/// Cases exempt from agreement (still reported), with the reason.
const KNOWN_GAPS: &[(&str, &str)] = &[
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

/// Writes the pltrs document `rendered` and the normalized item lists of
/// it and of the golden at `golden` to `spec/out/vector/<stem>.*`.
fn dump(stem: &str, format: VectorFormat, golden: &Path, rendered: Option<Vec<u8>>) {
    let out = pltrs_spec::spec_root().join("out/vector");
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
        vector::write_items(&out.join(format!("{stem}.{format}.{suffix}.txt")), &items);
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
            if let Some(reason) = case.vector_skip_reason(format, variant.rc) {
                println!(
                    "{:<30} {:<8} {format:<4} skipped: {reason}",
                    case.name, variant.name
                );
                continue;
            }
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
fn tex_documents_match_matplotlib_structurally() {
    let mut failures = Vec::new();
    for name in vector::TEX_VECTOR_CASES {
        let case = translated(name);
        for &format in vector::TEX_FORMATS {
            if let Some(reason) = case.vector_skip_reason(format, &[]) {
                println!("{name:<40} {format:<4} skipped: {reason}");
                continue;
            }
            dump(
                &name.replace('/', "__"),
                format,
                &case.vector_path(format),
                vector::render_vector(&case, format).ok(),
            );
            match vector::vector_report(&case, format) {
                Ok(report) => {
                    println!("{name:<40} {format:<4} {report}");
                    if let Some(problem) = problems(&report) {
                        failures.push(format!("{name} {format}: {problem}"));
                    }
                }
                Err(err) => failures.push(format!("{name} {format}: {err}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "TeX vector comparison failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn pdf_pages_match_matplotlib_structurally() {
    let cases: Vec<FigureCase> = vector::PDF_PAGES
        .iter()
        .map(|name| translated(name))
        .collect();
    let golden = pltrs_spec::spec_root().join("goldens/figures/vectors/pages.pdf");
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

/// Compares every translated case with the matplotlib documents in the
/// directory `$PLTRS_VECTOR_REFERENCE` (`<group>__<name>.<svg|pdf|ps|pgf>`, as
/// `spec/reference/figures/render_vectors.py` writes them) and reports per
/// format how many agree structurally and how many of those exactly.
#[test]
#[ignore = "needs matplotlib's documents of every case in $PLTRS_VECTOR_REFERENCE"]
fn every_case_matches_matplotlib_structurally() {
    let reference = std::env::var_os("PLTRS_VECTOR_REFERENCE")
        .map(PathBuf::from)
        .expect("PLTRS_VECTOR_REFERENCE names the directory of matplotlib's documents");
    let results = vector::wide_comparison(&figures::translated_cases(), &reference);
    print!("{}", vector::wide_summary(&results));
}
