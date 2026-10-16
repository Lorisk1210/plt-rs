//! Paths: sequences of moves, lines, Bézier curves and closures.
//!
//! [`Path`] uses matplotlib's compact representation — a vertex array plus
//! an optional parallel array of [`PathCode`]s (absent codes mean "a single
//! polyline") — so large line plots stay cheap. Consumers iterate it as
//! typed [`Segment`]s via [`Path::segments`].

mod shapes;

use std::fmt;

use crate::bezier::Bezier;
use crate::{Affine2D, Bbox, Point};

/// The instruction attached to each vertex, with matplotlib's numeric values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PathCode {
    /// Start a new sub-path at the vertex.
    MoveTo = 1,
    /// Straight line to the vertex.
    LineTo = 2,
    /// Quadratic Bézier: this vertex is the control point, the next the end.
    Curve3 = 3,
    /// Cubic Bézier: this and the next vertex are controls, the third the end.
    Curve4 = 4,
    /// Close the current sub-path; the vertex itself is ignored.
    ClosePoly = 79,
}

impl PathCode {
    /// Number of vertices the instruction consumes.
    pub const fn vertex_count(self) -> usize {
        match self {
            Self::MoveTo | Self::LineTo | Self::ClosePoly => 1,
            Self::Curve3 => 2,
            Self::Curve4 => 3,
        }
    }
}

impl TryFrom<u8> for PathCode {
    type Error = PathError;

    fn try_from(value: u8) -> Result<Self, PathError> {
        Ok(match value {
            1 => Self::MoveTo,
            2 => Self::LineTo,
            3 => Self::Curve3,
            4 => Self::Curve4,
            79 => Self::ClosePoly,
            other => return Err(PathError::UnknownCode(other)),
        })
    }
}

/// One drawing instruction with its resolved points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    /// Start a new sub-path.
    MoveTo(Point),
    /// Straight line from the current point.
    LineTo(Point),
    /// Quadratic Bézier: control point, end point.
    QuadTo(Point, Point),
    /// Cubic Bézier: two control points, end point.
    CubicTo(Point, Point, Point),
    /// Straight line back to the start of the sub-path.
    Close,
}

/// Why a [`Path`] could not be constructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathError {
    /// `codes` and `vertices` differ in length.
    LengthMismatch {
        /// Number of vertices supplied.
        vertices: usize,
        /// Number of codes supplied.
        codes: usize,
    },
    /// A non-empty path must start with [`PathCode::MoveTo`].
    MissingInitialMove,
    /// A numeric code that matplotlib does not define.
    UnknownCode(u8),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { vertices, codes } => write!(
                f,
                "path has {vertices} vertices but {codes} codes; they must match"
            ),
            Self::MissingInitialMove => f.write_str("the first path code must be MOVETO"),
            Self::UnknownCode(code) => write!(f, "unknown path code {code}"),
        }
    }
}

impl std::error::Error for PathError {}

/// A sequence of sub-paths made of lines and Bézier curves.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Path {
    vertices: Vec<Point>,
    codes: Option<Vec<PathCode>>,
    interpolation_steps: usize,
}

impl Path {
    /// Creates a path from vertices and their codes.
    pub fn new(vertices: Vec<Point>, codes: Vec<PathCode>) -> Result<Self, PathError> {
        if vertices.len() != codes.len() {
            return Err(PathError::LengthMismatch {
                vertices: vertices.len(),
                codes: codes.len(),
            });
        }
        if codes.first().is_some_and(|c| *c != PathCode::MoveTo) {
            return Err(PathError::MissingInitialMove);
        }
        Ok(Self {
            vertices,
            codes: Some(codes),
            interpolation_steps: 1,
        })
    }

