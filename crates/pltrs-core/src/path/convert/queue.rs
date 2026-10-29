//! A fixed-capacity FIFO for stages that emit several vertices per input
//! (matplotlib's `EmbeddedQueue`).

use super::Vertex;
use crate::Point;
use crate::path::PathCode;

/// Vertices waiting to be returned. Popping from an exhausted queue resets
/// it, so the capacity bounds the vertices queued between two drains.
#[derive(Clone, Debug)]
pub(super) struct VertexQueue<const N: usize> {
    items: [Vertex; N],
    read: usize,
    write: usize,
}

impl<const N: usize> Default for VertexQueue<N> {
    fn default() -> Self {
        Self {
            items: [Vertex::new(PathCode::MoveTo, Point::ORIGIN); N],
            read: 0,
            write: 0,
        }
    }
}

impl<const N: usize> VertexQueue<N> {
    /// Appends a vertex. The callers' state machines never queue more than
    /// `N` vertices between drains (the C++ sizes are tight bounds).
    pub(super) fn push(&mut self, vertex: Vertex) {
        debug_assert!(self.write < N, "vertex queue overflow");
        self.items[self.write] = vertex;
        self.write += 1;
    }

    /// Appends a vertex with the given code and point.
    pub(super) fn push_new(&mut self, code: PathCode, point: Point) {
        self.push(Vertex::new(code, point));
    }

    pub(super) fn is_empty(&self) -> bool {
        self.read >= self.write
    }

    /// The most recently pushed vertex.
    pub(super) fn last(&self) -> Option<&Vertex> {
        self.write.checked_sub(1).map(|i| &self.items[i])
    }

    /// Removes the oldest vertex; an empty queue resets its cursors.
    pub(super) fn pop(&mut self) -> Option<Vertex> {
        if self.is_empty() {
            self.clear();
            return None;
        }
        let vertex = self.items[self.read];
        self.read += 1;
        Some(vertex)
    }

    pub(super) fn clear(&mut self) {
        self.read = 0;
        self.write = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_order_and_reset() {
        let mut q = VertexQueue::<2>::default();
        q.push(Vertex::move_to(Point::new(1.0, 0.0)));
        q.push(Vertex::line_to(Point::new(2.0, 0.0)));
        assert_eq!(q.last().map(|v| v.point.x), Some(2.0));
        assert_eq!(q.pop().map(|v| v.code), Some(PathCode::MoveTo));
        assert_eq!(q.pop().map(|v| v.code), Some(PathCode::LineTo));
        assert!(q.pop().is_none());
        // Draining resets the cursors, so the full capacity is available again.
        q.push(Vertex::line_to(Point::new(3.0, 0.0)));
        q.push(Vertex::line_to(Point::new(4.0, 0.0)));
        assert_eq!(q.pop().map(|v| v.point.x), Some(3.0));
    }
}
