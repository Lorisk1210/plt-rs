//! `IUP`: interpolation of the untouched points of each contour between
//! the touched ones (FreeType's `Ins_IUP` and its IUP worker).

use super::{Exec, POST_IUP};
use crate::fixed::{div_fix, mul_fix};
use crate::hinting::zone::{TAG_TOUCH_X, TAG_TOUCH_Y, Vector, Zone};

/// One axis of the glyph zone's points.
struct Axis<'z> {
    zone: &'z mut Zone,
    x: bool,
}

impl Axis<'_> {
    fn pick(&self, v: Vector) -> i64 {
        if self.x { v.x } else { v.y }
    }

    fn org(&self, point: usize) -> i64 {
        self.pick(self.zone.org[point])
    }

    fn orus(&self, point: usize) -> i64 {
        self.pick(self.zone.orus[point])
    }

    fn cur(&self, point: usize) -> i64 {
        self.pick(self.zone.cur[point])
    }

    fn set_cur(&mut self, point: usize, value: i64) {
        let cur = &mut self.zone.cur[point];
        if self.x {
            cur.x = value;
        } else {
            cur.y = value;
        }
    }

    /// Shifts the points `first..=last` except `touched` by the
    /// displacement of `touched` (`iup_worker_shift_`).
    fn shift(&mut self, first: usize, last: usize, touched: usize) {
        let delta = self.cur(touched).wrapping_sub(self.org(touched));
        if delta == 0 {
            return;
        }
        for point in (first..touched).chain(touched + 1..=last) {
            let moved = self.cur(point).wrapping_add(delta);
            self.set_cur(point, moved);
        }
    }

    /// Interpolates the points `first..=last` between the touched points
    /// `ref1` and `ref2` (`iup_worker_interpolate_`).
    fn interpolate(&mut self, first: usize, last: usize, ref1: usize, ref2: usize) {
        let len = self.zone.len();
        if first > last || ref1 >= len || ref2 >= len {
            return;
        }
        let (ref1, ref2) = if self.orus(ref1) > self.orus(ref2) {
            (ref2, ref1)
        } else {
            (ref1, ref2)
        };
        let (orus1, orus2) = (self.orus(ref1), self.orus(ref2));
        let (org1, org2) = (self.org(ref1), self.org(ref2));
        let (cur1, cur2) = (self.cur(ref1), self.cur(ref2));
        let (delta1, delta2) = (cur1.wrapping_sub(org1), cur2.wrapping_sub(org2));
        let scale = (cur1 != cur2 && orus1 != orus2)
            .then(|| div_fix(cur2.wrapping_sub(cur1), orus2.wrapping_sub(orus1)));
        for point in first..=last {
            let org = self.org(point);
            let value = if org <= org1 {
                org.wrapping_add(delta1)
            } else if org >= org2 {
                org.wrapping_add(delta2)
            } else {
                match scale {
                    Some(scale) => {
                        cur1.wrapping_add(mul_fix(self.orus(point).wrapping_sub(orus1), scale))
                    }
                    None => cur1,
                }
            };
            self.set_cur(point, value);
        }
    }
}

impl Exec<'_> {
    /// `IUP[a]`: interpolates untouched points along y (`IUP[y]`) or x
    /// (`IUP[x]`). In backward compatibility mode nothing moves once both
    /// have run.
    pub(super) fn ins_iup(&mut self) {
        let bc = self.backward_compatibility();
        if bc == POST_IUP {
            return;
        }
        if bc != 0 {
            self.state.resources.backward_compatibility |= 1 << (self.opcode & 1);
        }
        if self.pts.contours.is_empty() || !self.charge(self.pts.len() as u64) {
            return;
        }
        let x = self.opcode & 1 != 0;
        let mask = if x { TAG_TOUCH_X } else { TAG_TOUCH_Y };
        let contours = self.pts.contours.clone();
        let len = self.pts.len();
        let mut axis = Axis {
            zone: &mut *self.pts,
            x,
        };
        let mut point = 0;
        for end in contours {
            let end = end.min(len - 1);
            let first = point;
            while point <= end && axis.zone.tags[point] & mask == 0 {
                point += 1;
            }
            if point > end {
                continue;
            }
            let first_touched = point;
            let mut touched = point;
            point += 1;
            while point <= end {
                if axis.zone.tags[point] & mask != 0 {
                    axis.interpolate(touched + 1, point - 1, touched, point);
                    touched = point;
                }
                point += 1;
            }
            if touched == first_touched {
                axis.shift(first, end, touched);
            } else {
                axis.interpolate(touched + 1, end, touched, first_touched);
                if first_touched > 0 {
                    axis.interpolate(first, first_touched - 1, touched, first_touched);
                }
            }
        }
    }
}
