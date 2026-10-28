//! Virtual fonts (matplotlib's `dviread.Vf`): characters defined as short
//! DVI programs setting glyphs of other fonts.

use std::collections::HashMap;

use super::dvi::{Machine, RawGlyph, RawRule, Registers, State};
use super::error::{TexError, TexFile};
use super::files::FontSource;

/// One character of a virtual font: the glyphs and rules its packet sets,
/// in units of the font's design size, and its width.
#[derive(Clone, Debug)]
pub struct VfPacket {
    pub(crate) text: Vec<RawGlyph>,
    pub(crate) boxes: Vec<RawRule>,
    width: i128,
}

impl VfPacket {
    /// The width the packet declares.
    pub fn width(&self) -> i128 {
        self.width
    }

    /// The glyphs the packet sets, in units of the design size.
    pub fn glyphs(&self) -> &[RawGlyph] {
        &self.text
    }

    /// The rules the packet sets, in units of the design size.
    pub fn rules(&self) -> &[RawRule] {
        &self.boxes
    }
}

/// A virtual font file.
#[derive(Clone, Debug, Default)]
pub struct Vf {
    packets: HashMap<i64, VfPacket>,
}

/// The packet being read: its character, width and where it ends.
struct Packet {
    code: i64,
    width: i128,
    ends: usize,
}

impl Vf {
    /// Parses a virtual font, loading the fonts it refers to from
    /// `fonts` (matplotlib's `Vf(filename)`). Long packets are read as the
    /// format defines them, where matplotlib 3.11 misreads them.
    ///
    /// # Errors
    ///
    /// When the file is malformed or a font it refers to cannot be read.
    pub fn parse(data: &[u8], fonts: &dyn FontSource) -> Result<Self, TexError> {
        let mut machine = Machine::new(data, fonts, TexFile::Vf);
        let mut first_font = None;
        let mut packets = HashMap::new();
        let mut packet: Option<Packet> = None;
        loop {
            let byte = machine.byte()?;
            if machine.state == State::InPage
                && let Some(current) = &packet
            {
                match (machine.pos - 1).cmp(&current.ends) {
                    std::cmp::Ordering::Equal => {
                        if machine.missing_font.take().is_none() {
                            packets.insert(
                                current.code,
                                VfPacket {
                                    text: std::mem::take(&mut machine.text),
                                    boxes: std::mem::take(&mut machine.boxes),
                                    width: current.width,
                                },
                            );
                        }
                        machine.state = State::Outer;
                        packet = None;
                    }
                    std::cmp::Ordering::Greater => {
                        return Err(machine.error("Packet length mismatch in vf file"));
                    }
                    std::cmp::Ordering::Less => {
                        if matches!(byte, 139 | 140) || byte >= 243 {
                            return Err(
                                machine.error(format!("Inappropriate opcode {byte} in vf file"))
                            );
                        }
                        machine.execute(byte)?;
                        continue;
                    }
                }
            }
            match byte {
                0..=242 => {
                    let (length, code, width) = if byte < 242 {
                        (
                            usize::from(byte),
                            machine.arg(1, false),
                            machine.arg(3, false),
                        )
                    } else {
                        let length = machine.arg(4, false);
                        (
                            usize::try_from(length).unwrap_or(usize::MAX),
                            machine.arg(4, false),
                            machine.arg(4, false),
                        )
                    };
                    if machine.state != State::Outer {
                        return Err(machine.error("Misplaced packet in vf file"));
                    }
                    machine.regs = Registers::default();
                    machine.stack.clear();
                    machine.text.clear();
                    machine.boxes.clear();
                    machine.font = first_font;
                    machine.missing_font = None;
                    packet = Some(Packet {
                        code: code as i64,
                        width,
                        ends: machine.pos.saturating_add(length),
                    });
                    machine.state = State::InPage;
                }
                243..=246 => {
                    let k = machine.arg(usize::from(byte - 242), byte == 246) as i64;
                    let checksum = machine.arg(4, false) as u32;
                    let scale = machine.arg(4, false);
                    machine.arg(4, false);
                    let a = machine.arg(1, false) as usize;
                    let l = machine.arg(1, false) as usize;
                    machine.define_font(k, checksum, scale, a, l)?;
                    first_font.get_or_insert(k);
                }
                247 => {
                    let i = machine.arg(1, false);
                    let k = machine.arg(1, false) as usize;
                    machine.read(k);
                    machine.arg(4, false);
                    machine.arg(4, false);
                    if machine.state != State::Pre {
                        return Err(machine.error("pre command in middle of vf file"));
                    }
                    if i != 202 {
                        return Err(machine.error(format!("Unknown vf format {i}")));
                    }
                    machine.state = State::Outer;
                }
                248 => break,
                _ => return Err(machine.error(format!("Unknown vf opcode {byte}"))),
            }
        }
        Ok(Self { packets })
    }

    /// The packet of character `code`, if the font defines it.
    pub fn packet(&self, code: i64) -> Option<&VfPacket> {
        self.packets.get(&code)
    }

    /// The characters the font defines, ascending.
    pub fn codes(&self) -> Vec<i64> {
        let mut codes: Vec<i64> = self.packets.keys().copied().collect();
        codes.sort_unstable();
        codes
    }
}
