//! The DVI opcode interpreter matplotlib's `Dvi` and `Vf` share: its
//! dispatch table, argument decoding, registers and font definitions.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{DviFont, RawGlyph, RawRule};
use crate::tex::bytes::Bytes;
use crate::tex::error::{TexError, TexFile, bytes_repr};
use crate::tex::files::FontSource;
use crate::tex::native::{LUATEX_CHECKSUM, NativeFont};

/// `_dvistate`: what the reader expects next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    /// The preamble.
    Pre,
    /// Between pages (or packets, in a virtual font).
    Outer,
    /// Inside a page (or packet).
    InPage,
    /// After the postamble.
    PostPost,
}

/// The position and spacing registers (`h, v, w, x, y, z`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Registers {
    pub(crate) h: i128,
    pub(crate) v: i128,
    w: i128,
    x: i128,
    y: i128,
    z: i128,
}

/// What an opcode did, as far as reading pages and the baseline detection
/// of `Dvi._read` care.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Executed {
    /// `push`.
    Push,
    /// `pop`.
    Pop,
    /// A `down` move by an amount.
    Down,
    /// `eop`, the end of a page.
    EndOfPage,
    /// Anything else.
    Other,
}

/// A font of a virtual font's packets, compared by identity: the key of
/// the fonts scaled for the virtual fonts using them.
#[derive(Clone, Debug)]
struct PacketFont(Arc<DviFont>);

impl PartialEq for PacketFont {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for PacketFont {}

impl Hash for PacketFont {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

/// A font number's definition: the font, or why it could not be loaded
/// (matplotlib defers that error until a glyph of the font is set).
#[derive(Clone, Debug)]
enum FontSlot {
    Loaded(Arc<DviFont>),
    Missing(TexError),
}

/// `_mul1220`: the product of two 12.20 fixed-point numbers.
pub(crate) fn mul1220(a: i128, b: i128) -> Result<i128, TexError> {
    a.checked_mul(b)
        .map(|product| product >> 20)
        .ok_or_else(|| TexError::malformed(TexFile::Dvi, "number too large"))
}

/// The interpreter: the input, the registers and what has been set.
pub(crate) struct Machine<'a> {
    input: Bytes<'a>,
    pub(crate) state: State,
    pub(crate) regs: Registers,
    pub(crate) stack: Vec<Registers>,
    /// The current font number (`self.f`), unset before the first
    /// font selection.
    pub(crate) font: Option<i64>,
    fonts: HashMap<i64, FontSlot>,
    /// The fonts of virtual font packets at the scales they were set at,
    /// made once rather than for every glyph.
    scaled: HashMap<(PacketFont, i128), Arc<DviFont>>,
    pub(crate) text: Vec<RawGlyph>,
    pub(crate) boxes: Vec<RawRule>,
    /// The error of a missing font a glyph was set in.
    pub(crate) missing_font: Option<TexError>,
    files: &'a dyn FontSource,
    /// The kind of file read, for error messages.
    pub(crate) kind: TexFile,
}

impl<'a> Machine<'a> {
    pub(crate) fn new(data: &'a [u8], files: &'a dyn FontSource, kind: TexFile) -> Self {
        Self {
            input: Bytes::new(data),
            state: State::Pre,
            regs: Registers::default(),
            stack: Vec::new(),
            font: None,
            fonts: HashMap::new(),
            scaled: HashMap::new(),
            text: Vec::new(),
            boxes: Vec::new(),
            missing_font: None,
            files,
            kind,
        }
    }

    pub(crate) fn error(&self, message: impl Into<String>) -> TexError {
        TexError::malformed(self.kind, message)
    }

    /// How many bytes of the file have been read.
    pub(crate) fn pos(&self) -> usize {
        self.input.pos()
    }

    /// The next byte (`file.read(1)[0]`), an error at the end.
    pub(crate) fn byte(&mut self) -> Result<u8, TexError> {
        self.input
            .byte()
            .ok_or_else(|| self.error("index out of range"))
    }

    /// The next `n` bytes, or as many as are left (`file.read(n)`).
    pub(crate) fn read(&mut self, n: usize) -> &'a [u8] {
        self.input.read(n)
    }

    /// `_read_arg`: a big-endian number of up to `n` bytes (fewer at the
    /// end of the file, none giving 0).
    pub(crate) fn arg(&mut self, n: usize, signed: bool) -> i128 {
        self.input.arg(n, signed)
    }

    /// `olen1`: `delta + 1` bytes, signed when there are four.
    fn olen1(&mut self, delta: u8) -> i128 {
        self.arg(usize::from(delta) + 1, delta == 3)
    }