    /// A single open polyline through `vertices` (matplotlib's `codes=None`).
    pub fn polyline(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            codes: None,
            interpolation_steps: 1,
        }
    }

    /// A closed polygon in matplotlib's `closed=True` convention: the last
    /// vertex receives [`PathCode::ClosePoly`] and is therefore ignored.
    pub fn closed(vertices: Vec<Point>) -> Self {
        let codes = closed_codes(vertices.len());
        Self {
            vertices,
            codes: Some(codes),
            interpolation_steps: 1,
        }
    }

    /// A closed polygon through `points` (the closing vertex is added).
    pub fn polygon<I: IntoIterator<Item = Point>>(points: I) -> Self {
        let mut vertices: Vec<Point> = points.into_iter().collect();
        if let Some(&first) = vertices.first() {
            vertices.push(first);
        }
        Self::closed(vertices)
    }

    /// Starts building a path segment by segment.
    pub fn builder() -> PathBuilder {
        PathBuilder::default()
    }

    /// Concatenates paths into one compound path (matplotlib's
    /// `make_compound_path`); code-less inputs become move + lines.
    pub fn compound<'a, I: IntoIterator<Item = &'a Self>>(paths: I) -> Self {
        let mut vertices = Vec::new();
        let mut codes = Vec::new();
        for path in paths {
            vertices.extend_from_slice(&path.vertices);
            match &path.codes {
                Some(c) => codes.extend_from_slice(c),
                None => codes.extend(polyline_codes(path.vertices.len())),
            }
        }
        Self {
            vertices,
            codes: Some(codes),
            interpolation_steps: 1,
        }
    }

    /// The vertex array.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    /// Mutable access to the vertices (codes stay unchanged).
    pub fn vertices_mut(&mut self) -> &mut [Point] {
        &mut self.vertices
    }

    /// The code array, or `None` for a plain polyline.
    pub fn codes(&self) -> Option<&[PathCode]> {
        self.codes.as_deref()
    }

    /// The code of vertex `index`, synthesising polyline codes if needed.
    pub fn code_at(&self, index: usize) -> PathCode {
        match &self.codes {
            Some(codes) => codes[index],
            None if index == 0 => PathCode::MoveTo,
            None => PathCode::LineTo,
        }
    }

    /// Every code, synthesising them for polylines.
    pub fn resolved_codes(&self) -> Vec<PathCode> {
        self.codes
            .clone()
            .unwrap_or_else(|| polyline_codes(self.vertices.len()).collect())
    }

    /// Number of vertices.
    pub fn len(&self) -> usize {
        self.vertices.len()
    }

    /// Returns `true` for a path without vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Returns `true` when any segment is a Bézier curve.
    pub fn has_curves(&self) -> bool {
        self.codes.as_ref().is_some_and(|c| {
            c.iter()
                .any(|c| matches!(c, PathCode::Curve3 | PathCode::Curve4))
        })
    }

    /// Hint for curvilinear projections (e.g. polar) to subdivide each
    /// segment into this many pieces before transforming.
    pub const fn interpolation_steps(&self) -> usize {
        self.interpolation_steps
    }

    /// Sets the [`interpolation_steps`](Self::interpolation_steps) hint.
    #[must_use]
    pub fn with_interpolation_steps(mut self, steps: usize) -> Self {
        self.interpolation_steps = steps.max(1);
        self
    }

    /// Iterates the drawing instructions.
    pub fn segments(&self) -> Segments<'_> {
        Segments {
            path: self,
            index: 0,
        }
    }

    /// Iterates every drawn piece as a [`Bezier`] (closures become lines back
    /// to the sub-path start; moves produce nothing).
    pub fn beziers(&self) -> impl Iterator<Item = Bezier> + '_ {
        let mut current = Point::ORIGIN;
        let mut start = Point::ORIGIN;
        self.segments().filter_map(move |segment| {
            let from = current;
            match segment {
                Segment::MoveTo(p) => {
                    current = p;
                    start = p;
                    None
                }
                Segment::LineTo(p) => {
                    current = p;
                    Some(Bezier::Line(from, p))
                }
                Segment::QuadTo(c, p) => {
                    current = p;
                    Some(Bezier::Quad(from, c, p))
                }
                Segment::CubicTo(c1, c2, p) => {
                    current = p;
                    Some(Bezier::Cubic(from, c1, c2, p))
                }
                Segment::Close => {
                    current = start;
                    Some(Bezier::Line(from, start))
                }
            }
        })
    }

    /// Applies an affine transformation in place.
    pub fn transform(&mut self, t: &Affine2D) {
        t.transform_in_place(&mut self.vertices);
    }

    /// Returns a transformed copy.
    #[must_use]
    pub fn transformed(&self, t: &Affine2D) -> Self {
        let mut path = self.clone();
        path.transform(t);
        path
    }

    /// The exact bounding box of the drawn geometry, including curve
    /// extrema. Non-finite points are skipped; an empty path yields
    /// [`Bbox::NULL`].
    pub fn extents(&self) -> Bbox {
        let mut bbox = Bbox::NULL;
        let mut include = |p: Point| bbox = bbox.including(p);
        if !self.has_curves() {
            for (p, code) in self.vertices.iter().zip(self.resolved_codes()) {
                if code != PathCode::ClosePoly {
                    include(*p);
                }
            }
            return bbox;
        }
        for segment in self.segments() {
            if let Segment::MoveTo(p) = segment {
                include(p);
            }
        }
        for curve in self.beziers() {
            include(curve.start());
            include(curve.end());
            for t in curve.axis_aligned_extrema() {
                include(curve.eval(t));
            }
        }
        bbox
    }

    /// Splits the path into its sub-paths (each starting with a move).
    pub fn subpaths(&self) -> Vec<Self> {
        let codes = self.resolved_codes();
        let mut starts: Vec<usize> = codes
            .iter()
            .enumerate()
            .filter(|(i, c)| *i == 0 || **c == PathCode::MoveTo)
            .map(|(i, _)| i)
            .collect();
        starts.push(codes.len());
        starts
            .windows(2)
            .filter(|w| w[0] < w[1])
            .map(|w| Self {
                vertices: self.vertices[w[0]..w[1]].to_vec(),
                codes: Some(codes[w[0]..w[1]].to_vec()),
                interpolation_steps: self.interpolation_steps,
            })
            .collect()
    }

    /// Subdivides every segment into `steps` straight pieces by linear
    /// interpolation of the vertices (matplotlib's `Path.interpolated`).
    #[must_use]
    pub fn interpolated(&self, steps: usize) -> Self {
        if steps <= 1 || self.is_empty() {
            return self.clone();
        }
        let codes = self.resolved_codes();
        if codes.iter().skip(1).any(|c| *c == PathCode::MoveTo) {
            let parts: Vec<Self> = self
                .subpaths()
                .iter()
                .map(|p| p.interpolated(steps))
                .collect();
            return Self::compound(&parts);
        }
        let first = self.vertices[0];
        let vertices: Vec<Point> = self
            .vertices
            .iter()
            .zip(&codes)
            .map(|(p, c)| if *c == PathCode::ClosePoly { first } else { *p })
            .collect();
        let mut out = Vec::with_capacity((vertices.len() - 1) * steps + 1);
        for pair in vertices.windows(2) {
            out.extend((0..steps).map(|i| pair[0].lerp(pair[1], i as f64 / steps as f64)));
        }
        out.push(vertices[vertices.len() - 1]);
        let new_codes = self.codes.as_ref().map(|codes| {
            let mut expanded = vec![PathCode::LineTo; (codes.len() - 1) * steps + 1];
            for (i, code) in codes.iter().enumerate() {
                expanded[i * steps] = *code;
            }
            expanded
        });
        Self {
            vertices: out,
            codes: new_codes,
            interpolation_steps: self.interpolation_steps,
        }
    }
}

