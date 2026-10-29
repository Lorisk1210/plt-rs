//! Robustness: hostile and degenerate geometry (NaN, infinities, huge and
//! tiny coordinates, empty and single-point paths, singular transforms,
//! inverted boxes) must never panic, hang or allocate without bound.

mod common;

use pltrs_core::bezier::{
    Bezier, BezierSegment, find_bezier_t_intersecting_with_closedpath, inside_circle,
    split_bezier_intersecting_with_closedpath, split_path_inout,
};
use pltrs_core::path::convert::{
    CleanupOptions, SketchParams, SnapMode, StringCodes, StringOptions,
};
use pltrs_core::path::{CollectionGeometry, DataLimits, PolygonOptions};
use pltrs_core::rng::SplitMix64;
use pltrs_core::{Affine2D, Bbox, Path, PathCode, Point};

/// Coordinates that break naive arithmetic.
const NASTY: [f64; 14] = [
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    1e300,
    -1e300,
    1e-300,
    f64::MAX,
    f64::MIN_POSITIVE,
    1e17,
];

/// A coordinate that is ordinary three times out of four and nasty otherwise.
fn coordinate(rng: &mut SplitMix64) -> f64 {
    if rng.next_u64().is_multiple_of(4) {
        NASTY[(rng.next_u64() % NASTY.len() as u64) as usize]
    } else {
        (rng.next_f64() - 0.5) * 200.0
    }
}

fn point(rng: &mut SplitMix64) -> Point {
    Point::new(coordinate(rng), coordinate(rng))
}

fn bbox(rng: &mut SplitMix64) -> Bbox {
    Bbox::new(point(rng), point(rng))
}

fn affine(rng: &mut SplitMix64) -> Affine2D {
    match rng.next_u64() % 4 {
        0 => Affine2D::IDENTITY,
        1 => Affine2D::scaling(0.0, coordinate(rng)),
        _ => Affine2D::from_values(
            coordinate(rng),
            coordinate(rng),
            coordinate(rng),
            coordinate(rng),
            coordinate(rng),
            coordinate(rng),
        ),
    }
}

/// A path with random codes (curves possibly truncated) and nasty vertices.
fn path(rng: &mut SplitMix64) -> Path {
    let len = (rng.next_u64() % 12) as usize;
    let vertices: Vec<Point> = (0..len).map(|_| point(rng)).collect();
    match rng.next_u64() % 3 {
        0 => Path::polyline(vertices),
        1 => Path::closed(vertices),
        _ => {
            let codes = (0..len)
                .map(|i| match (i, rng.next_u64() % 5) {
                    (0, _) | (_, 0) => PathCode::MoveTo,
                    (_, 1) => PathCode::LineTo,
                    (_, 2) => PathCode::Curve3,
                    (_, 3) => PathCode::Curve4,
                    _ => PathCode::ClosePoly,
                })
                .collect();
            Path::new(vertices, codes).expect("codes start with a move")
        }
    }
}

fn degenerate_paths() -> Vec<Path> {
    let p = Point::new;
    let huge = 1e300;
    vec![
        Path::default(),
        Path::polyline(vec![p(1.0, 1.0)]),
        Path::closed(vec![p(1.0, 1.0)]),
        Path::polyline(vec![p(1.0, 1.0); 5]),
        Path::polygon([p(0.0, 0.0), p(0.0, 0.0), p(0.0, 0.0)]),
        Path::polyline(vec![p(-huge, -huge), p(huge, huge), p(-huge, huge)]),
        Path::polyline(vec![p(0.0, 0.0), p(1e-300, 1e-300), p(2e-300, 0.0)]),
        Path::polyline(vec![p(f64::NAN, f64::NAN); 4]),
        Path::polygon([
            p(f64::INFINITY, 0.0),
            p(0.0, f64::NEG_INFINITY),
            p(f64::MAX, f64::MAX),
        ]),
        Path::builder()
            .move_to(p(0.0, 0.0))
            .cubic_to(p(huge, 0.0), p(0.0, huge), p(1.0, 1.0))
            .quad_to(p(f64::NAN, 1.0), p(2.0, 2.0))
            .close()
            .build(),
        Path::circle(Point::ORIGIN, f64::NAN),
    ]
}

fn cleanup_options(rng: &mut SplitMix64) -> CleanupOptions {
    let maybe = |rng: &mut SplitMix64| rng.next_u64().is_multiple_of(2);
    CleanupOptions {
        transform: affine(rng),
        remove_nans: maybe(rng),
        clip: maybe(rng).then(|| bbox(rng)),
        snap: [SnapMode::Auto, SnapMode::Always, SnapMode::Never][(rng.next_u64() % 3) as usize],
        stroke_width: coordinate(rng),
        simplify: maybe(rng).then(|| coordinate(rng)),
        curves: maybe(rng),
        sketch: maybe(rng).then(|| SketchParams {
            scale: coordinate(rng),
            length: coordinate(rng),
            randomness: coordinate(rng),
        }),
    }
}

