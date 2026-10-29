//! matplotlib's `Type1Font.subset`: a copy of the font that only defines
//! the glyphs some characters need, found by simulating their charstrings
//! (matplotlib's `_CharstringSimulator`) to follow `seac` components and
//! subroutine calls.

use std::collections::{BTreeSet, HashSet};

use super::crypt::{Type1Cipher, encrypt};
use super::error::Type1Error;
use super::font::Type1Font;
use super::props::Type1Encoding;
use super::python::{index, latin1, slice, slice_from};
use super::standard::standard_encoding;

/// How deeply subroutine calls may nest, about where Python's recursion
/// limit stops matplotlib.
const MAX_DEPTH: usize = 400;

/// The most operators one glyph's simulation may run: subroutines that
/// call each other several times per level multiply exponentially.
const MAX_STEPS: u32 = 1 << 20;

/// A Python number on the simulated stacks.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Num {
    Int(i64),
    Float(f64),
}

impl Num {
    /// Python's `int()`.
    fn truncate(self) -> Result<i64, Type1Error> {
        match self {
            Self::Int(value) => Ok(value),
            Self::Float(value) if value.is_finite() => Ok(value.trunc() as i64),
            Self::Float(_) => Err(Type1Error::BadCharstring("non-finite integer argument")),
        }
    }

    fn to_f64(self) -> f64 {
        match self {
            Self::Int(value) => value as f64,
            Self::Float(value) => value,
        }
    }

    /// Python's `self == value`.
    fn is(self, value: i64) -> bool {
        match self {
            Self::Int(number) => number == value,
            Self::Float(number) => number.to_bits() == (value as f64).to_bits(),
        }
    }
}

/// The operators the simulation tells apart.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Number(i64),
    /// An operator that only clears the stack in the simulation.
    Clear,
    Seac,
    Div,
    CallOtherSubr,
    CallSubr,
    Pop,
    Return,
    EndChar,
}

/// matplotlib's `_charstring_tokens`: the operators and integers of a
/// decrypted charstring, decoded one at a time.
struct Ops<'a> {
    bytes: std::slice::Iter<'a, u8>,
}

impl Ops<'_> {
    fn byte(&mut self) -> Result<u8, Type1Error> {
        self.bytes
            .next()
            .copied()
            .ok_or(Type1Error::BadCharstring("truncated operator or number"))
    }

    fn next_op(&mut self) -> Result<Option<Op>, Type1Error> {
        let Some(&byte) = self.bytes.next() else {
            return Ok(None);
        };
        let op = match byte {
            32..=246 => Op::Number(i64::from(byte) - 139),
            247..=250 => Op::Number((i64::from(byte) - 247) * 256 + i64::from(self.byte()?) + 108),
            251..=254 => Op::Number(-(i64::from(byte) - 251) * 256 - i64::from(self.byte()?) - 108),
            255 => {
                let mut bytes = [0; 4];
                for slot in &mut bytes {
                    *slot = self.byte()?;
                }
                Op::Number(i64::from(i32::from_be_bytes(bytes)))
            }
            12 => match self.byte()? {
                0..=2 | 7 | 33 => Op::Clear,
                6 => Op::Seac,
                12 => Op::Div,
                16 => Op::CallOtherSubr,
                17 => Op::Pop,
                _ => return Err(Type1Error::BadCharstring("unknown escaped operator")),
            },
            1 | 3..=9 | 13 | 21 | 22 | 30 | 31 => Op::Clear,
            10 => Op::CallSubr,
            11 => Op::Return,
            14 => Op::EndChar,
            _ => return Err(Type1Error::BadCharstring("unknown operator")),
        };
        Ok(Some(op))
    }
}

/// What a glyph or subroutine is.
#[derive(Clone, Copy, Debug)]
enum Program<'a> {
    Glyph(&'a str),
    Subr(i64),
}

