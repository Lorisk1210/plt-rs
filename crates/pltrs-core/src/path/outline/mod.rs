//! Agg's outline generators as used by matplotlib's containment tests:
//! `conv_contour` (a closed polygon offset by half a width, for
//! `contains_point(radius=...)`) and `conv_stroke` (the outline of a stroke,
//! for picking unfilled collections).
//!
//! Both use Agg's default joins (miter, inner miter, limit 4) and butt caps.
//! The offset side follows the path direction: a positive width grows
//! counter-clockwise polygons and shrinks clockwise ones, exactly as in
//! matplotlib.
//!
//! The building blocks of those generators — Agg's vertex sequence
//! ([`VertexSequence`]), its caps and joins ([`StrokeMath`], with any
//! [`StrokeShape`]) and its `agg_math.h` helpers ([`cross_product`],
//! [`calc_intersection`], [`calc_distance`]) — are public: pltrs-raster's
//! stroker and dasher are made of them too.

mod contour;
mod math;
mod stroke;
mod stroke_math;
mod vertex_seq;

use crate::Point;
use crate::path::PathCode;
use crate::path::convert::{Vertex, VertexSource};

pub(crate) use contour::Contour;
pub use math::{INTERSECTION_EPSILON, calc_distance, calc_intersection, cross_product};
pub(crate) use stroke::Stroke;
pub use stroke_math::{InnerJoin, LineCap, LineJoin, StrokeMath, StrokeShape};
pub use vertex_seq::{VERTEX_DIST_EPSILON, VertexDist, VertexSequence};

/// A per-sub-path outline generator (Agg's `vcgen_*` interface).
pub(crate) trait Generator {
    /// Discards the accumulated sub-path.
    fn remove_all(&mut self);
    /// Adds a vertex (moves replace the previous start; closures mark the
    /// sub-path closed).
    fn add_vertex(&mut self, vertex: Vertex);
    /// Prepares the output of the accumulated sub-path.
    fn rewind(&mut self);
    /// The next generated vertex; `None` ends the sub-path's output.
    fn vertex(&mut self) -> Option<Vertex>;
}

/// Agg's `add_vertex` of the outline generators: a move replaces the last
/// vertex of the sub-path `src`, a closure marks it `closed` and the other
/// commands append their point.
fn accumulate(src: &mut VertexSequence, closed: &mut bool, vertex: Vertex) {
    match vertex.code {
        PathCode::MoveTo => src.modify_last(VertexDist::new(vertex.point)),
        PathCode::ClosePoly => *closed = true,
        PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => {
            src.add(VertexDist::new(vertex.point));
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Initial,
    Accumulate,
    Generate,
}

/// Agg's `conv_adaptor_vcgen`: feeds a generator one sub-path at a time.
///
/// As in Agg, a sub-path ended by a closure is followed by one that starts
/// again at the previous move-to point unless a new move comes first.
#[derive(Clone, Debug)]
pub(crate) struct Outline<I, G> {
    source: I,
    generator: G,
    status: Status,
    /// `false` once the source reported its end.
    running: bool,
    start: Point,
}

impl<I: VertexSource, G: Generator> Outline<I, G> {
    pub(crate) const fn new(source: I, generator: G) -> Self {
        Self {
            source,
            generator,
            status: Status::Initial,
            running: false,
            start: Point::ORIGIN,
        }
    }

    /// Reads one sub-path into the generator.
    fn accumulate(&mut self) {
        self.generator.remove_all();
        self.generator.add_vertex(Vertex::move_to(self.start));
        loop {
            let Some(vertex) = self.source.next() else {
                self.running = false;
                break;
            };
            match vertex.code {
                PathCode::MoveTo => {
                    self.start = vertex.point;
                    break;
                }
                PathCode::ClosePoly => {
                    self.generator.add_vertex(vertex);
                    break;
                }
                PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => {
                    self.generator.add_vertex(vertex);
                }
            }
        }
        self.generator.rewind();
    }
}

impl<I: VertexSource, G: Generator> Iterator for Outline<I, G> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        loop {
            match self.status {
                Status::Initial => {
                    // The first vertex starts a sub-path, whatever its code.
                    let first = self.source.next();
                    self.running = first.is_some();
                    self.start = first.map_or(Point::ORIGIN, |v| v.point);
                    self.status = Status::Accumulate;
                }
                Status::Accumulate => {
                    if !self.running {
                        return None;
                    }
                    self.accumulate();
                    self.status = Status::Generate;
                }
                Status::Generate => match self.generator.vertex() {
                    Some(vertex) => return Some(vertex),
                    None => self.status = Status::Accumulate,
                },
            }
        }
    }
}

impl<I: VertexSource, G: Generator> VertexSource for Outline<I, G> {
    fn rewind(&mut self) {
        self.source.rewind();
        self.status = Status::Initial;
    }
}
