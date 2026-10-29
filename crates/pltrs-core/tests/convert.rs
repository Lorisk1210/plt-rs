//! The conditioning pipeline compared vertex for vertex with matplotlib's
//! `_path.cleanup_path` and `_path.convert_to_string`.

mod common;

use common::{Fixture, Json, affine, assert_close_path, assert_same_path, bbox, load, path};
use pltrs_core::path::convert::{
    CleanupOptions, SketchParams, SnapMode, StringCodes, StringOptions,
};

fn cleanup_options(case: &Json) -> CleanupOptions {
    let snap = case.field("snap");
    let sketch = case.field("sketch");
    let simplify = case.field("simplify");
    CleanupOptions {
        transform: affine(case.field("transform")),
        remove_nans: case.field("remove_nans").bool(),
        clip: (!case.field("clip").is_null()).then(|| bbox(case.field("clip"))),
        snap: SnapMode::from_option((!snap.is_null()).then(|| snap.bool())),
        stroke_width: case.field("stroke_width").f64(),
        simplify: (!simplify.is_null()).then(|| simplify.f64()),
        curves: case.field("curves").bool(),
        sketch: (!sketch.is_null()).then(|| {
            let [scale, length, randomness] = sketch.f64_vec()[..] else {
                panic!("sketch needs three values");
            };
            SketchParams {
                scale,
                length,
                randomness,
            }
        }),
    }
}

#[test]
fn cleanup_matches_matplotlib() {
    let fixture = load("convert.json");
    let cases = fixture.field("cleanup").array();
    for (i, case) in cases.iter().enumerate() {
        let input = path(case.field("path"));
        let options = cleanup_options(case);
        let cleaned = input.cleaned(&options);
        let context = format!("cleanup case {i}");
        if options.sketch.is_some_and(|s| s.scale != 0.0) {
            // The wiggle goes through libm's exp/sin: musl and glibc may
            // differ in the last bit.
            assert_close_path(&cleaned, case.field("expected"), 1e-9, &context);
        } else {
            assert_same_path(&cleaned, case.field("expected"), &context);
        }
    }
}

#[test]
fn long_noisy_lines_simplify_like_matplotlib() {
    let fixture = load("convert.json");
    let long: Vec<&Json> = fixture
        .field("cleanup")
        .array()
        .iter()
        .filter(|case| case.field("path").has("generator"))
        .collect();
    assert!(long.len() >= 6);
    for case in long {
        let input = path(case.field("path"));
        assert!(input.len() >= 1000);
        let options = cleanup_options(case);
        let cleaned = input.cleaned(&options);
        if options.simplify.is_some_and(|t| t > 0.0) {
            assert!(cleaned.len() < input.len());
        }
        assert_same_path(&cleaned, case.field("expected"), "long line");
    }
}

#[test]
fn convert_to_string_matches_matplotlib() {
    let fixture = load("convert.json");
    for (i, case) in fixture.field("strings").array().iter().enumerate() {
        let input = path(case.field("path"));
        let (codes, postfix) = match case.field("flavour").str() {
            "svg" => (StringCodes::SVG, false),
            "pdf" => (StringCodes::PDF, true),
            _ => (StringCodes::PS, true),
        };
        let clip = case.field("clip");
        let options = StringOptions {
            transform: affine(case.field("transform")),
            clip: (!clip.is_null()).then(|| bbox(clip)),
            precision: case.field("precision").usize(),
            codes,
            postfix,
            ..StringOptions::default()
        };
        assert_eq!(
            input.convert_to_string(&options),
            case.field("expected").str(),
            "string case {i}"
        );
    }
}
