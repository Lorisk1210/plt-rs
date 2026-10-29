//! The escaped (two-byte) operators of `cf2_interpT2CharString` in Type 1
//! mode: flex, arithmetic, `seac`, `sbw`, OtherSubrs and `pop`.

use super::interp::{Axis, Flow, Invocation, Pass, STORAGE_SIZE, add};
use crate::fixed::{div_fix, mul_fix};
use crate::type1::error::Type1Error;
use crate::type1::standard::standard_name;

/// Which flex values come from the stack (`dx1 dy1 ... dx6 dy6`).
type FlexArgs = [bool; 12];

const HFLEX: FlexArgs = [
    true, false, true, true, true, false, true, false, true, false, true, false,
];
const FLEX: FlexArgs = [true; 12];
const HFLEX1: FlexArgs = [
    true, true, true, true, true, false, true, false, true, true, true, false,
];
const FLEX1: FlexArgs = [
    true, true, true, true, true, true, true, true, true, true, false, false,
];

/// `cff_random`: a 32-bit xorshift.
const fn next_random(mut r: u32) -> u32 {
    r ^= r << 13;
    r ^= r >> 17;
    r ^= r << 5;
    r
}

/// `FT_SqrtFixed`: the square root of a 16.16 number, rounded up.
fn sqrt_fixed(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    let r = (u64::from(value) << 16) - 1;
    let mut q: u32 = 1 << u32::midpoint(17, value.ilog2());
    loop {
        let t = q;
        q = t.wrapping_add((r / u64::from(t)) as u32).wrapping_add(1) >> 1;
        if q == t {
            return q;
        }
    }
}

const fn invalid_othersubr() -> Type1Error {
    Type1Error::BadCharstring("invalid OtherSubr call")
}

impl Invocation<'_, '_> {
    /// Runs escaped operator `op2`.
    pub(super) fn escape(&mut self, op2: u8) -> Result<Flow, Type1Error> {
        Ok(match op2 {
            34 => self.flex(&HFLEX, false)?,
            // FreeType clears the stack after `flex` only.
            35 => {
                self.flex(&FLEX, false)?;
                Flow::Clear
            }
            36 => self.flex(&HFLEX1, false)?,
            37 => self.flex(&FLEX1, true)?,
            8 | 13 | 19 | 25 | 31 | 32 | 38.. => Flow::Clear,
            // The arguments were all handed to earlier `pop`s.
            _ if self.result_count > 0 && op2 != 17 => {
                self.result_count = 0;
                Flow::Clear
            }
            1 | 2 => self.stem3(if op2 == 1 {
                Axis::Vertical
            } else {
                Axis::Horizontal
            })?,
            6 => self.seac()?,
            7 => self.sbw()?,
            16 => self.call_other_subr()?,
            17 => self.pop()?,
            33 => self.set_current_point()?,
            // `dotsection`.
            0 => Flow::Clear,
            _ => self.arithmetic(op2)?,
        })
    }

    /// `cf2_doFlex`: two curves through the flex points.
    fn flex(&mut self, args: &FlexArgs, conditional_last: bool) -> Result<Flow, Type1Error> {
        let (cur_x, cur_y) = self.cur;
        let mut vals = [0i32; 14];
        vals[0] = cur_x;
        vals[1] = cur_y;
        let horizontal = !args[9];
        let top = if horizontal { 9 } else { 10 };
        let mut at = 0;
        for i in 0..top {
            vals[i + 2] = vals[i];
            if args[i] {
                vals[i + 2] = add(vals[i + 2], self.stack.get(at)?);
                at += 1;
            }
        }
        if horizontal {
            vals[11] = cur_y;
        }
        if conditional_last {
            let dx = vals[10].wrapping_sub(cur_x).wrapping_abs();
            let dy = vals[11].wrapping_sub(cur_y).wrapping_abs();
            let last = self.stack.get(at)?;
            if dx > dy {
                vals[12] = add(vals[10], last);
                vals[13] = cur_y;
            } else {
                vals[12] = cur_x;
                vals[13] = add(vals[11], last);
            }
        } else {
            vals[12] = if args[10] {
                let value = add(vals[10], self.stack.get(at)?);
                at += 1;
                value
            } else {
                cur_x
            };
            vals[13] = if args[11] {
                add(vals[11], self.stack.get(at)?)
            } else {
                cur_y
            };
        }
        for j in 0..2 {
            let p = &vals[j * 6 + 2..j * 6 + 8];
            self.curve_to([(p[0], p[1]), (p[2], p[3]), (p[4], p[5])])?;
        }
        self.stack.clear();
        self.cur = (vals[12], vals[13]);
        Ok(Flow::Keep)
    }

