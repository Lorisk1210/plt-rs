//! Incremental path construction.

use super::{Path, PathCode, Segment};
use crate::Point;

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

    /// Closes the current sub-path, storing its start as the (ignored)
    /// vertex of the closing command.
    #[must_use]
    pub fn close(self) -> Self {
        let start = self.subpath_start;
        self.close_at(start)
    }

    /// Closes the current sub-path, storing `vertex` as the closing
    /// command's vertex, which drawing ignores (`FreeType` outlines as
    /// matplotlib reads them store the glyph origin there).
    ///
    /// ```
    /// use plt_core::{Path, Point};
    /// let closed = Path::builder()
    ///     .move_to(Point::new(1.0, 1.0))
    ///     .line_to(Point::new(2.0, 1.0))
    ///     .close_at(Point::ORIGIN)
    ///     .build();
    /// assert_eq!(closed.vertices()[2], Point::ORIGIN);
    /// ```
    #[must_use]
    pub fn close_at(self, vertex: Point) -> Self {
        if self.codes.is_empty() {
            return self;
        }
        self.push(PathCode::ClosePoly, &[vertex])
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