/// The simulation of one glyph: which glyphs and subroutines it uses.
struct Simulator<'a> {
    font: &'a Type1Font,
    buildchar: Vec<Num>,
    postscript: Vec<Num>,
    glyphs: BTreeSet<String>,
    subrs: BTreeSet<i64>,
    depth: usize,
    steps: u32,
}

impl<'a> Simulator<'a> {
    fn new(font: &'a Type1Font) -> Self {
        Self {
            font,
            buildchar: Vec::new(),
            postscript: Vec::new(),
            glyphs: BTreeSet::new(),
            subrs: BTreeSet::new(),
            depth: 0,
            steps: 0,
        }
    }

    /// Runs a glyph or subroutine up to its `return` or `endchar`.
    fn run(&mut self, program: Program<'_>) -> Result<(), Type1Error> {
        let prop = self.font.prop();
        let code = match program {
            Program::Glyph(name) => {
                let code = prop
                    .char_strings()
                    .get(name)
                    .ok_or_else(|| Type1Error::MissingGlyph(name.to_owned()))?;
                self.glyphs.insert(name.to_owned());
                code
            }
            Program::Subr(number) => {
                let subrs = prop.subrs().ok_or(Type1Error::MissingKey("Subrs"))?;
                let at = index(subrs.len(), number).ok_or(Type1Error::MissingSubr(number))?;
                self.subrs.insert(number);
                subrs[at].decrypted()
            }
        };
        if self.depth >= MAX_DEPTH {
            return Err(Type1Error::CharstringLimit);
        }
        self.depth += 1;
        let mut ops = Ops { bytes: code.iter() };
        while let Some(op) = ops.next_op()? {
            if matches!(op, Op::Return | Op::EndChar) {
                break;
            }
            self.steps += 1;
            if self.steps > MAX_STEPS {
                return Err(Type1Error::CharstringLimit);
            }
            self.step(op)?;
        }
        self.depth -= 1;
        Ok(())
    }

    /// The last two values of the stack (`stack[-2:]`).
    fn last_two(&self) -> Result<(Num, Num), Type1Error> {
        match self.buildchar[..] {
            [.., first, second] => Ok((first, second)),
            _ => Err(Type1Error::BadCharstring("stack underflow")),
        }
    }

    fn step(&mut self, op: Op) -> Result<(), Type1Error> {
        match op {
            Op::Number(value) => self.buildchar.push(Num::Int(value)),
            Op::Clear => self.buildchar.clear(),
            Op::Seac => {
                let codes = &self.buildchar[slice(self.buildchar.len(), 3, 5)];
                for code in codes {
                    let code = code.truncate()?;
                    let name = u8::try_from(code)
                        .ok()
                        .and_then(|code| standard_encoding().find(|&(c, _)| c == code))
                        .map(|(_, name)| name)
                        .ok_or(Type1Error::NotStandardCode(code))?;
                    self.glyphs.insert(name.to_owned());
                }
                self.buildchar.clear();
            }
            Op::Div => {
                let (num1, num2) = self.last_two()?;
                let quotient = if num2.to_f64() == 0.0 {
                    Num::Int(0)
                } else {
                    Num::Float(num1.to_f64() / num2.to_f64())
                };
                self.buildchar.truncate(self.buildchar.len() - 2);
                self.buildchar.push(quotient);
            }
            Op::CallOtherSubr => {
                let (count, othersubr) = self.last_two()?;
                let count = count.truncate()?;
                let len = self.buildchar.len();
                let first = (-2i64).saturating_sub(count);
                let args = &self.buildchar[slice(len, first, -2)];
                if othersubr.is(3) {
                    let arg = *args
                        .first()
                        .ok_or(Type1Error::BadCharstring("othersubr 3 without argument"))?;
                    self.postscript.push(arg);
                } else {
                    self.postscript.extend(args.iter().rev());
                }
                self.buildchar.truncate(slice_from(len, first));
            }
            Op::CallSubr => {
                let subr = self
                    .buildchar
                    .pop()
                    .ok_or(Type1Error::BadCharstring("callsubr on an empty stack"))?;
                self.run(Program::Subr(subr.truncate()?))?;
            }
            Op::Pop => {
                let value = self.postscript.pop().unwrap_or(Num::Int(0));
                self.buildchar.push(value);
            }
            Op::Return | Op::EndChar => {}
        }
        Ok(())
    }
}

