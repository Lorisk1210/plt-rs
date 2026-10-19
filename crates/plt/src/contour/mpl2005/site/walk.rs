//! The three ways a curve advances: crossing zones between cut edges
//! (`zone_crosser`), walking boundary edges between the levels
//! (`edge_walker`) and running along a slit (`slit_cutter`).

use super::{
    KIND_EDGE1, KIND_EDGE2, KIND_SLIT_DOWN, KIND_SLIT_UP, KIND_ZONE, SADDLE_GT0, SADDLE_GT1,
    SADDLE_SET, Site, Step, boundary, forward, is_jedge, point0, point1, start_mark,
};
use crate::contour::mpl2005::init::{
    I_BNDY, I0_START, I1_START, J_BNDY, J0_START, OPEN_END, SLIT_DN, SLIT_DN_VISITED, SLIT_UP,
};
use crate::contour::quad::slot;

impl Site<'_> {
    /// Crosses zones along a level from a cut edge until the curve reaches
    /// a boundary, closes or meets a slit. On the first pass, clears the
    /// start marks of the i-edges it passes.
    pub(super) fn zone_crosser(&mut self, upper: bool) -> Step {
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
    pub(super) fn edge_walker(&mut self) -> Step {
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
}
