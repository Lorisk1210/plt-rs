//! Subsets of `CFF ` and `CFF2` tables, for fonts embedded in documents
//! (matplotlib's Type 42 embedding, through fontTools' subsetter, keeps
//! the outlines of OpenType/CFF fonts in their own format).
//!
//! The subset holds the kept glyphs' charstrings, renumbered in their
//! order, without subroutines ([`flatten`]); their charset and
//! `FDSelect`; the strings it names ([`strings`]); and the original Font
//! DICTs, Private DICTs (less their subroutines) and variation store.
//! Glyph names and `seac` components resolve as they did in the
//! original.

mod flatten;
mod strings;
mod write;

use super::bytes::CffBytes;
use super::dict::{DictKind, Operator, entries, last, raw_entries};
use super::index::{CFF, Index};
use super::{
    CFF_DICT_STACK, CFF2_STACK, Cff, CountSize, OP_CHARSET, OP_CHARSTRINGS, OP_FD_ARRAY,
    OP_FD_SELECT, OP_PRIVATE, OP_VSTORE, STANDARD_ENCODING, Version, header,
};
use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::types::{GlyphId, Tag};
use flatten::{Flattened, flatten};
use strings::Strings;
use write::{charset, entry, entry_length, fd_select, index, index_length};

const OP_ENCODING: Operator = 16;
const OP_SUBRS: Operator = 19;

/// A CFF or CFF2 table to take subsets of.
#[derive(Clone, Debug)]
pub(crate) struct CffSubsetter<'a> {
    data: &'a [u8],
    cff: Cff<'a>,
}

/// A DICT that holds a Private DICT, as a subset writes it: its entries
/// but those the subset writes anew, and its Private DICT's entries but
/// the subroutines.
struct Holder {
    own: Vec<u8>,
    private: Option<Vec<u8>>,
}

impl Holder {
    /// The holder's length with its `Private` entry.
    fn length(&self) -> usize {
        self.own.len()
            + self
                .private
                .as_ref()
                .map_or(0, |_| entry_length(OP_PRIVATE, 2))
    }

    /// The holder's bytes, its Private DICT at `at`.
    fn bytes(&self, at: usize) -> Vec<u8> {
        let mut out = self.own.clone();
        if let Some(private) = &self.private {
            out.extend(entry(OP_PRIVATE, &[private.len(), at]));
        }
        out
    }
}

/// The entries of `dict` but those of `dropped` operators, as written.
fn without(dict: &[u8], dropped: &[Operator], tag: Tag) -> Result<Vec<u8>> {
    Ok(raw_entries(dict, tag)?
        .into_iter()
        .filter(|(operator, _)| !dropped.contains(operator))
        .flat_map(|(_, bytes)| bytes.iter().copied())
        .collect())
}

impl<'a> CffSubsetter<'a> {
    /// The table `data` of `version`, in a face of `units_per_em`.
    ///
    /// # Errors
    ///
    /// When the table is malformed.
    pub(crate) fn new(data: &'a [u8], version: Version, units_per_em: u16) -> Result<Self> {
        Ok(Self {
            data,
            cff: Cff::parse(data, version, units_per_em, &[])?,
        })
    }

    /// The table's version.
    pub(crate) const fn version(&self) -> Version {
        self.cff.version()
    }

    fn tag(&self) -> Tag {
        self.cff.version.tag()
    }

    fn flattened(&self, glyph: GlyphId) -> Result<Flattened> {
        let (code, font) = self.cff.glyph_program(glyph)?;
        flatten(code, &font)
    }

    /// The glyphs an implied `seac` of `glyph` draws, which fontTools
    /// keeps with it: those of the Standard Encoding's names of its base
    /// and accent codes, in a name-keyed font.
    ///
    /// # Errors
    ///
    /// When the glyph's charstring is malformed.
    pub(crate) fn components(&self, glyph: GlyphId) -> Result<Vec<GlyphId>> {
        if !self.cff.has_glyph_names() {
            return Ok(Vec::new());
        }
        let Some((base, accent)) = self.flattened(glyph)?.seac else {
            return Ok(Vec::new());
        };
        Ok([base, accent]
            .into_iter()
            .filter_map(|code| {
                let sid = STANDARD_ENCODING.get(usize::try_from(code).ok()?)?;
                self.cff.glyph_for_sid(*sid)
            })
            .collect())
    }

    /// The table keeping `kept`, original glyphs in ascending order from
    /// glyph 0, renumbered in that order.
    ///
    /// # Errors
    ///
    /// When the table is malformed, a glyph has no charset entry, or a
    /// charstring cannot do without its subroutines.
    pub(crate) fn table(&self, kept: &[GlyphId]) -> Result<Vec<u8>> {
        let charstrings = kept
            .iter()
            .map(|&glyph| Ok(self.flattened(glyph)?.program))
            .collect::<Result<Vec<_>>>()?;
        let charstrings = index(
            charstrings.iter().map(Vec::as_slice),
            self.cff.version.count_size(),
        );
        let fd_select = (self.cff.is_cid || self.cff.subfonts.len() > 1).then(|| {
            let fds: Vec<u8> = kept
                .iter()
                .map(|&glyph| self.cff.fd_select.font_dict(glyph) as u8)
                .collect();
            fd_select(&fds)
        });
        match self.cff.version {
            Version::Cff => self.cff_table(kept, &charstrings, fd_select),
            Version::Cff2 => self.cff2_table(&charstrings, fd_select),
        }
    }

