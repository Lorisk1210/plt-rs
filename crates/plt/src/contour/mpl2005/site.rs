//! Tracing the curves of one trace (gcntr's `curve_tracer`,
//! `zone_crosser`, `edge_walker` and `slit_cutter`).
//!
//! A position is a directed edge: the edge index and `left`, the offset
//! to the zone on its left (±1 across an i-edge, ±imax across a j-edge).
//! Curves keep the filled region on their left, cross zones between cut
//! edges, and walk boundary edges anticlockwise around existing zones. A
//! filled curve closing around a hole is joined to the curve around it by
//! a slit: down from the hole to the outer curve and back up.
//!
//! The first pass only counts and decides which start marks remain; the
//! second records the points, each with a kind that tells slit points
//! apart for [`reorder`](super::reorder).

use plt_core::Point;

use super::Mesh;
use super::init::{
    ALL_DONE, ANY_START, I_BNDY, I0_START, I1_START, J_BNDY, J0_START, J1_START, OPEN_END, SLIT_DN,
    SLIT_DN_VISITED, SLIT_UP, START_ROW, Z_VALUE,
};
use crate::contour::quad::slot;

/// A point where a level cuts an edge.
pub(super) const KIND_ZONE: u8 = 0;
/// A boundary point between the levels.
const KIND_EDGE1: u8 = 1;
/// The first point of a curve starting on a boundary edge.
const KIND_EDGE2: u8 = 2;
/// A point on the upstroke of a slit.
pub(super) const KIND_SLIT_UP: u8 = 3;
/// A point on the downstroke of a slit.
const KIND_SLIT_DOWN: u8 = 4;
/// Added to the kind of the point a slit starts after.
const KIND_START_SLIT: u8 = 16;

/// The centre of a saddle zone has been compared with the levels.
const SADDLE_SET: u8 = 0x01;
/// The centre is above the lower level.
const SADDLE_GT0: u8 = 0x02;
/// The centre is above the upper level.
const SADDLE_GT1: u8 = 0x04;

/// What the tracer does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// Cross a zone along the lower (`false`) or upper level.
    Cross(bool),
    /// Walk a boundary edge.
    Walk,
    /// The curve closed.
    Closed,
    /// The curve ended: on a boundary (lines) or at a slit (first pass).
    Ended,
}

/// The direction of travel along an edge with the zone `left` on its
/// left.
fn forward(left: isize, imax: isize) -> isize {
    match left {
        l if l > 1 => 1,
        1 => -imax,
        l if l < -1 => -1,
        _ => imax,
    }
}

/// The first point of a directed edge.
fn point0(edge: isize, fwd: isize) -> isize {
    edge - fwd.max(0)
}

/// The second point of a directed edge.
fn point1(edge: isize, fwd: isize) -> isize {
    edge + fwd.min(0)
}

/// Whether a directed edge is a j-edge.
fn is_jedge(left: isize) -> bool {
    !(-1..=1).contains(&left)
}

/// The start mark of a directed edge.
fn start_mark(left: isize) -> u16 {
    match left {
        l if l > 1 => J1_START,
        1 => I1_START,
        l if l < -1 => J0_START,
        _ => I0_START,
    }
}

/// The boundary flag of an i-edge or j-edge.
fn boundary(jedge: bool) -> u16 {
    if jedge { J_BNDY } else { I_BNDY }
}

/// The state of a trace (gcntr's `Csite`).
#[derive(Debug)]
pub(super) struct Site<'a> {
    mesh: &'a Mesh,
    imax: isize,
    data: &'a mut [u16],
    saddle: &'a mut [u8],
    /// The levels; equal for lines.
    zlevel: [f64; 2],
    pass2: bool,
    /// The current directed edge.
    edge: isize,
    left: isize,
    /// Points on the curve so far.
    n: usize,
    /// Start marks not yet visited (first pass).
    count: isize,
    /// Where and on which level the curve started, for closure.
    edge0: isize,
    left0: isize,
    level0: u8,
    /// The previous start, for `START_ROW` marks.
    edge00: isize,
    /// The points of the curve and their kinds (second pass).
    pub(super) points: Vec<Point>,
    pub(super) kinds: Vec<u8>,
}

