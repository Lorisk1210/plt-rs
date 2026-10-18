//! Graphics-state instructions: vectors, reference points, zone pointers,
//! loop counter, rounding, delta shift, instruction and scan control, and
//! `GETINFO`.

use super::{BACKWARD_COMPATIBILITY, Exec, ZoneId};
use crate::fixed::normalize;
use crate::hinting::error::{BytecodeError, BytecodeProgram};
use crate::hinting::state::{Rounding, SuperRound, UnitVector};

/// FreeType's interpreter version reported by `GETINFO`.
const INTERPRETER_VERSION: i64 = 40;

/// Sets `target` to the unit vector along `(x, y)` (`Normalize`); a zero
/// vector leaves it unchanged.
fn normalize_into(target: &mut UnitVector, x: i64, y: i64) {
    if x == 0 && y == 0 {
        return;
    }
    let (ux, uy) = normalize(x, y);
    *target = UnitVector {
        x: (ux / 4) as i16,
        y: (uy / 4) as i16,
    };
}

/// The vector from `b` to `a`, rotated counter-clockwise when
/// `perpendicular`; `(0x4000, 0)` (and no rotation) when they coincide.
fn line_vector(dx: i64, dy: i64, perpendicular: &mut bool) -> (i64, i64) {
    let (dx, dy) = if dx == 0 && dy == 0 {
        *perpendicular = false;
        (0x4000, 0)
    } else {
        (dx, dy)
    };
    if *perpendicular {
        (dy.wrapping_neg(), dx)
    } else {
        (dx, dy)
    }
}

