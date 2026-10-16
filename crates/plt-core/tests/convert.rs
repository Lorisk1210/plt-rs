//! The conditioning pipeline compared vertex for vertex with matplotlib's
//! `_path.cleanup_path` and `_path.convert_to_string`.

mod common;

use common::{Json, affine, assert_close_path, assert_same_path, bbox, load, path};
use plt_core::path::convert::{CleanupOptions, SketchParams, SnapMode, StringCodes, StringOptions};

fn cleanup_options(case: &Json) -> CleanupOptions {
    let snap = case.get("snap");
    let sketch = case.get("sketch");
    let simplify = case.get("simplify");
    CleanupOptions {
        transform: affine(case.get("transform")),
        remove_nans: case.get("remove_nans").bool(),
        clip: (!case.get("clip").is_null()).then(|| bbox(case.get("clip"))),
        snap: SnapMode::from_option((!snap.is_null()).then(|| snap.bool())),
        stroke_width: case.get("stroke_width").f64(),
        simplify: (!simplify.is_null()).then(|| simplify.f64()),
        curves: case.get("curves").bool(),
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
    let cases = fixture.get("cleanup").array();
    for (i, case) in cases.iter().enumerate() {
        let input = path(case.get("path"));
        let options = cleanup_options(case);
        let cleaned = input.cleaned(&options);
        let context = format!("cleanup case {i}");
        if options.sketch.is_some_and(|s| s.scale != 0.0) {
            // The wiggle goes through libm's exp/sin: musl and glibc may
            // differ in the last bit.
            assert_close_path(&cleaned, case.get("expected"), 1e-9, &context);
        } else {
            assert_same_path(&cleaned, case.get("expected"), &context);
        }
    }
}

#[test]
fn long_noisy_lines_simplify_like_matplotlib() {
    let fixture = load("convert.json");
    let long: Vec<&Json> = fixture
        .get("cleanup")
        .array()
        .iter()
        .filter(|case| case.get("path").has("generator"))
        .collect();
    assert!(long.len() >= 6);
    for case in long {
        let input = path(case.get("path"));
        assert!(input.len() >= 1000);
        let options = cleanup_options(case);
        let cleaned = input.cleaned(&options);
        if options.simplify.is_some_and(|t| t > 0.0) {
            assert!(cleaned.len() < input.len());
        }
        assert_same_path(&cleaned, case.get("expected"), "long line");
    }
}

#[test]
fn convert_to_string_matches_matplotlib() {
    let fixture = load("convert.json");
    for (i, case) in fixture.get("strings").array().iter().enumerate() {
        let input = path(case.get("path"));
        let (codes, postfix) = match case.get("flavour").str() {
            "svg" => (StringCodes::SVG, false),
            "pdf" => (StringCodes::PDF, true),
            _ => (StringCodes::PS, true),
        };
        let clip = case.get("clip");
        let options = StringOptions {
            transform: affine(case.get("transform")),
            clip: (!clip.is_null()).then(|| bbox(clip)),
            precision: case.get("precision").usize(),
            codes,
            postfix,
            ..StringOptions::default()
        };
        assert_eq!(
            input.convert_to_string(&options),
            case.get("expected").str(),
            "string case {i}"
        );
    }
}
