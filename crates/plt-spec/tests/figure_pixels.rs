//! Pixel comparison of every translated figure case with its golden PNG.
//!
//! Prints matplotlib's `compare_images` RMS per case (run with
//! `--nocapture`). Lines, markers, patches, fills and text without
//! anti-aliasing are pixel-exact; the residual differences are glyph
//! anti-aliasing (outlines are filled by plt-raster instead of being
//! rendered by `FreeType`), so the bound is small but not zero.

use plt_spec::figures::{self, PixelReport};

/// Largest RMS accepted for a case.
const MAX_RMS: f64 = 1.0;

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
        println!("{name:<44} {report}");
        if report.rms > MAX_RMS {
            failures.push(format!("{name}: rms {:.3} > {MAX_RMS}", report.rms));
        }
    }
    assert!(
        failures.is_empty(),
        "pixel comparison failures:\n{}",
        failures.join("\n")
    );
}
