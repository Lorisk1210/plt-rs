//! `CFF ` and `CFF2` tables: Compact Font Format outlines (OpenType/CFF
//! fonts, `OTTO`), read as FreeType's CFF driver reads them (`cffload.c`,
//! `cffparse.c`, `cffobjs.c`, `cffgload.c`) and drawn by its Adobe engine
//! ([`crate::psaux`]).
//!
//! Name-keyed and CID-keyed fonts, local and global subroutines, the
//! implied `seac`, glyph names from the charset, font matrices, and the
//! CFF2 tables of variable fonts: their Font DICTs, Private DICTs blended
//! at an instance and the variation store `blend` reads.
//!
//! FreeType's CFF driver is copyright David Turner, Robert Wilhelm and
//! Werner Lemberg, and ported here under the FreeType License (FTL).

mod bytes;
mod charset;
mod dict;
mod glyphs;
mod index;
mod matrix;
mod private;
mod strings;
mod subset;
mod vstore;

use std::borrow::Cow;

use crate::error::{FontError, Result};
use crate::psaux::{Flavor, STACK_SIZE};
use crate::reader::Reader;
use crate::types::{GlyphId, Tag};

use bytes::CffBytes;
pub(crate) use bytes::CffParts;
use charset::{Charset, FdSelect};
use dict::{DictKind, Entries, Operator, entries, last, number};
pub(crate) use glyphs::CffScale;
use index::CFF;
pub(crate) use index::CountSize;
pub(crate) use index::Index;
use matrix::DictMatrix;
use private::{Instance, Private};
pub(crate) use strings::{STANDARD_ENCODING, STANDARD_STRINGS};
pub(crate) use subset::CffSubsetter;
use vstore::VariationStore;

const OP_CHARSET: Operator = 15;
const OP_CHARSTRINGS: Operator = 17;
const OP_PRIVATE: Operator = 18;
const OP_VSTORE: Operator = 24;
const OP_FONT_MATRIX: Operator = 0x107;
const OP_ROS: Operator = 0x11E;
const OP_FD_ARRAY: Operator = 0x124;
const OP_FD_SELECT: Operator = 0x125;

/// `CFF_MAX_STACK_DEPTH`: the DICT parser's stack in CFF (one more for a
/// Private DICT).
const CFF_DICT_STACK: usize = 96;

/// `CFF2_DEFAULT_STACK` (and `CFF2_MAX_STACK`): the DICT parser's stack
/// (one more for a Private DICT) and the charstrings' operand stack in
/// CFF2 (`maxstack` is clamped to it).
const CFF2_STACK: usize = 513;

/// `CFF_MAX_CID_FONTS`: the most Font DICTs FreeType reads.
const MAX_FONT_DICTS: usize = 256;

/// The two versions of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Version {
    /// `CFF `: Type 2 charstrings.
    Cff,
    /// `CFF2`: the charstrings of variable fonts.
    Cff2,
}

impl Version {
    /// The table's tag.
    pub(crate) const fn tag(self) -> Tag {
        match self {
            Self::Cff => CFF,
            Self::Cff2 => Tag::new(b"CFF2"),
        }
    }

    /// The width of the table's INDEX counts.
    pub(crate) const fn count_size(self) -> CountSize {
        match self {
            Self::Cff => CountSize::Short,
            Self::Cff2 => CountSize::Long,
        }
    }
}

/// A Font DICT (or the Top DICT of a name-keyed font): its Private DICT,
/// local subroutines and font matrix.
#[derive(Clone, Debug)]
struct SubFont<'a> {
    private: Private,
    subrs: Index<'a>,
    matrix: DictMatrix,
}

/// A parsed CFF or CFF2 table.
#[derive(Clone, Debug)]
pub(crate) struct Cff<'a> {
    version: Version,
    strings: Index<'a>,
    global_subrs: Index<'a>,
    charstrings: Index<'a>,
    charset: Charset<'a>,
    fd_select: FdSelect<'a>,
    /// Whether the font is CID-keyed (with an `ROS`): it has no glyph
    /// names.
    is_cid: bool,
    /// The Top DICT's font matrix.
    top_matrix: DictMatrix,
    /// The Font DICTs of a CID-keyed or CFF2 font, else the Top DICT.
    subfonts: Vec<SubFont<'a>>,
    vstore: VariationStore,
    /// The instance's normalized coordinates (16.16; zeros or none at
    /// the default instance).
    coords: Vec<i64>,
}