    /// `slen`: `delta` signed bytes, `None` for none.
    fn slen(&mut self, delta: u8) -> Option<i128> {
        (delta != 0).then(|| self.arg(usize::from(delta), true))
    }

    fn require(&self, state: State) -> Result<(), TexError> {
        if self.state == state {
            Ok(())
        } else {
            Err(self.error("state precondition failed"))
        }
    }

    fn add(&self, a: i128, b: i128) -> Result<i128, TexError> {
        a.checked_add(b)
            .ok_or_else(|| self.error("number too large"))
    }

    fn slot(&self) -> Result<&FontSlot, TexError> {
        self.font
            .and_then(|f| self.fonts.get(&f))
            .ok_or_else(|| self.error("undefined font"))
    }

    /// Reads and runs the next opcode of a DVI file.
    pub(crate) fn next_opcode(&mut self) -> Result<Executed, TexError> {
        let byte = self.byte()?;
        self.execute(byte)
    }

    /// Runs one opcode of a page or packet (`_dtable[byte]`).
    pub(crate) fn execute(&mut self, byte: u8) -> Result<Executed, TexError> {
        match byte {
            0..=137 => self.typeset(byte)?,
            138..=142 => return self.structure(byte),
            143..=170 => return self.move_by(byte),
            171..=246 => self.font_or_special(byte)?,
            247 => self.preamble()?,
            248 => {
                self.require(State::Outer)?;
                self.state = State::PostPost;
            }
            249..=251 => return Err(self.error(format!("opcode {byte} is not implemented"))),
            252 => self.define_native_font()?,
            253 | 254 => self.set_glyphs(byte == 254)?,
            255 => return Err(self.error("unknown command: byte 255")),
        }
        Ok(Executed::Other)
    }

    /// Characters and rules (opcodes 0 to 137).
    fn typeset(&mut self, byte: u8) -> Result<(), TexError> {
        match byte {
            0..=127 => self.set_char(i128::from(byte)),
            128..=131 => {
                self.require(State::InPage)?;
                let code = self.olen1(byte - 128);
                self.set_char(code)
            }
            133..=136 => {
                self.require(State::InPage)?;
                let code = self.olen1(byte - 133);
                self.put_char(code)
            }
            _ => {
                self.require(State::InPage)?;
                let (a, b) = (self.arg(4, true), self.arg(4, true));
                self.put_rule(a, b);
                if byte == 132 {
                    self.regs.h = self.add(self.regs.h, b)?;
                }
                Ok(())
            }
        }
    }

    /// `nop`, `bop`, `eop`, `push` and `pop` (opcodes 138 to 142).
    fn structure(&mut self, byte: u8) -> Result<Executed, TexError> {
        match byte {
            138 => {}
            139 => {
                self.require(State::Outer)?;
                for _ in 0..11 {
                    self.arg(4, true);
                }
                self.state = State::InPage;
                self.regs = Registers::default();
                self.stack.clear();
                self.text.clear();
                self.boxes.clear();
            }
            140 => {
                self.require(State::InPage)?;
                self.state = State::Outer;
                self.stack.clear();
                return Ok(Executed::EndOfPage);
            }
            141 => {
                self.require(State::InPage)?;
                self.stack.push(self.regs);
                return Ok(Executed::Push);
            }
            _ => {
                self.require(State::InPage)?;
                self.regs = self
                    .stack
                    .pop()
                    .ok_or_else(|| self.error("pop from empty list"))?;
                return Ok(Executed::Pop);
            }
        }
        Ok(Executed::Other)
    }

    /// The horizontal and vertical moves (opcodes 143 to 170): `right`
    /// and `down` by an amount, or by the `w`, `x`, `y` or `z` register,
    /// after setting it when the opcode carries a value.
    fn move_by(&mut self, byte: u8) -> Result<Executed, TexError> {
        self.require(State::InPage)?;
        let (step, horizontal) = match byte {
            143..=146 => (self.arg(usize::from(byte - 143) + 1, true), true),
            157..=160 => (self.arg(usize::from(byte - 157) + 1, true), false),
            _ => {
                let (first, horizontal) = match byte {
                    147..=151 => (147, true),
                    152..=156 => (152, true),
                    161..=165 => (161, false),
                    _ => (166, false),
                };
                let value = self.slen(byte - first);
                let register = match first {
                    147 => &mut self.regs.w,
                    152 => &mut self.regs.x,
                    161 => &mut self.regs.y,
                    _ => &mut self.regs.z,
                };
                if let Some(value) = value {
                    *register = value;
                }
                (*register, horizontal)
            }
        };
        if horizontal {
            self.regs.h = self.add(self.regs.h, step)?;
            Ok(Executed::Other)
        } else {
            self.regs.v = self.add(self.regs.v, step)?;
            Ok(if (157..=160).contains(&byte) {
                Executed::Down
            } else {
                Executed::Other
            })
        }
    }

