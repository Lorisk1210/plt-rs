//! Conversion to polygons and polygon clipping (matplotlib's
//! `Path.to_polygons` / `_path.convert_path_to_polygons` and
//! `Path.clip_to_bbox` / `_path.clip_path_to_rect`).

use super::convert::{SimplifySettings, VertexSource};
use super::{Path, PathCode};
use crate::{Affine2D, Bbox, Point};

/// The parameters of [`Path::to_polygons`]; the defaults are matplotlib's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PolygonOptions {
    /// Applied to the vertices first.
    pub transform: Option<Affine2D>,
    /// With `height`, the canvas size: when both are non-zero, lines are
    /// clipped to the canvas (grown by one unit).
    pub width: f64,
    /// See `width`.
    pub height: f64,
    /// Keep only polygons with at least three points, closing them.
    pub closed_only: bool,
    /// The `path.simplify*` rcParams deciding `should_simplify`.
    pub simplify: SimplifySettings,
}

impl Default for PolygonOptions {
    fn default() -> Self {
        Self {
            transform: None,
            width: 0.0,
            height: 0.0,
            closed_only: true,
            simplify: SimplifySettings::default(),
        }
    }
}

/// matplotlib's `_finalize_polygon`: tidies the last polygon of `polygons`
/// (drops it if empty; with `closed_only`, drops it below three points or
/// closes it).
fn finalize_last(polygons: &mut Vec<Vec<Point>>, closed_only: bool) {
    let Some(last) = polygons.last_mut() else {
        return;
    };
    if last.is_empty() || (closed_only && last.len() < 3) {
        polygons.pop();
    } else if closed_only && last.first() != last.last() {
        last.push(last[0]);
    }
}

/// One Sutherland–Hodgman pass against the half-plane `inside`; `bisect`
/// computes the crossing point of an edge `s → p` with the boundary.
fn clip_step(
    polygon: &[Point],
    inside: impl Fn(Point) -> bool,
    bisect: impl Fn(Point, Point) -> Point,
) -> Vec<Point> {
    let mut result = Vec::with_capacity(polygon.len() + 2);
    let Some(&last) = polygon.last() else {
        return result;
    };
    let mut s = last;
    for &p in polygon {
        if inside(s) != inside(p) {
            result.push(bisect(s, p));
        }
        if inside(p) {
            result.push(p);
        }
        s = p;
    }
    result
}

