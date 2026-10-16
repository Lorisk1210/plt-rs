//! Line clipping to a rectangle (matplotlib's `PathClipper` on top of Agg's
//! `clip_line_segment`).

use super::queue::VertexQueue;
use super::{Vertex, VertexSource};
use crate::path::PathCode;
use crate::{Bbox, Point};

/// Agg's `rect_d`: an ordered-by-convention rectangle `x1 <= x2`, `y1 <= y2`
/// (inverted rectangles clip everything away, as in Agg).
#[derive(Clone, Copy, Debug, PartialEq)]
struct ClipRect {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
}

impl ClipRect {
    /// `rect` grown by `pad` on every side.
    fn padded(rect: &Bbox, pad: f64) -> Self {
        Self {
            x1: rect.x0() - pad,
            y1: rect.y0() - pad,
            x2: rect.x1() + pad,
            y2: rect.y1() + pad,
        }
    }

    fn contains(&self, p: Point) -> bool {
        p.x >= self.x1 && p.x <= self.x2 && p.y >= self.y1 && p.y <= self.y2
    }

    /// Agg's Cohen–Sutherland style outcode: bit 0 right, 1 above, 2 left,
    /// 3 below.
    fn flags(&self, p: Point) -> u8 {
        u8::from(p.x > self.x2)
            | (u8::from(p.y > self.y2) << 1)
            | (u8::from(p.x < self.x1) << 2)
            | (u8::from(p.y < self.y1) << 3)
    }
}

const X_CLIPPED: u8 = 0b0101;
const Y_CLIPPED: u8 = 0b1010;
const X1_CLIPPED: u8 = 0b0100;
const Y1_CLIPPED: u8 = 0b1000;

/// Result bits of [`clip_line_segment`].
const FIRST_MOVED: u8 = 1;
const SECOND_MOVED: u8 = 2;
const FULLY_CLIPPED: u8 = 4;

/// Agg's `clip_move_point`: slides `p` along `a → b` onto the violated
/// boundary; `false` if the segment is parallel to it.
#[expect(
    clippy::float_cmp,
    reason = "Agg tests for exactly axis-parallel segments"
)]
fn clip_move_point(a: Point, b: Point, rect: &ClipRect, p: &mut Point, flags: u8) -> bool {
    if flags & X_CLIPPED != 0 {
        if a.x == b.x {
            return false;
        }
        let bound = if flags & X1_CLIPPED != 0 {
            rect.x1
        } else {
            rect.x2
        };
        p.y = (bound - a.x) * (b.y - a.y) / (b.x - a.x) + a.y;
        p.x = bound;
    }
    let flags = rect.flags(*p) & Y_CLIPPED;
    if flags != 0 {
        if a.y == b.y {
            return false;
        }
        let bound = if flags & Y1_CLIPPED != 0 {
            rect.y1
        } else {
            rect.y2
        };
        p.x = (bound - a.y) * (b.x - a.x) / (b.y - a.y) + a.x;
        p.y = bound;
    }
    true
}

/// Agg's `clip_line_segment`: clips `p1 → p2` in place. The result is
/// `FULLY_CLIPPED` when nothing remains, otherwise a combination of
/// `FIRST_MOVED` and `SECOND_MOVED`.
fn clip_line_segment(p1: &mut Point, p2: &mut Point, rect: &ClipRect) -> u8 {
    let f1 = rect.flags(*p1);
    let f2 = rect.flags(*p2);
    if f1 | f2 == 0 {
        return 0;
    }
    if (f1 & X_CLIPPED != 0 && f1 & X_CLIPPED == f2 & X_CLIPPED)
        || (f1 & Y_CLIPPED != 0 && f1 & Y_CLIPPED == f2 & Y_CLIPPED)
    {
        return FULLY_CLIPPED;
    }
    let (a, b) = (*p1, *p2);
    let mut moved = 0;
    if f1 != 0 {
        if !clip_move_point(a, b, rect, p1, f1) || *p1 == *p2 {
            return FULLY_CLIPPED;
        }
        moved |= FIRST_MOVED;
    }
    if f2 != 0 {
        if !clip_move_point(a, b, rect, p2, f2) || *p1 == *p2 {
            return FULLY_CLIPPED;
        }
        moved |= SECOND_MOVED;
    }
    moved
}

/// Clips line segments to a rectangle; see [`VertexSource::clip_to`].
///
/// The clip box is the given rectangle grown by one unit on every side
/// (matplotlib pads so that strokes along the canvas edge stay visible).
/// Curves are never clipped but always passed through whole. A closed
/// sub-path keeps its closure only if no segment of it was clipped.
#[derive(Clone, Debug)]
pub struct Clipper<I> {
    source: I,
    rect: Option<ClipRect>,
    queue: VertexQueue<3>,
    last: Point,
    init: Point,
    moveto: bool,
    has_init: bool,
    was_clipped: bool,
}

impl<I: VertexSource> Clipper<I> {
    pub(super) fn new(source: I, rect: Option<Bbox>) -> Self {
        Self {
            source,
            rect: rect.map(|r| ClipRect::padded(&r, 1.0)),
            queue: VertexQueue::default(),
            last: Point::new(f64::NAN, f64::NAN),
            init: Point::new(f64::NAN, f64::NAN),
            moveto: true,
            has_init: false,
            was_clipped: false,
        }
    }

    /// Queues the visible part of `a → b`; `true` if anything was queued.
    fn draw_clipped_line(
        &mut self,
        rect: &ClipRect,
        mut a: Point,
        mut b: Point,
        closed: bool,
    ) -> bool {
        let moved = clip_line_segment(&mut a, &mut b, rect);
        self.was_clipped |= moved != 0;
        if moved >= FULLY_CLIPPED {
            return false;
        }
        if moved & FIRST_MOVED != 0 || self.moveto {
            self.queue.push(Vertex::move_to(a));
        }
        self.queue.push(Vertex::line_to(b));
        if closed && !self.was_clipped {
            // Close the path only if its end point has not moved.
            self.queue.push_new(PathCode::ClosePoly, b);
        }
        self.moveto = false;
        true
    }

