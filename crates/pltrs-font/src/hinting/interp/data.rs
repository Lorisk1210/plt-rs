//! Data instructions: pushes, stack manipulation, arithmetic and logic,
//! the storage area and the control value table.

use super::Exec;
use crate::fixed::{mul_div, mul_div_no_round, mul_fix, pix_ceil, pix_floor};
use crate::hinting::error::BytecodeError;

impl Exec<'_> {
    /// `NPUSHB`, `NPUSHW`, `PUSHB[n]` and `PUSHW[n]`.
    pub(super) fn ins_push(&mut self, opcode: u8) {
        let mut ip = self.ip as usize;
        let (count, words) = match opcode {
            0x40 | 0x41 => {
                ip += 1;
                let Some(&count) = self.code.get(ip) else {
                    return self.fail(BytecodeError::CodeOverflow);
                };
                (usize::from(count), opcode == 0x41)
            }
            0xB0..=0xB7 => (usize::from(opcode - 0xB0) + 1, false),
            _ => (usize::from(opcode - 0xB8) + 1, true),
        };
        let width = if words { 2 } else { 1 };
        if ip + width * count >= self.code.len() {
            return self.fail(BytecodeError::CodeOverflow);
        }
        if count >= self.stack.len() + 1 - self.top {
            return self.fail(BytecodeError::StackOverflow);
        }
        let data = &self.code[ip + 1..=ip + width * count];
        let base = self.args;
        if words {
            for (slot, word) in self.stack[base..base + count]
                .iter_mut()
                .zip(data.as_chunks::<2>().0)
            {
                *slot = i64::from(i16::from_be_bytes(*word));
            }
        } else {
            for (slot, &byte) in self.stack[base..base + count].iter_mut().zip(data) {
                *slot = i64::from(byte);
            }
        }
        if matches!(opcode, 0x40 | 0x41) {
            self.new_top += count;
        }
        self.ip = (ip + width * count) as i64;
    }

    /// `DUP`, `POP`, `CLEAR`, `SWAP`, `DEPTH`, `CINDEX`, `MINDEX`, `ROLL`.
    pub(super) fn ins_stack(&mut self, opcode: u8) {
        match opcode {
            0x20 => self.set_arg(1, self.arg(0)),
            0x22 => self.new_top = 0,
            0x23 => self.stack.swap(self.args, self.args + 1),
            0x24 => self.set_arg(0, self.top as i64),
            0x25 => {
                let depth = self.arg(0);
                let value = if depth <= 0 || depth > self.args as i64 {
                    0
                } else {
                    self.below(depth as usize)
                };
                self.set_arg(0, value);
            }
            0x26 => {
                let depth = self.arg(0);
                if depth > 0 && depth <= self.args as i64 {
                    let depth = depth as usize;
                    if !self.charge(depth as u64) {
                        return;
                    }
                    let picked = self.below(depth);
                    self.stack
                        .copy_within(self.args - depth + 1..self.args, self.args - depth);
                    self.stack[self.args - 1] = picked;
                }
            }
            0x8A => {
                let (a, b, c) = (self.arg(2), self.arg(1), self.arg(0));
                self.set_arg(2, c);
                self.set_arg(1, a);
                self.set_arg(0, b);
            }
            // POP: the argument is simply dropped.
            _ => {}
        }
    }

    /// Comparisons, logic, arithmetic and rounding.
    pub(super) fn ins_arithmetic(&mut self, opcode: u8) {
        let a = self.arg(0);
        let b = || self.arg(1);
        let result = match opcode {
            0x50 => i64::from(a < b()),
            0x51 => i64::from(a <= b()),
            0x52 => i64::from(a > b()),
            0x53 => i64::from(a >= b()),
            0x54 => i64::from(a == b()),
            0x55 => i64::from(a != b()),
            0x56 => i64::from(self.rounding.round(a) & 64 == 64),
            0x57 => i64::from(self.rounding.round(a) & 64 == 0),
            0x5A => i64::from(a != 0 && b() != 0),
            0x5B => i64::from(a != 0 || b() != 0),
            0x5C => i64::from(a == 0),
            0x60 => a.wrapping_add(b()),
            0x61 => a.wrapping_sub(b()),
            0x62 => {
                if b() == 0 {
                    return self.fail(BytecodeError::DivideByZero);
                }
                mul_div_no_round(a, 64, b())
            }
            0x63 => mul_div(a, b(), 64),
            0x64 => a.wrapping_abs(),
            0x65 => a.wrapping_neg(),
            0x66 => pix_floor(a),
            0x67 => pix_ceil(a),
            0x68..=0x6B => self.rounding.round(a),
            0x8B => a.max(b()),
            0x8C => a.min(b()),
            // NROUND: no rounding, no engine compensation.
            _ => a,
        };
        self.set_arg(0, result);
    }

    /// The array index an argument names (FreeType's `(FT_ULong)` casts),
    /// if below `len`.
    fn index(value: i64, len: usize) -> Option<usize> {
        let index = value as u64;
        (index < len as u64).then_some(index as usize)
    }

    /// `RS`: reads the storage area (zero out of bounds).
    pub(super) fn ins_rs(&mut self) {
        let value = Self::index(self.arg(0), self.state.resources.storage.len())
            .map_or(0, |i| self.state.resources.storage[i]);
        self.set_arg(0, value);
    }

    /// `WS`: writes the storage area.
    pub(super) fn ins_ws(&mut self) {
        if let Some(i) = Self::index(self.arg(0), self.state.resources.storage.len()) {
            self.state.resources.storage[i] = self.arg(1);
        }
    }

    /// `RCVT`: reads a control value (zero out of bounds).
    pub(super) fn ins_rcvt(&mut self) {
        let value = Self::index(self.arg(0), self.state.resources.cvt.len())
            .map_or(0, |i| self.state.resources.cvt[i]);
        self.set_arg(0, value);
    }

    /// `WCVTP`: writes a control value in pixels.
    pub(super) fn ins_wcvtp(&mut self) {
        if let Some(i) = Self::index(self.arg(0), self.state.resources.cvt.len()) {
            self.state.resources.cvt[i] = self.arg(1);
        }
    }

    /// `WCVTF`: writes a control value in font units.
    pub(super) fn ins_wcvtf(&mut self) {
        if let Some(i) = Self::index(self.arg(0), self.state.resources.cvt.len()) {
            self.state.resources.cvt[i] = mul_fix(self.arg(1), self.state.metrics.scale);
        }
    }

    /// Adds `delta` to control value `index` (`Move_CVT`).
    pub(super) fn move_cvt(&mut self, index: usize, delta: i64) {
        self.state.resources.cvt[index] = self.state.resources.cvt[index].wrapping_add(delta);
    }

    /// The control value `index`, if it exists (`Read_CVT`).
    pub(super) fn cvt_value(&self, index: u64) -> Option<i64> {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.state.resources.cvt.get(i))
            .copied()
    }
}