/// Clips one polygon to `x ≤ xmax`, `x ≥ xmin`, `y ≤ ymax`, `y ≥ ymin`, in
/// that order.
fn clip_polygon(polygon: &[Point], xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Vec<Point> {
    let at_x = |x: f64| {
        move |s: Point, p: Point| Point::new(x, s.y + (p.y - s.y) * ((x - s.x) / (p.x - s.x)))
    };
    let at_y = |y: f64| {
        move |s: Point, p: Point| Point::new(s.x + (p.x - s.x) * ((y - s.y) / (p.y - s.y)), y)
    };
    let step = clip_step(polygon, |p| p.x <= xmax, at_x(xmax));
    let step = clip_step(&step, |p| p.x >= xmin, at_x(xmin));
    let step = clip_step(&step, |p| p.y <= ymax, at_y(ymax));
    clip_step(&step, |p| p.y >= ymin, at_y(ymin))
}

impl Path {
    /// matplotlib's `Path.to_polygons`: the path as a list of polylines.
    ///
    /// Code-less paths without a canvas size are returned as one polygon of
    /// their (transformed) vertices, non-finite ones included. Otherwise the
    /// path goes through NaN removal, canvas clipping (when `width` and
    /// `height` are both non-zero), simplification (when `should_simplify`)
    /// and curve flattening, and is split at moves and closures.
    pub fn to_polygons(&self, options: &PolygonOptions) -> Vec<Vec<Point>> {
        if self.is_empty() {
            return Vec::new();
        }
        let transform = options.transform.unwrap_or(Affine2D::IDENTITY);
        let canvas = options.width != 0.0 && options.height != 0.0;
        if self.codes.is_none() && !canvas {
            let mut vertices: Vec<Point> = match options.transform {
                Some(t) => self
                    .vertices
                    .iter()
                    .map(|p| t.transform_point(*p))
                    .collect(),
                None => self.vertices.clone(),
            };
            if options.closed_only {
                if vertices.len() < 3 {
                    return Vec::new();
                }
                if vertices.first() != vertices.last() {
                    vertices.push(vertices[0]);
                }
            }
            return vec![vertices];
        }
        let clip = canvas.then(|| Bbox::from_extents(0.0, 0.0, options.width, options.height));
        let stream = self
            .iter_vertices()
            .transform(transform)
            .remove_nans(true, self.codes.is_some())
            .clip_to(clip)
            .simplify(options.simplify.for_path(self))
            .flatten_curves(true);
        let mut polygons = vec![Vec::new()];
        for vertex in stream {
            match vertex.code {
                PathCode::ClosePoly => {
                    finalize_last(&mut polygons, true);
                    polygons.push(Vec::new());
                }
                code => {
                    if code == PathCode::MoveTo {
                        finalize_last(&mut polygons, options.closed_only);
                        polygons.push(Vec::new());
                    }
                    if let Some(polygon) = polygons.last_mut() {
                        polygon.push(vertex.point);
                    }
                }
            }
        }
        finalize_last(&mut polygons, options.closed_only);
        polygons
    }

    /// The polygons of matplotlib's `_path.clip_path_to_rect`: every
    /// closed polygon of the path clipped to the box, as
    /// [`clip_to_bbox`](Self::clip_to_bbox) joins them.
    pub fn clip_polygons(&self, bbox: &Bbox, inside: bool) -> Vec<Vec<Point>> {
        let (mut xmin, mut xmax) = (bbox.xmin(), bbox.xmax());
        let (mut ymin, mut ymax) = (bbox.ymin(), bbox.ymax());
        if !inside {
            std::mem::swap(&mut xmin, &mut xmax);
            std::mem::swap(&mut ymin, &mut ymax);
        }
        let mut results: Vec<Vec<Point>> = Vec::new();
        let mut stream = self.iter_vertices().flatten_curves(true).peekable();
        while stream.peek().is_some() {
            // Only closures end a polygon: moves inside it are kept as
            // ordinary vertices, and so are the closures' stored points.
            let mut polygon = Vec::new();
            for vertex in stream.by_ref() {
                polygon.push(vertex.point);
                if vertex.code == PathCode::ClosePoly {
                    break;
                }
            }
            let clipped = clip_polygon(&polygon, xmin, xmax, ymin, ymax);
            if !clipped.is_empty() {
                finalize_last(&mut results, true);
                results.push(clipped);
            }
        }
        finalize_last(&mut results, true);
        results
    }

    /// matplotlib's `Path.clip_to_bbox`: every closed polygon of the path
    /// clipped to the box (Sutherland–Hodgman), as one compound path of
    /// closed polylines. With `inside == false` the box bounds are swapped,
    /// exactly as matplotlib does. Curves are flattened; non-finite vertices
    /// are not removed.
    #[must_use]
    pub fn clip_to_bbox(&self, bbox: &Bbox, inside: bool) -> Self {
        let polygons: Vec<Self> = self
            .clip_polygons(bbox, inside)
            .into_iter()
            .map(Self::polyline)
            .collect();
        Self::compound(&polygons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    #[test]
    fn clipping_a_square() {
        let clipped =
            Path::unit_rectangle().clip_to_bbox(&Bbox::from_extents(0.5, 0.5, 2.0, 2.0), true);
        assert_eq!(
            clipped.vertices(),
            &[
                p(0.5, 0.5),
                p(1.0, 0.5),
                p(1.0, 1.0),
                p(0.5, 1.0),
                p(0.5, 0.5)
            ]
        );
        assert_eq!(clipped.code_at(0), PathCode::MoveTo);
        let gone =
            Path::unit_rectangle().clip_to_bbox(&Bbox::from_extents(2.0, 2.0, 3.0, 3.0), true);
        assert!(gone.is_empty());
    }

    #[test]
    fn polygons_of_code_less_paths() {
        let tri = Path::polyline(vec![p(0.0, 0.0), p(4.0, 0.0), p(0.0, 3.0)]);
        assert_eq!(
            tri.to_polygons(&PolygonOptions::default()),
            vec![vec![p(0.0, 0.0), p(4.0, 0.0), p(0.0, 3.0), p(0.0, 0.0)]]
        );
        let line = Path::polyline(vec![p(0.0, 0.0), p(1.0, 1.0)]);
        assert_eq!(
            line.to_polygons(&PolygonOptions::default()),
            Vec::<Vec<Point>>::new()
        );
    }

    #[test]
    fn polygons_split_at_moves_and_closures() {
        let two = Path::compound([
            &Path::unit_rectangle(),
            &Path::polyline(vec![p(5.0, 5.0), p(6.0, 5.0)]),
        ]);
        let open = PolygonOptions {
            closed_only: false,
            ..PolygonOptions::default()
        };
        let polygons = two.to_polygons(&open);
        assert_eq!(polygons.len(), 2);
        assert_eq!(polygons[0].len(), 5);
        assert_eq!(polygons[1], vec![p(5.0, 5.0), p(6.0, 5.0)]);
        assert_eq!(two.to_polygons(&PolygonOptions::default()).len(), 1);
    }
}