    /// Font selections, specials and font definitions (opcodes 171 to
    /// 246).
    fn font_or_special(&mut self, byte: u8) -> Result<(), TexError> {
        match byte {
            171..=234 => {
                self.require(State::InPage)?;
                self.font = Some(i64::from(byte - 171));
            }
            235..=238 => {
                self.require(State::InPage)?;
                self.font = Some(self.olen1(byte - 235) as i64);
            }
            239..=242 => {
                let length = self.arg(usize::from(byte - 239) + 1, false);
                self.read(usize::try_from(length).unwrap_or(usize::MAX));
            }
            _ => {
                self.font_definition(byte)?;
            }
        }
        Ok(())
    }

    /// `fnt_def1` to `fnt_def4` (opcodes 243 to 246, in DVI and VF files
    /// alike): defines a font; returns its number.
    pub(crate) fn font_definition(&mut self, byte: u8) -> Result<i64, TexError> {
        let k = self.olen1(byte - 243) as i64;
        let checksum = self.arg(4, false) as u32;
        let scale = self.arg(4, false);
        self.arg(4, false);
        let area = self.arg(1, false) as usize;
        let name = self.arg(1, false) as usize;
        self.define_font(k, checksum, scale, area, name)?;
        Ok(k)
    }

    /// `pre`: checks the format, units and magnification.
    fn preamble(&mut self) -> Result<(), TexError> {
        self.require(State::Pre)?;
        let i = self.arg(1, false);
        let num = self.arg(4, false);
        let den = self.arg(4, false);
        let mag = self.arg(4, false);
        let k = self.arg(1, false) as usize;
        self.read(k);
        if i != 2 && i != 7 {
            return Err(self.error(format!("Unknown dvi format {i}")));
        }
        if num != 25_400_000 || den != 7227 * (1 << 16) {
            return Err(self.error("Nonstandard units in dvi file"));
        }
        if mag != 1000 {
            return Err(self.error("Nonstandard magnification in dvi file"));
        }
        self.state = State::Outer;
        Ok(())
    }

    /// XeTeX's `define_native_font` (opcode 252): defines a font by its
    /// file, face and effects.
    fn define_native_font(&mut self) -> Result<(), TexError> {
        let k = self.arg(4, false) as i64;
        let scale = self.arg(4, false);
        let flags = self.arg(2, false) as u16;
        let length = self.arg(1, false) as usize;
        let name = self.read(length);
        let index = self.arg(4, false) as u32;
        let native = NativeFont::xetex(name, index, flags, |n, signed| self.input.arg(n, signed));
        let metrics = self.files.native_face(&native.path, native.subfont)?;
        let mut texname = Vec::with_capacity(name.len() + 2);
        texname.push(b'[');
        texname.extend_from_slice(name);
        texname.push(b']');
        self.fonts.insert(
            k,
            FontSlot::Loaded(Arc::new(DviFont::native(scale, texname, native, metrics))),
        );
        Ok(())
    }

    /// XeTeX's `set_glyphs` (opcode 253) and `set_text_and_glyphs` (254,
    /// whose UTF-16 text comes first and is skipped): glyphs of the current
    /// font at offsets from the current position, then a move right by
    /// their total width.
    fn set_glyphs(&mut self, with_text: bool) -> Result<(), TexError> {
        self.require(State::InPage)?;
        if with_text {
            let length = self.arg(2, false) as usize;
            self.read(2 * length);
        }
        let width = self.arg(4, false);
        let count = self.arg(2, false) as usize;
        let offsets: Vec<(i128, i128)> = (0..count)
            .map(|_| (self.arg(4, true), self.arg(4, true)))
            .collect();
        let glyphs: Vec<i128> = (0..count).map(|_| self.arg(2, false)).collect();
        let FontSlot::Loaded(font) = self.slot()? else {
            return Err(self.error("glyphs set in a missing font"));
        };
        let font = Arc::clone(font);
        for ((dx, dy), glyph) in offsets.into_iter().zip(glyphs) {
            self.text.push(RawGlyph {
                x: self.add(self.regs.h, dx)?,
                y: self.add(self.regs.v, dy)?,
                font: Arc::clone(&font),
                glyph,
                width: font.width_of(glyph)?,
            });
        }
        self.regs.h = self.add(self.regs.h, width)?;
        Ok(())
    }