    /// `vstem3` and `hstem3`: three stems relative to the side bearing.
    fn stem3(&mut self, axis: Axis) -> Result<Flow, Type1Error> {
        let (v0, v1, v2) = (self.stack.get(0)?, self.stack.get(2)?, self.stack.get(4)?);
        let first = v1.wrapping_sub(v0).wrapping_sub(self.stack.get(1)?);
        self.stack.set(2, first)?;
        let second = v2.wrapping_sub(v1).wrapping_sub(self.stack.get(3)?);
        self.stack.set(4, second)?;
        let offset = match axis {
            Axis::Vertical => self.decoder.left_bearing.0,
            Axis::Horizontal => self.decoder.left_bearing.1,
        };
        self.stems(axis, offset)?;
        Ok(Flow::Clear)
    }

    /// `sbw`: side bearing and advance in both directions.
    fn sbw(&mut self) -> Result<Flow, Type1Error> {
        let advance_y = self.stack.pop_fixed()?;
        let advance_x = self.stack.pop_fixed()?;
        self.decoder.advance = (advance_x, advance_y);
        let lsb_y = self.stack.pop_fixed()?;
        let lsb_x = self.stack.pop_fixed()?;
        let bearing = &mut self.decoder.left_bearing;
        *bearing = (add(bearing.0, lsb_x), add(bearing.1, lsb_y));
        self.have_width = true;
        if self.pass == Pass::Draw {
            self.cur = (add(self.cur.0, lsb_x), add(self.cur.1, lsb_y));
        }
        Ok(Flow::Clear)
    }

    /// `setcurrentpoint`, which ends a flex.
    fn set_current_point(&mut self) -> Result<Flow, Type1Error> {
        if self.pass == Pass::Draw {
            let y = self.stack.pop_fixed()?;
            let x = self.stack.pop_fixed()?;
            self.cur = (x, y);
            self.decoder.flex_state = false;
        }
        Ok(Flow::Clear)
    }

    /// The glyph whose name the Standard Encoding gives `code` (`.notdef`
    /// for unencoded codes), as `t1_lookup_glyph_by_stdcharcode_ps` finds
    /// it.
    fn standard_glyph(&self, code: i32) -> Result<usize, Type1Error> {
        let name = u8::try_from(code)
            .map(standard_name)
            .map_err(|_| Type1Error::NotStandardCode(i64::from(code)))?;
        self.decoder
            .set
            .names
            .iter()
            .position(|glyph| glyph == name)
            .ok_or_else(|| Type1Error::MissingGlyph(name.to_owned()))
    }

    /// `seac`: the base character, then the accent moved by `(adx - asb +
    /// side bearing, ady)`, keeping this glyph's metrics.
    fn seac(&mut self) -> Result<Flow, Type1Error> {
        let accent = self.stack.pop_int()?;
        let base = self.stack.pop_int()?;
        let ady = self.stack.pop_fixed()?;
        let adx = self.stack.pop_fixed()?;
        let asb = self.stack.pop_fixed()?;
        if self.doing_seac {
            return Err(Type1Error::BadCharstring("nested seac"));
        }
        let adx = i64::from(adx) + i64::from(self.decoder.left_bearing.0);
        let base = self.standard_glyph(base)?;
        let accent = self.standard_glyph(accent)?;
        let set = self.decoder.set;
        let program = |glyph: usize| {
            set.charstrings
                .get(glyph)
                .ok_or(Type1Error::InvalidFont("glyph without charstring"))
        };
        let (mut left_bearing, mut advance) = (self.decoder.left_bearing, self.decoder.advance);
        Invocation::new(&mut *self.decoder, true, (0, 0)).run(program(base)?)?;
        if !self.have_width {
            left_bearing = self.decoder.left_bearing;
            advance = self.decoder.advance;
        }
        self.decoder.left_bearing = (0, 0);
        let origin = ((adx - i64::from(asb)) as i32, ady);
        Invocation::new(&mut *self.decoder, true, origin).run(program(accent)?)?;
        self.decoder.left_bearing = left_bearing;
        self.decoder.advance = advance;
        Ok(Flow::Exit)
    }

