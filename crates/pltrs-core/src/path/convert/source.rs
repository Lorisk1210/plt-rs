//! The head of every pipeline: a path's stored vertices, optionally
//! transformed.

use super::{Vertex, VertexSource};
use crate::path::Path;
use crate::{Affine2D, Point};

/// Iterates a path's vertices with their codes (synthesised for polylines),
/// like matplotlib's C++ `PathIterator`; see [`Path::iter_vertices`].
#[derive(Clone, Debug)]
pub struct PathVertices<'a> {
    path: &'a Path,
    index: usize,
}

impl<'a> PathVertices<'a> {
    pub(crate) const fn new(path: &'a Path) -> Self {
        Self { path, index: 0 }
    }
}

impl Iterator for PathVertices<'_> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        let point = *self.path.vertices.get(self.index)?;
        let code = self.path.code_at(self.index);
        self.index += 1;
        Some(Vertex::new(code, point))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = self.path.len().saturating_sub(self.index);
        (left, Some(left))
    }
}

impl VertexSource for PathVertices<'_> {
    fn rewind(&mut self) {
        self.index = 0;
    }
}

/// Applies an affine map to every vertex except closures (Agg's
/// `conv_transform`); see [`VertexSource::transform`].
#[derive(Clone, Debug)]
pub struct Transformed<I> {
    source: I,
    transform: Affine2D,
}

impl<I> Transformed<I> {
    pub(super) const fn new(source: I, transform: Affine2D) -> Self {
        Self { source, transform }
    }
}

impl<I: VertexSource> Iterator for Transformed<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        let mut vertex = self.source.next()?;
        if vertex.is_vertex() {
            vertex.point = self.transform.agg_transform_point(vertex.point);
        }
        Some(vertex)
    }
}

impl<I: VertexSource> VertexSource for Transformed<I> {
    fn rewind(&mut self) {
        self.source.rewind();
    }
}

impl Path {
    /// The stored vertices as a conditioning stream; the start of every
    /// [`convert`](crate::path::convert) pipeline.
    pub fn iter_vertices(&self) -> PathVertices<'_> {
        PathVertices::new(self)
    }
}

/// Reads the next vertex's point the way Agg reads past a command's first
/// vertex: an exhausted source yields Agg's stop coordinates `(0, 0)`.
pub(super) fn next_point<I: Iterator<Item = Vertex>>(source: &mut I) -> Point {
    source.next().map_or(Point::ORIGIN, |v| v.point)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathCode;

    #[test]
    fn closures_are_not_transformed() {
        let path = Path::closed(vec![
            Point::new(1.0, 1.0),
            Point::new(2.0, 1.0),
            Point::new(7.0, 7.0),
        ]);
        let out: Vec<Vertex> = path
            .iter_vertices()
            .transform(Affine2D::translation(10.0, 0.0))
            .collect();
        assert_eq!(out[0], Vertex::move_to(Point::new(11.0, 1.0)));
        assert_eq!(
            out[2],
            Vertex::new(PathCode::ClosePoly, Point::new(7.0, 7.0))
        );
    }

    #[test]
    fn rewind_restarts() {
        let path = Path::polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 0.0)]);
        let mut it = path.iter_vertices();
        assert_eq!(it.by_ref().count(), 2);
        assert_eq!(it.next(), None);
        it.rewind();
        assert_eq!(it.next(), Some(Vertex::move_to(Point::ORIGIN)));
    }
}
