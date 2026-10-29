//! Agg's `vertex_sequence<vertex_dist>`: a polyline that lazily drops
//! coincident vertices and records the length of each segment.

use crate::Point;

/// Points closer than this are coincident (Agg's `vertex_dist_epsilon`).
pub const VERTEX_DIST_EPSILON: f64 = 1e-14;

/// A vertex with the distance to its successor (Agg's `vertex_dist`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexDist {
    /// The x coordinate.
    pub x: f64,
    /// The y coordinate.
    pub y: f64,
    /// The distance to the next vertex, once measured.
    pub dist: f64,
}

impl VertexDist {
    /// The vertex at `p`, not yet measured.
    pub const fn new(p: Point) -> Self {
        Self {
            x: p.x,
            y: p.y,
            dist: 0.0,
        }
    }

    /// The vertex's position.
    pub const fn point(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Agg's `vertex_dist::operator()`: stores the distance to `next` and
    /// reports whether the two points are distinct.
    fn measure(&mut self, next: &Self) -> bool {
        let dx = next.x - self.x;
        let dy = next.y - self.y;
        self.dist = (dx * dx + dy * dy).sqrt();
        let distinct = self.dist > VERTEX_DIST_EPSILON;
        if !distinct {
            self.dist = 1.0 / VERTEX_DIST_EPSILON;
        }
        distinct
    }
}

/// The filtered polyline of one sub-path, as Agg's stroke, dash and
/// contour generators accumulate it.
#[derive(Clone, Debug, Default)]
pub struct VertexSequence {
    v: Vec<VertexDist>,
}

impl VertexSequence {
    /// Removes every vertex.
    pub fn clear(&mut self) {
        self.v.clear();
    }

    /// The number of vertices.
    pub fn len(&self) -> usize {
        self.v.len()
    }

    /// Whether the sequence has no vertices.
    pub fn is_empty(&self) -> bool {
        self.v.is_empty()
    }

    /// The vertex at `i`.
    ///
    /// # Panics
    ///
    /// When `i` is out of bounds.
    pub fn get(&self, i: usize) -> &VertexDist {
        &self.v[i]
    }

    /// The vertex before `i`, wrapping around.
    ///
    /// # Panics
    ///
    /// When the sequence is empty.
    pub fn prev(&self, i: usize) -> &VertexDist {
        &self.v[(i + self.v.len() - 1) % self.v.len()]
    }

    /// The vertex after `i`, wrapping around.
    ///
    /// # Panics
    ///
    /// When the sequence is empty.
    pub fn next(&self, i: usize) -> &VertexDist {
        &self.v[(i + 1) % self.v.len()]
    }

    /// Checks the last pair (dropping the last vertex if it coincides with
    /// its predecessor), then appends `val`.
    pub fn add(&mut self, val: VertexDist) {
        let n = self.v.len();
        if n > 1 {
            let last = self.v[n - 1];
            if !self.v[n - 2].measure(&last) {
                self.v.pop();
            }
        }
        self.v.push(val);
    }

    /// Replaces the last vertex by `val`.
    pub fn modify_last(&mut self, val: VertexDist) {
        self.v.pop();
        self.add(val);
    }

    /// Finalises the sequence: removes trailing coincident vertices and,
    /// for closed sub-paths, a last vertex equal to the first; measures the
    /// closing segment.
    pub fn close(&mut self, closed: bool) {
        while self.v.len() > 1 {
            let n = self.v.len();
            let last = self.v[n - 1];
            if self.v[n - 2].measure(&last) {
                break;
            }
            self.v.pop();
            self.modify_last(last);
        }
        if closed {
            while self.v.len() > 1 {
                let first = self.v[0];
                let n = self.v.len();
                if self.v[n - 1].measure(&first) {
                    break;
                }
                self.v.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vd(x: f64, y: f64) -> VertexDist {
        VertexDist::new(Point::new(x, y))
    }

    #[test]
    fn coincident_vertices_are_dropped() {
        let mut s = VertexSequence::default();
        for (x, y) in [(0.0, 0.0), (3.0, 4.0), (3.0, 4.0), (6.0, 8.0), (6.0, 8.0)] {
            s.add(vd(x, y));
        }
        s.close(false);
        assert_eq!(s.len(), 3);
        assert_eq!(s.get(0).dist, 5.0);
        assert_eq!(s.get(1).dist, 5.0);
        assert_eq!((s.get(2).x, s.get(2).y), (6.0, 8.0));
    }

    #[test]
    fn closing_drops_repeated_start_and_measures_the_wrap() {
        let mut s = VertexSequence::default();
        for (x, y) in [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0), (0.0, 0.0)] {
            s.add(vd(x, y));
        }
        s.close(true);
        assert_eq!(s.len(), 3);
        assert_eq!(s.get(2).dist, 5.0);
        assert_eq!(s.prev(0).x, 4.0);
        assert_eq!(s.next(2).x, 0.0);
    }

    #[test]
    fn modify_last_replaces() {
        let mut s = VertexSequence::default();
        assert!(s.is_empty());
        s.modify_last(vd(1.0, 1.0));
        s.modify_last(vd(2.0, 2.0));
        assert_eq!(s.len(), 1);
        assert_eq!(s.get(0).x, 2.0);
    }
}
