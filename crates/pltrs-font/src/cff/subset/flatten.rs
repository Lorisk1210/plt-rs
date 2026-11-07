//! Charstrings without subroutines (fontTools' `desubroutinize`): every
//! `callsubr` and `callgsubr` replaced by the code it calls, so that a
//! subset needs none of the font's subroutines.
//!
//! The copy follows the operand stack as far as a subset needs it: the
//! literal numbers that name subroutines, the stem hints that size hint
//! masks, `blend`s, and the operands of an implied `seac`.

use super::super::Version;
use super::super::glyphs::Glyph;
use crate::error::{FontError, Result};
use crate::psaux::Charstrings;
use crate::types::Tag;

/// The deepest nesting of subroutine calls (Type 2's limit).
const MAX_NESTING: usize = 10;

/// The longest charstring a flattening may give; longer is an error, not
/// a large subset.
const MAX_LENGTH: usize = 0xFFFF;

const HSTEM: u8 = 1;
const VSTEM: u8 = 3;
const CALLSUBR: u8 = 10;
const RETURN: u8 = 11;
const ESCAPE: u8 = 12;
const ENDCHAR: u8 = 14;
const VSINDEX: u8 = 15;
const BLEND: u8 = 16;
const HSTEMHM: u8 = 18;
const HINTMASK: u8 = 19;
const CNTRMASK: u8 = 20;
const VSTEMHM: u8 = 23;
const SHORTINT: u8 = 28;
const CALLGSUBR: u8 = 29;

/// A glyph's charstring without subroutine calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::cff) struct Flattened {
    /// The charstring.
    pub(in crate::cff) program: Vec<u8>,
    /// The Standard Encoding codes of the base and accent glyphs of an
    /// `endchar` that implies a `seac`.
    pub(in crate::cff) seac: Option<(i64, i64)>,
}

/// The state of one flattening.
struct Flattener<'g, 'c, 'a> {
    glyph: &'g Glyph<'c, 'a>,
    tag: Tag,
    out: Vec<u8>,
    /// The operand stack: each operand's value when a literal number
    /// pushed it.
    stack: Vec<Option<i64>>,
    /// Where in `out` the last token starts, when it is a number.
    last_number: Option<usize>,
    stems: usize,
    vsindex: u32,
    seac: Option<(i64, i64)>,
    ended: bool,
}

/// `glyph`'s charstring `code` with its subroutines inlined.
///
/// # Errors
///
/// When the charstring is malformed, calls a subroutine the font lacks or
/// through an index it computes, nests calls too deeply, or grows too
/// long.
pub(in crate::cff) fn flatten(code: &[u8], glyph: &Glyph<'_, '_>) -> Result<Flattened> {
    let mut flattener = Flattener {
        glyph,
        tag: glyph.cff.version.tag(),
        out: Vec::with_capacity(code.len()),
        stack: Vec::new(),
        last_number: None,
        stems: 0,
        vsindex: glyph.private().vsindex,
        seac: None,
        ended: false,
    };
    flattener.run(code, 0)?;
    Ok(Flattened {
        program: flattener.out,
        seac: flattener.seac,
    })
}

/// The value and length of the number whose first byte is at `at`.
fn number(code: &[u8], at: usize, tag: Tag) -> Result<(Option<i64>, usize)> {
    let bytes = |length: usize| code.get(at..at + length).ok_or(FontError::Truncated(tag));
    let b0 = i64::from(code[at]);
    Ok(match code[at] {
        32..=246 => (Some(b0 - 139), 1),
        247..=250 => (Some((b0 - 247) * 256 + i64::from(bytes(2)?[1]) + 108), 2),
        251..=254 => (Some(-(b0 - 251) * 256 - i64::from(bytes(2)?[1]) - 108), 2),
        SHORTINT => {
            let b = bytes(3)?;
            (Some(i64::from(i16::from_be_bytes([b[1], b[2]]))), 3)
        }
        // A 16.16 number: a value only when it is whole.
        _ => {
            let b = bytes(5)?;
            let fixed = i64::from(i32::from_be_bytes([b[1], b[2], b[3], b[4]]));
            ((fixed.trailing_zeros() >= 16).then_some(fixed >> 16), 5)
        }
    })
}

/// How many operands an escaped arithmetic operator `op` pops and
/// pushes, if it is one.
const fn arithmetic(op: u8) -> Option<(usize, usize)> {
    Some(match op {
        // and, or, add, sub, div, eq, mul
        3 | 4 | 10 | 11 | 12 | 15 | 24 => (2, 1),
        // not, abs, neg, get, sqrt, index
        5 | 9 | 14 | 21 | 26 | 29 => (1, 1),
        // drop
        18 => (1, 0),
        // put, roll
        20 | 30 => (2, 0),
        // ifelse
        22 => (4, 1),
        // random
        23 => (0, 1),
        // dup
        27 => (1, 2),
        // exch
        28 => (2, 2),
        _ => return None,
    })
}