impl<'a> Site<'a> {
    /// A trace of `mesh` at `zlevel` over its classified `data`, with
    /// `count` start marks.
    pub(super) fn new(
        mesh: &'a Mesh,
        data: &'a mut [u16],
        saddle: &'a mut [u8],
        zlevel: [f64; 2],
        count: isize,
    ) -> Self {
        Self {
            mesh,
            imax: mesh.imax,
            data,
            saddle,
            zlevel,
            pass2: false,
            edge: 0,
            left: 0,
            n: 0,
            count,
            edge0: 0,
            left0: 0,
            level0: 0,
            edge00: 0,
            points: Vec::new(),
            kinds: Vec::new(),
        }
    }

    /// Starts the second pass, which records the points.
    pub(super) fn start_second_pass(&mut self) {
        self.pass2 = true;
    }

    fn two_levels(&self) -> bool {
        self.zlevel[1] > self.zlevel[0]
    }

    fn at(&self, index: isize) -> u16 {
        self.data[slot(index)]
    }

    fn has(&self, index: isize, flags: u16) -> bool {
        self.at(index) & flags != 0
    }

    fn set(&mut self, index: isize, flags: u16) {
        self.data[slot(index)] |= flags;
    }

    /// Clears a start mark if set, counting it visited.
    fn clear_start(&mut self, index: isize, mark: u16) {
        if self.has(index, mark) {
            self.data[slot(index)] &= !mark;
            self.count -= 1;
        }
    }

    fn z_value(&self, index: isize) -> u16 {
        self.at(index) & Z_VALUE
    }

    /// Records a point and returns its index on the curve (second pass).
    fn mark(&mut self, point: Point, kind: u8) -> usize {
        self.points.push(point);
        self.kinds.push(kind);
        self.n
    }

    /// Records the point where `level` cuts the edge from `p0` to `p1`.
    fn mark_cut(&mut self, p0: isize, p1: isize, level: f64, kind: u8) -> usize {
        let (mesh, i0, i1) = (self.mesh, slot(p0), slot(p1));
        let zcp = (level - mesh.z[i0]) / (mesh.z[i1] - mesh.z[i0]);
        let point = Point::new(
            zcp * (mesh.x[i1] - mesh.x[i0]) + mesh.x[i0],
            zcp * (mesh.y[i1] - mesh.y[i0]) + mesh.y[i0],
        );
        self.mark(point, kind)
    }

    /// Records a mesh point.
    fn mark_point(&mut self, p: isize, kind: u8) -> usize {
        let point = Point::new(self.mesh.x[slot(p)], self.mesh.y[slot(p)]);
        self.mark(point, kind)
    }

    /// Marks the point at `n_kind` (unless it is the first) as the one a
    /// slit starts after.
    fn mark_slit_start(&mut self, n_kind: usize) {
        if self.pass2 && n_kind != 0 {
            self.kinds[n_kind] += KIND_START_SLIT;
        }
    }

    /// Crosses zones along a level from a cut edge until the curve reaches
    /// a boundary, closes or meets a slit. On the first pass, clears the
    /// start marks of the i-edges it passes.
    fn zone_crosser(&mut self, upper: bool) -> Step {
        let imax = self.imax;
        let (mut edge, mut left, mut n) = (self.edge, self.left, self.n);
        let mut fwd = forward(left, imax);
        let mut jedge = is_jedge(left);
        let (edge0, left0) = (self.edge0, self.left0);
        let level0 = self.level0 == u8::from(upper);
        let two_levels = self.two_levels();
        let zlevel = self.zlevel[usize::from(upper)];
        let level: u16 = if upper { 2 } else { 0 };
        let mut done = 0;
        let mut n_kind;
        loop {
            n_kind = 0;
            let (p0, p1) = (point0(edge, fwd), point1(edge, fwd));
            if self.pass2 {
                n_kind = self.mark_cut(p0, p1, zlevel, KIND_ZONE);
            }
            if done == 0 && !jedge && n > 0 {
                if !two_levels && !self.pass2 && self.has(edge, OPEN_END) {
                    done = 4;
                    break;
                }
                if edge == edge0 && left == left0 {
                    // May close on a downstroke.
                    if level0 {
                        done = if !self.pass2 && two_levels && left < 0 {
                            5
                        } else {
                            3
                        };
                    }
                } else if !self.pass2 {
                    self.clear_start(edge, if fwd > 0 { I0_START } else { I1_START });
                    if !two_levels {
                        self.clear_start(edge, if fwd > 0 { I1_START } else { I0_START });
                    }
                }
            }
            n += 1;
            self.n = n;
            if done != 0 {
                break;
            }
            // Cross the zone to another cut edge.
            let z0 = self.z_value(p0) != level;
            let z1 = !z0;
            let z2 = self.z_value(p1 + left) != level;
            let z3 = self.z_value(p0 + left) != level;
            let backward = if z0 == z2 {
                z1 == z3 && !self.saddle_turns_right(edge, left, p0, p1, level, z1)
            } else {
                z1 == z3
            };
            if backward {
                // Bend left along the curve.
                jedge = !jedge;
                edge = p0 + left.max(0);
                (fwd, left) = (left, -fwd);
            } else if z0 == z2 {
                // Bend right along the curve.
                jedge = !jedge;
                edge = p1 + left.max(0);
                (fwd, left) = (-left, fwd);
            } else {
                // Straight across to the opposite edge.
                edge += left;
            }
            // A slit downstroke, checked after the step.
            if self.pass2 && two_levels && !jedge {
                if left > 0 {
                    if self.has(edge, SLIT_UP) {
                        done = 6;
                    }
                } else if self.has(edge, SLIT_DN) {
                    done = 5;
                }
            }
            if done == 0 && self.has(edge, boundary(jedge)) {
                done = if two_levels { 2 } else { 4 };
                // Flip back into the zone that exists.
                left = -left;
                fwd = -fwd;
                if !self.pass2 && (edge != edge0 || left != left0) {
                    self.clear_start(edge, start_mark(left));
                }
            }
        }
        self.edge = edge;
        self.n = n;
        self.left = left;
        match done {
            2 => Step::Walk,
            3 => Step::Closed,
            4 => Step::Ended,
            _ => {
                self.mark_slit_start(n_kind);
                self.slit_cutter(done == 6)
            }
        }
    }

