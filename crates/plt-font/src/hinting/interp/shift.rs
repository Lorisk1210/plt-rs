//! Shifting and interpolating instructions: `SHP`, `SHC`, `SHZ`, `SHPIX`,
//! `IP`, `UTP`, the flips and the delta exceptions.

use super::{Exec, POST_IUP, ZoneId};
use crate::fixed::{mul_div, mul_fix, mul_fix14};
use crate::hinting::zone::{TAG_ON, TAG_TOUCH_X, TAG_TOUCH_Y};

/// The displacement of a reference point, to shift other points by
/// (`Compute_Point_Displacement`).
struct Displacement {
    dx: i64,
    dy: i64,
    zone: ZoneId,
    point: usize,
}

impl Exec<'_> {
    /// How far `rp1` (in `zp0`, for opcodes with bit 0 set) or `rp2` (in
    /// `zp1`) has moved, along the freedom vector.
    fn displacement(&self) -> Option<Displacement> {
        let (zp, reference) = if self.opcode & 1 != 0 {
            (0, self.gs.rp1)
        } else {
            (1, self.gs.rp2)
        };
        let point = self.point16(zp, i64::from(reference))?;
        let d = self.project(self.cur(zp, point), self.org(zp, point));
        let (vx, vy) = self.funcs.move_vector;
        Some(Displacement {
            dx: mul_fix(d, vx),
            dy: mul_fix(d, vy),
            zone: self.zp[zp],
            point,
        })
    }

    /// Shifts a point of `zp2` (`Move_Zp2_Point`): no x movement in
    /// backward compatibility mode, none at all after both `IUP`s.
    fn shift_point(&mut self, point: usize, dx: i64, dy: i64, touch: bool) {
        let bc = self.backward_compatibility();
        let free = self.gs.free_vector;
        let zone = self.zone_mut(self.zp[2]);
        if free.x != 0 {
            if bc == 0 {
                zone.cur[point].x = zone.cur[point].x.wrapping_add(dx);
            }
            if touch {
                zone.tags[point] |= TAG_TOUCH_X;
            }
        }
        if free.y != 0 {
            if bc != POST_IUP {
                zone.cur[point].y = zone.cur[point].y.wrapping_add(dy);
            }
            if touch {
                zone.tags[point] |= TAG_TOUCH_Y;
            }
        }
    }

    /// Shifts the points `range` of `zp2` by `d`, except the reference
    /// point itself.
    fn shift_range(&mut self, range: std::ops::Range<usize>, d: &Displacement, touch: bool) {
        if !self.charge(range.len() as u64) {
            return;
        }
        for point in range {
            if d.zone != self.zp[2] || d.point != point {
                self.shift_point(point, d.dx, d.dy, touch);
            }
        }
    }

    /// `SHP[a]`: shifts points by the reference point's displacement.
    pub(super) fn ins_shp(&mut self) {
        let Some(count) = self.loop_arguments() else {
            return;
        };
        // Like FreeType, a missing reference point keeps the loop counter.
        let Some(d) = self.displacement() else {
            return;
        };
        for depth in 1..=count {
            if let Some(point) = self.point16(2, self.below(depth)) {
                self.shift_point(point, d.dx, d.dy, true);
            }
        }
        self.gs.loop_count = 1;
    }

    /// `SHC[a]`: shifts a contour (the whole twilight zone counts as
    /// contour 0).
    pub(super) fn ins_shc(&mut self) {
        let contour = usize::from(self.arg(0) as u16);
        let twilight = self.gs.gep2 == 0;
        let contours = &self.zone(self.zp[2]).contours;
        let bounds = if twilight { 1 } else { contours.len() };
        if contour >= bounds {
            return;
        }
        let Some(d) = self.displacement() else {
            return;
        };
        let contours = &self.zone(self.zp[2]).contours;
        let start = if contour == 0 {
            0
        } else {
            contours[contour - 1] + 1
        };
        let limit = if twilight {
            self.zone_len(self.zp[2])
        } else {
            contours[contour] + 1
        };
        self.shift_range(start..limit.max(start), &d, true);
    }

    /// `SHZ[a]`: shifts every point of `zp2` except the phantom points,
    /// without touching them.
    pub(super) fn ins_shz(&mut self) {
        if self.arg(0) as u32 >= 2 {
            return;
        }
        let Some(d) = self.displacement() else {
            return;
        };
        let limit = if self.gs.gep2 == 0 {
            self.zone_len(self.zp[2])
        } else {
            self.zone(self.zp[2])
                .contours
                .last()
                .map_or(0, |&end| end + 1)
        };
        self.shift_range(0..limit, &d, false);
    }

    /// `SHPIX`: shifts points by a pixel amount; in backward compatibility
    /// mode only along y, and only twilight points, composites or points
    /// already touched in y.
    pub(super) fn ins_shpix(&mut self) {
        let in_twilight = self.gs.gep0 == 0 || self.gs.gep1 == 0 || self.gs.gep2 == 0;
        let Some(count) = self.loop_arguments() else {
            return;
        };
        let amount = self.arg(0);
        let free = self.gs.free_vector;
        let dx = mul_fix14(amount, i64::from(free.x));
        let dy = mul_fix14(amount, i64::from(free.y));
        let bc = self.backward_compatibility();
        for depth in 1..=count {
            let Some(point) = self.point16(2, self.below(depth)) else {
                continue;
            };
            if bc == 0 {
                self.shift_point(point, dx, dy, true);
            } else if in_twilight || (bc != POST_IUP && self.y_adjustable(2, point)) {
                self.shift_point(point, 0, dy, true);
            }
        }
        self.gs.loop_count = 1;
    }

    /// Whether backward compatibility mode lets `SHPIX` or `DELTAP` move a
    /// point: in a composite with a vertical freedom component, or a point
    /// already touched in y.
    fn y_adjustable(&self, zp: usize, point: usize) -> bool {
        (self.is_composite && self.gs.free_vector.y != 0)
            || self.zone(self.zp[zp]).tags[point] & TAG_TOUCH_Y != 0
    }

    /// `IP`: interpolates points between `rp1` and `rp2`.
    pub(super) fn ins_ip(&mut self) {
        let Some(count) = self.loop_arguments() else {
            return;
        };
        let Some(rp1) = self.point16(0, i64::from(self.gs.rp1)) else {
            self.gs.loop_count = 1;
            return;
        };
        let twilight = self.gs.gep0 == 0 || self.gs.gep1 == 0 || self.gs.gep2 == 0;
        // Twilight points have no font-unit positions: use the originals.
        let original = |exec: &Self, zp: usize, point: usize| {
            if twilight {
                exec.org(zp, point)
            } else {
                exec.orus(zp, point)
            }
        };
        let original_base = original(self, 0, rp1);
        let current_base = self.cur(0, rp1);
        let (old_range, cur_range) =
            self.point16(1, i64::from(self.gs.rp2))
                .map_or((0, 0), |rp2| {
                    (
                        self.dual_project(original(self, 1, rp2), original_base),
                        self.project(self.cur(1, rp2), current_base),
                    )
                });
        for depth in 1..=count {
            let Some(point) = self.checked(2, u64::from(self.below(depth) as u32)) else {
                continue;
            };
            let org_dist = self.dual_project(original(self, 2, point), original_base);
            let cur_dist = self.project(self.cur(2, point), current_base);
            let new_dist = match (org_dist, old_range) {
                (0, _) => 0,
                (_, 0) => org_dist,
                _ => mul_div(org_dist, cur_range, old_range),
            };
            self.move_point(2, point, new_dist.wrapping_sub(cur_dist));
        }
        self.gs.loop_count = 1;
    }

    /// `UTP`: clears a point's touch flags along the freedom vector.
    pub(super) fn ins_utp(&mut self) {
        let Some(point) = self.point16(0, self.arg(0)) else {
            return;
        };
        let free = self.gs.free_vector;
        let mut mask = 0xFF;
        if free.x != 0 {
            mask &= !TAG_TOUCH_X;
        }
        if free.y != 0 {
            mask &= !TAG_TOUCH_Y;
        }
        self.zone_mut(self.zp[0]).tags[point] &= mask;
    }

    /// `FLIPPT`: toggles glyph points between on and off the curve (not
    /// after both `IUP`s in backward compatibility mode).
    pub(super) fn ins_flippt(&mut self) {
        let Some(count) = self.loop_arguments() else {
            return;
        };
        if self.backward_compatibility() != POST_IUP {
            for depth in 1..=count {
                let point = usize::from(self.below(depth) as u16);
                if let Some(tag) = self.pts.tags.get_mut(point) {
                    *tag ^= TAG_ON;
                }
            }
        }
        self.gs.loop_count = 1;
    }

    /// `FLIPRGON` and `FLIPRGOFF`: puts a range of glyph points on or off
    /// the curve.
    pub(super) fn ins_fliprg(&mut self) {
        if self.backward_compatibility() == POST_IUP {
            return;
        }
        let (last, first) = (
            usize::from(self.arg(1) as u16),
            usize::from(self.arg(0) as u16),
        );
        let len = self.pts.len();
        if last >= len || first > last || !self.charge((last - first + 1) as u64) {
            return;
        }
        let on = self.opcode == 0x81;
        for tag in &mut self.pts.tags[first..=last] {
            if on {
                *tag |= TAG_ON;
            } else {
                *tag &= !TAG_ON;
            }
        }
    }

    /// The `(selector, value)` pairs of a delta exception that apply at
    /// the current size, with the step they move by
    /// (`DELTAP`/`DELTAC` bookkeeping): the pairs are taken off the stack
    /// even when the size does not match.
    fn delta_pairs(&mut self, first_ppem: i64) -> Vec<(i64, i64)> {
        let available = (self.new_top / 2) as i64;
        let requested = self.arg(0);
        let count = if requested < 0 || requested > available {
            available
        } else {
            requested
        } as usize;
        if !self.charge(count as u64) {
            return Vec::new();
        }
        self.new_top -= 2 * count;
        let relative = self.state.metrics.ppem - i64::from(self.gs.delta_base) - first_ppem;
        if relative & !0xF != 0 {
            return Vec::new();
        }
        let step = 1i64 << (6 - self.gs.delta_shift);
        (0..count)
            .filter_map(|pair| {
                let target = self.below(2 * pair + 1);
                let value = self.below(2 * pair + 2);
                (value & 0xF0 == relative << 4).then(|| {
                    let magnitude = (value & 0xF) - 8;
                    let steps = if magnitude >= 0 {
                        magnitude + 1
                    } else {
                        magnitude
                    };
                    (target, steps * step)
                })
            })
            .collect()
    }

    /// `DELTAP1`–`DELTAP3`: moves points at specific sizes (in backward
    /// compatibility mode only where `SHPIX` could).
    pub(super) fn ins_deltap(&mut self) {
        let first_ppem = match self.opcode {
            0x71 => 16,
            0x72 => 32,
            _ => 0,
        };
        let bc = self.backward_compatibility();
        for (target, delta) in self.delta_pairs(first_ppem) {
            let Some(point) = self.point16(0, target) else {
                continue;
            };
            if bc == 0 || (bc != POST_IUP && self.y_adjustable(0, point)) {
                self.move_point(0, point, delta);
            }
        }
    }

    /// `DELTAC1`–`DELTAC3`: changes control values at specific sizes.
    pub(super) fn ins_deltac(&mut self) {
        let first_ppem = match self.opcode {
            0x74 => 16,
            0x75 => 32,
            _ => 0,
        };
        for (target, delta) in self.delta_pairs(first_ppem) {
            let index = target as u64;
            if index < self.state.resources.cvt.len() as u64 {
                self.move_cvt(index as usize, delta);
            }
        }
    }
}
