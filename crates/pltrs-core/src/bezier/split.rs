//! Splitting curves and paths where they leave a closed region
//! (matplotlib's `find_bezier_t_intersecting_with_closedpath`,
//! `split_bezier_intersecting_with_closedpath` and `split_path_inout`),
//! used to shrink arrows to the edges of patches.

use super::segment::{BezierError, BezierSegment};
use crate::Point;
use crate::path::convert::{CleanupOptions, SimplifySettings};
use crate::path::{Path, PathCode};

/// matplotlib's `inside_circle(cx, cy, r)`: a predicate for points strictly
/// inside the circle.
pub fn inside_circle(center: Point, r: f64) -> impl Fn(Point) -> bool {
    let r2 = r * r;
    move |p: Point| {
        let dx = p.x - center.x;
        let dy = p.y - center.y;
        dx * dx + dy * dy < r2
    }
}

/// More bisections than any finite bracket needs before its ends become
/// adjacent floats (about 2100 halvings from `f64::MAX` to the smallest
/// subnormal).
const MAX_BISECTIONS: usize = 4096;

/// Bisects `[t0, t1]` until the curve points at both ends are closer than
/// `tolerance`, keeping one end inside the closed path and the other
/// outside (matplotlib's `find_bezier_t_intersecting_with_closedpath`).
///
/// Fails when both ends start on the same side (and differ), or when `t0`
/// or `t1` is not finite. Like matplotlib, the search also stops when a
/// bisection no longer moves an end point; unlike matplotlib, it gives up
/// after 4096 steps (returning the current bracket), which
/// only curves producing NaN points ever reach.
pub fn find_bezier_t_intersecting_with_closedpath(
    point_at: impl Fn(f64) -> Point,
    inside: impl Fn(Point) -> bool,
    mut t0: f64,
    mut t1: f64,
    tolerance: f64,
) -> Result<(f64, f64), BezierError> {
    if !(t0.is_finite() && t1.is_finite()) {
        return Err(BezierError::NotIntersecting);
    }
    let mut start = point_at(t0);
    let mut end = point_at(t1);
    let mut start_inside = inside(start);
    if start_inside == inside(end) && start != end {
        return Err(BezierError::NotIntersecting);
    }
    for _ in 0..MAX_BISECTIONS {
        if (start.x - end.x).hypot(start.y - end.y) < tolerance {
            return Ok((t0, t1));
        }
        let middle_t = f64::midpoint(t0, t1);
        let middle = point_at(middle_t);
        let middle_inside = inside(middle);
        if start_inside ^ middle_inside {
            t1 = middle_t;
            if end == middle {
                return Ok((t0, t1));
            }
            end = middle;
        } else {
            t0 = middle_t;
            if start == middle {
                return Ok((t0, t1));
            }
            start = middle;
            start_inside = middle_inside;
        }
    }
    Ok((t0, t1))
}

/// Splits a planar Bézier curve where it crosses the boundary of the
/// closed path described by `inside` (matplotlib's
/// `split_bezier_intersecting_with_closedpath`).
pub fn split_bezier_intersecting_with_closedpath(
    control_points: &[Point],
    inside: impl Fn(Point) -> bool,
    tolerance: f64,
) -> Result<(Vec<Point>, Vec<Point>), BezierError> {
    let curve = BezierSegment::from_points(control_points)?;
    let (t0, t1) = find_bezier_t_intersecting_with_closedpath(
        |t| curve.point_at(t),
        inside,
        0.0,
        1.0,
        tolerance,
    )?;
    let (left, right) = curve.split(f64::midpoint(t0, t1));
    Ok((planar(&left), planar(&right)))
}

fn planar(curve: &BezierSegment) -> Vec<Point> {
    curve.points().expect("split keeps the planar dimension")
}

