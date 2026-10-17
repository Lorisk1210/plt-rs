//! Pixel comparison of every translated figure case with its golden PNG.
//!
//! Prints matplotlib's `compare_images` RMS per case (run with
//! `--nocapture`). Lines, markers, patches and fills are pixel-exact; the
//! residual differences are glyph anti-aliasing (outlines are filled by
//! plt-raster instead of being rendered by `FreeType`), so the bound is
//! small but not zero.

use plt_spec::figures::{self, PixelReport};

/// Largest RMS accepted for a case.
const MAX_RMS: f64 = 1.0;

/// Cases exempt from [`MAX_RMS`] (still reported), with the reason.
const KNOWN_GAPS: &[(&str, &str)] = &[
    (
        "contour/tripcolor_facecolors",
        "Delaunay triangles come in a different order than qhull's (same triangles)",
    ),
    (
        "contour/tripcolor_flat",
        "Delaunay triangles come in a different order than qhull's, so edges overlap differently",
    ),
    (
        "contour/triplot_masked",
        "Delaunay triangles come in a different order than qhull's, so other triangles are masked",
    ),
];

#[test]
fn figure_cases_match_golden_pixels() {
    let mut failures = Vec::new();
    let mut reports: Vec<(String, PixelReport)> = Vec::new();
    for case in figures::translated_cases() {
        match figures::pixel_report(&case) {
            Ok(report) => reports.push((case.name.to_owned(), report)),
            Err(err) => failures.push(format!("{}: {err}", case.name)),
        }
    }
    for (name, report) in &reports {
        let gap = KNOWN_GAPS.iter().find(|(case, _)| case == name);
        let note = gap.map_or(String::new(), |(_, why)| format!("  [known gap: {why}]"));
        println!("{name:<44} {report}{note}");
        if report.rms > MAX_RMS && gap.is_none() {
            failures.push(format!("{name}: rms {:.3} > {MAX_RMS}", report.rms));
        }
    }
    assert!(
        failures.is_empty(),
        "pixel comparison failures:\n{}",
        failures.join("\n")
    );
}
