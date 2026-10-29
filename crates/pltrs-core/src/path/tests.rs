use super::*;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

#[test]
fn validation() {
    assert_eq!(
        Path::new(vec![p(0.0, 0.0)], vec![]),
        Err(PathError::LengthMismatch {
            vertices: 1,
            codes: 0
        })
    );
    assert_eq!(
        Path::new(vec![p(0.0, 0.0)], vec![PathCode::LineTo]),
        Err(PathError::MissingInitialMove)
    );
    assert_eq!(PathCode::try_from(0), Err(PathError::UnknownCode(0)));
}

#[test]
fn polyline_segments() {
    let path = Path::polyline(vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)]);
    let segs: Vec<_> = path.segments().collect();
    assert_eq!(
        segs,
        vec![
            Segment::MoveTo(p(0.0, 0.0)),
            Segment::LineTo(p(1.0, 0.0)),
            Segment::LineTo(p(1.0, 1.0))
        ]
    );
}

#[test]
fn builder_codes_repeat_for_curve_vertices() {
    let path = Path::builder()
        .move_to(p(0.0, 0.0))
        .cubic_to(p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0))
        .close()
        .build();
    assert_eq!(
        path.codes().expect("codes"),
        &[
            PathCode::MoveTo,
            PathCode::Curve4,
            PathCode::Curve4,
            PathCode::Curve4,
            PathCode::ClosePoly
        ]
    );
    assert_eq!(path.segments().count(), 3);
}

#[test]
fn truncated_curve_ends_iteration() {
    let path = Path::new(
        vec![p(0.0, 0.0), p(1.0, 1.0)],
        vec![PathCode::MoveTo, PathCode::Curve4],
    )
    .expect("valid");
    assert_eq!(path.segments().count(), 1);
}

#[test]
fn extents_include_curve_extrema() {
    let path = Path::builder()
        .move_to(p(0.0, 0.0))
        .cubic_to(p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0))
        .build();
    assert_eq!(path.extents(), Bbox::from_extents(0.0, 0.0, 1.0, 0.75));
}

#[test]
fn extents_ignore_closepoly_vertex_and_nans() {
    let path = Path::new(
        vec![p(0.0, 0.0), p(2.0, 1.0), p(f64::NAN, 0.0), p(9.0, 9.0)],
        vec![
            PathCode::MoveTo,
            PathCode::LineTo,
            PathCode::LineTo,
            PathCode::ClosePoly,
        ],
    )
    .expect("valid");
    assert_eq!(path.extents(), Bbox::from_extents(0.0, 0.0, 2.0, 1.0));
    assert!(Path::default().extents().is_null());
}

#[test]
fn compound_and_subpaths_round_trip() {
    let a = Path::polyline(vec![p(0.0, 0.0), p(1.0, 0.0)]);
    let b = Path::polygon([p(5.0, 5.0), p(6.0, 5.0), p(6.0, 6.0)]);
    let c = Path::compound([&a, &b]);
    let parts = c.subpaths();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1].vertices(), b.vertices());
    assert_eq!(parts[0].resolved_codes(), a.resolved_codes());
}

#[test]
fn interpolated_matches_matplotlib() {
    let path = Path::polyline(vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0)]);
    let fine = path.interpolated(2);
    assert_eq!(
        fine.vertices(),
        &[
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(2.0, 2.0)
        ]
    );
    assert!(fine.codes().is_none());
    let closed = Path::closed(vec![p(0.0, 0.0), p(2.0, 0.0), p(7.0, 7.0)]).interpolated(2);
    assert_eq!(closed.vertices()[4], p(0.0, 0.0));
    assert_eq!(closed.code_at(4), PathCode::ClosePoly);
    assert_eq!(closed.code_at(3), PathCode::LineTo);
}

#[test]
fn interpolating_long_paths_keeps_them_within_the_limit() {
    // A million vertices cut 75-fold, as on a map, would be 75 million.
    let long = Path::polyline((0..1_000_000).map(|i| p(f64::from(i), 0.0)).collect());
    let mapped = long.interpolated(75);
    assert!(mapped.len() <= MAX_INTERPOLATED_VERTICES);
    assert_eq!(mapped.len(), 999_999 * 4 + 1, "four pieces a segment");
    let compound = Path::compound(&[long.clone(), long]);
    assert!(compound.interpolated(75).len() <= MAX_INTERPOLATED_VERTICES);
    let short = Path::polyline(vec![p(0.0, 0.0), p(1.0, 0.0)]);
    assert_eq!(
        short.interpolated(usize::MAX).len(),
        MAX_INTERPOLATED_VERTICES / 2 + 1
    );
    assert_eq!(short.interpolated(0), short);
}

#[test]
fn compound_edge_cases_match_matplotlib() {
    assert!(Path::compound([]).codes().is_none());
    assert_eq!(Path::compound([&Path::default()]).codes(), Some(&[][..]));
    assert_eq!(
        Path::closed(vec![p(1.0, 2.0)]).codes(),
        Some(&[PathCode::ClosePoly][..])
    );
    let empty = Path::closed(Vec::new());
    assert!(empty.codes().is_none());
    assert_eq!(empty.extents(), Bbox::NULL);
    let polys = Path::compound_from_polygons(&[vec![p(0.0, 0.0)], vec![p(1.0, 1.0)]]);
    assert_eq!(
        polys.vertices(),
        &[p(0.0, 0.0), Point::ORIGIN, p(1.0, 1.0), Point::ORIGIN]
    );
    assert_eq!(
        polys.codes(),
        Some(
            &[
                PathCode::MoveTo,
                PathCode::ClosePoly,
                PathCode::MoveTo,
                PathCode::ClosePoly
            ][..]
        )
    );
}

#[test]
fn beziers_close_back_to_start() {
    let tri = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
    let pieces: Vec<_> = tri.beziers().collect();
    assert_eq!(pieces.len(), 3);
    assert_eq!(pieces[2], Bezier::Line(p(0.0, 1.0), p(0.0, 0.0)));
}
