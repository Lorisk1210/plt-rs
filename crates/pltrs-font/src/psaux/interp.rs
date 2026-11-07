//! The interpreter loop of `cf2_interpT2CharString`: numbers, subroutine
//! calls, stems and hint masks, `endchar` and the CFF2 blends (the drawing
//! operators are in `draw`, the escaped ones in `escape`).

use super::blues::Blues;
use super::buffer::Buffer;
use super::builder::Builder;
use super::error::CharstringError;
use super::hints::StemHint;
use super::path::{GlyphPath, Point, Scale};
use super::stack::Stack;
use super::{Charstrings, Flavor};
use crate::fixed::{div_fix, mul_fix};
use crate::outline::Outline;

/// The charstring plus `CF2_MAX_SUBR` (and `T1_MAX_SUBRS_CALLS`) nested
/// subroutines.
const MAX_CALLS: usize = 17;

/// `CF2_STORAGE_SIZE`: the `put`/`get` registers.
pub(super) const STORAGE_SIZE: usize = 32;

/// The operators one charstring invocation may run ("matches Avalon").
const INSTRUCTION_LIMIT: u32 = 20_000_000;

/// The seed of the `random` operator. FreeType seeds it from memory
/// addresses; this is the seed it falls back to.
const RANDOM_SEED: u32 = 0x7384;

/// One-byte operators.
const RETURN: u8 = 11;
const ESCAPE: u8 = 12;
const ENDCHAR: u8 = 14;
const VSINDEX: u8 = 15;
const BLEND: u8 = 16;

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

/// `CF2_MAX_SIZE`: the largest size the engine hints, in pixels per em.
const MAX_HINTED_PPEM: i64 = 2000;

/// `cf2_getScaleAndHintFlag` and `cf2_checkTransform`: how a glyph of a
/// font of `units_per_em` is decoded at a size of the 16.16 scales
/// `x_scale` and `y_scale`: hinted when asked for, at the engine's scales
/// (the size's plus 32, divided by 64), unless the size is too large for
/// the engine (FreeType then decodes the glyph unhinted and scales it).
///
/// # Errors
///
/// For sizes the engine refuses: a scale that is not positive, or a font
/// of more than 32 767 units per em.
pub(crate) fn engine_rendering(
    x_scale: i64,
    y_scale: i64,
    hinting: bool,
    units_per_em: u16,
) -> Result<Rendering, CharstringError> {
    let engine = |scale: i64| (scale as i32).wrapping_add(32) / 64;
    let (x, y) = (engine(x_scale), engine(y_scale));
    if hinting && (x <= 0 || y <= 0) {
        return Err(CharstringError::InvalidFont("invalid size"));
    }
    if units_per_em > 0x7FFF {
        return Err(CharstringError::InvalidFont("glyph too big"));
    }
    let largest = div_fix(MAX_HINTED_PPEM << 16, i64::from(units_per_em) << 16);
    if hinting && i64::from(x) <= largest && i64::from(y) <= largest {
        Ok(Rendering::Hinted { x, y })
    } else {
        Ok(Rendering::Unhinted)
    }
}

/// A decoded glyph: the outline (in whole font units, or 26.6 pixels when
/// hinted) and, for Type 1, the metrics in 16.16 font units.
#[derive(Clone, Debug)]
pub(crate) struct Decoded {
    pub(crate) outline: Outline,
    pub(crate) advance: (i32, i32),
}

/// The state shared by a glyph and its `seac` components: the outline,
/// the side bearing and advance, the flex state and the blend.
pub(super) struct Decoder<'a> {
    pub(super) font: &'a dyn Charstrings<'a>,
    pub(super) flavor: Flavor,
    pub(super) scale: Scale,
    pub(super) hinted: bool,
    pub(super) blues: Blues,
    pub(super) out: Builder,
    pub(super) left_bearing: (i32, i32),
    pub(super) advance: (i32, i32),
    pub(super) flex_state: bool,
    pub(super) flex_vectors: u32,
    pub(super) random: u32,
    /// The item variation data `blend` uses (CFF2).
    vsindex: u32,
    /// The blend vector last built, and the data it is for.
    blend: Option<(u32, Vec<i32>)>,
    /// Whether a `blend` ran, after which `vsindex` is an error.
    blended: bool,
}

