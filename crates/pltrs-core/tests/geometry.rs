//! Containment, picking, intersections, clipping, polygons, extents and
//! hatches compared with matplotlib's `_path` and `hatch` modules.

mod common;

use common::{Fixture, Json, affine, assert_same_path, bbox, load, path, point, points, same};
use pltrs_core::path::convert::SimplifySettings;
use pltrs_core::path::{CollectionGeometry, PolygonOptions, count_overlapping};
use pltrs_core::{Affine2D, Bbox, Path, Point, hatch};

fn polygons_json(value: &Json) -> Vec<Vec<Point>> {
    value.array().iter().map(points).collect()
}

fn assert_same_polygons(actual: &[Vec<Point>], expected: &[Vec<Point>], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: polygon count");
    for (a, e) in actual.iter().zip(expected) {
        assert_eq!(a.len(), e.len(), "{context}: polygon length");
        for (p, q) in a.iter().zip(e) {
            assert!(
                same(p.x, q.x) && same(p.y, q.y),
                "{context}: {p:?} != {q:?}"
            );
        }
    }
}

#[test]
fn containment_matches_matplotlib() {
    let fixture = load("geometry.json");
    for (i, case) in fixture.field("containment").array().iter().enumerate() {
        let shape = path(case.field("path"));
        let transform = case.field("transform");
        let t = (!transform.is_null()).then(|| affine(transform));
        let result = shape.contains_points(
            &points(case.field("points")),
            t.as_ref(),
            case.field("radius").f64(),
        );
        let expected: Vec<bool> = case
            .field("expected")
            .array()
            .iter()
            .map(Json::bool)
            .collect();
        assert_eq!(result, expected, "containment case {i}");
    }
}

#[test]
fn collections_match_matplotlib() {
    let fixture = load("geometry.json");
    for (i, case) in fixture.field("picking").array().iter().enumerate() {
        let paths: Vec<Path> = case.field("paths").array().iter().map(path).collect();
        let transforms: Vec<Affine2D> = case
            .field("transforms")
            .array()
            .iter()
            .map(affine)
            .collect();
        let offsets = points(case.field("offsets"));
        let geometry = CollectionGeometry {
            master_transform: affine(case.field("master")),
            paths: &paths,
            transforms: &transforms,
            offsets: &offsets,
            offset_transform: affine(case.field("offset_transform")),
        };
        let radius = case.field("radius").f64();
        let filled = case.field("filled").bool();
        for query in case.field("queries").array() {
            let hits: Vec<usize> = query
                .field("hits")
                .array()
                .iter()
                .map(Json::usize)
                .collect();
            assert_eq!(
                geometry.pick(point(query.field("point")), radius, filled),
                hits,
                "picking case {i}"
            );
        }
        if offsets.is_empty() {
            continue;
        }
        let limits = geometry.extents().expect("paths and offsets");
        let want = case.field("extents").f64_vec();
        let got = limits.bbox.extents();
        let minpos = case.field("minpos").f64_vec();
        for (g, w) in [got.0, got.1, got.2, got.3, limits.minpos.x, limits.minpos.y]
            .into_iter()
            .zip(want.into_iter().chain(minpos))
        {
            assert!(same(g, w), "extents case {i}: {g} != {w}");
        }
    }
}

#[test]
fn intersections_match_matplotlib() {
    let fixture = load("geometry.json");
    for (i, case) in fixture.field("intersections").array().iter().enumerate() {
        let a = path(case.field("a"));
        let b = path(case.field("b"));
        let rect = bbox(case.field("rect"));
        let flags =
            |key: &str| -> Vec<bool> { case.field(key).array().iter().map(Json::bool).collect() };
        assert_eq!(
            vec![a.intersects_path(&b, false), a.intersects_path(&b, true)],
            flags("intersects"),
            "intersects_path case {i}"
        );
        assert_eq!(
            vec![
                a.intersects_bbox(&rect, false),
                a.intersects_bbox(&rect, true)
            ],
            flags("intersects_rect"),
            "intersects_bbox case {i}"
        );
        assert_eq!(
            a.contains_path(&b, None),
            case.field("contains").bool(),
            "contains_path case {i}"
        );
    }
}

#[test]
fn clipping_and_polygons_match_matplotlib() {
    let fixture = load("geometry.json");
    for (i, case) in fixture.field("clipping").array().iter().enumerate() {
        let shape = path(case.field("path"));
        let clipped = shape.clip_to_bbox(&bbox(case.field("rect")), case.field("inside").bool());
        let expected: Vec<Point> = polygons_json(case.field("clipped")).concat();
        assert_same_polygons(
            &[clipped.vertices().to_vec()],
            &[expected],
            &format!("clip case {i}"),
        );
        let simplify = SimplifySettings {
            enabled: case.field("simplify").bool(),
            ..SimplifySettings::default()
        };
        let options = PolygonOptions {
            transform: Some(affine(case.field("transform"))),
            width: case.field("width").f64(),
            height: case.field("height").f64(),
            closed_only: case.field("closed_only").bool(),
            simplify,
        };
        let polygons = shape.to_polygons(&options);
        let expected = polygons_json(case.field("polygons"));
        assert_same_polygons(&polygons, &expected, &format!("polygons case {i}"));
    }
}

#[test]
fn bbox_overlaps_match_matplotlib() {
    let fixture = load("geometry.json");
    for case in fixture.field("overlaps").array() {
        let boxes: Vec<Bbox> = case.field("boxes").array().iter().map(bbox).collect();
        assert_eq!(
            count_overlapping(&bbox(case.field("bbox")), &boxes),
            case.field("count").usize()
        );
    }
}

#[test]
fn hatches_match_matplotlib() {
    let fixture = load("geometry.json");
    for case in fixture.field("hatches").array() {
        let pattern = case.field("pattern").str();
        let density = case.field("density").f64() as i32;
        let built = hatch::pattern_path(pattern, density).expect("valid hatch");
        assert_same_path(
            &built,
            case.field("expected"),
            &format!("hatch {pattern:?}x{density}"),
        );
    }
}
