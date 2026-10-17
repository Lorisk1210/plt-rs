//! Filled arrows built around a single quadratic Bézier curve (matplotlib's
//! `ArrowStyle.Simple`, `Fancy` and `Wedge`).

use plt_core::bezier::{
    get_parallels, inside_circle, make_wedged_bezier2, split_bezier_intersecting_with_closedpath,
};
use plt_core::{Path, PathCode, Point};

use crate::patches::PatchError;

/// The bisection tolerance of matplotlib's curve splitting.
const TOLERANCE: f64 = 0.01;

/// The three control points of `path` if it is exactly one quadratic
/// Bézier curve (`ArrowStyle._Base.ensure_quadratic_bezier`).
pub(super) fn quadratic(path: &Path) -> Result<[Point; 3], PatchError> {
    let vertices = path.vertices();
    match (vertices, path.codes()) {
        ([p0, p1, p2], Some([PathCode::MoveTo, PathCode::Curve3, PathCode::Curve3])) => {
            Ok([*p0, *p1, *p2])
        }
        _ => Err(PatchError::NotQuadratic),
    }
}

/// The point at distance `d` from `p0` towards `p1`
/// (`_point_along_a_line`).
fn point_along(p0: Point, p1: Point, d: f64) -> Point {
    let (dx, dy) = (p0.x - p1.x, p0.y - p1.y);
    let ff = d / (dx * dx + dy * dy).sqrt();
    Point::new(p0.x - ff * dx, p0.y - ff * dy)
}

/// A straight quadratic curve of length `head_length` ending at `tip`,
/// coming from the direction of `control`.
fn straight_head(control: Point, tip: Point, head_length: f64) -> [Point; 3] {
    let start = point_along(tip, control, head_length);
    let mid = Point::new(f64::midpoint(start.x, tip.x), f64::midpoint(start.y, tip.y));
    [start, mid, tip]
}

fn triple(points: &[Point]) -> [Point; 3] {
    [points[0], points[1], points[2]]
}

/// Splits the curve where it enters the circle of `radius` around `center`.
fn split_at_circle(
    curve: [Point; 3],
    center: Point,
    radius: f64,
) -> Result<([Point; 3], [Point; 3]), PatchError> {
    split_bezier_intersecting_with_closedpath(&curve, inside_circle(center, radius), TOLERANCE)
        .map(|(outside, inside)| (triple(&outside), triple(&inside)))
        .map_err(|_| PatchError::NotIntersecting)
}

/// A path from `(code, point)` pairs.
fn coded(segments: &[(PathCode, Point)]) -> Path {
    let (codes, vertices) = segments.iter().copied().unzip();
    Path::new(vertices, codes).expect("arrow outlines start with a move")
}

/// matplotlib's `Simple` arrow: a tail of constant width and a head.
pub(super) fn simple(
    curve: [Point; 3],
    mutation_size: f64,
    (head_length, head_width, tail_width): (f64, f64, f64),
) -> Path {
    use PathCode::{ClosePoly, Curve3, LineTo, MoveTo};
    let [_, control, tip] = curve;
    let head_length = head_length * mutation_size;
    let (tail, head) = match split_at_circle(curve, tip, head_length) {
        Ok((outside, inside)) => (Some(outside), inside),
        Err(_) => (None, straight_head(control, tip, head_length)),
    };
    let (head_left, head_right) =
        make_wedged_bezier2(head, head_width * mutation_size / 2.0, 1.0, 0.5, 0.0);
    match tail {
        Some(tail) => {
            let (tail_left, tail_right) = get_parallels(tail, tail_width * mutation_size / 2.0);
            coded(&[
                (MoveTo, tail_right[0]),
                (Curve3, tail_right[1]),
                (Curve3, tail_right[2]),
                (LineTo, head_right[0]),
                (Curve3, head_right[1]),
                (Curve3, head_right[2]),
                (Curve3, head_left[1]),
                (Curve3, head_left[0]),
                (LineTo, tail_left[2]),
                (Curve3, tail_left[1]),
                (Curve3, tail_left[0]),
                (LineTo, tail_right[0]),
                (ClosePoly, tail_right[0]),
            ])
        }
        None => coded(&[
            (MoveTo, head_right[0]),
            (Curve3, head_right[1]),
            (Curve3, head_right[2]),
            (Curve3, head_left[1]),
            (Curve3, head_left[0]),
            (ClosePoly, head_left[0]),
        ]),
    }
}

/// matplotlib's `Fancy` arrow: a tapering tail and a head.
pub(super) fn fancy(
    curve: [Point; 3],
    mutation_size: f64,
    (head_length, head_width, tail_width): (f64, f64, f64),
) -> Result<Path, PatchError> {
    use PathCode::{ClosePoly, Curve3, LineTo, MoveTo};
    let [_, control, tip] = curve;
    let head_length = head_length * mutation_size;
    let (arrow, head) = if let Ok((_, inside)) = split_at_circle(curve, tip, head_length) {
        (curve, inside)
    } else {
        let straight = straight_head(control, tip, head_length);
        (straight, straight)
    };
    let (tail, _) = split_at_circle(arrow, tip, head_length * 0.8)?;
    let (head_left, head_right) =
        make_wedged_bezier2(head, head_width * mutation_size / 2.0, 1.0, 0.6, 0.0);
    let tail_width = tail_width * mutation_size;
    let (tail_left, tail_right) = make_wedged_bezier2(tail, tail_width * 0.5, 1.0, 0.6, 0.3);
    // matplotlib measures from the start of the (possibly straightened)
    // arrow curve.
    let (inside_start, _) = split_bezier_intersecting_with_closedpath(
        &arrow,
        inside_circle(arrow[0], tail_width * 0.3),
        TOLERANCE,
    )
    .map_err(|_| PatchError::NotIntersecting)?;
    let tail_start = inside_start[inside_start.len() - 1];
    Ok(coded(&[
        (MoveTo, tail_start),
        (LineTo, tail_right[0]),
        (Curve3, tail_right[1]),
        (Curve3, tail_right[2]),
        (LineTo, head_right[0]),
        (Curve3, head_right[1]),
        (Curve3, head_right[2]),
        (Curve3, head_left[1]),
        (Curve3, head_left[0]),
        (LineTo, tail_left[2]),
        (Curve3, tail_left[1]),
        (Curve3, tail_left[0]),
        (LineTo, tail_start),
        (ClosePoly, tail_start),
    ]))
}

/// matplotlib's `Wedge` arrow: `tail_width` wide at the start, narrowing
/// to a point.
pub(super) fn wedge(
    curve: [Point; 3],
    mutation_size: f64,
    (tail_width, shrink_factor): (f64, f64),
) -> Path {
    use PathCode::{ClosePoly, Curve3, LineTo, MoveTo};
    let (plus, minus) = make_wedged_bezier2(
        curve,
        tail_width * mutation_size / 2.0,
        1.0,
        shrink_factor,
        0.0,
    );
    coded(&[
        (MoveTo, plus[0]),
        (Curve3, plus[1]),
        (Curve3, plus[2]),
        (LineTo, minus[2]),
        (Curve3, minus[1]),
        (Curve3, minus[0]),
        (ClosePoly, minus[0]),
    ])
}
