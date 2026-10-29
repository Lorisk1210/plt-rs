//! Agg's `vcgen_stroke` with butt caps and miter joins: the closed outline
//! of a stroked sub-path.

use super::stroke_math::{StrokeMath, StrokeShape};
use super::vertex_seq::VertexSequence;
use super::{Generator, accumulate};
use crate::Point;
use crate::path::PathCode;
use crate::path::convert::Vertex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Initial,
    Ready,
    Cap1,
    Cap2,
    Outline1,
    CloseFirst,
    Outline2,
    OutVertices,
    EndPoly1,
    EndPoly2,
    Stop,
}

/// The stroke generator; see [`super::Outline`].
#[derive(Clone, Debug)]
pub(crate) struct Stroke {
    math: StrokeMath,
    src: VertexSequence,
    out: Vec<Point>,
    status: Status,
    prev_status: Status,
    closed: bool,
    src_vertex: usize,
    out_vertex: usize,
    /// The command of the next emitted vertex (a move after each start).
    cmd: PathCode,
}

impl Stroke {
    /// A generator for strokes `width` wide.
    pub(crate) fn new(width: f64) -> Self {
        Self {
            math: StrokeMath::new(width, &StrokeShape::default()),
            src: VertexSequence::default(),
            out: Vec::new(),
            status: Status::Initial,
            prev_status: Status::Initial,
            closed: false,
            src_vertex: 0,
            out_vertex: 0,
            cmd: PathCode::MoveTo,
        }
    }

    /// Emits the computed points, then continues with `next`.
    fn emit_then(&mut self, next: Status) {
        self.prev_status = next;
        self.status = Status::OutVertices;
        self.out_vertex = 0;
    }
}

impl Generator for Stroke {
    fn remove_all(&mut self) {
        self.src.clear();
        self.closed = false;
        self.status = Status::Initial;
    }

    fn add_vertex(&mut self, vertex: Vertex) {
        self.status = Status::Initial;
        accumulate(&mut self.src, &mut self.closed, vertex);
    }

    fn rewind(&mut self) {
        if self.status == Status::Initial {
            self.src.close(self.closed);
            if self.src.len() < 3 {
                self.closed = false;
            }
        }
        self.status = Status::Ready;
        self.src_vertex = 0;
        self.out_vertex = 0;
    }

    fn vertex(&mut self) -> Option<Vertex> {
        loop {
            match self.status {
                Status::Initial => self.rewind(),
                Status::Ready => {
                    if self.src.len() < 2 + usize::from(self.closed) {
                        return None;
                    }
                    self.status = if self.closed {
                        Status::Outline1
                    } else {
                        Status::Cap1
                    };
                    self.cmd = PathCode::MoveTo;
                    self.src_vertex = 0;
                    self.out_vertex = 0;
                }
                Status::Cap1 => {
                    let (v0, v1) = (self.src.get(0), self.src.get(1));
                    self.math.calc_cap(&mut self.out, v0, v1, v0.dist);
                    self.src_vertex = 1;
                    self.emit_then(Status::Outline1);
                }
                Status::Cap2 => {
                    let n = self.src.len();
                    let (v0, v1) = (self.src.get(n - 1), self.src.get(n - 2));
                    self.math.calc_cap(&mut self.out, v0, v1, v1.dist);
                    self.emit_then(Status::Outline2);
                }
                Status::Outline1 => {
                    let end = if self.closed {
                        self.src.len()
                    } else {
                        self.src.len() - 1
                    };
                    if self.src_vertex >= end {
                        if self.closed {
                            self.prev_status = Status::CloseFirst;
                            self.status = Status::EndPoly1;
                        } else {
                            self.status = Status::Cap2;
                        }
                        continue;
                    }
                    let i = self.src_vertex;
                    let (prev, curr, next) = (self.src.prev(i), self.src.get(i), self.src.next(i));
                    self.math
                        .calc_join(&mut self.out, prev, curr, next, prev.dist, curr.dist);
                    self.src_vertex += 1;
                    self.emit_then(Status::Outline1);
                }
                Status::CloseFirst => {
                    self.status = Status::Outline2;
                    self.src_vertex = self.src.len();
                    self.cmd = PathCode::MoveTo;
                }
                Status::Outline2 => {
                    if self.src_vertex <= usize::from(!self.closed) {
                        self.status = Status::EndPoly2;
                        self.prev_status = Status::Stop;
                        continue;
                    }
                    self.src_vertex -= 1;
                    let i = self.src_vertex;
                    let (next, curr, prev) = (self.src.next(i), self.src.get(i), self.src.prev(i));
                    self.math
                        .calc_join(&mut self.out, next, curr, prev, curr.dist, prev.dist);
                    self.emit_then(Status::Outline2);
                }
                Status::OutVertices => {
                    if let Some(&p) = self.out.get(self.out_vertex) {
                        self.out_vertex += 1;
                        let cmd = std::mem::replace(&mut self.cmd, PathCode::LineTo);
                        return Some(Vertex::new(cmd, p));
                    }
                    self.status = self.prev_status;
                }
                Status::EndPoly1 | Status::EndPoly2 => {
                    self.status = self.prev_status;
                    return Some(Vertex::new(PathCode::ClosePoly, Point::ORIGIN));
                }
                Status::Stop => return None,
            }
        }
    }
}
