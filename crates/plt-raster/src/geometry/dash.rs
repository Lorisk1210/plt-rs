//! Dashing — a port of Agg's `vcgen_dash` driven like `conv_dash`.
//!
//! Each sub-path is cut into dashes; every dash is emitted as an open
//! sub-path (gaps become moves), so the downstream stroker caps both ends
//! of each dash.

use plt_core::Point;

use super::vertex_seq::{VertexDist, VertexSequence};
use super::{PathSink, SubpathAccumulator, SubpathGenerator};
use crate::style::DashPattern;

/// Reusable dash generator state.
#[derive(Clone, Debug, Default)]
pub(crate) struct Dasher {
    lengths: Vec<f64>,
    dash_start: f64,
    curr_dash: usize,
    curr_dash_start: f64,
    src: VertexSequence,
    closed: bool,
}

impl Dasher {
    /// Configures the pattern and the initial phase.
    pub(crate) fn configure(&mut self, pattern: &DashPattern) {
        self.lengths.clear();
        self.lengths
            .extend(pattern.pairs().iter().flat_map(|&(d, g)| [d, g]));
        self.dash_start = pattern.offset();
        self.calc_dash_start(pattern.offset().abs());
    }

    /// Positions the pattern `ds` into its cycle.
    fn calc_dash_start(&mut self, mut ds: f64) {
        self.curr_dash = 0;
        self.curr_dash_start = 0.0;
        while ds > 0.0 {
            if ds > self.lengths[self.curr_dash] {
                ds -= self.lengths[self.curr_dash];
                self.curr_dash += 1;
                self.curr_dash_start = 0.0;
                if self.curr_dash >= self.lengths.len() {
                    self.curr_dash = 0;
                }
            } else {
                self.curr_dash_start = ds;
                ds = 0.0;
            }
        }
    }
}

impl SubpathGenerator for Dasher {
    fn begin(&mut self, p: Point) {
        self.src.clear();
        self.closed = false;
        self.src.modify_last(VertexDist::new(p));
    }

    fn add(&mut self, p: Point) {
        self.src.add(VertexDist::new(p));
    }

    fn mark_closed(&mut self) {
        self.closed = true;
    }

    fn generate<S: PathSink + ?Sized>(&mut self, sink: &mut S) {
        self.src.close(self.closed);
        let n = self.src.len();
        if self.lengths.len() < 2 || n < 2 {
            return;
        }
        let first = *self.src.get(0);
        sink.move_to(Point::new(first.x, first.y));
        if self.dash_start >= 0.0 {
            self.calc_dash_start(self.dash_start);
        }

        let mut src_vertex = 1;
        let mut v1 = first;
        let mut v2 = *self.src.get(1);
        let mut curr_rest = v1.dist;
        loop {
            let dash_rest = self.lengths[self.curr_dash] - self.curr_dash_start;
            let in_gap = self.curr_dash % 2 == 1;
            let point = if curr_rest > dash_rest {
                curr_rest -= dash_rest;
                self.curr_dash += 1;
                if self.curr_dash >= self.lengths.len() {
                    self.curr_dash = 0;
                }
                self.curr_dash_start = 0.0;
                Point::new(
                    v2.x - (v2.x - v1.x) * curr_rest / v1.dist,
                    v2.y - (v2.y - v1.y) * curr_rest / v1.dist,
                )
            } else {
                self.curr_dash_start += curr_rest;
                let point = Point::new(v2.x, v2.y);
                src_vertex += 1;
                v1 = v2;
                curr_rest = v1.dist;
                let done = if self.closed {
                    src_vertex > n
                } else {
                    src_vertex >= n
                };
                if done {
                    emit(sink, in_gap, point);
                    return;
                }
                v2 = *self.src.get(if src_vertex >= n { 0 } else { src_vertex });
                point
            };
            emit(sink, in_gap, point);
        }
    }
}

fn emit<S: PathSink + ?Sized>(sink: &mut S, in_gap: bool, p: Point) {
    if in_gap {
        sink.move_to(p);
    } else {
        sink.line_to(p);
    }
}

/// A [`PathSink`] that dashes everything it receives into `out`.
#[derive(Debug)]
pub(crate) struct DashSink<'a, S: PathSink + ?Sized> {
    dasher: &'a mut Dasher,
    acc: SubpathAccumulator,
    out: &'a mut S,
}

impl<'a, S: PathSink + ?Sized> DashSink<'a, S> {
    pub(crate) fn new(dasher: &'a mut Dasher, pattern: &DashPattern, out: &'a mut S) -> Self {
        dasher.configure(pattern);
        Self {
            dasher,
            acc: SubpathAccumulator::default(),
            out,
        }
    }

    /// Dashes the last pending sub-path.
    pub(crate) fn finish(mut self) {
        self.acc.finish(self.dasher, self.out);
    }
}

impl<S: PathSink + ?Sized> PathSink for DashSink<'_, S> {
    fn move_to(&mut self, p: Point) {
        self.acc.move_to(self.dasher, self.out, p);
    }

    fn line_to(&mut self, p: Point) {
        self.acc.line_to(self.dasher, self.out, p);
    }

    fn close(&mut self) {
        self.acc.close(self.dasher, self.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::testing::{Command, Recorder};

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn dash(
        pattern: &DashPattern,
        build: impl FnOnce(&mut DashSink<'_, Recorder>),
    ) -> Vec<Command> {
        let mut dasher = Dasher::default();
        let mut rec = Recorder::default();
        let mut sink = DashSink::new(&mut dasher, pattern, &mut rec);
        build(&mut sink);
        sink.finish();
        rec.0
    }

    #[test]
    fn splits_a_line_into_dashes() {
        let pattern = DashPattern::new(vec![(3.0, 2.0)], 0.0).unwrap();
        let cmds = dash(&pattern, |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(12.0, 0.0));
        });
        assert_eq!(
            cmds,
            vec![
                Command::Move(p(0.0, 0.0)),
                Command::Line(p(3.0, 0.0)),
                Command::Move(p(5.0, 0.0)),
                Command::Line(p(8.0, 0.0)),
                Command::Move(p(10.0, 0.0)),
                Command::Line(p(12.0, 0.0)),
            ]
        );
    }

    #[test]
    fn offset_shifts_the_phase() {
        let pattern = DashPattern::new(vec![(3.0, 2.0)], 4.0).unwrap();
        let cmds = dash(&pattern, |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(6.0, 0.0));
        });
        assert_eq!(
            cmds,
            vec![
                Command::Move(p(0.0, 0.0)),
                Command::Move(p(1.0, 0.0)),
                Command::Line(p(4.0, 0.0)),
                Command::Move(p(6.0, 0.0)),
            ]
        );
    }

    #[test]
    fn closed_paths_dash_back_to_the_start() {
        let pattern = DashPattern::new(vec![(100.0, 1.0)], 0.0).unwrap();
        let cmds = dash(&pattern, |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(4.0, 0.0));
            s.line_to(p(4.0, 4.0));
            s.close();
        });
        assert_eq!(cmds.last(), Some(&Command::Line(p(0.0, 0.0))));
        assert!(!cmds.contains(&Command::Close));
    }
}