/// The header of a table of `version`, checked as FreeType checks it, and
/// a reader after it.
fn header(data: CffBytes<'_>, version: Version) -> Result<(Reader<'_>, usize)> {
    let tag = version.tag();
    let mut r = data.at(0)?;
    let major = r.u8()?;
    r.skip(1)?;
    let header_size = usize::from(r.u8()?);
    let (expected, smallest) = match version {
        Version::Cff => (1, 4),
        Version::Cff2 => (2, 5),
    };
    let fourth = match version {
        Version::Cff => usize::from(r.u8()?),
        Version::Cff2 => usize::from(r.u16()?),
    };
    if major != expected || header_size < smallest || (version == Version::Cff && fourth > 4) {
        return Err(FontError::UnsupportedFormat {
            table: tag,
            format: u32::from(major),
        });
    }
    Ok((data.at(header_size)?, fourth))
}

/// The Top DICT data of the table and a reader after them: the first
/// entry of the Top DICT INDEX after the Name INDEX (CFF), or the bytes
/// after the header (CFF2).
fn top_dict(data: CffBytes<'_>, version: Version) -> Result<(&[u8], Reader<'_>)> {
    let (mut r, top_length) = header(data, version)?;
    match version {
        Version::Cff => {
            let names = Index::read(&mut r)?;
            if names.len() > 1 {
                return Err(FontError::malformed(CFF, "several fonts in one CFF table"));
            }
            let tops = Index::read(&mut r)?;
            if names.len() > tops.len() {
                return Err(FontError::malformed(CFF, "missing Top DICT"));
            }
            let top = tops
                .get(0)
                .ok_or(FontError::malformed(CFF, "missing Top DICT"))?;
            Ok((top, r))
        }
        Version::Cff2 => Ok((r.bytes(top_length)?, r)),
    }
}

/// The DICT kind and parser stack of the Top DICT of `version`.
const fn top_kind(version: Version) -> (DictKind, usize) {
    match version {
        Version::Cff => (DictKind::CffTop, CFF_DICT_STACK),
        Version::Cff2 => (DictKind::Cff2Top, CFF2_STACK),
    }
}

/// Where the `CharStrings` INDEX of a table of `version` is, in the
/// table, when `head`, its first bytes, hold its header and Top DICT:
/// what a font file held for drawing reads glyph by glyph.
pub(crate) fn charstrings_offset(head: &[u8], version: Version) -> Result<usize> {
    let (top, _) = top_dict(CffBytes::Whole(head), version)?;
    let (kind, stack) = top_kind(version);
    let top = entries(top, kind, stack, version.tag())?;
    offset(&top, OP_CHARSTRINGS, version.tag())
}

/// The offset `operator` gives, which must be there.
fn offset(dict: &Entries<'_>, operator: Operator, tag: Tag) -> Result<usize> {
    number(dict, operator)
        .and_then(|at| usize::try_from(at).ok())
        .ok_or(FontError::malformed(tag, "missing DICT offset"))
}

impl<'a> Cff<'a> {
    /// Parses a table of `version` of a face of `units_per_em`, at the
    /// instance of normalized coordinates `coords`.
    pub(crate) fn parse(
        data: &'a [u8],
        version: Version,
        units_per_em: u16,
        coords: &[i64],
    ) -> Result<Self> {
        Self::parse_bytes(CffBytes::Whole(data), version, units_per_em, coords)
    }

    /// Parses a table held but for its `CharStrings` data, which are then
    /// read glyph by glyph.
    pub(crate) fn parse_parts(
        parts: CffParts<'a>,
        units_per_em: u16,
        coords: &[i64],
    ) -> Result<Self> {
        Self::parse_bytes(CffBytes::Parts(parts), parts.version, units_per_em, coords)
    }

