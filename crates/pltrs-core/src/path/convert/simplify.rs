//! Line simplification (matplotlib's `PathSimplifier`).

use super::queue::VertexQueue;
use super::{Vertex, VertexSource};
use crate::Point;
use crate::path::PathCode;

/// Merges runs of nearly collinear line segments; see
/// [`VertexSource::simplify`].
///
/// This is the algorithm of Allan Haldane, Michael Droettboom and Kevin
/// Rose: a run grows while the perpendicular distance of each new vertex
/// from the run's direction stays below `threshold` (in output units, i.e.
/// pixels). When a run ends, its furthest forward excursion and — if the
/// run also went backwards — its furthest backward excursion are emitted, in
/// the order that preserves the drawn extent. Closures become lines back to
/// the sub-path start. Curves must not reach this stage (matplotlib only
/// simplifies paths made of moves and lines).
#[derive(Clone, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the flags of the C++ state machine"
)]
pub struct Simplifier<I> {
    source: I,
    /// The squared threshold, or `None` to pass vertices through.
    threshold_sq: Option<f64>,
    queue: VertexQueue<9>,
    /// Set once the end of the stream has been queued (Agg's queued STOP).
    finished: bool,
    moveto: bool,
    after_moveto: bool,
    clipped: bool,
    /// The most recent move-to point, if finite.
    init: Option<Point>,
    last: Point,
    /// The reference direction of the current run (zero when none).
    orig: Point,
    orig_norm2: f64,
    forward_max_norm2: f64,
    backward_max_norm2: f64,
    last_forward_max: bool,
    last_backward_max: bool,
    next: Point,
    next_backward: Point,
    run_start: Point,
}

impl<I: VertexSource> Simplifier<I> {
    pub(super) fn new(source: I, threshold: Option<f64>) -> Self {
        Self {
            source,
            threshold_sq: threshold.map(|t| t * t),
            queue: VertexQueue::default(),
            finished: false,
            moveto: true,
            after_moveto: false,
            clipped: false,
            init: None,
            last: Point::ORIGIN,
            orig: Point::ORIGIN,
            orig_norm2: 0.0,
            forward_max_norm2: 0.0,
            backward_max_norm2: 0.0,
            last_forward_max: false,
            last_backward_max: false,
            next: Point::ORIGIN,
            next_backward: Point::ORIGIN,
            run_start: Point::ORIGIN,
        }
    }

    /// Starts a run along `p - last` from the last point.
    fn start_run(&mut self, p: Point) {
        self.orig = p - self.last;
        self.orig_norm2 = self.orig.x * self.orig.x + self.orig.y * self.orig.y;
        self.forward_max_norm2 = self.orig_norm2;
        self.last_forward_max = true;
        self.backward_max_norm2 = 0.0;
        self.last_backward_max = false;
    }

    /// Emits the finished run and starts a new one towards `p`.
    fn push_run(&mut self, p: Point) {
        if self.backward_max_norm2 > 0.0 {
            // If the last vertex was the furthest forward, the backward
            // excursion has to be drawn first.
            let (first, second) = if self.last_forward_max {
                (self.next_backward, self.next)
            } else {
                (self.next, self.next_backward)
            };
            self.queue.push(Vertex::line_to(first));
            self.queue.push(Vertex::line_to(second));
        } else {
            self.queue.push(Vertex::line_to(self.next));
        }
        if self.clipped {
            self.queue.push(Vertex::move_to(self.last));
        } else if !self.last_forward_max && !self.last_backward_max {
            // The run ended between its extremes: go back to its last point.
            self.queue.push(Vertex::line_to(self.last));
        }
        self.start_run(p);
        self.run_start = self.queue.last().map_or(self.last, |v| v.point);
        self.last = p;
        self.next = p;
        self.clipped = false;
    }

    /// The code of the vertices queued when the stream ends.
    fn final_code(&self) -> PathCode {
        if self.moveto || self.after_moveto {
            PathCode::MoveTo
        } else {
            PathCode::LineTo
        }
    }

    /// Queues the end of the stream.
    fn finish(&mut self) {
        if self.orig_norm2 != 0.0 {
            self.queue.push_new(self.final_code(), self.next);
            if self.backward_max_norm2 > 0.0 {
                self.queue.push_new(self.final_code(), self.next_backward);
            }
            self.moveto = false;
        }
        self.queue.push_new(self.final_code(), self.last);
        self.moveto = false;
        self.finished = true;
    }

