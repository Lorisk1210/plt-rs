//! The interpreter loop of `cf2_interpT2CharString` in Type 1 mode: the
//! one-byte operators, numbers and subroutine calls (the escaped
//! operators are in `escape`).

use super::blues::{Blues, PrivateDict};
use super::buffer::Buffer;
use super::builder::Builder;
use super::hints::StemHint;
use super::path::{GlyphPath, Point, Scale};
use super::stack::Stack;
use crate::outline::Outline;
use crate::type1::error::Type1Error;

/// The charstring plus `T1_MAX_SUBRS_CALLS` nested subroutines.
const MAX_CALLS: usize = 17;

/// `CF2_STORAGE_SIZE`: the `put`/`get` registers.
pub(super) const STORAGE_SIZE: usize = 32;

/// The operators one charstring invocation may run ("matches Avalon").
const INSTRUCTION_LIMIT: u32 = 20_000_000;

/// The seed of the `random` operator. FreeType seeds it from memory
/// addresses for Type 1 fonts; this is the seed it falls back to.
const RANDOM_SEED: u32 = 0x7384;

/// One-byte operators.
const RETURN: u8 = 11;
const ESCAPE: u8 = 12;
const ENDCHAR: u8 = 14;

/// The programs a charstring can reach, decrypted: every glyph (`seac`
/// components are looked up by name) and the subroutines, and the
/// alignment zones that hint them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GlyphSet<'a> {
    pub(crate) names: &'a [String],
    pub(crate) charstrings: &'a [Vec<u8>],
    pub(crate) subrs: &'a [Vec<u8>],
    pub(crate) private: &'a PrivateDict,
}

/// How a glyph is decoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Rendering {
    /// Unhinted, in whole font units.
    Unhinted,
    /// Hinted, in 26.6 pixels, at FreeType's engine scales (the size's
    /// 16.16 scales plus 32, divided by 64).
    Hinted {
        /// The horizontal scale.
        x: i32,
        /// The vertical scale.
        y: i32,
    },
}

/// A decoded glyph: the outline (in whole font units, or 26.6 pixels when
/// hinted) and the metrics in 16.16 font units.
#[derive(Clone, Debug)]
pub(crate) struct Decoded {
    pub(crate) outline: Outline,
    pub(crate) advance: (i32, i32),
}

/// The state shared by a glyph and its `seac` components: the outline,
/// the side bearing and advance, and the flex state.
pub(super) struct Decoder<'a> {
    pub(super) set: GlyphSet<'a>,
    pub(super) scale: Scale,
    pub(super) hinted: bool,
    pub(super) blues: Blues,
    pub(super) out: Builder,
    pub(super) left_bearing: (i32, i32),
    pub(super) advance: (i32, i32),
    pub(super) flex_state: bool,
    pub(super) flex_vectors: u32,
    pub(super) random: u32,
}

/// Decodes glyph `glyph` of `set`: unhinted, as FreeType loads it with
/// `FT_LOAD_NO_SCALE` or `FT_LOAD_NO_HINTING`, or hinted at a size.
pub(crate) fn decode(
    set: GlyphSet<'_>,
    glyph: usize,
    rendering: Rendering,
) -> Result<Decoded, Type1Error> {
    let code = set
        .charstrings
        .get(glyph)
        .ok_or(Type1Error::InvalidFont("glyph index out of range"))?;
    let (scale, hinted) = match rendering {
        Rendering::Unhinted => (Scale::UNHINTED, false),
        Rendering::Hinted { x, y } => (Scale { x, c: 0, y }, true),
    };
    let mut decoder = Decoder {
        set,
        scale,
        hinted,
        blues: Blues::new(set.private, scale.y),
        out: Builder::default(),
        left_bearing: (0, 0),
        advance: (0, 0),
        flex_state: false,
        flex_vectors: 0,
        random: RANDOM_SEED,
    };
    Invocation::new(&mut decoder, false, (0, 0)).run(code)?;
    Ok(Decoded {
        outline: decoder.out.finish(),
        advance: decoder.advance,
    })
}

/// What happens to the operand stack after an operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flow {
    /// Clear it (the operator `break`s out of FreeType's switch).
    Clear,
    /// Keep it (the operator `continue`s).
    Keep,
    /// The charstring is done.
    Exit,
}

/// The two passes over a Type 1 charstring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pass {
    /// The first pass collects the stem hints and skips drawing.
    Hints,
    /// The second pass draws.
    Draw,
}

/// Which stems an operator declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

