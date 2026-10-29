//! Dashing — a port of Agg's `vcgen_dash` driven like `conv_dash`.
//!
//! Each sub-path is cut into dashes; every dash is emitted as an open
//! sub-path (gaps become moves), so the downstream stroker caps both ends
//! of each dash.
//!
//! Unlike Agg, the work is bounded: offsets spanning many pattern periods
//! are reduced modulo the period first, and once a path has been cut into
//! [`DASH_BUDGET`] dashes (or meets an infinitely long segment) the rest of
//! it is drawn solid. Agg would loop forever on a `1e300` offset or line.

use pltrs_core::Point;
use pltrs_core::path::outline::{VertexDist, VertexSequence};

use super::{PathSink, SubpathAccumulator, SubpathGenerator};
use crate::style::DashPattern;

/// The most dashes one path is cut into: far beyond any figure, yet a bound
/// for lines that are huge (or patterns that are tiny) relative to each
/// other.
const DASH_BUDGET: usize = 1 << 22;

/// Offsets beyond this many pattern periods are reduced modulo the period
/// before Agg's walk through the pattern.
const MAX_WALKED_PERIODS: f64 = 1024.0;

/// Reusable dash generator state.
#[derive(Clone, Debug, Default)]
pub(crate) struct Dasher {
    lengths: Vec<f64>,
    dash_start: f64,
    curr_dash: usize,
    curr_dash_start: f64,
    src: VertexSequence,
    closed: bool,
    /// Dashes left before the rest of the path is drawn solid.
    budget: usize,
}

impl Dasher {
    /// Configures the pattern and the initial phase.
    pub(crate) fn configure(&mut self, pattern: &DashPattern) {
        self.lengths.clear();
        self.lengths
            .extend(pattern.pairs().iter().flat_map(|&(d, g)| [d, g]));
        self.dash_start = pattern.offset();
        self.budget = DASH_BUDGET;
        self.calc_dash_start(pattern.offset().abs());
    }

    /// Positions the pattern `ds` into its cycle.
    fn calc_dash_start(&mut self, mut ds: f64) {
        self.curr_dash = 0;
        self.curr_dash_start = 0.0;
        let period: f64 = self.lengths.iter().sum();
        if ds > period * MAX_WALKED_PERIODS {
            ds %= period;
        }
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
            if sink.is_saturated() {
                return;
            }
            let dash_rest = self.lengths[self.curr_dash] - self.curr_dash_start;
            let in_gap = self.curr_dash % 2 == 1;
            let point = if curr_rest > dash_rest {
                if self.budget == 0 || curr_rest.is_infinite() {
                    self.finish_solid(sink, src_vertex);
                    return;
                }
                self.budget -= 1;
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

impl Dasher {
    /// Draws the source vertices from `src_vertex` on (back to the start
    /// for closed sub-paths) as one solid line.
    fn finish_solid<S: PathSink + ?Sized>(&self, sink: &mut S, src_vertex: usize) {
        let n = self.src.len();
        let end = if self.closed { n + 1 } else { n };
        for i in src_vertex..end {
            let v = self.src.get(if i >= n { 0 } else { i });
            sink.line_to(Point::new(v.x, v.y));
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

    fn is_saturated(&self) -> bool {
        self.out.is_saturated()
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
    fn huge_offsets_are_reduced_modulo_the_period() {
        // Walking a 1e300 offset in steps of 3 and 2 would never finish.
        let line = |s: &mut DashSink<'_, Recorder>| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(6.0, 0.0));
        };
        let huge = DashPattern::new(vec![(3.0, 2.0)], 1e300).unwrap();
        let reduced = DashPattern::new(vec![(3.0, 2.0)], 1e300 % 5.0).unwrap();
        assert_eq!(dash(&huge, line), dash(&reduced, line));
    }

    /// Counts commands without storing them.
    #[derive(Default)]
    struct Counter {
        commands: usize,
        last: Option<Point>,
    }

    impl PathSink for Counter {
        fn move_to(&mut self, p: Point) {
            self.commands += 1;
            self.last = Some(p);
        }
        fn line_to(&mut self, p: Point) {
            self.move_to(p);
        }
        fn close(&mut self) {
            self.commands += 1;
        }
    }

    #[test]
    fn dashing_stops_at_the_budget() {
        let mut dasher = Dasher::default();
        let mut counter = Counter::default();
        // A 1e9-pixel line would hold 5e20 dashes of this pattern.
        let pattern = DashPattern::new(vec![(1e-12, 1e-12)], 0.0).unwrap();
        let mut sink = DashSink::new(&mut dasher, &pattern, &mut counter);
        sink.move_to(p(0.0, 0.0));
        sink.line_to(p(1e9, 0.0));
        sink.line_to(p(1e9, 5.0));
        sink.finish();
        assert_eq!(counter.commands, 1 + DASH_BUDGET + 2);
        assert_eq!(counter.last, Some(p(1e9, 5.0)));
    }

    #[test]
    fn infinite_segments_are_drawn_solid() {
        let pattern = DashPattern::new(vec![(3.0, 2.0)], 0.0).unwrap();
        let cmds = dash(&pattern, |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(f64::INFINITY, 0.0));
            s.line_to(p(0.0, 1.0));
        });
        assert_eq!(
            cmds,
            vec![
                Command::Move(p(0.0, 0.0)),
                Command::Line(p(f64::INFINITY, 0.0)),
                Command::Line(p(0.0, 1.0)),
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
