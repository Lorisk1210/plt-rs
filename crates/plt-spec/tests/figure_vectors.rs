//! Structural comparison of plt-rs's SVG, PDF and PostScript documents
//! with matplotlib's for the cases in `VECTOR_CASES`.
//!
//! Both documents are reduced to drawing items (see
//! `plt_spec::figures::vector`): ids, object numbers, metadata, comments
//! (plt-rs recovers the text of SVG comments from the glyphs),
//! compression, image encodings and redundant graphics-state operators do
//! not count, every drawn element and its effective style does. Every
//! case must agree structurally, with all numbers within the formatting
//! precision, apart from these accepted differences:
//!
//! * **SVG text placement.** matplotlib's SVG backend measures text with
//!   `TextToPath` at `FONT_SCALE` (100 pt) and typesets mathtext at that
//!   size, rounding its boxes there. plt-rs lays text out once, at the
//!   drawing size, for every backend (the same metrics matplotlib's PDF
//!   and PostScript backends use, which agree exactly). Text groups
//!   (`text:` items) may therefore be placed slightly differently —
//!   hundredths of a point for plain text, up to about half a point for
//!   mathtext, whose glyph offsets inside the group differ by up to
//!   [`SVG_TEXT_LIMIT`] font units. Geometry sized from text extents,
//!   such as the box behind a text, may move by up to
//!   [`SVG_METRICS_LIMIT`] points.
//! * **Document structure outside the drawing.** plt-rs opens groups only
//!   for lines, patches and collections, so the SVG group hierarchy and
//!   ids differ (groups carrying no style are transparent to the
//!   comparison). Metadata is left out of the plt-rs documents.
//! * The cases in [`KNOWN_GAPS`].
//!
//! Prints one line per case and format (run with `--nocapture`) and
//! writes the plt-rs documents and both normalized item lists to
//! `spec/out/vector/` for inspection.

use std::fmt::Write as _;

use plt_spec::figures::vector::{self, VectorFormat, VectorReport};
use plt_spec::figures::{self, FigureCase};

/// Cases exempt from agreement (still reported), with the reason.
const KNOWN_GAPS: &[(&str, &str)] = &[(
    "contour/tripcolor_gouraud",
    "Delaunay triangles come in a different order than qhull's (same triangles)",
)];

/// The largest difference accepted in SVG text items.
const SVG_TEXT_LIMIT: f64 = 10.0;

/// The largest difference accepted in other SVG items (points).
const SVG_METRICS_LIMIT: f64 = 0.05;

/// Why a report fails the acceptance policy, if it does.
fn problems(report: &VectorReport, format: VectorFormat) -> Option<String> {
    if !report.is_structurally_equal() {
        return Some(format!("structural differences:\n{}", report.details(4)));
    }
    let unexplained: Vec<&(String, f64)> = report
        .numeric
        .iter()
        .filter(|(item, deviation)| {
            let limit = match format {
                VectorFormat::Svg if item.starts_with("text:") => SVG_TEXT_LIMIT,
                VectorFormat::Svg => SVG_METRICS_LIMIT,
                VectorFormat::Pdf | VectorFormat::Ps => 0.0,
            };
            *deviation > limit
        })
        .collect();
    (!unexplained.is_empty()).then(|| {
        let mut text = String::from("numbers differ:\n");
        for (item, deviation) in unexplained.iter().take(4) {
            let short: String = item.chars().take(200).collect();
            let _ = writeln!(text, "    ({deviation:.4}) {short}");
        }
        text
    })
}

/// Writes the plt-rs document and both normalized item lists of a case.
fn dump(case: &FigureCase, format: VectorFormat) {
    let out = plt_spec::spec_root().join("out/vector");
    let name = case.name.replace('/', "__");
    let golden = std::fs::read(case.vector_path(format)).ok();
    let rendered = vector::render_vector(case, format).ok();
    if let Some(document) = &rendered {
        let _ = std::fs::create_dir_all(&out);
        let _ = std::fs::write(out.join(format!("{name}.{format}")), document);
    }
    for (suffix, document) in [("expected", golden), ("actual", rendered)] {
        let Some(items) = document.and_then(|d| format.items(&d).ok()) else {
            continue;
        };
        let text = items.iter().fold(String::new(), |mut text, item| {
            let _ = writeln!(text, "{item}");
            text
        });
        let _ = std::fs::write(out.join(format!("{name}.{format}.{suffix}.txt")), text);
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
            dump(case, format);
            match vector::vector_report(case, format) {
                Ok(report) => {
                    let note =
                        gap.map_or(String::new(), |(_, why)| format!("  [known gap: {why}]"));
                    println!("{name:<40} {format:<4} {report}{note}");
                    if let (Some(problem), None) = (problems(&report, format), gap) {
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