/// One run of `cf2_interpT2CharString`: a glyph, or a `seac` component.
pub(super) struct Invocation<'d, 'a> {
    pub(super) decoder: &'d mut Decoder<'a>,
    pub(super) stack: Stack,
    pub(super) path: GlyphPath,
    pub(super) have_width: bool,
    /// Results of known OtherSubrs the next `pop`s skip.
    pub(super) known_results: u32,
    /// Arguments of unknown OtherSubrs the next `pop`s return.
    pub(super) results: [i32; 3],
    pub(super) result_count: usize,
    pub(super) large_int: bool,
    /// Which of the two passes is running.
    pub(super) pass: Pass,
    pub(super) storage: [i32; STORAGE_SIZE],
    pub(super) flex_store: [i32; 6],
    pub(super) doing_seac: bool,
    pub(super) cur: Point,
    instructions_left: u32,
}

/// `ADD_INT32` of two character-space coordinates.
pub(super) const fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

impl<'d, 'a> Invocation<'d, 'a> {
    pub(super) fn new(decoder: &'d mut Decoder<'a>, doing_seac: bool, cur: Point) -> Self {
        let path = GlyphPath::new(decoder.scale, decoder.hinted, decoder.blues.clone(), cur.1);
        Self {
            decoder,
            stack: Stack::default(),
            path,
            have_width: false,
            known_results: 0,
            results: [0; 3],
            result_count: 0,
            large_int: false,
            pass: Pass::Hints,
            storage: [0; STORAGE_SIZE],
            flex_store: [0; 6],
            doing_seac,
            cur,
            instructions_left: INSTRUCTION_LIMIT,
        }
    }

    /// Runs `code` to its `endchar` (or its end).
    pub(super) fn run(mut self, code: &'a [u8]) -> Result<(), Type1Error> {
        let mut calls = vec![Buffer::new(code)];
        loop {
            let depth = calls.len() - 1;
            let current = &mut calls[depth];
            let op = if current.is_end() {
                if depth > 0 { RETURN } else { ENDCHAR }
            } else {
                current.read()
            };
            let hint_or_flow = matches!(op, 1 | 3 | 10 | RETURN | ESCAPE | 13 | ENDCHAR);
            if self.pass == Pass::Hints && !hint_or_flow && op < 32 {
                // The first pass skips drawing.
                self.stack.clear();
                continue;
            }
            if self.result_count > 0 && !matches!(op, 10 | RETURN | ESCAPE) && op < 32 {
                self.result_count = 0;
            }
            if self.large_int && op < 32 && op != ESCAPE {
                self.large_int = false;
            }
            self.instructions_left -= 1;
            if self.instructions_left == 0 {
                return Err(Type1Error::CharstringLimit);
            }
            let flow = match op {
                10 | 29 => self.call(op, &mut calls)?,
                RETURN => {
                    if calls.len() < 2 {
                        return Err(Type1Error::BadCharstring("return from the charstring"));
                    }
                    calls.pop();
                    Flow::Keep
                }
                ESCAPE => {
                    let op2 = calls[depth].read();
                    self.escape(op2)?
                }
                ENDCHAR => self.end_char(&mut calls)?,
                19 | 20 => self.mask(op, &mut calls[depth])?,
                28 => {
                    let current = &mut calls[depth];
                    let value = i16::from_be_bytes([current.read(), current.read()]);
                    self.stack.push_int(i32::from(value))?;
                    Flow::Keep
                }
                32..=255 => self.number(op, &mut calls[depth])?,
                _ => self.draw(op)?,
            };
            match flow {
                Flow::Clear => self.stack.clear(),
                Flow::Keep => {}
                Flow::Exit => return Ok(()),
            }
        }
    }

    /// `callsubr` (and `callgsubr`, which fails: Type 1 fonts have no
    /// global subroutines).
    fn call(&mut self, op: u8, calls: &mut Vec<Buffer<'a>>) -> Result<Flow, Type1Error> {
        if calls.len() >= MAX_CALLS {
            return Err(Type1Error::CharstringLimit);
        }
        let number = self.stack.pop_int()?;
        let subrs = self.decoder.set.subrs;
        let code = usize::try_from(number)
            .ok()
            .filter(|_| op == 10)
            .and_then(|number| subrs.get(number))
            .ok_or(Type1Error::MissingSubr(i64::from(number)))?;
        calls.push(Buffer::new(code));
        Ok(Flow::Keep)
    }

    /// `endchar`: after the first pass, rewinds for the second; after the
    /// second, closes the path.
    fn end_char(&mut self, calls: &mut Vec<Buffer<'a>>) -> Result<Flow, Type1Error> {
        if self.pass == Pass::Hints {
            self.path
                .move_to(self.cur.0, self.cur.1, &mut self.decoder.out)?;
            self.pass = Pass::Draw;
            self.path.hints.reset();
            calls.truncate(1);
            calls[0].rewind();
            return Ok(Flow::Clear);
        }
        self.have_width = true;
        self.path.close_open_path(&mut self.decoder.out)?;
        Ok(Flow::Exit)
    }