impl Type1Font {
    /// A copy of the font that only defines the glyphs the character codes
    /// `characters` need (through the font's encoding, `seac` accents and
    /// `.notdef`), with unused subroutines reduced to stubs, the encoding
    /// limited to `characters`, `name_prefix` before the font name and no
    /// `UniqueID` (matplotlib's `Type1Font.subset`, which its PDF backend
    /// embeds).
    pub fn subset(
        &self,
        characters: impl IntoIterator<Item = u32>,
        name_prefix: &str,
    ) -> Result<Self, Type1Error> {
        let characters: HashSet<i64> = characters.into_iter().map(i64::from).collect();
        let prop = self.prop();
        let encoding = prop.encoding().ok_or(Type1Error::MissingKey("Encoding"))?;
        let kept = Type1Encoding::from_codes(
            encoding
                .iter()
                .filter(|(code, _)| characters.contains(code))
                .map(|(code, glyph)| (code, glyph.to_owned())),
        );
        let mut todo: BTreeSet<String> = kept.iter().map(|(_, glyph)| glyph.to_owned()).collect();
        todo.insert(".notdef".to_owned());
        let mut done = BTreeSet::new();
        let mut used_subrs: BTreeSet<i64> = (0..4).collect();
        while let Some(glyph) = todo.pop_first() {
            let mut simulator = Simulator::new(self);
            simulator.run(Program::Glyph(&glyph))?;
            done.insert(glyph);
            todo.extend(simulator.glyphs.into_iter().filter(|g| !done.contains(g)));
            used_subrs.extend(simulator.subrs);
        }
        let char_strings = self.subset_char_strings(&done)?;
        let subrs = self.subset_subrs(&used_subrs)?;
        let name = format!("/FontName /{name_prefix}{} def", prop.font_name());
        let mut replacements = self.replace_defined("FontName", &name)?;
        replacements.push((self.first_span("CharStrings")?, char_strings));
        replacements.push((self.first_span("Subrs")?, subrs));
        replacements.push((
            self.first_span("Encoding")?,
            super::edit::postscript_encoding(&kept)?,
        ));
        replacements.extend(self.replace_all("UniqueID", "")?);
        self.rewrite(replacements)
    }

    /// One `<length> RD <binary> <procedure>` entry body.
    fn entry(&self, prefix: &str, program: &[u8], procedure: &str) -> Result<Vec<u8>, Type1Error> {
        let encrypted = encrypt(program, Type1Cipher::Charstring, self.prop().len_iv);
        let rd = &self.abbreviations().rd;
        let mut out = latin1(&format!("{prefix} {} {rd} ", encrypted.len()))?;
        out.extend(encrypted);
        out.push(b' ');
        out.extend(latin1(procedure)?);
        Ok(out)
    }

    /// The `/CharStrings` dictionary of `glyphs` in name order (matplotlib's
    /// `_subset_charstrings`).
    fn subset_char_strings(&self, glyphs: &BTreeSet<String>) -> Result<Vec<u8>, Type1Error> {
        let char_strings = self.prop().char_strings();
        let mut out = format!("/CharStrings {} dict dup begin", glyphs.len()).into_bytes();
        for glyph in glyphs {
            let program = char_strings
                .get(glyph)
                .ok_or_else(|| Type1Error::MissingGlyph(glyph.clone()))?;
            out.push(b'\n');
            out.extend(self.entry(&format!("/{glyph}"), program, &self.abbreviations().nd)?);
        }
        out.extend_from_slice(b"\nend\n");
        Ok(out)
    }