    fn parse_bytes(
        data: CffBytes<'a>,
        version: Version,
        units_per_em: u16,
        coords: &[i64],
    ) -> Result<Self> {
        let tag = version.tag();
        let (top_data, mut r) = top_dict(data, version)?;
        let strings = match version {
            Version::Cff => Index::read(&mut r)?,
            Version::Cff2 => Index::default(),
        };
        let global_subrs = Index::read_sized(&mut r, version.count_size())?;
        let (kind, stack) = top_kind(version);
        let top = entries(top_data, kind, stack, tag)?;
        let top_matrix = match last(&top, OP_FONT_MATRIX) {
            Some(operands) => DictMatrix::parse(operands)?,
            None => DictMatrix::absent(i64::from(units_per_em)),
        }
        .normalized();
        let charstrings_at = offset(&top, OP_CHARSTRINGS, tag)?;
        let mut r = data.at(charstrings_at)?;
        let charstrings = match data.charstrings() {
            Some(held_elsewhere) => {
                Index::read_offsets(&mut r, held_elsewhere, version.count_size())?
            }
            None => Index::read_sized(&mut r, version.count_size())?,
        };
        let is_cid = version == Version::Cff && last(&top, OP_ROS).is_some();
        let vstore = match number(&top, OP_VSTORE) {
            Some(at) if version == Version::Cff2 && at != 0 => {
                let at = usize::try_from(at).map_err(|_| FontError::Truncated(tag))?;
                VariationStore::parse(data, at)?
            }
            _ => VariationStore::default(),
        };
        let mut cff = Self {
            version,
            strings,
            global_subrs,
            charstrings,
            charset: Charset::Unsupported,
            fd_select: FdSelect::Single,
            is_cid,
            top_matrix,
            subfonts: Vec::new(),
            vstore,
            coords: coords.to_vec(),
        };
        if is_cid || version == Version::Cff2 {
            cff.read_font_dicts(data, &top)?;
        } else {
            let subfont = cff.subfont(data, &top, top_matrix)?;
            cff.subfonts.push(subfont);
        }
        if version == Version::Cff {
            let at = number(&top, OP_CHARSET).unwrap_or(0);
            cff.charset = Self::charset(data, at, cff.charstrings.len())?;
        }
        Ok(cff)
    }

    /// The Font DICTs of a CID-keyed or CFF2 font and its `FDSelect`.
    fn read_font_dicts(&mut self, data: CffBytes<'a>, top: &Entries<'_>) -> Result<()> {
        let version = self.version;
        let tag = version.tag();
        let at = offset(top, OP_FD_ARRAY, tag)?;
        let fd_array = Index::read_sized(&mut data.at(at)?, version.count_size())?;
        if fd_array.len() > MAX_FONT_DICTS {
            return Err(FontError::malformed(tag, "too many Font DICTs"));
        }
        let (kind, stack) = match version {
            Version::Cff => (DictKind::CffTop, CFF_DICT_STACK),
            Version::Cff2 => (DictKind::Cff2Font, CFF2_STACK),
        };
        for index in 0..fd_array.len() {
            let dict = fd_array
                .get(index)
                .ok_or(FontError::malformed(tag, "Font DICT outside its INDEX"))?;
            let dict = entries(dict, kind, stack, tag)?;
            let own = match last(&dict, OP_FONT_MATRIX) {
                Some(operands) => DictMatrix::parse(operands)?,
                None => DictMatrix::absent(self.top_matrix.units_per_em),
            };
            let matrix = own.under(&self.top_matrix);
            let subfont = self.subfont(data, &dict, matrix)?;
            self.subfonts.push(subfont);
        }
        if version == Version::Cff || fd_array.len() > 1 {
            let at = offset(top, OP_FD_SELECT, tag)?;
            self.fd_select = FdSelect::parse(data.tail(at)?, self.charstrings.len())?;
        }
        Ok(())
    }