    /// Whether a contour crossing the saddle zone left of `edge` turns
    /// right: by the mean of its corners, compared with the levels once.
    fn saddle_turns_right(
        &mut self,
        edge: isize,
        left: isize,
        p0: isize,
        p1: isize,
        level: u16,
        z1: bool,
    ) -> bool {
        let zone = slot(edge + left.max(0));
        if self.saddle[zone] & SADDLE_SET == 0 {
            let z = |p: isize| self.mesh.z[slot(p)];
            let centre = (z(p0) + z(p0 + left) + z(p1) + z(p1 + left)) / 4.0;
            let mut saddle = SADDLE_SET;
            if centre > self.zlevel[0] {
                saddle |= if self.two_levels() && centre > self.zlevel[1] {
                    SADDLE_GT0 | SADDLE_GT1
                } else {
                    SADDLE_GT0
                };
            }
            self.saddle[zone] = saddle;
        }
        let gt = if level == 2 { SADDLE_GT1 } else { SADDLE_GT0 };
        (self.saddle[zone] & gt != 0) != (z1 != (level == 2))
    }

    /// Walks boundary edges anticlockwise while their points lie between
    /// the levels (filled contours only).
    fn edge_walker(&mut self) -> Step {
        let imax = self.imax;
        let (mut edge, mut left, mut n) = (self.edge, self.left, self.n);
        let mut fwd = forward(left, imax);
        let (mut p0, mut p1) = (point0(edge, fwd), point1(edge, fwd));
        let mut jedge = is_jedge(left);
        let (edge0, left0) = (self.edge0, self.left0);
        let level0 = self.level0 == 2;
        let mut heads_up = false;
        loop {
            let (z0, z1) = (self.z_value(p0), self.z_value(p1));
            let mut marked = false;
            let mut n_kind = 0;
            if z0 == 1 {
                if self.pass2 {
                    n_kind = self.mark_point(p0, KIND_EDGE1);
                }
                marked = true;
            } else if n == 0 {
                // The first point is not between the levels: mark the cut
                // here, which the zone crosser marks again on closing.
                if self.pass2 {
                    let level = self.zlevel[usize::from(z0 != 0)];
                    n_kind = self.mark_cut(p0, p1, level, KIND_EDGE2);
                }
                marked = true;
            }
            if n > 0 {
                if level0 && edge == edge0 && left == left0 {
                    self.edge = edge;
                    self.left = left;
                    self.n = n + usize::from(marked);
                    // Closing on a hole needs a downward slit.
                    if fwd < 0 && !self.has(edge, boundary(jedge)) {
                        self.mark_slit_start(n_kind);
                        return self.slit_cutter(false);
                    }
                    if fwd < 0 && left < 0 {
                        // The upward slit from the line below includes
                        // this boundary.
                        self.data[slot(edge)] &= !J0_START;
                        self.mark_slit_start(n_kind);
                        return self.slit_cutter(false);
                    }
                    return Step::Closed;
                } else if self.pass2 {
                    if heads_up || (fwd < 0 && self.has(edge, SLIT_DN)) {
                        if !heads_up && !self.has(edge, SLIT_DN_VISITED) {
                            self.set(edge, SLIT_DN_VISITED);
                        } else {
                            self.edge = edge;
                            self.left = left;
                            self.n = n + usize::from(marked);
                            self.mark_slit_start(n_kind);
                            return self.slit_cutter(heads_up);
                        }
                    }
                } else {
                    self.clear_start(edge, start_mark(left));
                }
            }
            if marked {
                n += 1;
                self.n = n;
            }
            // Back to crossing zones once the next point is off the band.
            if z1 != 1 {
                self.edge = edge;
                self.left = left;
                self.n = n;
                return Step::Cross(z1 != 0);
            }
            // Step to p1 and turn left if possible, else go straight, else
            // turn right, looking for the start of an upward slit.
            edge = p1 + left.max(0);
            if self.pass2 && jedge && fwd > 0 && self.has(edge, SLIT_UP) {
                jedge = !jedge;
                heads_up = true;
            } else if self.has(edge, boundary(!jedge)) {
                (fwd, left) = (left, -fwd);
                jedge = !jedge;
            } else {
                edge = p1 + fwd.max(0);
                if self.pass2 && !jedge && fwd > 0 && self.has(edge, SLIT_UP) {
                    heads_up = true;
                } else if !self.has(edge, boundary(jedge)) {
                    edge = p1 - left.min(0);
                    jedge = !jedge;
                    (fwd, left) = (-left, fwd);
                }
            }
            p0 = p1;
            p1 = point1(edge, fwd);
        }
    }