    /// The `/Subrs` array with the subroutines not in `used` replaced by a
    /// bare `return` (matplotlib's `_subset_subrs`).
    fn subset_subrs(&self, used: &BTreeSet<i64>) -> Result<Vec<u8>, Type1Error> {
        let subrs = self.prop().subrs().ok_or(Type1Error::MissingKey("Subrs"))?;
        let mut out = format!("/Subrs {} array", subrs.len()).into_bytes();
        for (number, subr) in subrs.iter().enumerate() {
            let program = if used.contains(&(number as i64)) {
                subr.decrypted()
            } else {
                b"\x0b"
            };
            out.push(b'\n');
            out.extend(self.entry(&format!("dup {number}"), program, &self.abbreviations().np)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type1::test_support::{fixture, pfa};

    #[test]
    fn subset_of_cmr10() {
        let font = Type1Font::from_bytes(&fixture("cmr10.pfb")).unwrap();
        let subset = font
            .subset("Hello World".chars().map(u32::from), "ABCDEF+")
            .unwrap();
        let prop = subset.prop();
        assert_eq!(prop.font_name().to_string(), "ABCDEF+CMR10");
        let mut names: Vec<_> = prop.char_strings().names().collect();
        names.sort_unstable();
        // The space of cmr10's encoding is `suppress`.
        assert_eq!(
            names,
            [".notdef", "H", "W", "d", "e", "l", "o", "r", "suppress"]
        );
        assert_eq!(prop.encoding().map(Type1Encoding::len), Some(8));
        assert_eq!(
            prop.subrs().map(<[_]>::len),
            font.prop().subrs().map(<[_]>::len)
        );
        assert_eq!(subset.spans("UniqueID"), []);
        for glyph in names {
            assert_eq!(
                prop.char_strings().get(glyph),
                font.prop().char_strings().get(glyph)
            );
        }
    }

    #[test]
    fn simulator_follows_subroutines_and_accents() {
        let charstring = |code: &[u8]| {
            let font = Type1Font::from_bytes(&pfa(&[
                b"/Subrs 1 array\ndup 0 5 RD ".as_slice(),
                &encrypt(b"\x0b", Type1Cipher::Charstring, 4),
                b" NP\n/CharStrings 1 begin\n/.notdef ",
                format!("{} RD ", code.len() + 4).as_bytes(),
                &encrypt(code, Type1Cipher::Charstring, 4),
                b" ND\nend\n",
            ]
            .concat()))
            .unwrap();
            let mut simulator = Simulator::new(&font);
            simulator
                .run(Program::Glyph(".notdef"))
                .map(|()| (simulator.glyphs, simulator.subrs, simulator.buildchar))
        };
        // 1 0 div pushes 0; -1 callsubr calls the last subroutine.
        // 0 0 0 65 194 seac: the accent components by standard code.
        let (glyphs, _, _) = charstring(&[139, 139, 139, 204, 247, 86, 12, 6, 14]).unwrap();
        assert_eq!(
            glyphs,
            BTreeSet::from([".notdef", "A", "acute"].map(String::from))
        );
        let (_, subrs, stack) = charstring(&[140, 139, 12, 12, 138, 10, 14]).unwrap();
        assert_eq!(subrs, BTreeSet::from([-1]));
        assert_eq!(stack, [Num::Int(0)]);
        // 5 7 2 0 callothersubr pop pop: the arguments come back reversed.
        let (_, _, stack) = charstring(&[144, 146, 141, 139, 12, 16, 12, 17, 12, 17, 14]).unwrap();
        assert_eq!(stack, [Num::Int(5), Num::Int(7)]);
        assert!(charstring(&[12, 12]).is_err());
        assert!(charstring(&[2]).is_err());
        assert!(charstring(&[247]).is_err());
        assert!(charstring(&[139, 10, 10]).is_err());
        assert!(charstring(&[139, 139, 139, 247, 20, 139, 12, 6]).is_err());
    }
}