    /// `hintmask` and `cntrmask`: pending arguments are vertical stems,
    /// then the mask bytes follow.
    fn mask(&mut self, op: u8, code: &mut Buffer<'_>) -> Result<Flow, Type1Error> {
        if self.stack.count() > 1 && self.path.hints.mask.valid {
            return Ok(Flow::Clear);
        }
        self.stems(Axis::Vertical, 0)?;
        if op == 19 {
            let stems = self.path.hints.count();
            self.path.hints.mask.read(code, stems)?;
        } else {
            self.path.counter_mask(code)?;
        }
        Ok(Flow::Clear)
    }

    /// A number: integers in Type 1 mode, even the 32-bit ones, which must
    /// be divided down.
    fn number(&mut self, op: u8, code: &mut Buffer<'_>) -> Result<Flow, Type1Error> {
        let value = match op {
            32..=246 => i32::from(op) - 139,
            247..=250 => (i32::from(op) - 247) * 256 + i32::from(code.read()) + 108,
            251..=254 => -((i32::from(op) - 251) * 256 + i32::from(code.read())) - 108,
            _ => {
                let value =
                    i32::from_be_bytes([code.read(), code.read(), code.read(), code.read()]);
                if !(-32000..=32000).contains(&value) {
                    self.large_int = true;
                }
                value
            }
        };
        self.stack.push_int(value)?;
        Ok(Flow::Keep)
    }

    /// `cf2_doStems`: stem pairs relative to `offset` (the side bearing in
    /// Type 1 mode); an odd count skips the first value.
    pub(super) fn stems(&mut self, axis: Axis, offset: i32) -> Result<(), Type1Error> {
        let count = self.stack.count();
        let mut position = offset;
        let mut stems = Vec::with_capacity(count / 2);
        for i in (count & 1..count).step_by(2) {
            let min = add(position, self.stack.get(i)?);
            let max = add(min, self.stack.get(i + 1)?);
            position = max;
            stems.push(StemHint::new(min, max));
        }
        let hints = &mut self.path.hints;
        match axis {
            Axis::Horizontal => hints.horizontal.extend(stems),
            Axis::Vertical => hints.vertical.extend(stems),
        }
        self.stack.clear();
        self.have_width = true;
        Ok(())
    }

    fn line_to(&mut self, x: i32, y: i32) -> Result<(), Type1Error> {
        self.cur = (x, y);
        self.path.line_to(x, y, &mut self.decoder.out)
    }

    pub(super) fn curve_to(&mut self, points: [Point; 3]) -> Result<(), Type1Error> {
        self.cur = points[2];
        self.path.curve_to(points, &mut self.decoder.out)
    }

    /// A curve from the current point by six deltas.
    fn relative_curve(&mut self, at: usize) -> Result<(), Type1Error> {
        let get = |i| self.stack.get(at + i);
        let p1 = (add(get(0)?, self.cur.0), add(get(1)?, self.cur.1));
        let p2 = (add(get(2)?, p1.0), add(get(3)?, p1.1));
        let p3 = (add(get(4)?, p2.0), add(get(5)?, p2.1));
        self.curve_to([p1, p2, p3])
    }

    /// `rmoveto`, `hmoveto` and `vmoveto`; during a flex the moves only
    /// collect its points.
    fn move_by(&mut self, dx: i32, dy: i32) -> Result<Flow, Type1Error> {
        self.have_width = true;
        self.cur = (add(self.cur.0, dx), add(self.cur.1, dy));
        if !self.decoder.flex_state {
            self.path
                .move_to(self.cur.0, self.cur.1, &mut self.decoder.out)?;
        }
        Ok(Flow::Clear)
    }

