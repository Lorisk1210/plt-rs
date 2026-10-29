//! Point-moving instructions: `MDAP`, `MIAP`, `MDRP`, `MIRP`, `MSIRP`,
//! `ALIGNRP`, `ALIGNPTS`, `ISECT`, `SCFS`, plus the measuring `GC` and
//! `MD`.

use super::Exec;
use crate::fixed::{mul_div, mul_fix, mul_fix14};
use crate::hinting::zone::{TAG_TOUCH_BOTH, Vector};

impl Exec<'_> {
    /// The distance from `zp0`'s point `rp0` to `zp1`'s point `point` in
    /// the original outline, along the dual projection vector: from the
    /// scaled original positions when a twilight point is involved, from
    /// the font-unit positions otherwise.
    fn original_distance(&self, point: usize, reference: usize) -> i64 {
        if self.gs.gep0 == 0 || self.gs.gep1 == 0 {
            self.dual_project(self.org(1, point), self.org(0, reference))
        } else {
            let d = self.dual_project(self.orus(1, point), self.orus(0, reference));
            mul_fix(d, self.state.metrics.orus_scale)
        }
    }

    /// Keeps `distance` at least the minimum distance away from zero, on
    /// the side of `original`.
    fn at_least_minimum(&self, distance: i64, original: i64) -> i64 {
        let minimum = self.gs.minimum_distance;
        if original >= 0 {
            distance.max(minimum)
        } else {
            distance.min(minimum.wrapping_neg())
        }
    }

    /// The reference point updates of `MDRP`/`MIRP`.
    fn after_relative_move(&mut self, point: u16) {
        self.gs.rp1 = self.gs.rp0;
        self.gs.rp2 = point;
        if self.opcode & 16 != 0 {
            self.gs.rp0 = point;
        }
    }

    /// `MDAP[a]`: touches a point, rounding its position if asked.
    pub(super) fn ins_mdap(&mut self) {
        let Some(point) = self.point16(0, self.arg(0)) else {
            return;
        };
        let distance = if self.opcode & 1 != 0 {
            let current = self.project(self.cur(0, point), Vector::default());
            self.rounding.round(current).wrapping_sub(current)
        } else {
            0
        };
        self.move_point(0, point, distance);
        (self.gs.rp0, self.gs.rp1) = (point as u16, point as u16);
    }

    /// `MIAP[a]`: moves a point to a control value, subject to the cut-in.
    pub(super) fn ins_miap(&mut self) {
        let raw_point = self.arg(0) as u16;
        let entry = self.arg(1) as u64;
        let target = self.point16(0, self.arg(0)).zip(self.cvt_value(entry));
        if let Some((point, mut distance)) = target {
            if self.gs.gep0 == 0 {
                let free = self.gs.free_vector;
                let position = Vector::new(
                    mul_fix14(distance, i64::from(free.x)),
                    mul_fix14(distance, i64::from(free.y)),
                );
                let zone = self.zone_mut(self.zp[0]);
                zone.org[point] = position;
                zone.cur[point] = position;
            }
            let original = self.project(self.cur(0, point), Vector::default());
            if self.opcode & 1 != 0 {
                if distance.wrapping_sub(original).wrapping_abs() > self.gs.control_value_cutin {
                    distance = original;
                }
                distance = self.rounding.round(distance);
            }
            self.move_point(0, point, distance.wrapping_sub(original));
        }
        (self.gs.rp0, self.gs.rp1) = (raw_point, raw_point);
    }

    /// `MDRP[abcde]`: keeps a point's original distance to `rp0`.
    pub(super) fn ins_mdrp(&mut self) {
        let raw_point = self.arg(0) as u16;
        let point = self.point16(1, self.arg(0));
        let reference = self.point16(0, i64::from(self.gs.rp0));
        if let (Some(point), Some(reference)) = (point, reference) {
            let mut original = self.original_distance(point, reference);
            let (value, cutin) = (self.gs.single_width_value, self.gs.single_width_cutin);
            if cutin > 0
                && original < value.wrapping_add(cutin)
                && original > value.wrapping_sub(cutin)
            {
                original = if original >= 0 {
                    value
                } else {
                    value.wrapping_neg()
                };
            }
            let mut distance = if self.opcode & 4 != 0 {
                self.rounding.round(original)
            } else {
                original
            };
            if self.opcode & 8 != 0 {
                distance = self.at_least_minimum(distance, original);
            }
            let current = self.project(self.cur(1, point), self.cur(0, reference));
            self.move_point(1, point, distance.wrapping_sub(current));
        }
        self.after_relative_move(raw_point);
    }

    /// `MIRP[abcde]`: moves a point to a control value distance from
    /// `rp0`.
    pub(super) fn ins_mirp(&mut self) {
        let raw_point = self.arg(0) as u16;
        let entry = self.arg(1).wrapping_add(1) as u64;
        let point = self.point16(1, self.arg(0));
        let reference = self.point16(0, i64::from(self.gs.rp0));
        // Entry -1 is an implicit zero control value.
        let control = if entry == 0 {
            Some(0)
        } else {
            self.cvt_value(entry - 1)
        };
        if let (Some(point), Some(reference), Some(mut cvt_distance)) = (point, reference, control)
        {
            let (value, cutin) = (self.gs.single_width_value, self.gs.single_width_cutin);
            if cvt_distance.wrapping_sub(value).wrapping_abs() < cutin {
                cvt_distance = if cvt_distance >= 0 {
                    value
                } else {
                    value.wrapping_neg()
                };
            }
            if self.gs.gep1 == 0 {
                let free = self.gs.free_vector;
                let origin = self.org(0, reference);
                let position = Vector::new(
                    origin
                        .x
                        .wrapping_add(mul_fix14(cvt_distance, i64::from(free.x))),
                    origin
                        .y
                        .wrapping_add(mul_fix14(cvt_distance, i64::from(free.y))),
                );
                let zone = self.zone_mut(self.zp[1]);
                zone.org[point] = position;
                zone.cur[point] = position;
            }
            let original = self.dual_project(self.org(1, point), self.org(0, reference));
            let current = self.project(self.cur(1, point), self.cur(0, reference));
            if self.gs.auto_flip && (original ^ cvt_distance) < 0 {
                cvt_distance = cvt_distance.wrapping_neg();
            }
            let mut distance = if self.opcode & 4 != 0 {
                if self.gs.gep0 == self.gs.gep1
                    && cvt_distance.wrapping_sub(original).wrapping_abs()
                        > self.gs.control_value_cutin
                {
                    cvt_distance = original;
                }
                self.rounding.round(cvt_distance)
            } else {
                cvt_distance
            };
            if self.opcode & 8 != 0 {
                distance = self.at_least_minimum(distance, original);
            }
            self.move_point(1, point, distance.wrapping_sub(current));
        }
        self.after_relative_move(raw_point);
    }

    /// `MSIRP[a]`: moves a point to a distance from `rp0` given on the
    /// stack.
    pub(super) fn ins_msirp(&mut self) {
        let point = self.point16(1, self.arg(0));
        let reference = self.point16(0, i64::from(self.gs.rp0));
        let (Some(point), Some(reference)) = (point, reference) else {
            return;
        };
        let distance = self.arg(1);
        if self.gs.gep1 == 0 {
            let origin = self.org(0, reference);
            self.zone_mut(self.zp[1]).org[point] = origin;
            self.move_original(1, point, distance);
            let zone = self.zone_mut(self.zp[1]);
            zone.cur[point] = zone.org[point];
        }
        let current = self.project(self.cur(1, point), self.cur(0, reference));
        self.move_point(1, point, distance.wrapping_sub(current));
        self.gs.rp1 = self.gs.rp0;
        self.gs.rp2 = point as u16;
        if self.opcode & 1 != 0 {
            self.gs.rp0 = point as u16;
        }
    }

    /// Takes the loop counter's worth of arguments off the stack for a
    /// looping instruction; `None` (resetting the counter) when there are
    /// too few or the work budget is spent.
    pub(super) fn loop_arguments(&mut self) -> Option<usize> {
        let count = self.gs.loop_count as usize;
        if self.new_top < count || !self.charge(count as u64) {
            self.gs.loop_count = 1;
            return None;
        }
        self.new_top -= count;
        Some(count)
    }

    /// `ALIGNRP`: moves points onto `rp0` along the projection vector.
    pub(super) fn ins_alignrp(&mut self) {
        let Some(count) = self.loop_arguments() else {
            return;
        };
        if let Some(reference) = self.point16(0, i64::from(self.gs.rp0)) {
            for depth in 1..=count {
                if let Some(point) = self.point16(1, self.below(depth)) {
                    let distance = self.project(self.cur(1, point), self.cur(0, reference));
                    self.move_point(1, point, distance.wrapping_neg());
                }
            }
        }
        self.gs.loop_count = 1;
    }

    /// `ALIGNPTS`: moves two points halfway towards each other.
    pub(super) fn ins_alignpts(&mut self) {
        let p1 = self.point16(1, self.arg(0));
        let p2 = self.point16(0, self.arg(1));
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return;
        };
        let distance = self.project(self.cur(0, p2), self.cur(1, p1)) / 2;
        self.move_point(1, p1, distance);
        self.move_point(0, p2, distance.wrapping_neg());
    }

    /// `ISECT`: moves a point to the intersection of two lines (or the
    /// middle of their midpoints when they are nearly parallel).
    pub(super) fn ins_isect(&mut self) {
        let point = self.point16(2, self.arg(0));
        let a0 = self.point16(1, self.arg(1));
        let a1 = self.point16(1, self.arg(2));
        let b0 = self.point16(0, self.arg(3));
        let b1 = self.point16(0, self.arg(4));
        let (Some(point), Some(a0), Some(a1), Some(b0), Some(b1)) = (point, a0, a1, b0, b1) else {
            return;
        };
        let (a0, a1) = (self.cur(1, a0), self.cur(1, a1));
        let (b0, b1) = (self.cur(0, b0), self.cur(0, b1));
        let db = b1.minus(b0);
        let da = a1.minus(a0);
        let d = b0.minus(a0);
        let discriminant =
            mul_div(da.x, db.y.wrapping_neg(), 0x40).wrapping_add(mul_div(da.y, db.x, 0x40));
        let dot_product = mul_div(da.x, db.x, 0x40).wrapping_add(mul_div(da.y, db.y, 0x40));
        let position = if discriminant.wrapping_abs().wrapping_mul(19) > dot_product.wrapping_abs()
        {
            let value =
                mul_div(d.x, db.y.wrapping_neg(), 0x40).wrapping_add(mul_div(d.y, db.x, 0x40));
            Vector::new(
                a0.x.wrapping_add(mul_div(value, da.x, discriminant)),
                a0.y.wrapping_add(mul_div(value, da.y, discriminant)),
            )
        } else {
            let middle = |a: i64, b: i64, c: i64, e: i64| {
                a.wrapping_add(b).wrapping_add(c.wrapping_add(e)) / 4
            };
            Vector::new(
                middle(a0.x, a1.x, b0.x, b1.x),
                middle(a0.y, a1.y, b0.y, b1.y),
            )
        };
        let zone = self.zone_mut(self.zp[2]);
        zone.cur[point] = position;
        zone.tags[point] |= TAG_TOUCH_BOTH;
    }

    /// `GC[a]`: a point's current coordinate, or its original one along
    /// the dual projection vector.
    pub(super) fn ins_gc(&mut self) {
        let value = self.checked(2, self.arg(0) as u64).map_or(0, |point| {
            if self.opcode & 1 != 0 {
                self.dual_project(self.org(2, point), Vector::default())
            } else {
                self.project(self.cur(2, point), Vector::default())
            }
        });
        self.set_arg(0, value);
    }

    /// `SCFS`: moves a point to a coordinate given on the stack.
    pub(super) fn ins_scfs(&mut self) {
        let Some(point) = self.point16(2, self.arg(0)) else {
            return;
        };
        let current = self.project(self.cur(2, point), Vector::default());
        self.move_point(2, point, self.arg(1).wrapping_sub(current));
        if self.gs.gep2 == 0 {
            let zone = self.zone_mut(self.zp[2]);
            zone.org[point] = zone.cur[point];
        }
    }

    /// `MD[a]`: the distance between two points, in the grid-fitted or the
    /// original outline.
    pub(super) fn ins_md(&mut self) {
        let l = self.point16(0, self.arg(0));
        let k = self.point16(1, self.arg(1));
        let distance = match (l, k) {
            (Some(l), Some(k)) if self.opcode & 1 != 0 => {
                self.project(self.cur(0, l), self.cur(1, k))
            }
            (Some(l), Some(k)) if self.gs.gep0 == 0 || self.gs.gep1 == 0 => {
                self.dual_project(self.org(0, l), self.org(1, k))
            }
            (Some(l), Some(k)) => mul_fix(
                self.dual_project(self.orus(0, l), self.orus(1, k)),
                self.state.metrics.orus_scale,
            ),
            _ => 0,
        };
        self.set_arg(0, distance);
    }
}