/// The commands of `path` with their vertices (matplotlib's `ctl_points`
/// from `iter_segments`), including the stored points of closures.
fn command_points(path: &Path) -> Vec<(Vec<Point>, PathCode)> {
    let mut out = Vec::new();
    let mut vertices = path.vertices().iter().zip(path.resolved_codes());
    while let Some((&p, code)) = vertices.next() {
        let mut points = vec![p];
        points.extend(
            vertices
                .by_ref()
                .take(code.vertex_count() - 1)
                .map(|(q, _)| *q),
        );
        out.push((points, code));
    }
    out
}

/// Splits `path` at the first segment that crosses the boundary of the
/// closed region `inside`, returning the parts before and after the
/// crossing (matplotlib's `split_path_inout`).
///
/// The path is walked with matplotlib's `iter_segments` defaults (default
/// simplification settings). With `reorder_inout`, the part inside the
/// region comes first. Fails if no segment end changes side
/// ([`BezierError::PathMissesRegion`]) or the crossing segment cannot be
/// split.
pub fn split_path_inout(
    path: &Path,
    inside: impl Fn(Point) -> bool,
    tolerance: f64,
    reorder_inout: bool,
) -> Result<(Path, Path), BezierError> {
    let options = CleanupOptions::segment_defaults(path, &SimplifySettings::default());
    let commands = command_points(&path.cleaned(&options));
    let ((first, _), rest) = commands.split_first().ok_or(BezierError::NotIntersecting)?;
    let last_point = |points: &[Point]| *points.last().expect("commands have points");
    let begin_inside = inside(last_point(first));
    let mut previous = first;
    let mut index = 1;
    let mut crossing = None;
    for (points, _) in rest {
        let start = index;
        index += points.len();
        if inside(last_point(points)) != begin_inside {
            let mut bezier = vec![last_point(previous)];
            bezier.extend_from_slice(points);
            crossing = Some((start, bezier));
            break;
        }
        previous = points;
    }
    let (start, bezier) = crossing.ok_or(BezierError::PathMissesRegion)?;
    let (left, right) = split_bezier_intersecting_with_closedpath(&bezier, &inside, tolerance)?;
    let code = match left.len() {
        2 => PathCode::LineTo,
        3 => PathCode::Curve3,
        _ => PathCode::Curve4,
    };
    // The indices count the cleaned stream's vertices but slice the input,
    // as in matplotlib; clamp them like numpy slicing does.
    let vertices = path.vertices();
    let (start, index) = (start.min(vertices.len()), index.min(vertices.len()));
    let (inner, outer) = match path.codes() {
        None => (
            Path::polyline([&vertices[..index], &left[1..]].concat()),
            Path::polyline([&right[..], &vertices[index..]].concat()),
        ),
        Some(codes) => {
            let mut in_codes = codes[..start].to_vec();
            in_codes.extend(std::iter::repeat_n(code, left.len() - 1));
            let mut out_codes = vec![PathCode::MoveTo];
            out_codes.extend(std::iter::repeat_n(code, right.len() - 1));
            out_codes.extend_from_slice(&codes[index..]);
            let inner = Path::new([&vertices[..start], &left[1..]].concat(), in_codes)
                .expect("the inner part keeps the path's initial move");
            let outer = Path::new([&right[..], &vertices[index..]].concat(), out_codes)
                .expect("the outer part starts with a move");
            (inner, outer)
        }
    };
    Ok(if reorder_inout && !begin_inside {
        (outer, inner)
    } else {
        (inner, outer)
    })
}