/// Decodes the glyph `charstring` of `font`: unhinted, as FreeType loads
/// it with `FT_LOAD_NO_SCALE` or `FT_LOAD_NO_HINTING`, or hinted at a size.
pub(crate) fn decode<'a>(
    font: &'a dyn Charstrings<'a>,
    charstring: &'a [u8],
    rendering: Rendering,
) -> Result<Decoded, CharstringError> {
    let (scale, hinted) = match rendering {
        Rendering::Unhinted => (Scale::UNHINTED, false),
        Rendering::Hinted { x, y } => (Scale { x, c: 0, y }, true),
    };
    let private = font.private();
    let mut decoder = Decoder {
        font,
        flavor: font.flavor(),
        scale,
        hinted,
        blues: Blues::new(private, scale.y),
        out: Builder::default(),
        left_bearing: (0, 0),
        advance: (0, 0),
        flex_state: false,
        flex_vectors: 0,
        random: RANDOM_SEED,
        vsindex: private.vsindex,
        blend: None,
        blended: false,
    };
    Invocation::new(&mut decoder, false, (0, 0)).run(charstring)?;
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

/// The passes over a charstring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pass {
    /// The first pass over a Type 1 charstring collects the stem hints
    /// and skips drawing.
    Hints,
    /// The pass that draws (the only one of CFF and CFF2 charstrings).
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
    /// Which pass is running.
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
        let path = GlyphPath::new(
            decoder.flavor,
            decoder.scale,
            decoder.hinted,
            decoder.blues.clone(),
            cur.1,
        );
        let type1 = decoder.flavor == Flavor::Type1;
        Self {
            stack: Stack::new(decoder.font.stack_size()),
            decoder,
            path,
            have_width: !type1,
            known_results: 0,
            results: [0; 3],
            result_count: 0,
            large_int: false,
            pass: if type1 { Pass::Hints } else { Pass::Draw },
            storage: [0; STORAGE_SIZE],
            flex_store: [0; 6],
            doing_seac,
            cur,
            instructions_left: INSTRUCTION_LIMIT,
        }
    }

    pub(super) fn is_type1(&self) -> bool {
        self.decoder.flavor == Flavor::Type1
    }

    /// Runs `code` to its `endchar` (or its end).
    pub(super) fn run(mut self, code: &'a [u8]) -> Result<(), CharstringError> {
        let type1 = self.is_type1();
        let cff2 = self.decoder.flavor == Flavor::Cff2;
        let mut calls = vec![Buffer::new(code)];
        loop {
            let depth = calls.len() - 1;
            let current = &mut calls[depth];
            let op = if current.is_end() {
                if depth > 0 { RETURN } else { ENDCHAR }
            } else {
                match current.read() {
                    // Explicit `return` and `endchar` are no CFF2 operators.
                    RETURN | ENDCHAR if cff2 => 0,
                    op => op,
                }
            };
            if type1 {
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
            }
            self.instructions_left -= 1;
            if self.instructions_left == 0 {
                return Err(CharstringError::Limit);
            }
            let flow = match op {
                10 | 29 => self.call(op, &mut calls)?,
                RETURN => {
                    if calls.len() < 2 {
                        return Err(CharstringError::Malformed("return from the charstring"));
                    }
                    calls.pop();
                    Flow::Keep
                }
                ESCAPE => {
                    let op2 = calls[depth].read();
                    self.escape(op2)?
                }
                ENDCHAR => self.end_char(&mut calls)?,
                VSINDEX => self.vsindex()?,
                BLEND => self.blend()?,
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

    /// `callsubr` and `callgsubr` (which fails in Type 1 fonts: they have
    /// no global subroutines).
    fn call(&mut self, op: u8, calls: &mut Vec<Buffer<'a>>) -> Result<Flow, CharstringError> {
        if calls.len() >= MAX_CALLS {
            return Err(CharstringError::Limit);
        }
        let number = self.stack.pop_int()?;
        let font = self.decoder.font;
        let code = if op == 10 {
            font.local_subr(number)
        } else {
            font.global_subr(number)
        };
        let code = code.ok_or(CharstringError::MissingSubr(i64::from(number)))?;
        calls.push(Buffer::new(code));
        Ok(Flow::Keep)
    }

    /// `endchar`: after the first pass over a Type 1 charstring, rewinds
    /// for the second; otherwise closes the path, and runs the implied
    /// `seac` of a CFF charstring ending with four arguments.
    fn end_char(&mut self, calls: &mut Vec<Buffer<'a>>) -> Result<Flow, CharstringError> {
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
        if self.decoder.flavor == Flavor::Cff && self.stack.count() > 1 {
            self.implied_seac()?;
        }
        Ok(Flow::Exit)
    }

    /// The `seac` of a CFF `endchar` with the arguments `adx ady bchar
    /// achar`: the accent drawn at the offset, then the base.
    fn implied_seac(&mut self) -> Result<(), CharstringError> {
        if self.doing_seac {
            return Err(CharstringError::Malformed("nested seac"));
        }
        let accent = self.stack.pop_int()?;
        let base = self.stack.pop_int()?;
        let y = self.stack.pop_fixed()?;
        let x = self.stack.pop_fixed()?;
        let font = self.decoder.font;
        let accent = font.standard_glyph(accent)?;
        Invocation::new(&mut *self.decoder, true, (x, y)).run(accent)?;
        let base = font.standard_glyph(base)?;
        Invocation::new(&mut *self.decoder, true, (0, 0)).run(base)
    }

    /// `hintmask` and `cntrmask`: pending arguments are vertical stems,
    /// then the mask bytes follow.
    fn mask(&mut self, op: u8, code: &mut Buffer<'_>) -> Result<Flow, CharstringError> {
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

    /// A number: integers, except the 32-bit numbers of CFF charstrings,
    /// which are 16.16; those of Type 1 charstrings are integers that must
    /// be divided down.
    fn number(&mut self, op: u8, code: &mut Buffer<'_>) -> Result<Flow, CharstringError> {
        let value = match op {
            32..=246 => i32::from(op) - 139,
            247..=250 => (i32::from(op) - 247) * 256 + i32::from(code.read()) + 108,
            251..=254 => -((i32::from(op) - 251) * 256 + i32::from(code.read())) - 108,
            _ => {
                let value =
                    i32::from_be_bytes([code.read(), code.read(), code.read(), code.read()]);
                if !self.is_type1() {
                    self.stack.push_fixed(value)?;
                    return Ok(Flow::Keep);
                }
                if !(-32000..=32000).contains(&value) {
                    self.large_int = true;
                }
                value
            }
        };
        self.stack.push_int(value)?;
        Ok(Flow::Keep)
    }

    /// `hstem`, `vstem` and their `hm` forms: the stems of a CFF
    /// charstring are ignored once a hint mask was read; those of a Type 1
    /// charstring are relative to the side bearing.
    pub(super) fn declare_stems(&mut self, axis: Axis) -> Result<Flow, CharstringError> {
        if !self.is_type1() && self.path.hints.mask.valid {
            return Ok(Flow::Clear);
        }
        let offset = match (self.is_type1(), axis) {
            (false, _) => 0,
            (true, Axis::Horizontal) => self.decoder.left_bearing.1,
            (true, Axis::Vertical) => self.decoder.left_bearing.0,
        };
        self.stems(axis, offset)?;
        Ok(Flow::Clear)
    }

    /// `cf2_doStems`: stem pairs relative to `offset`; an odd count skips
    /// the first value (a width).
    pub(super) fn stems(&mut self, axis: Axis, offset: i32) -> Result<(), CharstringError> {
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

    /// `vsindex`: the item variation data later blends use (CFF2 only; an
    /// error after a blend).
    fn vsindex(&mut self) -> Result<Flow, CharstringError> {
        if self.decoder.flavor != Flavor::Cff2 {
            return Ok(Flow::Clear);
        }
        if self.decoder.blended {
            return Err(CharstringError::Malformed("vsindex after blend"));
        }
        if let Ok(vsindex) = u32::try_from(self.stack.pop_int()?) {
            self.decoder.vsindex = vsindex;
        }
        Ok(Flow::Clear)
    }

    /// `blend` (`cf2_doBlend`): each of the `n` default values on the
    /// stack plus its deltas weighted by the blend vector, the deltas
    /// dropped (CFF2 only).
    fn blend(&mut self) -> Result<Flow, CharstringError> {
        if self.decoder.flavor != Flavor::Cff2 {
            return Ok(Flow::Clear);
        }
        let vsindex = self.decoder.vsindex;
        if self
            .decoder
            .blend
            .as_ref()
            .is_none_or(|(at, _)| *at != vsindex)
        {
            let vector = self.decoder.font.blend_vector(vsindex)?;
            self.decoder.blend = Some((vsindex, vector));
        }
        let Some((_, weights)) = &self.decoder.blend else {
            return Err(CharstringError::Malformed("no blend vector"));
        };
        let blends = self.stack.pop_int()? as u32 as usize;
        if blends > self.decoder.font.stack_size() {
            return Err(CharstringError::Malformed("too many blends"));
        }
        let operands = blends * weights.len();
        let base = self
            .stack
            .count()
            .checked_sub(operands)
            .ok_or(CharstringError::Malformed("blend stack underflow"))?;
        let mut delta = base + blends;
        for i in 0..blends {
            let mut sum = self.stack.get(base + i)?;
            for &weight in &weights[1..] {
                let term = mul_fix(i64::from(weight), i64::from(self.stack.get(delta)?));
                sum = add(sum, term as i32);
                delta += 1;
            }
            self.stack.set(base + i, sum)?;
        }
        self.stack.truncate(base + blends);
        self.decoder.blended = true;
        Ok(Flow::Keep)
    }
}