    /// Runs a slit up its left side (`up`, second pass only) or down its
    /// right side until it reaches a boundary or a point off the band.
    fn slit_cutter(&mut self, up: bool) -> Step {
        let imax = self.imax;
        let mut n = self.n;
        if up && self.pass2 {
            let mut p1 = self.edge;
            loop {
                let z1 = self.z_value(p1);
                if z1 != 1 {
                    (self.edge, self.left, self.n) = (p1, -1, n);
                    return Step::Cross(z1 != 0);
                }
                if self.has(p1, J_BNDY) {
                    // Closing on a mesh hole, a very unusual case.
                    (self.edge, self.left, self.n) = (p1, -imax, n);
                    return Step::Walk;
                }
                self.mark_point(p1, KIND_SLIT_UP);
                n += 1;
                self.n = n;
                p1 += imax;
            }
        }
        let mut p0 = self.edge;
        self.set(p0, SLIT_DN);
        p0 -= imax;
        loop {
            let z0 = self.z_value(p0);
            if !self.pass2 {
                if z0 != 1 || self.has(p0, I_BNDY) || self.has(p0 + 1, J_BNDY) {
                    self.set(p0 + imax, SLIT_UP);
                    // One more point to splice into the outer curve.
                    self.n = n + 1;
                    return Step::Ended;
                }
            } else if z0 != 1 {
                (self.edge, self.left, self.n) = (p0 + imax, 1, n);
                return Step::Cross(z0 != 0);
            } else if self.has(p0 + 1, J_BNDY) {
                (self.edge, self.left, self.n) = (p0 + 1, imax, n);
                return Step::Walk;
            } else if self.has(p0, I_BNDY) {
                (self.edge, self.left, self.n) = (p0, 1, n);
                return Step::Walk;
            }
            if self.pass2 {
                self.mark_point(p0, KIND_SLIT_DOWN);
                n += 1;
                self.n = n;
            } else {
                // The first pass counts the upstroke as well.
                n += 2;
            }
            p0 -= imax;
        }
    }

