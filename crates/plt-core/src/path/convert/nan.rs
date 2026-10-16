//! Removal of non-finite vertices (matplotlib's `PathNanRemover`).

use super::queue::VertexQueue;
use super::source::next_point;
use super::{Vertex, VertexSource, extra_vertices};
use crate::Point;
use crate::path::PathCode;

/// Skips non-finite vertices by starting new sub-paths around them; see
/// [`VertexSource::remove_nans`].
///
/// Polylines (`has_codes == false`) restart with a move at the next finite
/// vertex. Paths with codes are processed per command: a curve containing
/// any non-finite point is dropped as a whole, and a closure whose sub-path
/// was broken becomes a line back to the sub-path start when both ends are
/// finite (and is dropped otherwise). Closures before the first valid
/// segment are treated like ordinary vertices, as in matplotlib.
///
/// The stream is not fused: when the input ends right after a dropped
/// curve, the move to that curve's end point stays queued and is returned
/// if the caller keeps polling after `None` — matplotlib's intersection
/// tests do exactly that.
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the flags of the C++ state machine"
)]
pub struct NanRemover<I> {
    source: I,
    enabled: bool,
    has_codes: bool,
    queue: VertexQueue<4>,
    valid_segment_exists: bool,
    last_segment_valid: bool,
    was_broken: bool,
    init: Point,
}

impl<I: VertexSource> NanRemover<I> {
    pub(super) fn new(source: I, enabled: bool, has_codes: bool) -> Self {
        Self {
            source,
            enabled,
            has_codes,
            queue: VertexQueue::default(),
            valid_segment_exists: false,
            last_segment_valid: false,
            was_broken: false,
            init: Point::new(f64::NAN, f64::NAN),
        }
    }

    fn is_valid_closure(&self, vertex: &Vertex) -> bool {
        vertex.code == PathCode::ClosePoly && self.valid_segment_exists
    }

    /// The fast path for code-less polylines.
    fn next_polyline(&mut self) -> Option<Vertex> {
        let vertex = self.source.next()?;
        if self.is_valid_closure(&vertex) {
            return Some(vertex);
        }
        if !vertex.point.is_finite() {
            loop {
                let next = self.source.next()?;
                if self.is_valid_closure(&next) {
                    return Some(next);
                }
                if next.point.is_finite() {
                    return Some(Vertex::move_to(next.point));
                }
            }
        }
        self.valid_segment_exists = true;
        Some(vertex)
    }

    /// The command-aware path for curves and closures.
    fn next_with_codes(&mut self) -> Option<Vertex> {
        if let Some(vertex) = self.queue.pop() {
            return Some(vertex);
        }
        let mut needs_move_to = false;
        loop {
            // At the end of the source a queued restart stays queued: like
            // Agg, a caller polling past the end receives it.
            let vertex = self.source.next()?;
            if self.is_valid_closure(&vertex) {
                if !self.was_broken {
                    return Some(vertex);
                }
                if self.last_segment_valid && self.init.is_finite() {
                    // Join back to the start when both ends are valid.
                    self.queue.push(Vertex::line_to(self.init));
                    break;
                }
                // Skip the closure: more sub-paths may follow.
                continue;
            }
            if vertex.code == PathCode::MoveTo {
                self.init = vertex.point;
                self.was_broken = false;
            }
            if needs_move_to {
                self.queue.push(Vertex::move_to(vertex.point));
            }
            let mut last = vertex.point;
            self.last_segment_valid = last.is_finite();
            self.queue.push(vertex);
            for _ in 0..extra_vertices(vertex.code) {
                last = next_point(&mut self.source);
                self.last_segment_valid &= last.is_finite();
                self.queue.push_new(vertex.code, last);
            }
            if self.last_segment_valid {
                self.valid_segment_exists = true;
                break;
            }
            self.was_broken = true;
            self.queue.clear();
            // Restart at the segment's end if it is finite, otherwise at the
            // first vertex of the next segment.
            needs_move_to = !last.is_finite();
            if !needs_move_to {
                self.queue.push(Vertex::move_to(last));
            }
        }
        self.queue.pop()
    }
}

impl<I: VertexSource> Iterator for NanRemover<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        if !self.enabled {
            self.source.next()
        } else if self.has_codes {
            self.next_with_codes()
        } else {
            self.next_polyline()
        }
    }
}

impl<I: VertexSource> VertexSource for NanRemover<I> {
    fn rewind(&mut self) {
        self.queue.clear();
        self.source.rewind();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Path;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn run(path: &Path) -> Vec<Vertex> {
        path.iter_vertices()
            .remove_nans(true, path.codes().is_some())
            .collect()
    }

    #[test]
    fn polylines_restart_after_nans() {
        let path = Path::polyline(vec![
            p(0.0, 0.0),
            p(1.0, 1.0),
            p(f64::NAN, 2.0),
            p(3.0, f64::INFINITY),
            p(4.0, 4.0),
            p(5.0, 5.0),
        ]);
        assert_eq!(
            run(&path),
            vec![
                Vertex::move_to(p(0.0, 0.0)),
                Vertex::line_to(p(1.0, 1.0)),
                Vertex::move_to(p(4.0, 4.0)),
                Vertex::line_to(p(5.0, 5.0)),
            ]
        );
    }

    #[test]
    fn curves_with_nans_are_dropped_whole() {
        let path = Path::builder()
            .move_to(p(0.0, 0.0))
            .line_to(p(1.0, 1.0))
            .cubic_to(p(1.0, f64::NAN), p(2.0, 0.0), p(3.0, 3.0))
            .line_to(p(4.0, 0.0))
            .build();
        let codes: Vec<PathCode> = run(&path).iter().map(|v| v.code).collect();
        assert_eq!(
            codes,
            [
                PathCode::MoveTo,
                PathCode::LineTo,
                PathCode::MoveTo,
                PathCode::LineTo
            ]
        );
        assert_eq!(run(&path)[2].point, p(3.0, 3.0));
    }

    #[test]
    fn broken_closures_join_back_to_the_start() {
        let path = Path::closed(vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(f64::NAN, 1.0),
            p(0.0, 1.0),
            p(0.0, 0.0),
        ]);
        assert_eq!(
            run(&path),
            vec![
                Vertex::move_to(p(0.0, 0.0)),
                Vertex::line_to(p(1.0, 0.0)),
                Vertex::move_to(p(0.0, 1.0)),
                Vertex::line_to(p(0.0, 1.0)),
                Vertex::line_to(p(0.0, 0.0)),
            ]
        );
    }

    #[test]
    fn disabled_passes_through() {
        let path = Path::polyline(vec![p(f64::NAN, 0.0), p(1.0, 1.0)]);
        assert_eq!(path.iter_vertices().remove_nans(false, false).count(), 2);
    }
}