    /// `_set_char`: sets the character and moves right by its width.
    fn set_char(&mut self, code: i128) -> Result<(), TexError> {
        self.require(State::InPage)?;
        self.put_char(code)?;
        if let FontSlot::Loaded(font) = self.slot()? {
            let width = font.width_of(code)?;
            self.regs.h = self.add(self.regs.h, width)?;
        }
        Ok(())
    }

    /// `_put_char_real`: the character in the current font, the packet of
    /// a virtual font expanded in place.
    fn put_char(&mut self, code: i128) -> Result<(), TexError> {
        let font = match self.slot()? {
            FontSlot::Missing(err) => {
                self.missing_font = Some(err.clone());
                return Ok(());
            }
            FontSlot::Loaded(font) => Arc::clone(font),
        };
        let (h, v) = (self.regs.h, self.regs.v);
        let Some(vf) = font.vf() else {
            let width = font.width_of(code)?;
            self.text.push(RawGlyph {
                x: h,
                y: v,
                font,
                glyph: code,
                width,
            });
            return Ok(());
        };
        let scale = font.scale();
        let packet = i64::try_from(code)
            .ok()
            .and_then(|code| vf.packet(code))
            .ok_or_else(|| self.error(format!("virtual font has no character {code}")))?;
        for item in &packet.text {
            let inner = self.scaled_font(&item.font, scale)?;
            let width = inner.width_of(item.glyph)?;
            self.text.push(RawGlyph {
                x: self.add(h, mul1220(item.x, scale)?)?,
                y: self.add(v, mul1220(item.y, scale)?)?,
                font: inner,
                glyph: item.glyph,
                width,
            });
        }
        for item in &packet.boxes {
            self.boxes.push(RawRule {
                x: self.add(h, mul1220(item.x, scale)?)?,
                y: self.add(v, mul1220(item.y, scale)?)?,
                height: mul1220(item.height, scale)?,
                width: mul1220(item.width, scale)?,
            });
        }
        Ok(())
    }

    /// `font`, of a virtual font's packet, scaled by `scale`, the scale of
    /// the virtual font (`DviFont._scale` times the packet font's scale).
    fn scaled_font(&mut self, font: &Arc<DviFont>, scale: i128) -> Result<Arc<DviFont>, TexError> {
        let key = (PacketFont(Arc::clone(font)), scale);
        if let Some(scaled) = self.scaled.get(&key) {
            return Ok(Arc::clone(scaled));
        }
        let scaled = Arc::new(font.rescaled(mul1220(scale, font.scale())?));
        self.scaled.insert(key, Arc::clone(&scaled));
        Ok(scaled)
    }

    /// `_put_rule_real`: a rule of positive height `a` and width `b`.
    fn put_rule(&mut self, a: i128, b: i128) {
        if a > 0 && b > 0 {
            self.boxes.push(RawRule {
                x: self.regs.h,
                y: self.regs.v,
                height: a,
                width: b,
            });
        }
    }

    /// `_fnt_def_real`: defines font number `k` from the `a + l` bytes of
    /// its area and name that follow.
    fn define_font(
        &mut self,
        k: i64,
        checksum: u32,
        scale: i128,
        a: usize,
        l: usize,
    ) -> Result<(), TexError> {
        let texname = self.read(a + l).to_vec();
        // Python's `n[-l:]`, which is all of `n` when `l` is 0.
        let name = if l == 0 || l > texname.len() {
            &texname[..]
        } else {
            &texname[texname.len() - l..]
        };
        if name.starts_with(b"[") && checksum == LUATEX_CHECKSUM {
            let native = NativeFont::luatex(&texname)?;
            let metrics = self.files.native_face(&native.path, native.subfont)?;
            self.fonts.insert(
                k,
                FontSlot::Loaded(Arc::new(DviFont::native(scale, texname, native, metrics))),
            );
            return Ok(());
        }
        let name = std::str::from_utf8(name)
            .ok()
            .filter(|name| name.is_ascii())
            .ok_or_else(|| self.error("font name is not ASCII"))?;
        let tfm = match self.files.tfm(name) {
            Ok(tfm) => tfm,
            Err(missing @ TexError::NotFound { .. }) => {
                self.fonts.insert(k, FontSlot::Missing(missing));
                return Ok(());
            }
            Err(err) => return Err(err),
        };
        if checksum != 0 && tfm.checksum() != 0 && checksum != tfm.checksum() {
            return Err(self.error(format!("tfm checksum mismatch: {}", bytes_repr(&texname))));
        }
        let vf = self.files.vf(name)?;
        self.fonts.insert(
            k,
            FontSlot::Loaded(Arc::new(DviFont::new(scale, tfm, texname, vf))),
        );
        Ok(())
    }
}
