//! Concatenating the lines or rings traced for one level into the
//! compound path matplotlib stores for it.

use plt_core::{Path, PathCode, Point};

/// The traced pieces of one level, with their path codes.
#[derive(Clone, Debug, Default)]
pub(crate) struct LevelPath {
    vertices: Vec<Point>,
    codes: Vec<PathCode>,
}

impl LevelPath {
    /// Appends a polyline: MOVETO then LINETOs; when `closed`, the last
    /// code becomes CLOSEPOLY (the last vertex then repeats the first).
    /// Empty polylines are skipped.
    pub(crate) fn add_polyline(&mut self, points: &[Point], closed: bool) {
        let Some((&first, rest)) = points.split_first() else {
            return;
        };
        self.vertices.push(first);
        self.codes.push(PathCode::MoveTo);
        self.vertices.extend_from_slice(rest);
        self.codes
            .extend(std::iter::repeat_n(PathCode::LineTo, rest.len()));
        if closed && let Some(last) = self.codes.last_mut() {
            *last = PathCode::ClosePoly;
        }
    }

    /// Appends a traced line, closed when it has more than one point and
    /// its last point is exactly its first.
    pub(crate) fn add_line(&mut self, points: &[Point]) {
        let closed = points.len() > 1 && points.first() == points.last();
        self.add_polyline(points, closed);
    }

    /// Appends a polygon ring given without its closing vertex: the first
    /// point is repeated with CLOSEPOLY.
    pub(crate) fn add_ring(&mut self, points: &[Point]) {
        if let Some(&first) = points.first() {
            self.add_polyline(points, false);
            self.vertices.push(first);
            self.codes.push(PathCode::ClosePoly);
        }
    }

    /// The compound path; without any piece, an empty path without codes
    /// (matplotlib's `Path(np.empty((0, 2)))`).
    pub(crate) fn finish(self) -> Path {
        if self.codes.is_empty() {
            Path::polyline(Vec::new())
        } else {
            self.finish_coded()
        }
    }

    /// The compound path, with an (empty) code array even when nothing was
    /// traced, as the triangular filled generator returns it.
    pub(crate) fn finish_coded(self) -> Path {
        Path::new(self.vertices, self.codes).expect("every piece starts with a MOVETO")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(coords: &[(f64, f64)]) -> Vec<Point> {
        coords.iter().map(|&(x, y)| Point::new(x, y)).collect()
    }

    #[test]
    fn lines_close_only_when_exactly_looping() {
        use PathCode::{ClosePoly as C, LineTo as L, MoveTo as M};
        let mut path = LevelPath::default();
        path.add_line(&pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]));
        path.add_line(&pts(&[(0.0, 0.0), (1.0, 0.0), (1e-17, 0.0)]));
        path.add_line(&pts(&[(2.0, 2.0)]));
        let path = path.finish();
        assert_eq!(path.codes().unwrap(), [M, L, C, M, L, L, M]);
    }

    #[test]
    fn rings_repeat_their_first_point() {
        let mut path = LevelPath::default();
        path.add_ring(&pts(&[(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]));
        let path = path.finish();
        assert_eq!(path.vertices().len(), 4);
        assert_eq!(path.vertices()[3], Point::new(0.0, 0.0));
        assert_eq!(path.codes().unwrap()[3], PathCode::ClosePoly);
    }

    #[test]
    fn empty_levels() {
        assert_eq!(LevelPath::default().finish().codes(), None);
        assert_eq!(LevelPath::default().finish_coded().codes(), Some(&[][..]));
    }
}
