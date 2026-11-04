//! Kerning as HarfBuzz applies it while shaping: the `kern` table in its
//! OpenType (version 0) and Apple (version 1) forms, and the extended
//! `kerx` table (HarfBuzz's `hb-ot-kern-table.hh` and
//! `hb-aat-layout-kerx-table.hh`).
//!
//! Subtables come in formats:
//!
//! * 0, 2, 3 and 6 ([`pairs`]): values for pairs of glyphs, listed or
//!   found through class tables, added between neighbouring glyphs;
//! * 1 ([`contextual`]): a state machine pushing glyphs on a stack and
//!   popping them with kerning values.
//!
//! (FreeType's reading of `kern`, which matplotlib's `get_kerning` gives,
//! is [`crate::tables::kern`]: it knows the OpenType form only.) `kerx`
//! format 4, which attaches glyphs by anchor or contour points, is not
//! applied; no font of macOS uses it.

mod contextual;
mod pairs;

use crate::aat::buffer::AatBuffer;
use crate::aat::state::{Flavor, StateTable};
use crate::face::Face;
use crate::layout::buffer::{AttachKind, Buffer, Mask};
use crate::reader::peek;
use crate::scale::Scaler;
use crate::types::Tag;
use contextual::ContextualKerning;
use pairs::{PairTable, kern_pairs};

/// The forms of the kerning tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    /// `kern` version 0 (OpenType).
    OpenType,
    /// `kern` version 1 (Apple).
    Apple,
    /// `kerx`.
    Extended,
}

impl Form {
    /// The state table and field generation of the form.
    const fn flavor(self) -> Flavor {
        match self {
            Self::Extended => Flavor::Extended,
            Self::OpenType | Self::Apple => Flavor::Obsolete,
        }
    }
}

/// A `kern` or `kerx` table.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KernTable<'a> {
    data: &'a [u8],
    form: Form,
}

/// What shaping needs to kern: the scaler, the mask of glyphs to kern,
/// the face's glyph count and the direction.
#[derive(Clone, Copy, Debug)]
pub(crate) struct KernContext<'s> {
    /// Scales font units to the output's.
    pub(crate) scaler: &'s Scaler<'s>,
    /// The mask bits of the `kern` feature (zero: kerning not requested).
    pub(crate) kern_mask: Mask,
    /// The number of glyphs of the face.
    pub(crate) num_glyphs: u16,
    /// Whether the text runs left to right.
    pub(crate) forward: bool,
}

/// One subtable, with its header read.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the independent coverage bits of the subtable header"
)]
struct Subtable<'a> {
    /// The subtable from its header on.
    data: &'a [u8],
    form: Form,
    header_len: usize,
    format: u8,
    horizontal: bool,
    cross_stream: bool,
    variation: bool,
    backwards: bool,
    tuple_count: u32,
}

impl<'a> KernTable<'a> {
    /// The face's `kern` table as HarfBuzz reads it (either version).
    pub(crate) fn kern(face: &Face<'a>) -> Option<Self> {
        let data = face.table(Tag::new(b"kern"))?;
        let form = match peek::<u16>(data, 0)? {
            0 => Form::OpenType,
            1 => Form::Apple,
            _ => return None,
        };
        (peek::<u32>(data, 0)? != 0).then_some(Self { data, form })
    }

    /// The face's `kerx` table (version 2 or later).
    pub(crate) fn kerx(face: &Face<'a>) -> Option<Self> {
        let data = face.table(Tag::new(b"kerx"))?;
        (peek::<u16>(data, 0)? >= 2).then_some(Self {
            data,
            form: Form::Extended,
        })
    }

    /// The subtables, as far as the table holds them; the last one
    /// reaches to the end of the table (OpenType `kern` subtables longer
    /// than 64 KiB cannot give their length).
    fn subtables(&self) -> impl Iterator<Item = Subtable<'a>> + use<'a> {
        let (data, form) = (self.data, self.form);
        let (count, mut at) = match form {
            Form::OpenType => (peek::<u16>(data, 2).map(u32::from), 4),
            Form::Apple | Form::Extended => (peek::<u32>(data, 4), 8),
        };
        let count = count.unwrap_or(0);
        (0..count).map_while(move |index| {
            let subtable = Subtable::read(data.get(at..)?, form)?;
            let length = match form {
                Form::OpenType => usize::from(peek::<u16>(data, at + 2)?),
                Form::Apple | Form::Extended => usize::try_from(peek::<u32>(data, at)?).ok()?,
            };
            let last = index + 1 == count;
            if length < subtable.header_len && !last {
                return None;
            }
            let end = if last {
                data.len()
            } else {
                at.checked_add(length)?
            };
            let subtable = Subtable {
                data: data.get(at..end)?,
                ..subtable
            };
            at = end;
            Some(subtable)
        })
    }

    /// Whether a subtable is a state machine (HarfBuzz's
    /// `has_state_machine`).
    pub(crate) fn has_state_machine(&self) -> bool {
        self.subtables().any(|subtable| subtable.format == 1)
    }