    /// Finds the next start: the directed edge and the level to set off
    /// on (0 or 1: crossing a zone, 2: walking a boundary), and whether it
    /// is a second start on the previous edge. `None` when there are no
    /// more curves.
    fn next_start(&mut self) -> Option<Start> {
        let imax = self.imax;
        let (mut edge, left0, edge00) = (self.edge0, self.left0, self.edge00);
        // An edge may start several curves: take them in the order i1,
        // i0, j1, j0.
        let two_starts = match left0 {
            1 => self.at(edge) & (I0_START | J1_START | J0_START),
            -1 => self.at(edge) & (J1_START | J0_START),
            l if l == imax => self.at(edge) & J0_START,
            _ => 0,
        } != 0;
        if self.pass2 || edge == 0 {
            if !two_starts {
                // Up to the row the first pass marked.
                let first = edge == 0 && !self.pass2;
                let e0 = edge;
                if self.has(edge, ALL_DONE) {
                    return None;
                }
                while !self.has(edge, START_ROW) {
                    edge += imax;
                }
                if e0 == edge {
                    edge += 1;
                }
                if first {
                    // The very first start: remove data_init's mark.
                    self.data[slot(edge - edge % imax)] &= !START_ROW;
                }
            }
        } else {
            if self.count <= 0 {
                // Mark the end for the second pass and start it over.
                self.set(edge00, ALL_DONE);
                (self.edge0, self.edge00, self.left0) = (0, 0, 0);
                return None;
            }
            if !two_starts {
                edge += 1;
            }
        }
        let (left, level) = if two_starts {
            if left0 == 1 && self.has(edge, I0_START) {
                (-1, if self.has(edge, I_BNDY) { 2 } else { 0 })
            } else if (left0 == 1 || left0 == -1) && self.has(edge, J1_START) {
                (imax, 2)
            } else {
                (-imax, 2)
            }
        } else {
            while !self.has(edge, ANY_START) {
                edge += 1;
            }
            let left = if self.has(edge, I1_START) {
                1
            } else if self.has(edge, I0_START) {
                -1
            } else if self.has(edge, J1_START) {
                imax
            } else {
                -imax
            };
            let level = if !self.has(edge, I1_START | I0_START) || self.has(edge, I_BNDY) {
                2
            } else {
                0
            };
            (left, level)
        };
        Some(Start {
            edge,
            left,
            level,
            two_starts,
        })
    }

    /// Finds the next start and traces its curve. Returns the number of
    /// points on it, 0 when there are no more curves, or minus the count
    /// on the first pass for a curve the second pass traces as part of
    /// another (a hole, or an open line found from its wrong end).
    pub(super) fn curve_tracer(&mut self) -> isize {
        let Some(Start {
            edge: edge0,
            left: left0,
            mut level,
            two_starts,
        }) = self.next_start()
        else {
            return 0;
        };
        let (imax, edge00, two_levels) = (self.imax, self.edge00, self.two_levels());
        if !self.pass2 {
            self.count -= 1;
        }
        // A curve starting on an interior i-edge starts on the level it
        // crosses.
        if level == 0 && two_levels {
            let below = if left0 > 0 { edge0 - imax } else { edge0 };
            level = u8::from(self.z_value(below) != 0);
        }
        (self.edge, self.edge0) = (edge0, edge0);
        (self.left, self.left0) = (left0, left0);
        self.level0 = level;
        self.n = 0;
        let mut step = match level {
            _ if !two_levels => Step::Cross(false),
            0 => Step::Cross(false),
            1 => Step::Cross(true),
            _ => Step::Walk,
        };
        loop {
            step = match step {
                Step::Cross(upper) => self.zone_crosser(upper),
                Step::Walk => self.edge_walker(),
                Step::Closed | Step::Ended => break,
            };
        }
        let mut n = isize::try_from(self.n).expect("curves fit in memory");
        // An open line, or a filled curve closed by a slit: the second pass
        // traces it from elsewhere, so drop its start and its row mark.
        let mark_row = if !self.pass2 && step == Step::Ended && (two_levels || level == 0) {
            if !two_levels {
                self.set(edge0, OPEN_END);
            }
            self.data[slot(edge0)] &= !(if left0 > 0 { I1_START } else { I0_START });
            n = -n;
            false
        } else {
            !two_levels || !two_starts
        };
        if !self.pass2 && mark_row {
            self.set(edge0 - (edge0 - edge00) % imax, START_ROW);
            self.edge00 = edge0;
        }
        n
    }
}

/// Where a curve starts (see [`Site::next_start`]).
#[derive(Clone, Copy, Debug)]
struct Start {
    edge: isize,
    left: isize,
    level: u8,
    two_starts: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directed_edges() {
        let imax = 5;
        // Across an i-edge with the zone on the right, travel is down.
        assert_eq!(forward(1, imax), -imax);
        assert_eq!(forward(-1, imax), imax);
        assert_eq!(forward(imax, imax), 1);
        assert_eq!(forward(-imax, imax), -1);
        assert_eq!((point0(12, 1), point1(12, 1)), (11, 12));
        assert_eq!((point0(12, -imax), point1(12, -imax)), (12, 7));
        assert!(is_jedge(imax) && !is_jedge(-1));
        assert_eq!(start_mark(-imax), J0_START);
    }
}
