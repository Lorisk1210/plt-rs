//! The vertex pipeline in front of the rasterizer: curve flattening,
//! dashing and stroking.
//!
//! Agg chains *pull*-based vertex sources (`conv_curve → conv_dash →
//! conv_stroke → rasterizer`); here every stage is a *push*-based
//! [`PathSink`] that forwards its output to the next stage. Each stage
//! keeps Agg's exact per-sub-path semantics.

pub(crate) mod curve;
pub(crate) mod dash;
pub(crate) mod stroke;
pub(crate) mod stroke_math;
mod vertex_seq;

use plt_core::Point;

/// A consumer of flattened path commands (straight segments only).
pub trait PathSink {
    /// Starts a new sub-path at `p`.
    fn move_to(&mut self, p: Point);
    /// Adds a straight segment from the current point to `p`.
    fn line_to(&mut self, p: Point);
    /// Closes the current sub-path.
    fn close(&mut self);
}

/// Wraps a sub-path generator (stroker or dasher) the way Agg's
/// `conv_adaptor_vcgen` does: vertices are accumulated per sub-path and the
/// generator runs when the sub-path ends (at the next move, a close, or
/// [`SubpathAccumulator::finish`]). A line after a close starts a new
/// sub-path at the previous move-to point.
#[derive(Clone, Debug, Default)]
pub(crate) struct SubpathAccumulator {
    start: Point,
    has_start: bool,
    pending: bool,
}

/// What a generator must provide to be driven by [`SubpathAccumulator`].
pub(crate) trait SubpathGenerator {
    /// Discards the accumulated sub-path and starts one at `p`.
    fn begin(&mut self, p: Point);
    /// Appends a vertex.
    fn add(&mut self, p: Point);
    /// Marks the sub-path as closed.
    fn mark_closed(&mut self);
    /// Emits the generated geometry for the accumulated sub-path.
    fn generate<S: PathSink + ?Sized>(&mut self, out: &mut S);
}

impl SubpathAccumulator {
    pub(crate) fn move_to<G: SubpathGenerator, S: PathSink + ?Sized>(
        &mut self,
        generator: &mut G,
        out: &mut S,
        p: Point,
    ) {
        self.flush(generator, out);
        generator.begin(p);
        self.start = p;
        self.has_start = true;
        self.pending = true;
    }

    pub(crate) fn line_to<G: SubpathGenerator, S: PathSink + ?Sized>(
        &mut self,
        generator: &mut G,
        out: &mut S,
        p: Point,
    ) {
        if !self.has_start {
            // Agg treats the very first vertex as the start, whatever its command.
            self.move_to(generator, out, p);
            return;
        }
        if !self.pending {
            generator.begin(self.start);
            self.pending = true;
        }
        generator.add(p);
    }

    pub(crate) fn close<G: SubpathGenerator, S: PathSink + ?Sized>(
        &mut self,
        generator: &mut G,
        out: &mut S,
    ) {
        if self.pending {
            generator.mark_closed();
            self.flush(generator, out);
        }
    }

    pub(crate) fn finish<G: SubpathGenerator, S: PathSink + ?Sized>(
        &mut self,
        generator: &mut G,
        out: &mut S,
    ) {
        self.flush(generator, out);
        self.has_start = false;
    }

    fn flush<G: SubpathGenerator, S: PathSink + ?Sized>(&mut self, generator: &mut G, out: &mut S) {
        if self.pending {
            generator.generate(out);
            self.pending = false;
        }
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A sink recording commands, for tests.
    #[derive(Debug, Default, PartialEq)]
    pub(crate) struct Recorder(pub(crate) Vec<Command>);

    /// One recorded command.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub(crate) enum Command {
        Move(Point),
        Line(Point),
        Close,
    }

    impl PathSink for Recorder {
        fn move_to(&mut self, p: Point) {
            self.0.push(Command::Move(p));
        }
        fn line_to(&mut self, p: Point) {
            self.0.push(Command::Line(p));
        }
        fn close(&mut self) {
            self.0.push(Command::Close);
        }
    }
}
