//! Shared helpers for the fixture-based comparisons with matplotlib.
//!
//! The fixtures are produced by `tests/fixtures/generate.py`; long inputs
//! are not stored but regenerated here with the same `SplitMix64` stream.

#![allow(dead_code, reason = "each test binary uses a different subset")]

pub(crate) mod json;

use std::path::PathBuf;

use pltrs_core::rng::SplitMix64;
use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point};

pub(crate) use json::{Fixture, Json};

/// Loads `tests/fixtures/<name>`.
pub(crate) fn load(name: &str) -> Json {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", path.display()));
    json::parse(&text)
}

pub(crate) fn point(value: &Json) -> Point {
    match value.f64_vec().as_slice() {
        [x, y] => Point::new(*x, *y),
        other => panic!("expected [x, y], got {other:?}"),
    }
}

pub(crate) fn points(value: &Json) -> Vec<Point> {
    value.array().iter().map(point).collect()
}

pub(crate) fn code(value: &Json) -> PathCode {
    PathCode::try_from(value.f64() as u8).expect("fixture codes are valid")
}

/// Decodes `{"vertices": [...], "codes": [...] | null}`, or regenerates a
/// `{"generator": ...}` input.
pub(crate) fn path(value: &Json) -> Path {
    if value.has("generator") {
        return generated(value.field("generator"));
    }
    let vertices = points(value.field("vertices"));
    let codes = value.field("codes");
    if codes.is_null() {
        Path::polyline(vertices)
    } else {
        Path::new(vertices, codes.array().iter().map(code).collect()).expect("valid fixture path")
    }
}

/// `[a, b, c, d, e, f]` or `null` (identity).
pub(crate) fn affine(value: &Json) -> Affine2D {
    if value.is_null() {
        return Affine2D::IDENTITY;
    }
    match value.f64_vec().as_slice() {
        [a, b, c, d, e, f] => Affine2D::from_values(*a, *b, *c, *d, *e, *f),
        other => panic!("expected six affine values, got {other:?}"),
    }
}

/// `[x0, y0, x1, y1]`.
pub(crate) fn bbox(value: &Json) -> Bbox {
    match value.f64_vec().as_slice() {
        [x0, y0, x1, y1] => Bbox::from_extents(*x0, *y0, *x1, *y1),
        other => panic!("expected four extents, got {other:?}"),
    }
}

/// Whether `a` and `b` are the same: bit for bit (NaN equals NaN, `-0.0`
/// equals `0.0`) with glibc's `sin` and `cos`, with which matplotlib
/// computed the fixtures (on Linux); within four units in the last place
/// of the larger of them and 1, the fixtures' scale, with other C
/// libraries, which round some values otherwise (Windows' UCRT, Wine's,
/// MinGW-w64's `sincos`).
#[expect(clippy::float_cmp, reason = "fixtures are compared exactly")]
pub(crate) fn same(a: f64, b: f64) -> bool {
    let glibc = cfg!(all(target_os = "linux", target_env = "gnu"));
    a == b
        || (a.is_nan() && b.is_nan())
        || (!glibc && (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0))
}

/// Asserts that `actual` has exactly the vertices and codes of `expected`.
pub(crate) fn assert_same_path(actual: &Path, expected: &Json, context: &str) {
    let want = points(expected.field("vertices"));
    let codes: Vec<PathCode> = expected.field("codes").array().iter().map(code).collect();
    let got_codes = actual.resolved_codes();
    assert_eq!(got_codes, codes, "{context}: codes differ");
    for (i, (got, want)) in actual.vertices().iter().zip(&want).enumerate() {
        assert!(
            same(got.x, want.x) && same(got.y, want.y),
            "{context}: vertex {i} is {got:?}, matplotlib has {want:?}"
        );
    }
    assert_eq!(actual.len(), want.len(), "{context}: vertex count differs");
}

/// Asserts that `actual` has the codes of `expected` and vertices within
/// `tol` (relative to the coordinate magnitude, at least 1).
pub(crate) fn assert_close_path(actual: &Path, expected: &Json, tol: f64, context: &str) {
    let want = points(expected.field("vertices"));
    let codes: Vec<PathCode> = expected.field("codes").array().iter().map(code).collect();
    assert_eq!(actual.resolved_codes(), codes, "{context}: codes differ");
    let close = |a: f64, b: f64| same(a, b) || (a - b).abs() <= tol * a.abs().max(b.abs()).max(1.0);
    for (i, (got, want)) in actual.vertices().iter().zip(&want).enumerate() {
        assert!(
            close(got.x, want.x) && close(got.y, want.y),
            "{context}: vertex {i} is {got:?}, matplotlib has {want:?}"
        );
    }
}

/// A long noisy polyline: a random walk whose noise amplitude changes every
/// 64 points, with optional NaN gaps (see `noisy_line` in the generator).
fn generated(spec: &Json) -> Path {
    assert_eq!(spec.field("kind").str(), "noisy");
    let n = spec.field("n").usize();
    let gaps = spec.field("nan_gaps").bool();
    let mut rng = SplitMix64::new(spec.field("seed").f64() as u64);
    let amplitudes = [0.01, 0.05, 0.3, 2.0];
    let mut amplitude = 0.0;
    let mut trend = 0.0;
    let mut slope = 0.0;
    let mut vertices = Vec::with_capacity(n);
    for i in 0..n {
        if i % 64 == 0 {
            amplitude = amplitudes[(rng.next_u64() % 4) as usize];
            slope = (rng.next_f64() - 0.5) * 0.8;
        }
        trend += slope;
        let y = trend + amplitude * (2.0 * rng.next_f64() - 1.0);
        let gap = gaps && rng.next_u64().is_multiple_of(97);
        vertices.push(Point::new(i as f64 * 0.5, if gap { f64::NAN } else { y }));
    }
    Path::polyline(vertices)
}
