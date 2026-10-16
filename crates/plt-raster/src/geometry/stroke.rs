//! Stroke outline generation — a port of Agg's `vcgen_stroke` driven like
//! `conv_stroke`.
//!
//! For an open sub-path the outline is a single polygon: start cap, the
//! joins along one side, end cap, the joins back along the other side.
//! A closed sub-path produces two polygons (outer and inner contour),
//! which the non-zero fill rule turns into a ring.

use plt_core::Point;

use super::stroke_math::StrokeMath;
use super::vertex_seq::{VertexDist, VertexSequence};
use super::{PathSink, SubpathAccumulator, SubpathGenerator};
use crate::style::StrokeStyle;

/// Reusable stroke generator state.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stroker {
    src: VertexSequence,
    out: Vec<Point>,
    closed: bool,
    math: Option<StrokeMath>,
}

impl Stroker {
    /// Configures the stroker for `style`.
    pub(crate) fn configure(&mut self, style: &StrokeStyle) {
        self.math = Some(StrokeMath::new(style));
    }

    /// Emits the generated vertices of `out`, the first one as a move-to
    /// when `first` is set.
    fn emit<S: PathSink + ?Sized>(points: &[Point], first: &mut bool, sink: &mut S) {
        for &p in points {
            if *first {
                sink.move_to(p);
                *first = false;
            } else {
                sink.line_to(p);
            }
        }
    }
}

impl SubpathGenerator for Stroker {
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
        let Some(math) = &self.math else {
            return;
        };
        self.src.close(self.closed);
        if self.src.len() < 3 {
            self.closed = false;
        }
        let n = self.src.len();
        if n < 2 + usize::from(self.closed) {
            return;
        }
        let src = &self.src;
        let out = &mut self.out;
        let mut first = true;
        if self.closed {
            for i in 0..n {
                math.calc_join(
                    out,
                    src.prev(i),
                    src.get(i),
                    src.next(i),
                    src.prev(i).dist,
                    src.get(i).dist,
                );
                Self::emit(out, &mut first, sink);
            }
            sink.close();
            first = true;
            for i in (0..n).rev() {
                math.calc_join(
                    out,
                    src.next(i),
                    src.get(i),
                    src.prev(i),
                    src.get(i).dist,
                    src.prev(i).dist,
                );
                Self::emit(out, &mut first, sink);
            }
        } else {
            math.calc_cap(out, src.get(0), src.get(1), src.get(0).dist);
            Self::emit(out, &mut first, sink);
            for i in 1..n - 1 {
                math.calc_join(
                    out,
                    src.prev(i),
                    src.get(i),
                    src.next(i),
                    src.prev(i).dist,
                    src.get(i).dist,
                );
                Self::emit(out, &mut first, sink);
            }
            math.calc_cap(out, src.get(n - 1), src.get(n - 2), src.get(n - 2).dist);
            Self::emit(out, &mut first, sink);
            for i in (1..n - 1).rev() {
                math.calc_join(
                    out,
                    src.next(i),
                    src.get(i),
                    src.prev(i),
                    src.get(i).dist,
                    src.prev(i).dist,
                );
                Self::emit(out, &mut first, sink);
            }
        }
        sink.close();
    }
}

/// A [`PathSink`] that strokes everything it receives into `out`.
#[derive(Debug)]
pub(crate) struct StrokeSink<'a, S: PathSink + ?Sized> {
    stroker: &'a mut Stroker,
    acc: SubpathAccumulator,
    out: &'a mut S,
}

impl<'a, S: PathSink + ?Sized> StrokeSink<'a, S> {
    pub(crate) fn new(stroker: &'a mut Stroker, style: &StrokeStyle, out: &'a mut S) -> Self {
        stroker.configure(style);
        Self {
            stroker,
            acc: SubpathAccumulator::default(),
            out,
        }
    }

    /// Strokes the last pending sub-path.
    pub(crate) fn finish(mut self) {
        self.acc.finish(self.stroker, self.out);
    }
}

impl<S: PathSink + ?Sized> PathSink for StrokeSink<'_, S> {
    fn move_to(&mut self, p: Point) {
        self.acc.move_to(self.stroker, self.out, p);
    }

    fn line_to(&mut self, p: Point) {
        self.acc.line_to(self.stroker, self.out, p);
    }

    fn close(&mut self) {
        self.acc.close(self.stroker, self.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::testing::{Command, Recorder};
    use crate::style::LineCap;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn stroke(
        style: &StrokeStyle,
        build: impl FnOnce(&mut StrokeSink<'_, Recorder>),
    ) -> Vec<Command> {
        let mut stroker = Stroker::default();
        let mut rec = Recorder::default();
        let mut sink = StrokeSink::new(&mut stroker, style, &mut rec);
        build(&mut sink);
        sink.finish();
        rec.0
    }

    #[test]
    fn open_segment_becomes_a_rectangle() {
        let cmds = stroke(&StrokeStyle::new(2.0), |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(10.0, 0.0));
        });
        assert_eq!(
            cmds,
            vec![
                Command::Move(p(0.0, 1.0)),
                Command::Line(p(0.0, -1.0)),
                Command::Line(p(10.0, -1.0)),
                Command::Line(p(10.0, 1.0)),
                Command::Close,
            ]
        );
    }

    #[test]
    fn closed_triangle_has_two_contours() {
        let cmds = stroke(&StrokeStyle::new(1.0), |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(10.0, 0.0));
            s.line_to(p(0.0, 10.0));
            s.close();
        });
        let moves = cmds
            .iter()
            .filter(|c| matches!(c, Command::Move(_)))
            .count();
        let closes = cmds.iter().filter(|c| matches!(c, Command::Close)).count();
        assert_eq!((moves, closes), (2, 2));
    }

    #[test]
    fn degenerate_input_produces_nothing() {
        let style = StrokeStyle::new(2.0).with_cap(LineCap::Round);
        assert_eq!(stroke(&style, |s| s.move_to(p(1.0, 1.0))), []);
        let coincident = stroke(&style, |s| {
            s.move_to(p(1.0, 1.0));
            s.line_to(p(1.0, 1.0));
        });
        assert_eq!(coincident, []);
    }

    #[test]
    fn line_after_close_restarts_at_subpath_start() {
        let cmds = stroke(&StrokeStyle::new(2.0), |s| {
            s.move_to(p(0.0, 0.0));
            s.line_to(p(10.0, 0.0));
            s.line_to(p(10.0, 10.0));
            s.close();
            s.line_to(p(0.0, 20.0));
        });
        // The trailing open segment from (0, 0) to (0, 20) starts with a butt cap at y = 0.
        let last_move = cmds
            .iter()
            .rev()
            .find_map(|c| {
                if let Command::Move(q) = c {
                    Some(*q)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(last_move, p(-1.0, 0.0));
    }
}