fn polyline_codes(len: usize) -> impl Iterator<Item = PathCode> {
    (0..len).map(|i| {
        if i == 0 {
            PathCode::MoveTo
        } else {
            PathCode::LineTo
        }
    })
}

fn closed_codes(len: usize) -> Vec<PathCode> {
    let mut codes: Vec<PathCode> = polyline_codes(len).collect();
    if len > 1 {
        codes[len - 1] = PathCode::ClosePoly;
    }
    codes
}

/// Iterator over a path's [`Segment`]s; see [`Path::segments`].
///
/// A curve truncated by the end of the vertex array ends the iteration.
#[derive(Clone, Debug)]
pub struct Segments<'a> {
    path: &'a Path,
    index: usize,
}

impl Iterator for Segments<'_> {
    type Item = Segment;

    fn next(&mut self) -> Option<Segment> {
        let vertices = &self.path.vertices;
        let i = self.index;
        if i >= vertices.len() {
            return None;
        }
        let code = self.path.code_at(i);
        let needed = code.vertex_count();
        if i + needed > vertices.len() {
            self.index = vertices.len();
            return None;
        }
        self.index += needed;
        Some(match code {
            PathCode::MoveTo => Segment::MoveTo(vertices[i]),
            PathCode::LineTo => Segment::LineTo(vertices[i]),
            PathCode::Curve3 => Segment::QuadTo(vertices[i], vertices[i + 1]),
            PathCode::Curve4 => Segment::CubicTo(vertices[i], vertices[i + 1], vertices[i + 2]),
            PathCode::ClosePoly => Segment::Close,
        })
    }
}

