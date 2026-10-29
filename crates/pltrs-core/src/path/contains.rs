//! Point and path containment (matplotlib's `_path.points_in_path`,
//! `point_in_path`, `point_on_path` and `path_in_path`).

use super::convert::{Vertex, VertexSource};
use super::outline::{Contour, Outline, Stroke};
use super::{Path, PathCode};
use crate::{Affine2D, Point};

/// The crossing-number test of `point_in_path_impl` (after Eric Haines):
/// every sub-path is tested on its own and the results are OR-ed, so holes
/// do not exclude points. A sub-path ended by a closure or by the end of
/// the stream is implicitly closed; one ended by a move is not (its last
/// edge back to the start is not tested), as in matplotlib. Points with a
/// non-finite coordinate are never inside.
fn crossings<I: VertexSource>(points: &[Point], path: &mut I) -> Vec<bool> {
    let n = points.len();
    let mut inside = vec![false; n];
    let mut yflag0 = vec![false; n];
    let mut subpath = vec![false; n];
    let finite = |p: &Point| p.is_finite();
    path.rewind();
    let mut pending_move: Option<Point> = None;
    loop {
        let start = match pending_move.take() {
            Some(p) => p,
            None => match path.next() {
                None => break,
                Some(v) if v.code == PathCode::ClosePoly => continue,
                Some(v) => v.point,
            },
        };
        let (mut v0, mut v1) = (start, start);
        for (i, p) in points.iter().enumerate() {
            if p.y.is_finite() {
                yflag0[i] = v0.y >= p.y;
                subpath[i] = false;
            }
        }
        let mut stream_ended = false;
        loop {
            let (next, subpath_ended) = match path.next() {
                None => {
                    stream_ended = true;
                    (start, true)
                }
                Some(Vertex {
                    code: PathCode::ClosePoly,
                    ..
                }) => (start, true),
                Some(Vertex {
                    code: PathCode::MoveTo,
                    point,
                }) => {
                    pending_move = Some(point);
                    break;
                }
                Some(v) => (v.point, false),
            };
            for (i, p) in points.iter().enumerate().filter(|(_, p)| finite(p)) {
                let yflag1 = v1.y >= p.y;
                if yflag0[i] != yflag1 && crosses(v0, v1, *p, yflag1) {
                    subpath[i] = !subpath[i];
                }
                yflag0[i] = yflag1;
            }
            v0 = v1;
            v1 = next;
            if subpath_ended {
                break;
            }
        }
        let mut all_inside = true;
        for (i, p) in points.iter().enumerate().filter(|(_, p)| finite(p)) {
            let yflag1 = v1.y >= p.y;
            if yflag0[i] != yflag1 && crosses(v0, v1, *p, yflag1) {
                subpath[i] = !subpath[i];
            }
            inside[i] |= subpath[i];
            all_inside &= inside[i];
        }
        if all_inside || stream_ended {
            break;
        }
    }
    inside
}

/// Whether the edge `v0 → v1` (straddling the horizontal through `t`)
/// crosses the ray from `t` towards +x.
fn crosses(v0: Point, v1: Point, t: Point, yflag1: bool) -> bool {
    ((v1.y - t.y) * (v0.x - v1.x) >= (v1.x - t.x) * (v0.y - v1.y)) == yflag1
}

