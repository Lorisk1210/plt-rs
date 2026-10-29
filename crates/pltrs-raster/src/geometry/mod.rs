//! The vertex pipeline in front of the rasterizer: dashing and stroking.
//!
//! Agg chains *pull*-based vertex sources (`conv_curve → conv_dash →
//! conv_stroke → rasterizer`). Curves are flattened by pltrs-core's port of
//! `conv_curve`; after that every stage here is a *push*-based
//! [`PathSink`] that forwards its output to the next stage, keeping Agg's
//! exact per-sub-path semantics.

pub(crate) mod dash;
pub(crate) mod stroke;

use pltrs_core::path::convert::{Vertex, VertexSource};
use pltrs_core::{Path, PathCode, Point};

/// A consumer of flattened path commands (straight segments only).
pub trait PathSink {
    /// Starts a new sub-path at `p`.
    fn move_to(&mut self, p: Point);
    /// Adds a straight segment from the current point to `p`.
    fn line_to(&mut self, p: Point);
    /// Closes the current sub-path.
    fn close(&mut self);
    /// Returns `true` once further input cannot change the outcome (the
    /// rasterizer exceeded its cell limit), so producers may stop early.
    fn is_saturated(&self) -> bool {
        false
    }
}

/// Sends a flattened vertex stream to `sink`, stopping early once the sink
/// is saturated. Curve vertices (absent from flattened streams) are treated
/// as line vertices.
pub(crate) fn feed_vertices<S: PathSink + ?Sized>(
    vertices: impl IntoIterator<Item = Vertex>,
    sink: &mut S,
) {
    for vertex in vertices {
        if sink.is_saturated() {
            return;
        }
        match vertex.code {
            PathCode::MoveTo => sink.move_to(vertex.point),
            PathCode::ClosePoly => sink.close(),
            PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => sink.line_to(vertex.point),
        }
    }
}

/// Whether `path` encloses no area: it is made of straight segments whose
/// vertices (closures aside: they only return to a sub-path start) all lie
/// on one vertical or one horizontal line. The contributions such a path
/// makes to each rasterizer cell cancel, so filling it covers no pixel.
#[expect(
    clippy::float_cmp,
    reason = "only exactly equal coordinates make the path flat"
)]
pub(crate) fn is_flat(path: &Path) -> bool {
    let codes = path.codes();
    let mut first = None;
    let (mut vertical, mut horizontal) = (true, true);
    for (i, &p) in path.vertices().iter().enumerate() {
        match codes.map_or(PathCode::LineTo, |codes| codes[i]) {
            PathCode::ClosePoly => continue,
            PathCode::Curve3 | PathCode::Curve4 => return false,
            PathCode::MoveTo | PathCode::LineTo => {}
        }
        let first: Point = *first.get_or_insert(p);
        vertical &= p.x == first.x;
        horizontal &= p.y == first.y;
        if !(vertical || horizontal) {
            return false;
        }
    }
    true
}

/// Sends `path` to `sink`, flattening its curves exactly like Agg's
/// `conv_curve` (pltrs-core's [`flatten_curves`](VertexSource::flatten_curves)),
/// which passes the vertices of a path without curves through as they are.
pub(crate) fn feed_path<S: PathSink + ?Sized>(path: &Path, sink: &mut S) {
    if path.has_curves() {
        feed_vertices(path.iter_vertices().flatten_curves(true), sink);
    } else {
        feed_vertices(path.iter_vertices(), sink);
    }
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