    /// Whether a subtable kerns across the text (HarfBuzz's
    /// `has_cross_stream`).
    pub(crate) fn has_cross_stream(&self) -> bool {
        self.subtables().any(|subtable| subtable.cross_stream)
    }

    /// Kerns the glyphs of `buffer` (HarfBuzz's `KerxTable::apply`).
    pub(crate) fn apply(&self, face: &Face<'_>, buffer: &mut Buffer, context: &KernContext<'_>) {
        let mut buffer = AatBuffer::new(buffer, face.gdef());
        let mut seen_cross_stream = false;
        for subtable in self.subtables() {
            if (subtable.variation && self.form != Form::Extended) || !subtable.horizontal {
                continue;
            }
            let Some(kind) = subtable.kind() else {
                continue;
            };
            if !buffer.intersects(|glyph| kind.starts_with(glyph, context.num_glyphs)) {
                continue;
            }
            let reverse = subtable.backwards == context.forward;
            if !seen_cross_stream && subtable.cross_stream {
                seen_cross_stream = true;
                let chain = if context.forward { -1 } else { 1 };
                for pos in &mut buffer.buffer.pos {
                    pos.attach_kind = AttachKind::Cursive;
                    pos.attach_chain = chain;
                }
            }
            if reverse != buffer.is_reversed() {
                buffer.reverse();
            }
            match kind {
                Kind::Pairs(table) => {
                    if context.kern_mask != 0 && !subtable.backwards {
                        kern_pairs(buffer.buffer, context, subtable.cross_stream, |l, r| {
                            table.kerning(l, r, context.num_glyphs)
                        });
                    }
                }
                Kind::Contextual(table, machine) => {
                    if context.kern_mask != 0 || subtable.cross_stream {
                        machine.run(&table, &mut buffer, context);
                    }
                }
            }
        }
        if buffer.is_reversed() {
            buffer.reverse();
        }
    }
}

/// What a subtable does.
enum Kind<'a> {
    /// Kerns pairs of glyphs.
    Pairs(PairTable<'a>),
    /// Runs a state machine.
    Contextual(StateTable<'a>, ContextualKerning<'a>),
}

impl Kind<'_> {
    /// Whether `glyph` can start the subtable's work (HarfBuzz's first
    /// set of the subtable).
    fn starts_with(&self, glyph: u16, num_glyphs: u16) -> bool {
        match self {
            Self::Pairs(table) => table.has_left(glyph, num_glyphs),
            Self::Contextual(table, machine) => machine.can_start(table, glyph, num_glyphs),
        }
    }
}

impl<'a> Subtable<'a> {
    /// The header of a subtable of a table of `form` starting `data`.
    fn read(data: &'a [u8], form: Form) -> Option<Self> {
        let subtable = match form {
            Form::OpenType => {
                let coverage = peek::<u8>(data, 5)?;
                Self {
                    data,
                    form,
                    header_len: 6,
                    format: peek::<u8>(data, 4)?,
                    horizontal: coverage & 0x01 != 0,
                    cross_stream: coverage & 0x04 != 0,
                    variation: false,
                    backwards: false,
                    tuple_count: 0,
                }
            }
            Form::Apple => {
                let coverage = peek::<u8>(data, 4)?;
                Self {
                    data,
                    form,
                    header_len: 8,
                    format: peek::<u8>(data, 5)?,
                    horizontal: coverage & 0x80 == 0,
                    cross_stream: coverage & 0x40 != 0,
                    variation: coverage & 0x20 != 0,
                    backwards: false,
                    tuple_count: 0,
                }
            }
            Form::Extended => {
                let coverage = peek::<u32>(data, 4)?;
                Self {
                    data,
                    form,
                    header_len: 12,
                    format: (coverage & 0xFF) as u8,
                    horizontal: coverage & 0x8000_0000 == 0,
                    cross_stream: coverage & 0x4000_0000 != 0,
                    variation: coverage & 0x2000_0000 != 0,
                    backwards: coverage & 0x1000_0000 != 0,
                    tuple_count: peek::<u32>(data, 8)?,
                }
            }
        };
        Some(subtable)
    }

    /// What the subtable does, by format; `None` for formats not applied
    /// or subtables too short for their format.
    fn kind(&self) -> Option<Kind<'a>> {
        let flavor = self.form.flavor();
        match (self.format, self.form) {
            (1, _) => {
                let machine = self.data.get(self.header_len..)?;
                let table =
                    StateTable::new(machine, flavor, usize::from(flavor == Flavor::Extended))?;
                let actions = flavor.read_field(machine, table.header_len())?;
                Some(Kind::Contextual(
                    table,
                    ContextualKerning::new(
                        table,
                        flavor,
                        actions,
                        self.tuple_count,
                        self.cross_stream,
                    ),
                ))
            }
            (0 | 2, _) | (3, Form::OpenType | Form::Apple) | (6, Form::Extended) => PairTable::new(
                self.data,
                self.header_len,
                self.format,
                flavor,
                self.tuple_count,
            )
            .map(Kind::Pairs),
            _ => None,
        }
    }
}
