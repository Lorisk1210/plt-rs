//! Paths: sequences of moves, lines, Bézier curves and closures.
//!
//! [`Path`] uses matplotlib's compact representation — a vertex array plus
//! an optional parallel array of [`PathCode`]s (absent codes mean "a single
//! polyline") — so large line plots stay cheap. Consumers iterate it as
//! typed [`Segment`]s via [`Path::segments`].
//!
//! Beyond construction (including matplotlib's unit shapes), this module
//! reproduces matplotlib's C++ `_path` algorithms exactly:
//!
//! * [`convert`] — the conditioning pipeline (`path_converters.h`),
//!   [`Path::cleaned`], [`Path::iter_segments`] and
//!   [`Path::convert_to_string`];
//! * containment ([`Path::contains_points`] with Agg's contour offset for a
//!   radius, [`Path::contains_path`], [`Path::stroke_contains_point`]),
//!   whose offset outlines are built from the Agg primitives in
//!   [`outline`];
//! * intersections ([`Path::intersects_path`], [`Path::intersects_bbox`]);
//! * polygons ([`Path::to_polygons`], [`Path::clip_to_bbox`]);
//! * data limits with `minpos` ([`DataLimits`]), path collections
//!   ([`CollectionGeometry`]) and [`count_overlapping`].

mod builder;
mod collection;
mod contains;
pub mod convert;
mod extents;
mod intersect;
pub mod outline;
mod polygons;
mod shapes;

use std::fmt;

pub use builder::PathBuilder;
pub use collection::{CollectionError, CollectionGeometry};
pub use extents::{DataLimits, count_overlapping};
pub use polygons::PolygonOptions;

use crate::bezier::Bezier;
use crate::{Affine2D, Bbox, Point};

/// The most vertices [`Path::interpolated`] makes: a path of more
/// segments is cut into fewer pieces each, down to none, so that the
/// subdivision curved projections apply to every path (75 pieces a
/// segment on maps) cannot grow a long path beyond memory. matplotlib
/// subdivides whatever the length.
pub const MAX_INTERPOLATED_VERTICES: usize = 1 << 22;

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
    /// Without vertices there is nothing to close, and the path has no
    /// codes, as in matplotlib.
    pub fn closed(vertices: Vec<Point>) -> Self {
        let codes = (!vertices.is_empty()).then(|| closed_codes(vertices.len()));
        Self {
            vertices,
            codes,
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
    /// `make_compound_path`); code-less inputs become move + lines. As in
    /// matplotlib, compounding no paths at all gives an empty path without
    /// codes.
    pub fn compound<'a, I: IntoIterator<Item = &'a Self>>(paths: I) -> Self {
        let mut paths = paths.into_iter().peekable();
        if paths.peek().is_none() {
            return Self::polyline(Vec::new());
        }
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

    /// One closed polygon per entry, each followed by an ignored closing
    /// vertex at the origin (matplotlib's `make_compound_path_from_polys`).
    pub fn compound_from_polygons<P: AsRef<[Point]>>(polygons: &[P]) -> Self {
        let mut vertices = Vec::new();
        let mut codes = Vec::new();
        for polygon in polygons {
            let polygon = polygon.as_ref();
            vertices.extend_from_slice(polygon);
            vertices.push(Point::ORIGIN);
            codes.extend(closed_codes(polygon.len() + 1));
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
    /// interpolation of the vertices (matplotlib's `Path.interpolated`, with
    /// `numpy.interp`'s arithmetic), or into fewer when that would make
    /// more than [`MAX_INTERPOLATED_VERTICES`] vertices.
    #[must_use]
    pub fn interpolated(&self, steps: usize) -> Self {
        let steps = steps.min(MAX_INTERPOLATED_VERTICES / self.len().max(1));
        self.subdivided(steps)
    }

    /// [`Path::interpolated`] without the limit.
    fn subdivided(&self, steps: usize) -> Self {
        if steps <= 1 || self.is_empty() {
            return self.clone();
        }
        let codes = self.resolved_codes();
        if codes.iter().skip(1).any(|c| *c == PathCode::MoveTo) {
            let parts: Vec<Self> = self
                .subpaths()
                .iter()
                .map(|p| p.subdivided(steps))
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
            out.extend((0..steps).map(|k| {
                Point::new(
                    interp(pair[0].x, pair[1].x, k, steps),
                    interp(pair[0].y, pair[1].y, k, steps),
                )
            }));
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

/// `numpy.interp` between samples `a` and `b`, `steps` apart, at offset
/// `k`: samples are returned verbatim and a NaN from one side is retried
/// from the other (so NaN neighbours do not leak into exact samples).
#[expect(clippy::float_cmp, reason = "numpy compares the samples exactly")]
fn interp(a: f64, b: f64, k: usize, steps: usize) -> f64 {
    if k == 0 {
        return a;
    }
    let slope = (b - a) / steps as f64;
    let value = slope * k as f64 + a;
    if !value.is_nan() {
        return value;
    }
    let value = slope * (k as f64 - steps as f64) + b;
    if value.is_nan() && a == b { a } else { value }
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

/// matplotlib's `closed=True` codes: a move, lines, and a closure on the
/// last vertex (which wins for a single vertex).
fn closed_codes(len: usize) -> Vec<PathCode> {
    let mut codes: Vec<PathCode> = polyline_codes(len).collect();
    if let Some(last) = codes.last_mut() {
        *last = PathCode::ClosePoly;
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

#[cfg(test)]
mod tests;