    /// The drawing, hinting and width operators.
    fn draw(&mut self, op: u8) -> Result<Flow, Type1Error> {
        let count = self.stack.count();
        match op {
            1 | 18 => self.stems(Axis::Horizontal, self.decoder.left_bearing.1)?,
            3 | 23 => self.stems(Axis::Vertical, self.decoder.left_bearing.0)?,
            4 => {
                let dy = self.stack.pop_fixed()?;
                return self.move_by(0, dy);
            }
            21 => {
                let dy = self.stack.pop_fixed()?;
                let dx = self.stack.pop_fixed()?;
                return self.move_by(dx, dy);
            }
            22 => {
                let dx = self.stack.pop_fixed()?;
                return self.move_by(dx, 0);
            }
            5 => {
                for i in (0..count).step_by(2) {
                    let (dx, dy) = (self.stack.get(i)?, self.stack.get(i + 1)?);
                    self.line_to(add(self.cur.0, dx), add(self.cur.1, dy))?;
                }
            }
            6 | 7 => {
                let mut horizontal = op == 6;
                for i in 0..count {
                    let d = self.stack.get(i)?;
                    let (x, y) = self.cur;
                    if horizontal {
                        self.line_to(add(x, d), y)?;
                    } else {
                        self.line_to(x, add(y, d))?;
                    }
                    horizontal = !horizontal;
                }
            }
            8 | 24 => {
                let mut i = 0;
                while i + 6 <= count {
                    self.relative_curve(i)?;
                    i += 6;
                }
                if op == 24 {
                    let (dx, dy) = (self.stack.get(i)?, self.stack.get(i + 1)?);
                    self.line_to(add(self.cur.0, dx), add(self.cur.1, dy))?;
                }
            }
            25 => {
                let mut i = 0;
                while i + 6 < count {
                    let (dx, dy) = (self.stack.get(i)?, self.stack.get(i + 1)?);
                    self.line_to(add(self.cur.0, dx), add(self.cur.1, dy))?;
                    i += 2;
                }
                while i < count {
                    self.relative_curve(i)?;
                    i += 6;
                }
            }
            26 | 27 => self.flat_curves(op == 27, count)?,
            30 | 31 => self.alternating_curves(op == 31, count)?,
            9 => {
                self.path.close_open_path(&mut self.decoder.out)?;
                self.have_width = true;
                return Ok(Flow::Clear);
            }
            13 => {
                let advance = self.stack.pop_fixed()?;
                self.decoder.advance = (advance, 0);
                let lsb = self.stack.pop_fixed()?;
                self.decoder.left_bearing.0 = add(self.decoder.left_bearing.0, lsb);
                self.have_width = true;
                if self.pass == Pass::Draw {
                    self.cur.0 = add(self.cur.0, lsb);
                }
                return Ok(Flow::Clear);
            }
            // Reserved, and `vsindex` and `blend`, which are CFF2 only.
            _ => return Ok(Flow::Clear),
        }
        self.stack.clear();
        Ok(Flow::Keep)
    }

    /// `vvcurveto` and `hhcurveto`: an optional leading delta across, then
    /// groups of four.
    fn flat_curves(&mut self, horizontal: bool, count1: usize) -> Result<(), Type1Error> {
        let count = count1 & !2;
        let mut i = count1 - count;
        while i < count {
            let lead = if (count - i) & 1 == 1 {
                i += 1;
                Some(self.stack.get(i - 1)?)
            } else {
                None
            };
            let get = |k| self.stack.get(i + k);
            let (x, y) = self.cur;
            let points = if horizontal {
                let y1 = lead.map_or(y, |d| add(d, y));
                let p1 = (add(get(0)?, x), y1);
                let p2 = (add(get(1)?, p1.0), add(get(2)?, y1));
                [p1, p2, (add(get(3)?, p2.0), p2.1)]
            } else {
                let x1 = lead.map_or(x, |d| add(d, x));
                let p1 = (x1, add(get(0)?, y));
                let p2 = (add(get(1)?, x1), add(get(2)?, p1.1));
                [p1, p2, (p2.0, add(get(3)?, p2.1))]
            };
            self.curve_to(points)?;
            i += 4;
        }
        Ok(())
    }

    /// `hvcurveto` and `vhcurveto`: curves alternating between horizontal
    /// and vertical tangents; a fifth value bends the last one's end.
    fn alternating_curves(
        &mut self,
        mut horizontal: bool,
        count1: usize,
    ) -> Result<(), Type1Error> {
        let count = count1 & !2;
        let mut i = count1 - count;
        while i < count {
            let get = |k| self.stack.get(i + k);
            let (x, y) = self.cur;
            let last = count - i == 5;
            let points = if horizontal {
                let p1 = (add(get(0)?, x), y);
                let p2 = (add(get(1)?, p1.0), add(get(2)?, p1.1));
                let y3 = add(get(3)?, p2.1);
                let x3 = if last { add(get(4)?, p2.0) } else { p2.0 };
                [p1, p2, (x3, y3)]
            } else {
                let p1 = (x, add(get(0)?, y));
                let p2 = (add(get(1)?, p1.0), add(get(2)?, p1.1));
                let x3 = add(get(3)?, p2.0);
                let y3 = if last { add(get(4)?, p2.1) } else { p2.1 };
                [p1, p2, (x3, y3)]
            };
            if last {
                i += 1;
            }
            horizontal = !horizontal;
            self.curve_to(points)?;
            i += 4;
        }
        Ok(())
    }
}