    /// The Font DICT (or Top DICT) `dict` with its Private DICT and local
    /// subroutines.
    fn subfont(
        &self,
        data: CffBytes<'a>,
        dict: &Entries<'_>,
        matrix: DictMatrix,
    ) -> Result<SubFont<'a>> {
        let tag = self.version.tag();
        let Some(&[size, offset]) = last(dict, OP_PRIVATE) else {
            return Ok(SubFont {
                private: Private::absent(),
                subrs: Index::default(),
                matrix,
            });
        };
        let location = |operand: dict::Operand<'_>| {
            usize::try_from(operand.num())
                .map_err(|_| FontError::malformed(tag, "negative Private DICT location"))
        };
        let (size, offset) = (location(size)?, location(offset)?);
        let (stack, instance) = match self.version {
            Version::Cff => (CFF_DICT_STACK + 1, None),
            Version::Cff2 => (
                CFF2_STACK + 1,
                Some(Instance {
                    store: &self.vstore,
                    coords: &self.coords,
                }),
            ),
        };
        let private = Private::read(data, (size, offset), stack, instance, tag)?;
        let subrs = match private.subrs {
            Some(subrs) => Index::read_sized(
                &mut data.at(offset.saturating_add(subrs))?,
                self.version.count_size(),
            )?,
            None => Index::default(),
        };
        Ok(SubFont {
            private,
            subrs,
            matrix,
        })
    }

    fn charset(data: CffBytes<'a>, offset: i64, glyphs: usize) -> Result<Charset<'a>> {
        match offset {
            0 => Ok(Charset::IsoAdobe),
            1 | 2 => Ok(Charset::Unsupported),
            _ => {
                let offset = usize::try_from(offset).map_err(|_| FontError::Truncated(CFF))?;
                let mut r = data.at(offset)?;
                match r.u8()? {
                    0 => Ok(Charset::Sids(r.array(glyphs.saturating_sub(1))?)),
                    format @ (1 | 2) => Ok(Charset::Ranges {
                        data: data.tail(offset + 1)?,
                        wide: format == 2,
                    }),
                    format => Err(FontError::UnsupportedFormat {
                        table: CFF,
                        format: u32::from(format),
                    }),
                }
            }
        }
    }

    /// The table's version.
    pub(crate) const fn version(&self) -> Version {
        self.version
    }

    /// The engine's flavor of the table's charstrings.
    const fn flavor(&self) -> Flavor {
        match self.version {
            Version::Cff => Flavor::Cff,
            Version::Cff2 => Flavor::Cff2,
        }
    }

    /// The depth of the charstrings' operand stack.
    const fn stack_size(&self) -> usize {
        match self.version {
            Version::Cff => STACK_SIZE,
            Version::Cff2 => CFF2_STACK,
        }
    }

    /// Whether glyph names are available (name-keyed CFF fonts only).
    pub(crate) fn has_glyph_names(&self) -> bool {
        self.version == Version::Cff && !self.is_cid
    }

    /// PostScript name of `glyph`.
    pub(crate) fn glyph_name(&self, glyph: GlyphId) -> Option<&'a str> {
        if !self.has_glyph_names() {
            return None;
        }
        let sid = usize::from(self.charset.sid(glyph)?);
        match sid.checked_sub(STANDARD_STRINGS.len()) {
            None => STANDARD_STRINGS.get(sid).copied(),
            Some(custom) => std::str::from_utf8(self.strings.get(custom)?).ok(),
        }
    }

    /// The name fontTools' `cffLib` gives `glyph` in a `CFF ` table: its
    /// glyph name in a name-keyed font, `cidNNNNN` of its CID in a
    /// CID-keyed one.
    pub(crate) fn fonttools_name(&self, glyph: GlyphId) -> Option<Cow<'a, str>> {
        if self.version != Version::Cff {
            return None;
        }
        if !self.is_cid {
            return self.glyph_name(glyph).map(Cow::Borrowed);
        }
        if glyph.0 == 0 {
            return Some(Cow::Borrowed(".notdef"));
        }
        Some(Cow::Owned(format!("cid{:05}", self.charset.sid(glyph)?)))
    }

    /// Glyph whose charset SID is `sid` (`seac` components).
    fn glyph_for_sid(&self, sid: u16) -> Option<GlyphId> {
        (0..self.charstrings.len())
            .filter_map(|g| u16::try_from(g).ok())
            .map(GlyphId)
            .find(|&g| self.charset.sid(g) == Some(sid))
    }
}

#[cfg(test)]
mod tests;