    /// `callothersubr`: flex (0 to 2), hint replacement (3), counter
    /// control (12, 13), arithmetic (20 to 23, 27) and random (28); the
    /// arguments of unknown OtherSubrs are kept for `pop`.
    fn call_other_subr(&mut self) -> Result<Flow, Type1Error> {
        let number = self.stack.pop_int()?;
        let arg_count = self.stack.pop_int()?;
        if arg_count as u32 as usize > self.stack.count() {
            return Err(Type1Error::BadCharstring("OtherSubr stack underflow"));
        }
        self.known_results = 0;
        self.result_count = 0;
        let expect = |count: i32| {
            if arg_count == count {
                Ok(())
            } else {
                Err(invalid_othersubr())
            }
        };
        match number {
            0 => {
                expect(3)?;
                if self.pass == Pass::Draw
                    && (!self.decoder.flex_state || self.decoder.flex_vectors != 7)
                {
                    return Err(Type1Error::BadCharstring("unexpected flex end"));
                }
                // Popped by the `setcurrentpoint` that follows.
                self.stack.push_fixed(self.cur.0)?;
                self.stack.push_fixed(self.cur.1)?;
                self.known_results = 2;
            }
            1 => {
                expect(0)?;
                if self.pass == Pass::Draw {
                    if !self.decoder.out.has_room(6) {
                        // FreeType stops the glyph here without an error.
                        return Ok(Flow::Exit);
                    }
                    self.decoder.flex_state = true;
                    self.decoder.flex_vectors = 0;
                }
            }
            2 => {
                expect(0)?;
                if self.pass == Pass::Draw {
                    self.add_flex_vector()?;
                }
            }
            3 => {
                expect(1)?;
                if self.pass == Pass::Draw {
                    self.path.hints.reset();
                }
                self.known_results = 1;
            }
            12 | 13 => self.stack.clear(),
            20..=23 | 27 | 28 => {
                expect(match number {
                    27 => 4,
                    28 => 0,
                    _ => 2,
                })?;
                let value = match number {
                    28 => self.random(),
                    27 => self.if_else()?,
                    _ => {
                        let b = self.stack.pop_fixed()?;
                        let a = self.stack.pop_fixed()?;
                        match number {
                            20 => add(a, b),
                            21 => a.wrapping_sub(b),
                            22 => mul_fix(i64::from(a), i64::from(b)) as i32,
                            _ if b == 0 => return Err(invalid_othersubr()),
                            _ => div_fix(i64::from(a), i64::from(b)) as i32,
                        }
                    }
                };
                self.stack.push_fixed(value)?;
                self.known_results = 1;
            }
            // Multiple master operators need a blend, which plain Type 1
            // fonts lack.
            14..=19 | 24 | 25 => return Err(invalid_othersubr()),
            _ if arg_count >= 0 && number >= 0 => {
                let kept = (arg_count as usize).min(self.results.len());
                self.result_count = kept;
                for i in 1..=kept {
                    self.results[kept - i] = self.stack.pop_fixed()?;
                }
            }
            _ => return Err(invalid_othersubr()),
        }
        Ok(Flow::Keep)
    }

    /// OtherSubr 2: records a flex point; points 3 and 6 complete a curve.
    fn add_flex_vector(&mut self) -> Result<(), Type1Error> {
        if !self.decoder.flex_state {
            return Err(Type1Error::BadCharstring("missing flex start"));
        }
        let index = self.decoder.flex_vectors;
        self.decoder.flex_vectors += 1;
        if (1..7).contains(&index) {
            if !self.decoder.out.has_room(1) {
                return Err(Type1Error::BadCharstring("too many outline points"));
            }
            let slot = (if index > 3 { index - 3 } else { index }) as usize * 2;
            self.flex_store[slot - 2] = self.cur.0;
            self.flex_store[slot - 1] = self.cur.1;
            if index == 3 || index == 6 {
                let s = self.flex_store;
                self.curve_to([(s[0], s[1]), (s[2], s[3]), (s[4], s[5])])?;
            }
        }
        Ok(())
    }

    /// `pop`: the results of known OtherSubrs are already on the stack;
    /// otherwise the kept arguments come back.
    fn pop(&mut self) -> Result<Flow, Type1Error> {
        if self.known_results > 0 {
            self.known_results -= 1;
            return Ok(Flow::Keep);
        }
        if self.result_count == 0 {
            return Err(Type1Error::BadCharstring("no more operands for othersubr"));
        }
        self.result_count -= 1;
        self.stack.push_fixed(self.results[self.result_count])?;
        Ok(Flow::Keep)
    }

    /// The next random number in (0, 1].
    fn random(&mut self) -> i32 {
        let value = (self.decoder.random & 0xFFFF) as i32 + 1;
        self.decoder.random = next_random(self.decoder.random);
        value
    }