impl Flattener<'_, '_, '_> {
    fn malformed(&self, reason: &'static str) -> FontError {
        FontError::malformed(self.tag, reason)
    }

    /// Appends `bytes` to the output.
    fn emit(&mut self, bytes: &[u8]) -> Result<()> {
        if self.out.len() + bytes.len() > MAX_LENGTH {
            return Err(self.malformed("charstring too long without subroutines"));
        }
        self.out.extend_from_slice(bytes);
        Ok(())
    }

    /// Pops `count` operands.
    fn pop(&mut self, count: usize) -> Result<()> {
        let depth = self
            .stack
            .len()
            .checked_sub(count)
            .ok_or(self.malformed("charstring stack underflow"))?;
        self.stack.truncate(depth);
        Ok(())
    }

    /// The literal number on top of the stack, which an operator takes.
    fn top_number(&self) -> Result<i64> {
        self.stack
            .last()
            .copied()
            .flatten()
            .ok_or(self.malformed("charstring operand computed at run time"))
    }

    /// Copies `code`, a charstring or a subroutine `depth` calls deep.
    fn run(&mut self, code: &[u8], depth: usize) -> Result<()> {
        let mut at = 0;
        while at < code.len() && !self.ended {
            let op = code[at];
            if op == SHORTINT || op >= 32 {
                let (value, length) = number(code, at, self.tag)?;
                if self.stack.len() >= self.glyph.stack_size() {
                    return Err(self.malformed("charstring stack overflow"));
                }
                self.last_number = Some(self.out.len());
                self.emit(&code[at..at + length])?;
                self.stack.push(value);
                at += length;
                continue;
            }
            let last_number = self.last_number.take();
            at += 1;
            match op {
                CALLSUBR | CALLGSUBR => {
                    let index = i32::try_from(self.top_number()?)
                        .map_err(|_| self.malformed("subroutine index out of range"))?;
                    let start = last_number.ok_or(self.malformed("computed subroutine index"))?;
                    self.out.truncate(start);
                    self.pop(1)?;
                    let subr = if op == CALLSUBR {
                        self.glyph.local_subr(index)
                    } else {
                        self.glyph.global_subr(index)
                    }
                    .ok_or(self.malformed("missing subroutine"))?;
                    if depth >= MAX_NESTING {
                        return Err(self.malformed("subroutines nested too deeply"));
                    }
                    self.run(subr, depth + 1)?;
                }
                // CFF2 has no `return` or `endchar`; FreeType ignores them.
                RETURN | ENDCHAR if self.glyph.cff.version == Version::Cff2 => {}
                RETURN if depth > 0 => return Ok(()),
                RETURN => return Err(self.malformed("return outside a subroutine")),
                ENDCHAR => {
                    if let [.., Some(_), Some(_), Some(base), Some(accent)] = self.stack[..] {
                        self.seac = Some((base, accent));
                    }
                    self.emit(&[op])?;
                    self.ended = true;
                }
                HSTEM | VSTEM | HSTEMHM | VSTEMHM => {
                    self.stems += self.stack.len() / 2;
                    self.emit(&[op])?;
                    self.stack.clear();
                }
                HINTMASK | CNTRMASK => {
                    // Operands before a mask are vertical stems.
                    self.stems += self.stack.len() / 2;
                    let mask = at..at + self.stems.div_ceil(8);
                    let bytes = code
                        .get(mask.clone())
                        .ok_or(FontError::Truncated(self.tag))?;
                    self.emit(&[op])?;
                    self.emit(bytes)?;
                    self.stack.clear();
                    at = mask.end;
                }
                VSINDEX if self.glyph.cff.version == Version::Cff2 => {
                    self.vsindex = u32::try_from(self.top_number()?)
                        .map_err(|_| self.malformed("vsindex out of range"))?;
                    self.pop(1)?;
                    self.emit(&[op])?;
                }
                BLEND if self.glyph.cff.version == Version::Cff2 => {
                    let values = usize::try_from(self.top_number()?)
                        .map_err(|_| self.malformed("blend of a negative count"))?;
                    let regions = self
                        .glyph
                        .cff
                        .vstore
                        .region_count(self.vsindex)
                        .ok_or(self.malformed("blend of missing variation data"))?;
                    self.pop(values * (regions + 1) + 1)?;
                    self.stack.extend(std::iter::repeat_n(None, values));
                    self.emit(&[op])?;
                }
                ESCAPE => {
                    let escaped = *code.get(at).ok_or(FontError::Truncated(self.tag))?;
                    at += 1;
                    match arithmetic(escaped) {
                        Some((pops, pushes)) => {
                            self.pop(pops)?;
                            self.stack.extend(std::iter::repeat_n(None, pushes));
                        }
                        None => self.stack.clear(),
                    }
                    self.emit(&[op, escaped])?;
                }
                _ => {
                    self.emit(&[op])?;
                    self.stack.clear();
                }
            }
        }
        Ok(())
    }
}