impl Path {
    /// matplotlib's `Path.iter_bezier`: every command of the conditioned
    /// path as control points (a move is a one-point curve, a closure a
    /// line back to the sub-path start). Pass
    /// [`CleanupOptions::segment_defaults`] for matplotlib's defaults.
    pub fn iter_bezier(&self, options: &CleanupOptions) -> Vec<(Vec<Point>, PathCode)> {
        let mut first = Point::ORIGIN;
        let mut previous = Point::ORIGIN;
        let mut out = Vec::new();
        for (points, code) in command_points(&self.cleaned(options)) {
            let curve = match code {
                PathCode::MoveTo => {
                    first = points[0];
                    vec![first]
                }
                PathCode::ClosePoly => vec![previous, first],
                PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => std::iter::once(previous)
                    .chain(points.iter().copied())
                    .collect(),
            };
            previous = *points.last().expect("commands have points");
            out.push((curve, code));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn bisection_brackets_the_crossing() {
        let line = |t: f64| p(4.0 * t, 2.0 * t);
        let (t0, t1) = find_bezier_t_intersecting_with_closedpath(
            line,
            inside_circle(p(0.0, 0.0), 1.0),
            0.0,
            1.0,
            1e-4,
        )
        .expect("crosses");
        let crossing = 1.0 / 20f64.sqrt();
        assert!(t0 <= crossing && crossing <= t1 && t1 - t0 < 1e-4);
        assert_eq!(
            find_bezier_t_intersecting_with_closedpath(
                line,
                inside_circle(p(10.0, 10.0), 1.0),
                0.0,
                1.0,
                0.01
            ),
            Err(BezierError::NotIntersecting)
        );
    }

    #[test]
    fn splitting_a_polyline() {
        let path = Path::polyline(vec![p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)]);
        let (inner, outer) =
            split_path_inout(&path, inside_circle(p(0.0, 0.0), 1.5), 0.01, false).expect("crosses");
        assert!(inner.codes().is_none());
        assert!((outer.vertices()[0].x - 1.5).abs() < 0.01);
        assert_eq!(*outer.vertices().last().expect("non-empty"), p(3.0, 0.0));
        let far = Path::polyline(vec![p(5.0, 5.0), p(6.0, 6.0)]);
        assert_eq!(
            split_path_inout(&far, inside_circle(p(0.0, 0.0), 1.0), 0.01, false),
            Err(BezierError::PathMissesRegion)
        );
    }

    #[test]
    fn nan_restarts_split_like_matplotlib() {
        let path = Path::new(
            vec![p(0.0, 0.0), p(f64::NAN, 1.0), p(0.5, 0.0), p(5.0, 0.0)],
            vec![
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::LineTo,
            ],
        )
        .expect("valid");
        let (inner, outer) =
            split_path_inout(&path, inside_circle(p(0.0, 0.0), 1.5), 0.01, false).expect("crosses");
        // matplotlib keeps the NaN vertex of the input in the inner part.
        assert_eq!(inner.len(), 4);
        assert!(inner.vertices()[1].x.is_nan());
        assert_eq!(inner.vertices()[3], p(1.497_558_593_75, 0.0));
        assert_eq!(outer.vertices(), &[p(1.497_558_593_75, 0.0), p(5.0, 0.0)]);
        assert_eq!(
            outer.codes(),
            Some(&[PathCode::MoveTo, PathCode::LineTo][..])
        );
    }

    #[test]
    fn iter_bezier_shapes() {
        let square = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)]);
        let options = CleanupOptions::segment_defaults(&square, &SimplifySettings::default());
        let pieces = square.iter_bezier(&options);
        assert_eq!(pieces[0], (vec![p(0.0, 0.0)], PathCode::MoveTo));
        assert_eq!(
            pieces[3],
            (vec![p(1.0, 1.0), p(0.0, 0.0)], PathCode::ClosePoly)
        );
    }

    #[test]
    fn nan_curves_stop_bisecting() {
        // The end that stays NaN never compares equal, so matplotlib's own
        // termination tests never fire.
        let half_nan = |t: f64| if t > 0.5 { p(f64::NAN, 0.0) } else { p(t, 0.0) };
        let inside = |q: Point| q.x.is_nan();
        let (t0, t1) = find_bezier_t_intersecting_with_closedpath(half_nan, inside, 0.0, 1.0, 0.01)
            .expect("bracketed");
        assert!(t0 <= 0.5 && t1 > 0.5);
        assert_eq!(
            find_bezier_t_intersecting_with_closedpath(half_nan, inside, f64::NAN, 1.0, 0.01),
            Err(BezierError::NotIntersecting)
        );
    }
}