    /// `ifelse`: the first of two values if `cond1 <= cond2`.
    fn if_else(&mut self) -> Result<i32, Type1Error> {
        let cond2 = self.stack.pop_fixed()?;
        let cond1 = self.stack.pop_fixed()?;
        let second = self.stack.pop_fixed()?;
        let first = self.stack.pop_fixed()?;
        Ok(if cond1 <= cond2 { first } else { second })
    }

    /// The arithmetic and stack operators, which keep their results on
    /// the stack.
    fn arithmetic(&mut self, op2: u8) -> Result<Flow, Type1Error> {
        match op2 {
            12 => {
                let stack = &mut self.stack;
                let (divisor, dividend) = if self.large_int {
                    self.large_int = false;
                    (stack.pop_int()?, stack.pop_int()?)
                } else {
                    (stack.pop_fixed()?, stack.pop_fixed()?)
                };
                stack.push_fixed(div_fix(i64::from(dividend), i64::from(divisor)) as i32)?;
            }
            20 => {
                let index = self.stack.pop_int()? as u32 as usize;
                let value = self.stack.pop_fixed()?;
                if let Some(slot) = self.storage.get_mut(index) {
                    *slot = value;
                }
            }
            21 => {
                let index = self.stack.pop_int()? as u32 as usize;
                if index < STORAGE_SIZE {
                    self.stack.push_fixed(self.storage[index])?;
                }
            }
            22 => {
                let value = self.if_else()?;
                self.stack.push_fixed(value)?;
            }
            23 => {
                let value = self.random();
                self.stack.push_fixed(value)?;
            }
            27..=30 => self.rearrange(op2)?,
            _ => self.compute(op2)?,
        }
        Ok(Flow::Keep)
    }

    /// The operators computing a value from the top of the stack.
    fn compute(&mut self, op2: u8) -> Result<(), Type1Error> {
        let stack = &mut self.stack;
        match op2 {
            5 | 9 | 14 | 26 => {
                let a = stack.pop_fixed()?;
                match op2 {
                    5 => stack.push_int(i32::from(a == 0)),
                    26 => {
                        let root = if a > 0 {
                            sqrt_fixed(a as u32) as i32
                        } else {
                            0
                        };
                        stack.push_fixed(root)
                    }
                    _ => stack.push_fixed(match a {
                        i32::MIN => i32::MAX,
                        _ if op2 == 9 => a.abs(),
                        _ => -a,
                    }),
                }
            }
            18 => stack.pop_fixed().map(drop),
            // 3 `and`, 4 `or`, 10 `add`, 11 `sub`, 15 `eq` and 24 `mul`.
            _ => {
                let b = stack.pop_fixed()?;
                let a = stack.pop_fixed()?;
                match op2 {
                    3 => stack.push_int(i32::from(a != 0 && b != 0)),
                    4 => stack.push_int(i32::from(a != 0 || b != 0)),
                    15 => stack.push_int(i32::from(a == b)),
                    10 => stack.push_fixed(add(a, b)),
                    11 => stack.push_fixed(a.wrapping_sub(b)),
                    _ => stack.push_fixed(mul_fix(i64::from(a), i64::from(b)) as i32),
                }
            }
        }
    }

    /// `dup`, `exch`, `index` and `roll`.
    fn rearrange(&mut self, op2: u8) -> Result<(), Type1Error> {
        let stack = &mut self.stack;
        match op2 {
            27 => {
                let a = stack.pop_fixed()?;
                stack.push_fixed(a)?;
                stack.push_fixed(a)
            }
            28 => {
                let b = stack.pop_fixed()?;
                let a = stack.pop_fixed()?;
                stack.push_fixed(b)?;
                stack.push_fixed(a)
            }
            29 => {
                let index = stack.pop_int()?;
                let size = stack.count();
                if size == 0 {
                    return Ok(());
                }
                let at = match usize::try_from(index) {
                    Err(_) => size - 1,
                    Ok(index) if index >= size => 0,
                    Ok(index) => size - 1 - index,
                };
                let value = stack.get(at)?;
                stack.push_fixed(value)
            }
            _ => {
                let shift = stack.pop_int()?;
                let count = stack.pop_int()?;
                stack.roll(count, shift)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_point_helpers() {
        assert_eq!(sqrt_fixed(0), 0);
        assert_eq!(sqrt_fixed(4 << 16), 2 << 16);
        assert_eq!(sqrt_fixed(2 << 16), 92682);
        assert_eq!(next_random(1), 270_369);
    }
}