impl Path {
    /// The transformed, NaN-free, flattened stream all containment tests
    /// start from.
    fn containment_stream(&self, transform: &Affine2D) -> impl VertexSource + '_ {
        self.iter_vertices()
            .transform(*transform)
            .remove_nans(true, self.codes.is_some())
            .flatten_curves(true)
    }

    /// Whether the area enclosed by the path contains `point` (matplotlib's
    /// `Path.contains_point`); see [`contains_points`](Self::contains_points).
    pub fn contains_point(&self, point: Point, transform: Option<&Affine2D>, radius: f64) -> bool {
        self.contains_points(&[point], transform, radius)[0]
    }

    /// matplotlib's `Path.contains_points`.
    ///
    /// The path is transformed by `transform`, cleaned of non-finite
    /// vertices and flattened; every sub-path is treated as closed and
    /// tested on its own with the crossing-number rule, and the results are
    /// OR-ed (holes do not exclude points). A non-zero `radius` first
    /// offsets each sub-path by `radius / 2` with Agg's contour generator:
    /// like matplotlib, this grows counter-clockwise sub-paths and shrinks
    /// clockwise ones for positive radii. Paths with fewer than three
    /// vertices contain nothing.
    pub fn contains_points(
        &self,
        points: &[Point],
        transform: Option<&Affine2D>,
        radius: f64,
    ) -> Vec<bool> {
        if self.len() < 3 {
            return vec![false; points.len()];
        }
        let stream = self.containment_stream(transform.unwrap_or(&Affine2D::IDENTITY));
        if radius == 0.0 {
            let mut stream = stream;
            crossings(points, &mut stream)
        } else {
            crossings(points, &mut Outline::new(stream, Contour::new(radius)))
        }
    }

    /// Whether this path contains every vertex of `other` transformed by
    /// `transform` (matplotlib's `Path.contains_path`).
    ///
    /// The vertices tested are those of `other`'s NaN-free, flattened
    /// stream, including the stored coordinates of closures, as in
    /// matplotlib. An empty `other` is contained in any path with at least
    /// three vertices.
    pub fn contains_path(&self, other: &Self, transform: Option<&Affine2D>) -> bool {
        if self.len() < 3 {
            return false;
        }
        let points: Vec<Point> = other
            .containment_stream(transform.unwrap_or(&Affine2D::IDENTITY))
            .map(|v| v.point)
            .collect();
        self.contains_points(&points, None, 0.0)
            .into_iter()
            .all(|inside| inside)
    }

    /// Whether `point` lies on the path stroked with a line `2 * radius`
    /// wide (butt caps, miter joins), after transforming the path by
    /// `transform` (matplotlib's `point_on_path`, used to pick unfilled
    /// collections).
    pub fn stroke_contains_point(&self, point: Point, transform: &Affine2D, radius: f64) -> bool {
        let stream = self.containment_stream(transform);
        crossings(
            &[point],
            &mut Outline::new(stream, Stroke::new(radius * 2.0)),
        )[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn clockwise_square() -> Path {
        Path::polygon([p(0.0, 0.0), p(0.0, 1.0), p(1.0, 1.0), p(1.0, 0.0)])
    }

    #[test]
    fn crossing_rule() {
        let sq = Path::unit_rectangle();
        assert_eq!(
            sq.contains_points(&[p(0.5, 0.5), p(1.5, 0.5), p(f64::NAN, 0.5)], None, 0.0),
            vec![true, false, false]
        );
        assert!(
            !Path::polyline(vec![p(0.0, 0.0), p(1.0, 1.0)]).contains_point(p(0.5, 0.5), None, 0.0)
        );
    }

    #[test]
    fn radius_sign_follows_orientation() {
        let outside = p(1.1, 0.5);
        let inside = p(0.9, 0.5);
        assert!(Path::unit_rectangle().contains_point(outside, None, 0.5));
        assert!(!Path::unit_rectangle().contains_point(inside, None, -0.5));
        assert!(!clockwise_square().contains_point(inside, None, 0.5));
        assert!(clockwise_square().contains_point(outside, None, -0.5));
    }

    #[test]
    fn holes_do_not_exclude_points() {
        let donut = Path::compound([
            &Path::polygon([p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)]),
            &Path::polygon([p(1.0, 1.0), p(1.0, 3.0), p(3.0, 3.0), p(3.0, 1.0)]),
        ]);
        assert!(donut.contains_point(p(2.0, 2.0), None, 0.0));
    }

    #[test]
    fn subpaths_ended_by_a_move_stay_open() {
        // The missing edge (0, 2) → (0, 0) is not tested: (-1, 1) is "inside".
        let path = Path::new(
            vec![
                p(0.0, 0.0),
                p(2.0, 0.0),
                p(2.0, 2.0),
                p(0.0, 2.0),
                p(5.0, 5.0),
                p(6.0, 5.0),
            ],
            vec![
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::LineTo,
                PathCode::MoveTo,
                PathCode::LineTo,
            ],
        )
        .expect("valid");
        assert!(path.contains_point(p(-1.0, 1.0), None, 0.0));
    }

    #[test]
    fn containment_of_paths() {
        let small = Path::polygon([p(0.25, 0.25), p(0.75, 0.25), p(0.75, 0.75), p(0.25, 0.75)]);
        assert!(Path::unit_rectangle().contains_path(&small, None));
        assert!(!small.contains_path(&Path::unit_rectangle(), None));
        let shifted = Affine2D::translation(2.0, 0.0);
        assert!(!Path::unit_rectangle().contains_path(&small, Some(&shifted)));
        assert!(Path::unit_rectangle().contains_path(&Path::default(), None));
    }

    #[test]
    fn stroke_hit_test() {
        let line = Path::polyline(vec![p(0.0, 0.0), p(10.0, 0.0)]);
        let id = Affine2D::IDENTITY;
        assert!(line.stroke_contains_point(p(5.0, 0.4), &id, 0.5));
        assert!(!line.stroke_contains_point(p(5.0, 0.6), &id, 0.5));
        // Butt caps: no extension beyond the end points.
        assert!(!line.stroke_contains_point(p(10.2, 0.0), &id, 0.5));
    }
}