/// Incremental [`Path`] construction.
///
/// ```
/// use plt_core::{Path, Point};
/// let triangle = Path::builder()
///     .move_to(Point::new(0.0, 0.0))
///     .line_to(Point::new(1.0, 0.0))
///     .line_to(Point::new(0.0, 1.0))
///     .close()
///     .build();
/// assert_eq!(triangle.len(), 4);
/// ```
#[derive(Clone, Debug, Default)]
pub struct PathBuilder {
    vertices: Vec<Point>,
    codes: Vec<PathCode>,
    subpath_start: Point,
}

impl PathBuilder {
    /// Starts a new sub-path.
    #[must_use]
    pub fn move_to(mut self, p: Point) -> Self {
        self.subpath_start = p;
        self.push(PathCode::MoveTo, &[p])
    }

    /// Adds a straight line (starting a sub-path if none is open).
    #[must_use]
    pub fn line_to(self, p: Point) -> Self {
        self.ensure_started(p).push(PathCode::LineTo, &[p])
    }

    /// Adds a quadratic Bézier curve.
    #[must_use]
    pub fn quad_to(self, control: Point, end: Point) -> Self {
        self.ensure_started(control)
            .push(PathCode::Curve3, &[control, end])
    }

    /// Adds a cubic Bézier curve.
    #[must_use]
    pub fn cubic_to(self, c1: Point, c2: Point, end: Point) -> Self {
        self.ensure_started(c1)
            .push(PathCode::Curve4, &[c1, c2, end])
    }

    /// Closes the current sub-path.
    #[must_use]
    pub fn close(self) -> Self {
        if self.codes.is_empty() {
            return self;
        }
        let start = self.subpath_start;
        self.push(PathCode::ClosePoly, &[start])
    }

    /// Appends every segment of an existing path.
    #[must_use]
    pub fn extend(mut self, path: &Path) -> Self {
        for segment in path.segments() {
            self = match segment {
                Segment::MoveTo(p) => self.move_to(p),
                Segment::LineTo(p) => self.line_to(p),
                Segment::QuadTo(c, p) => self.quad_to(c, p),
                Segment::CubicTo(c1, c2, p) => self.cubic_to(c1, c2, p),
                Segment::Close => self.close(),
            };
        }
        self
    }

    /// Finishes the path.
    pub fn build(self) -> Path {
        Path {
            vertices: self.vertices,
            codes: Some(self.codes),
            interpolation_steps: 1,
        }
    }

    fn ensure_started(self, p: Point) -> Self {
        if self.codes.is_empty() {
            self.move_to(p)
        } else {
            self
        }
    }

    fn push(mut self, code: PathCode, points: &[Point]) -> Self {
        self.vertices.extend_from_slice(points);
        self.codes.extend(std::iter::repeat_n(code, points.len()));
        self
    }
}

#[cfg(test)]
mod tests {
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
    fn beziers_close_back_to_start() {
        let tri = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(0.0, 1.0)]);
        let pieces: Vec<_> = tri.beziers().collect();
        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[2], Bezier::Line(p(0.0, 1.0), p(0.0, 0.0)));
    }
}