impl Exec<'_> {
    /// `SVTCA`, `SPVTCA`, `SFVTCA`: vectors to an axis.
    pub(super) fn ins_sxytca(&mut self) {
        let axis = if self.opcode & 1 == 0 {
            UnitVector::Y
        } else {
            UnitVector::X
        };
        if self.opcode < 4 {
            self.gs.proj_vector = axis;
            self.gs.dual_vector = axis;
        }
        if self.opcode & 2 == 0 {
            self.gs.free_vector = axis;
        }
        self.compute_funcs();
    }

    /// The unit vector along the line from `zp2`'s point `args[1]` to
    /// `zp1`'s point `args[0]` (`Ins_SxVTL`), if both exist.
    fn line(&self) -> Option<(i64, i64)> {
        let p2 = self.point16(2, self.arg(1));
        let p1 = self.point16(1, self.arg(0));
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return None;
        };
        let d = self.cur(1, p1).minus(self.cur(2, p2));
        Some(line_vector(d.x, d.y, &mut (self.opcode & 1 != 0)))
    }

    /// `SPVTL`.
    pub(super) fn ins_spvtl(&mut self) {
        if let Some((x, y)) = self.line() {
            normalize_into(&mut self.gs.proj_vector, x, y);
            self.gs.dual_vector = self.gs.proj_vector;
            self.compute_funcs();
        }
    }

    /// `SFVTL`.
    pub(super) fn ins_sfvtl(&mut self) {
        if let Some((x, y)) = self.line() {
            normalize_into(&mut self.gs.free_vector, x, y);
            self.compute_funcs();
        }
    }

    /// `SPVFS`: projection vector from the stack (low 16 bits).
    pub(super) fn ins_spvfs(&mut self) {
        let (x, y) = (i64::from(self.arg(0) as i16), i64::from(self.arg(1) as i16));
        normalize_into(&mut self.gs.proj_vector, x, y);
        self.gs.dual_vector = self.gs.proj_vector;
        self.compute_funcs();
    }

    /// `SFVFS`: freedom vector from the stack (low 16 bits).
    pub(super) fn ins_sfvfs(&mut self) {
        let (x, y) = (i64::from(self.arg(0) as i16), i64::from(self.arg(1) as i16));
        normalize_into(&mut self.gs.free_vector, x, y);
        self.compute_funcs();
    }

    /// `GPV`.
    pub(super) fn ins_gpv(&mut self) {
        let v = self.gs.proj_vector;
        self.set_arg(0, i64::from(v.x));
        self.set_arg(1, i64::from(v.y));
    }

    /// `GFV`.
    pub(super) fn ins_gfv(&mut self) {
        let v = self.gs.free_vector;
        self.set_arg(0, i64::from(v.x));
        self.set_arg(1, i64::from(v.y));
    }

    /// `SFVTPV`.
    pub(super) fn ins_sfvtpv(&mut self) {
        self.gs.free_vector = self.gs.proj_vector;
        self.compute_funcs();
    }

    /// `SDPVTL`: dual projection vector from the original positions,
    /// projection vector from the current ones.
    pub(super) fn ins_sdpvtl(&mut self) {
        let p1 = self.point16(2, self.arg(1));
        let p2 = self.point16(1, self.arg(0));
        let (Some(p1), Some(p2)) = (p1, p2) else {
            return;
        };
        let mut perpendicular = self.opcode & 1 != 0;
        let d = self.org(1, p2).minus(self.org(2, p1));
        let (x, y) = line_vector(d.x, d.y, &mut perpendicular);
        normalize_into(&mut self.gs.dual_vector, x, y);
        let d = self.cur(1, p2).minus(self.cur(2, p1));
        let (x, y) = line_vector(d.x, d.y, &mut perpendicular);
        normalize_into(&mut self.gs.proj_vector, x, y);
        self.compute_funcs();
    }

    /// `SRP0`, `SRP1`, `SRP2`.
    pub(super) fn ins_srp(&mut self) {
        let point = self.arg(0) as u16;
        match self.opcode {
            0x10 => self.gs.rp0 = point,
            0x11 => self.gs.rp1 = point,
            _ => self.gs.rp2 = point,
        }
    }

    /// `SZP0`, `SZP1`, `SZP2`, `SZPS`: zone pointers (other numbers are
    /// ignored).
    pub(super) fn ins_szp(&mut self) {
        let value = self.arg(0);
        let zone = match value as i32 {
            0 => ZoneId::Twilight,
            1 => ZoneId::Glyph,
            _ => return,
        };
        let number = value as u16;
        match self.opcode {
            0x13 => (self.zp[0], self.gs.gep0) = (zone, number),
            0x14 => (self.zp[1], self.gs.gep1) = (zone, number),
            0x15 => (self.zp[2], self.gs.gep2) = (zone, number),
            _ => {
                self.zp = [zone; 3];
                (self.gs.gep0, self.gs.gep1, self.gs.gep2) = (number, number, number);
            }
        }
    }

    /// `SLOOP`: the loop counter, limited to 16 bits.
    pub(super) fn ins_sloop(&mut self) {
        let count = self.arg(0);
        if count < 0 {
            self.fail(BytecodeError::BadArgument);
        } else {
            self.gs.loop_count = count.min(0xFFFF);
        }
    }

    /// `SDS`.
    pub(super) fn ins_sds(&mut self) {
        let shift = self.arg(0);
        if shift as u64 > 6 {
            self.fail(BytecodeError::BadArgument);
        } else {
            self.gs.delta_shift = shift as u16;
        }
    }

    /// `SROUND` and `S45ROUND`.
    pub(super) fn ins_sround(&mut self) {
        let selector = self.arg(0);
        self.rounding = if self.opcode == 0x76 {
            Rounding::Super(SuperRound::new(0x4000, selector))
        } else {
            Rounding::Super45(SuperRound::new(0x2D41, selector))
        };
    }

    /// `INSTCTRL`: instruction control flags in the control value
    /// program; in glyph programs, selector 3 toggles backward
    /// compatibility for the glyph.
    pub(super) fn ins_instctrl(&mut self) {
        let (selector, value) = (self.arg(1) as u64, self.arg(0) as u64);
        if !(1..=3).contains(&selector) {
            return;
        }
        let flag = 1u64 << (selector - 1);
        if value != 0 && value != flag {
            return;
        }
        match self.initial {
            BytecodeProgram::ControlValue => {
                self.gs.instruct_control = (self.gs.instruct_control & !(flag as u8)) | value as u8;
            }
            BytecodeProgram::Glyph if selector == 3 => {
                self.state.resources.backward_compatibility =
                    (value as u8 & 4) ^ BACKWARD_COMPATIBILITY;
            }
            _ => {}
        }
    }

    /// `SCANCTRL`: dropout control (glyphs are never rotated or
    /// stretched here).
    pub(super) fn ins_scanctrl(&mut self) {
        let value = self.arg(0);
        let threshold = value & 0xFF;
        match threshold {
            0xFF => self.gs.scan_control = true,
            0 => self.gs.scan_control = false,
            _ => {
                let ppem = self.state.metrics.ppem;
                if value & 0x100 != 0 && ppem <= threshold {
                    self.gs.scan_control = true;
                }
                if value & 0x800 != 0 && ppem > threshold {
                    self.gs.scan_control = false;
                }
            }
        }
    }

    /// `SCANTYPE`.
    pub(super) fn ins_scantype(&mut self) {
        let value = self.arg(0);
        if value >= 0 {
            self.gs.scan_type = (value & 0xFFFF) as i32;
        }
    }

    /// `GETINFO` as the version 40 interpreter answers it for grayscale
    /// rendering (`FT_RENDER_MODE_NORMAL`).
    pub(super) fn ins_getinfo(&mut self) {
        let selector = self.arg(0);
        let mut info = 0;
        if selector & 1 != 0 {
            info = INTERPRETER_VERSION;
        }
        // Subpixel hinting, subpixel positioning, symmetrical smoothing
        // and ClearType-style grayscale rendering.
        for (asked, answer) in [(64, 13), (1024, 17), (2048, 18), (4096, 19)] {
            if selector & asked != 0 {
                info |= 1 << answer;
            }
        }
        self.set_arg(0, info);
    }
}