    /// Whether a pending move-to should be emitted on its own.
    fn pending_move_visible(&self, rect: &ClipRect) -> bool {
        self.moveto && self.has_init && rect.contains(self.last)
    }

    fn next_clipped(&mut self, rect: &ClipRect) -> Option<Vertex> {
        if let Some(vertex) = self.queue.pop() {
            return Some(vertex);
        }
        while let Some(vertex) = self.source.next() {
            match vertex.code {
                PathCode::ClosePoly => {
                    if self.has_init {
                        self.draw_clipped_line(rect, self.last, self.init, true);
                    } else {
                        // A closure before any move: pass it on.
                        self.queue.push_new(PathCode::ClosePoly, self.last);
                    }
                    if !self.queue.is_empty() {
                        break;
                    }
                }
                PathCode::MoveTo => {
                    let emit = self.pending_move_visible(rect);
                    if emit {
                        self.queue.push(Vertex::move_to(self.last));
                    }
                    self.init = vertex.point;
                    self.last = vertex.point;
                    self.has_init = true;
                    self.moveto = true;
                    self.was_clipped = false;
                    if emit {
                        break;
                    }
                }
                PathCode::LineTo => {
                    let drawn = self.draw_clipped_line(rect, self.last, vertex.point, false);
                    self.last = vertex.point;
                    if drawn {
                        break;
                    }
                }
                PathCode::Curve3 | PathCode::Curve4 => {
                    if self.moveto {
                        self.queue.push(Vertex::move_to(self.last));
                        self.moveto = false;
                    }
                    self.queue.push(vertex);
                    self.last = vertex.point;
                    break;
                }
            }
        }
        if let Some(vertex) = self.queue.pop() {
            return Some(vertex);
        }
        if self.pending_move_visible(rect) {
            self.moveto = false;
            return Some(Vertex::move_to(self.last));
        }
        None
    }
}

impl<I: VertexSource> Iterator for Clipper<I> {
    type Item = Vertex;

    fn next(&mut self) -> Option<Vertex> {
        match self.rect {
            Some(rect) => self.next_clipped(&rect),
            None => self.source.next(),
        }
    }
}

impl<I: VertexSource> VertexSource for Clipper<I> {
    fn rewind(&mut self) {
        self.has_init = false;
        self.was_clipped = false;
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

    fn rect(x1: f64, y1: f64, x2: f64, y2: f64) -> ClipRect {
        ClipRect { x1, y1, x2, y2 }
    }

    #[test]
    fn segment_clipping_reports_moved_ends() {
        let r = rect(0.0, 0.0, 10.0, 10.0);
        let (mut a, mut b) = (p(-5.0, 5.0), p(5.0, 5.0));
        assert_eq!(clip_line_segment(&mut a, &mut b, &r), FIRST_MOVED);
        assert_eq!((a, b), (p(0.0, 5.0), p(5.0, 5.0)));
        let (mut a, mut b) = (p(-5.0, -5.0), p(15.0, 15.0));
        assert_eq!(
            clip_line_segment(&mut a, &mut b, &r),
            FIRST_MOVED | SECOND_MOVED
        );
        assert_eq!((a, b), (p(0.0, 0.0), p(10.0, 10.0)));
        let (mut a, mut b) = (p(-5.0, 1.0), p(-1.0, 9.0));
        assert_eq!(clip_line_segment(&mut a, &mut b, &r), FULLY_CLIPPED);
        let (mut a, mut b) = (p(1.0, 1.0), p(2.0, 2.0));
        assert_eq!(clip_line_segment(&mut a, &mut b, &r), 0);
    }

    #[test]
    fn long_line_is_clipped_to_padded_box() {
        let path = Path::polyline(vec![p(-100.0, 5.0), p(100.0, 5.0)]);
        let out: Vec<Vertex> = path
            .iter_vertices()
            .clip_to(Some(Bbox::from_extents(0.0, 0.0, 10.0, 10.0)))
            .collect();
        assert_eq!(
            out,
            vec![Vertex::move_to(p(-1.0, 5.0)), Vertex::line_to(p(11.0, 5.0))]
        );
    }

    #[test]
    fn closure_survives_only_when_unclipped() {
        let square = Path::polygon([p(1.0, 1.0), p(2.0, 1.0), p(2.0, 2.0)]);
        let codes = |r: Bbox| -> Vec<PathCode> {
            square
                .iter_vertices()
                .clip_to(Some(r))
                .map(|v| v.code)
                .collect()
        };
        assert_eq!(
            codes(Bbox::from_extents(0.0, 0.0, 10.0, 10.0)).last(),
            Some(&PathCode::ClosePoly)
        );
        assert_eq!(
            codes(Bbox::from_extents(0.0, 0.0, 0.5, 10.0)).last(),
            Some(&PathCode::LineTo)
        );
    }

    #[test]
    fn isolated_visible_moves_are_kept() {
        let path = Path::new(
            vec![p(5.0, 5.0), p(50.0, 50.0), p(60.0, 50.0)],
            vec![PathCode::MoveTo, PathCode::MoveTo, PathCode::LineTo],
        )
        .expect("valid");
        let out: Vec<Vertex> = path
            .iter_vertices()
            .clip_to(Some(Bbox::from_extents(0.0, 0.0, 10.0, 10.0)))
            .collect();
        assert_eq!(out, vec![Vertex::move_to(p(5.0, 5.0))]);
    }
}