    fn next_simplified(&mut self, threshold_sq: f64) -> Option<Vertex> {
        if let Some(vertex) = self.queue.pop() {
            return Some(vertex);
        }
        if self.finished {
            return None;
        }
        let mut exhausted = true;
        while let Some(vertex) = self.source.next() {
            let mut p = vertex.point;
            if self.moveto || vertex.code == PathCode::MoveTo {
                if self.orig_norm2 != 0.0 && !self.after_moveto {
                    self.push_run(p);
                }
                self.after_moveto = true;
                self.init = p.is_finite().then_some(p);
                self.last = p;
                self.moveto = false;
                self.orig_norm2 = 0.0;
                self.backward_max_norm2 = 0.0;
                self.clipped = true;
                if !self.queue.is_empty() {
                    exhausted = false;
                    break;
                }
                continue;
            }
            self.after_moveto = false;
            if vertex.code == PathCode::ClosePoly {
                match self.init {
                    Some(init) => p = init,
                    None => continue,
                }
            }
            if self.orig_norm2 == 0.0 {
                if self.clipped {
                    self.queue.push(Vertex::move_to(self.last));
                    self.clipped = false;
                }
                self.start_run(p);
                self.run_start = self.last;
                self.last = p;
                self.next = p;
                continue;
            }
            // Split the offset from the run start into its components along
            // and across the run direction.
            let tot = p - self.run_start;
            let tot_dot = self.orig.x * tot.x + self.orig.y * tot.y;
            let para = Point::new(
                tot_dot * self.orig.x / self.orig_norm2,
                tot_dot * self.orig.y / self.orig_norm2,
            );
            let perp = tot - para;
            let perp_norm2 = perp.x * perp.x + perp.y * perp.y;
            if perp_norm2 < threshold_sq {
                let para_norm2 = para.x * para.x + para.y * para.y;
                self.last_forward_max = false;
                self.last_backward_max = false;
                if tot_dot > 0.0 {
                    if para_norm2 > self.forward_max_norm2 {
                        self.last_forward_max = true;
                        self.forward_max_norm2 = para_norm2;
                        self.next = p;
                    }
                } else if para_norm2 > self.backward_max_norm2 {
                    self.last_backward_max = true;
                    self.backward_max_norm2 = para_norm2;
                    self.next_backward = p;
                }
                self.last = p;
                continue;
            }
            self.push_run(p);
            exhausted = false;
            break;
        }
        if exhausted {
            self.finish();
        }
        self.queue.pop()
    }
}

impl<I: VertexSource> Iterator for Simplifier<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        match self.threshold_sq {
            Some(threshold_sq) => self.next_simplified(threshold_sq),
            None => self.source.next(),
        }
    }
}

impl<I: VertexSource> VertexSource for Simplifier<I> {
    fn rewind(&mut self) {
        self.queue.clear();
        self.finished = false;
        self.moveto = true;
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

    fn simplified(path: &Path) -> Vec<Vertex> {
        path.iter_vertices().simplify(Some(1.0 / 9.0)).collect()
    }

    #[test]
    fn straight_runs_collapse_with_matplotlibs_repeated_end() {
        let path = Path::polyline((0..200).map(|i| p(f64::from(i), 0.0)).collect());
        assert_eq!(
            simplified(&path),
            vec![
                Vertex::move_to(p(0.0, 0.0)),
                Vertex::line_to(p(199.0, 0.0)),
                Vertex::line_to(p(199.0, 0.0)),
            ]
        );
    }

    #[test]
    fn backward_excursions_are_kept() {
        let path = Path::polyline(vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(2.0, 0.0),
            p(3.0, 0.0),
            p(2.0, 0.0),
            p(1.0, 0.0),
            p(5.0, 0.0),
        ]);
        let xs: Vec<f64> = simplified(&path).iter().map(|v| v.point.x).collect();
        assert_eq!(xs, vec![0.0, 5.0, 5.0]);
        let path = Path::polyline(vec![p(0.0, 0.0), p(3.0, 0.0), p(-2.0, 0.0), p(1.0, 0.0)]);
        let xs: Vec<f64> = simplified(&path).iter().map(|v| v.point.x).collect();
        assert_eq!(xs, vec![0.0, 3.0, -2.0, 1.0]);
    }

    #[test]
    fn corners_are_preserved() {
        let path = Path::polyline(vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(2.0, 0.0),
            p(2.0, 1.0),
            p(2.0, 2.0),
        ]);
        let points: Vec<Point> = simplified(&path).iter().map(|v| v.point).collect();
        assert_eq!(
            points,
            vec![p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0), p(2.0, 2.0)]
        );
    }

    #[test]
    fn closures_become_lines_to_the_start() {
        let path = Path::polygon([p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)]);
        let out = simplified(&path);
        assert!(out.iter().all(|v| v.code != PathCode::ClosePoly));
        assert_eq!(out.last().map(|v| v.point), Some(p(0.0, 0.0)));
    }
}