/// Runs every path operation of the crate on `path`.
fn exercise(path: &Path, rng: &mut SplitMix64) {
    let _ = path.extents();
    let _ = path.segments().count();
    let _ = path.beziers().count();
    let _ = path.subpaths();
    let _ = path.interpolated(3);
    let _ = path.transformed(&affine(rng));
    for _ in 0..2 {
        let options = cleanup_options(rng);
        let _ = path.cleaned(&options);
        let _ = path.iter_segments(&options).count();
        let _ = path.iter_bezier(&options);
        let _ = path.convert_to_string(&StringOptions {
            transform: options.transform,
            clip: options.clip,
            simplify: options.simplify,
            sketch: options.sketch,
            precision: (rng.next_u64() % 20) as usize,
            codes: StringCodes::PDF,
            postfix: true,
        });
    }
    let _ = path.to_polygons(&PolygonOptions {
        transform: Some(affine(rng)),
        width: coordinate(rng),
        height: coordinate(rng),
        closed_only: rng.next_u64().is_multiple_of(2),
        ..PolygonOptions::default()
    });
    let _ = path.clip_to_bbox(&bbox(rng), rng.next_u64().is_multiple_of(2));
    let points: Vec<Point> = (0..4).map(|_| point(rng)).collect();
    let transform = affine(rng);
    let _ = path.contains_points(&points, Some(&transform), coordinate(rng));
    let _ = path.contains_path(&path.transformed(&transform), Some(&transform));
    let _ = path.stroke_contains_point(points[0], &transform, coordinate(rng));
    let other = self::path(rng);
    let _ = path.intersects_path(&other, true);
    let _ = path.intersects_bbox(&bbox(rng), false);
    let mut limits = DataLimits::NULL;
    limits.update_from_path(path, true, true);
    limits.update_from_transformed_path(path, &transform);
    let _ = split_path_inout(path, inside_circle(point(rng), coordinate(rng)), 0.01, true);
    let paths = [path.clone(), other];
    let offsets = [point(rng), point(rng)];
    let collection = CollectionGeometry {
        master_transform: affine(rng),
        paths: &paths,
        transforms: &[transform],
        offsets: &offsets,
        offset_transform: affine(rng),
    };
    let _ = collection.extents();
    let _ = collection.pick(point(rng), coordinate(rng), true);
    let _ = collection.pick(point(rng), coordinate(rng), false);
}

#[test]
fn hostile_paths_do_not_panic() {
    let mut rng = SplitMix64::new(0xC0FF_EE00);
    for path in degenerate_paths() {
        exercise(&path, &mut rng);
    }
    for _ in 0..120 {
        let path = path(&mut rng);
        exercise(&path, &mut rng);
    }
}

#[test]
fn hostile_beziers_do_not_panic() {
    let mut rng = SplitMix64::new(17);
    for _ in 0..500 {
        let (a, b, c, d) = (
            point(&mut rng),
            point(&mut rng),
            point(&mut rng),
            point(&mut rng),
        );
        for curve in [
            Bezier::Line(a, b),
            Bezier::Quad(a, b, c),
            Bezier::Cubic(a, b, c, d),
        ] {
            let t = coordinate(&mut rng);
            let _ = (curve.eval(t), curve.derivative(t), curve.split(t));
            let _ = curve.axis_aligned_extrema();
            let mut out = Vec::new();
            curve.flatten_into(coordinate(&mut rng), &mut out);
            assert!(out.len() <= 10_001);
            let points = curve.control_points();
            let tolerance = coordinate(&mut rng);
            let inside = inside_circle(point(&mut rng), coordinate(&mut rng));
            let _ = split_bezier_intersecting_with_closedpath(&points, &inside, tolerance);
            let segment = BezierSegment::from_points(&points).expect("non-empty");
            let _ = segment.axis_aligned_extrema();
            let _ = segment.polynomial_coefficients();
            let _ = find_bezier_t_intersecting_with_closedpath(
                |t| segment.point_at(t),
                |p| p.x.is_nan() || inside(p),
                coordinate(&mut rng),
                coordinate(&mut rng),
                tolerance,
            );
        }
    }
}

#[test]
fn hostile_boxes_and_transforms_do_not_panic() {
    let mut rng = SplitMix64::new(99);
    for _ in 0..500 {
        let (a, b) = (bbox(&mut rng), bbox(&mut rng));
        let t = affine(&mut rng);
        let _ = (a.union(&b), a.intersection(&b), a.overlaps(&b));
        let _ = (a.transformed(&t), a.transformed_enclosing(&t));
        let _ = a.rotated(coordinate(&mut rng));
        let _ = a.shrunk_to_aspect(coordinate(&mut rng), Some(&b), coordinate(&mut rng));
        let _ = a.split_x(&[coordinate(&mut rng), coordinate(&mut rng)]);
        let _ = a.split_y(&[]);
        let _ = (t.inverted(), t.determinant(), t.then(affine(&mut rng)));
        let _ = Path::arc(coordinate(&mut rng), coordinate(&mut rng), None);
        let _ = Path::wedge(coordinate(&mut rng), coordinate(&mut rng), Some(3));
    }
    for sides in 0..4 {
        let _ = Path::unit_regular_polygon(sides);
        let _ = Path::unit_regular_star(sides, f64::NAN);
    }
}