    /// The Private DICT `dict` (parsed as `kind` with `stack`) points at,
    /// without its subroutines.
    fn private(&self, dict: &[u8], kind: DictKind, stack: usize) -> Result<Option<Vec<u8>>> {
        let tag = self.tag();
        let parsed = entries(dict, kind, stack, tag)?;
        let Some(&[size, offset]) = last(&parsed, OP_PRIVATE) else {
            return Ok(None);
        };
        let location = |operand: super::dict::Operand<'_>| {
            usize::try_from(operand.num())
                .map_err(|_| FontError::malformed(tag, "negative Private DICT location"))
        };
        let (size, offset) = (location(size)?, location(offset)?);
        let private = self
            .data
            .get(offset..offset.saturating_add(size))
            .ok_or(FontError::Truncated(tag))?;
        without(private, &[OP_SUBRS], tag).map(Some)
    }

    /// The Font DICTs of the `FDArray` the Top DICT `top` (parsed as
    /// `kind` with `stack`) points at.
    fn font_dicts(&self, top: &[u8], kind: DictKind, stack: usize) -> Result<Vec<&'a [u8]>> {
        let tag = self.tag();
        let parsed = entries(top, kind, stack, tag)?;
        let at = super::offset(&parsed, OP_FD_ARRAY, tag)?;
        let mut r = Reader::at(self.data, at, tag)?;
        let fd_array = Index::read_sized(&mut r, self.cff.version.count_size())?;
        (0..fd_array.len())
            .map(|i| {
                fd_array
                    .get(i)
                    .ok_or(FontError::malformed(tag, "Font DICT outside its INDEX"))
            })
            .collect()
    }

    /// A `CFF ` table: header, names, Top DICT, strings, no global
    /// subroutines, charset, `FDSelect`, `CharStrings`, `FDArray` and
    /// Private DICTs.
    fn cff_table(
        &self,
        kept: &[GlyphId],
        charstrings: &[u8],
        fd_select: Option<Vec<u8>>,
    ) -> Result<Vec<u8>> {
        let (mut r, _) = header(CffBytes::Whole(self.data), Version::Cff)?;
        let names_at = r.pos();
        Index::read(&mut r)?;
        let names = &self.data[names_at..r.pos()];
        let top = Index::read(&mut r)?
            .get(0)
            .ok_or(FontError::malformed(CFF, "missing Top DICT"))?;
        let ids = kept
            .iter()
            .skip(1)
            .map(|&glyph| {
                self.cff
                    .charset
                    .sid(glyph)
                    .ok_or(FontError::malformed(CFF, "glyph without a charset entry"))
            })
            .collect::<Result<Vec<_>>>()?;
        let font_dicts = if self.cff.is_cid {
            self.font_dicts(top, DictKind::CffTop, CFF_DICT_STACK)?
        } else {
            Vec::new()
        };
        let mut used = Strings::used_by(top)?;
        for dict in &font_dicts {
            used.extend(Strings::used_by(dict)?);
        }
        // A CID-keyed font's charset holds CIDs, not strings.
        if !self.cff.is_cid {
            used.extend(&ids);
        }
        let strings = Strings::new(self.cff.strings, used);
        let ids: Vec<u16> = if self.cff.is_cid {
            ids
        } else {
            ids.into_iter().map(|sid| strings.id(sid)).collect()
        };
        let charset = charset(&ids);
        let dropped = [
            OP_CHARSET,
            OP_ENCODING,
            OP_CHARSTRINGS,
            OP_PRIVATE,
            OP_FD_ARRAY,
            OP_FD_SELECT,
        ];
        let top_holder = Holder {
            own: strings.rewrite(top, &dropped)?,
            private: self.private(top, DictKind::CffTop, CFF_DICT_STACK)?,
        };
        let fonts = font_dicts
            .iter()
            .map(|dict| {
                Ok(Holder {
                    own: strings.rewrite(dict, &[OP_PRIVATE])?,
                    private: self.private(dict, DictKind::CffTop, CFF_DICT_STACK)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let string_index = strings.string_index()?;
        let mut top_length =
            top_holder.length() + entry_length(OP_CHARSET, 1) + entry_length(OP_CHARSTRINGS, 1);
        if self.cff.is_cid {
            top_length += entry_length(OP_FD_SELECT, 1) + entry_length(OP_FD_ARRAY, 1);
        }
        let mut at = 4 + names.len() + index_length([top_length], CountSize::Short);
        at += string_index.len() + index_length([], CountSize::Short);
        let charset_at = at;
        at += charset.len();
        let fd_select_at = at;
        at += fd_select.as_ref().map_or(0, Vec::len);
        let charstrings_at = at;
        at += charstrings.len();
        let fd_array_at = at;
        if self.cff.is_cid {
            at += index_length(fonts.iter().map(Holder::length), CountSize::Short);
        }
        let mut top_dict = top_holder.bytes(at);
        at += top_holder.private.as_ref().map_or(0, Vec::len);
        top_dict.extend(entry(OP_CHARSET, &[charset_at]));
        top_dict.extend(entry(OP_CHARSTRINGS, &[charstrings_at]));
        if self.cff.is_cid {
            top_dict.extend(entry(OP_FD_SELECT, &[fd_select_at]));
            top_dict.extend(entry(OP_FD_ARRAY, &[fd_array_at]));
        }
        let (fd_array, privates) = Self::font_dict_array(&fonts, at, CountSize::Short);
        let mut out = vec![1, 0, 4, 4];
        out.extend(names);
        out.extend(index([top_dict.as_slice()], CountSize::Short));
        out.extend(string_index);
        out.extend(index([], CountSize::Short));
        out.extend(charset);
        out.extend(fd_select.unwrap_or_default());
        out.extend(charstrings);
        if self.cff.is_cid {
            out.extend(fd_array);
        }
        out.extend(top_holder.private.unwrap_or_default());
        out.extend(privates);
        Ok(out)
    }

    /// The `FDArray` INDEX of `fonts` and their Private DICTs, these
    /// written from `at` on.
    fn font_dict_array(fonts: &[Holder], mut at: usize, count: CountSize) -> (Vec<u8>, Vec<u8>) {
        let mut dicts = Vec::with_capacity(fonts.len());
        let mut privates = Vec::new();
        for font in fonts {
            dicts.push(font.bytes(at));
            if let Some(private) = &font.private {
                at += private.len();
                privates.extend(private);
            }
        }
        (index(dicts.iter().map(Vec::as_slice), count), privates)
    }

    /// A `CFF2` table: header, Top DICT, no global subroutines, variation
    /// store, `FDSelect`, `CharStrings`, `FDArray` and Private DICTs.
    fn cff2_table(&self, charstrings: &[u8], fd_select: Option<Vec<u8>>) -> Result<Vec<u8>> {
        let tag = self.tag();
        let (mut r, top_length) = header(CffBytes::Whole(self.data), Version::Cff2)?;
        let top = r.bytes(top_length)?;
        let parsed = entries(top, DictKind::Cff2Top, CFF2_STACK, tag)?;
        let vstore = match super::dict::number(&parsed, OP_VSTORE) {
            Some(at) if at != 0 => {
                let at = usize::try_from(at).map_err(|_| FontError::Truncated(tag))?;
                let length = usize::from(Reader::at(self.data, at, tag)?.u16()?);
                self.data
                    .get(at..at + 2 + length)
                    .ok_or(FontError::Truncated(tag))?
            }
            _ => &[],
        };
        let fonts = self
            .font_dicts(top, DictKind::Cff2Top, CFF2_STACK)?
            .into_iter()
            .map(|dict| {
                Ok(Holder {
                    own: without(dict, &[OP_PRIVATE], tag)?,
                    private: self.private(dict, DictKind::Cff2Font, CFF2_STACK)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let own = without(
            top,
            &[OP_CHARSTRINGS, OP_FD_ARRAY, OP_FD_SELECT, OP_VSTORE],
            tag,
        )?;
        let mut new_length =
            own.len() + entry_length(OP_CHARSTRINGS, 1) + entry_length(OP_FD_ARRAY, 1);
        if !vstore.is_empty() {
            new_length += entry_length(OP_VSTORE, 1);
        }
        if fd_select.is_some() {
            new_length += entry_length(OP_FD_SELECT, 1);
        }
        let new_length_u16 = u16::try_from(new_length)
            .map_err(|_| FontError::malformed(tag, "Top DICT too long"))?;
        let mut at = 5 + new_length + index_length([], CountSize::Long);
        let vstore_at = at;
        at += vstore.len();
        let fd_select_at = at;
        at += fd_select.as_ref().map_or(0, Vec::len);
        let charstrings_at = at;
        at += charstrings.len();
        let fd_array_at = at;
        at += index_length(fonts.iter().map(Holder::length), CountSize::Long);
        let mut top_dict = own;
        if !vstore.is_empty() {
            top_dict.extend(entry(OP_VSTORE, &[vstore_at]));
        }
        if fd_select.is_some() {
            top_dict.extend(entry(OP_FD_SELECT, &[fd_select_at]));
        }
        top_dict.extend(entry(OP_FD_ARRAY, &[fd_array_at]));
        top_dict.extend(entry(OP_CHARSTRINGS, &[charstrings_at]));
        let (fd_array, privates) = Self::font_dict_array(&fonts, at, CountSize::Long);
        let mut out = vec![2, 0, 5];
        out.extend(new_length_u16.to_be_bytes());
        out.extend(top_dict);
        out.extend(index([], CountSize::Long));
        out.extend(vstore);
        out.extend(fd_select.unwrap_or_default());
        out.extend(charstrings);
        out.extend(fd_array);
        out.extend(privates);
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
