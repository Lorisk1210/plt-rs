//! Agg's `vcgen_contour`: every sub-path as a closed polygon offset by half
//! the width.

use super::Generator;
use super::join::JoinMath;
use super::vertex_seq::{VertexDist, VertexSequence};
use crate::Point;
use crate::path::PathCode;
use crate::path::convert::Vertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Initial,
    Ready,
    Outline,
    OutVertices,
    EndPoly,
    Stop,
}

/// The contour generator; see [`super::Outline`].
#[derive(Clone, Debug)]
pub(crate) struct Contour {
    math: JoinMath,
    src: VertexSequence,
    out: Vec<Point>,
    status: Status,
    closed: bool,
    src_vertex: usize,
    out_vertex: usize,
    /// The command of the next emitted vertex (a move for the first one).
    cmd: PathCode,
}

impl Contour {
    /// A generator offsetting by `width / 2` (Agg's `width(w)`).
    pub(crate) fn new(width: f64) -> Self {
        Self {
            math: JoinMath::new(width),
            src: VertexSequence::default(),
            out: Vec::new(),
            status: Status::Initial,
            closed: false,
            src_vertex: 0,
            out_vertex: 0,
            cmd: PathCode::MoveTo,
        }
    }
}

impl Generator for Contour {
    fn remove_all(&mut self) {
        self.src.clear();
        self.closed = false;
        self.status = Status::Initial;
    }

    fn add_vertex(&mut self, vertex: Vertex) {
        self.status = Status::Initial;
        match vertex.code {
            PathCode::MoveTo => self.src.modify_last(VertexDist::new(vertex.point)),
            PathCode::ClosePoly => self.closed = true,
            PathCode::LineTo | PathCode::Curve3 | PathCode::Curve4 => {
                self.src.add(VertexDist::new(vertex.point));
            }
        }
    }

    fn rewind(&mut self) {
        if self.status == Status::Initial {
            self.src.close(true);
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
    }

    fn vertex(&mut self) -> Option<Vertex> {
        loop {
            match self.status {
                Status::Initial => self.rewind(),
                Status::Ready => {
                    if self.src.len() < 2 + usize::from(self.closed) {
                        return None;
                    }
                    self.status = Status::Outline;
                    self.cmd = PathCode::MoveTo;
                    self.src_vertex = 0;
                    self.out_vertex = 0;
                }
                Status::Outline => {
                    if self.src_vertex >= self.src.len() {
                        self.status = Status::EndPoly;
                        continue;
                    }
                    let i = self.src_vertex;
                    let (prev, curr, next) = (self.src.prev(i), self.src.get(i), self.src.next(i));
                    self.math
                        .join(&mut self.out, prev, curr, next, prev.dist, curr.dist);
                    self.src_vertex += 1;
                    self.status = Status::OutVertices;
                    self.out_vertex = 0;
                }
                Status::OutVertices => {
                    if let Some(&p) = self.out.get(self.out_vertex) {
                        self.out_vertex += 1;
                        let cmd = std::mem::replace(&mut self.cmd, PathCode::LineTo);
                        return Some(Vertex::new(cmd, p));
                    }
                    self.status = Status::Outline;
                }
                Status::EndPoly => {
                    if !self.closed {
                        return None;
                    }
                    self.status = Status::Stop;
                    return Some(Vertex::new(PathCode::ClosePoly, Point::ORIGIN));
                }
                Status::Stop => return None,
            }
        }
    }
}
