//! `CFF ` table: Compact Font Format outlines (OpenType/CFF fonts, `OTTO`).
//!
//! Supports name-keyed and CID-keyed fonts, local/global subroutines, the
//! `seac` accent composition and glyph names from the charset.

mod bytes;
mod charstring;
mod index;
mod strings;

use crate::error::{FontError, Result};
use crate::outline::Outline;
use crate::reader::{Array, Reader};
use crate::tables::glyf::Scaling;
use crate::types::GlyphId;

use bytes::CffBytes;
pub(crate) use bytes::CffParts;
use charstring::{Subrs, interpret};
pub(crate) use index::Index;
use index::{CFF, Operator, dict_get, dict_usize, parse_dict};
pub(crate) use strings::{STANDARD_ENCODING, STANDARD_STRINGS};

const OP_CHARSET: Operator = 15;
const OP_CHARSTRINGS: Operator = 17;
const OP_PRIVATE: Operator = 18;
const OP_SUBRS: Operator = 19;
const OP_ROS: Operator = 1230;
const OP_FD_ARRAY: Operator = 1236;
const OP_FD_SELECT: Operator = 1237;

/// Number of glyphs covered by the predefined `ISOAdobe` charset.
const ISO_ADOBE_GLYPHS: u16 = 229;

/// Glyph id to string id mapping.
#[derive(Clone, Copy, Debug)]
enum Charset<'a> {
    IsoAdobe,
    Unsupported,
    Sids(Array<'a, u16>),
    Ranges { data: &'a [u8], wide: bool },
}

impl Charset<'_> {
    fn sid(&self, glyph: GlyphId) -> Option<u16> {
        if glyph.0 == 0 {
            return Some(0);
        }
        match self {
            Self::IsoAdobe => (glyph.0 < ISO_ADOBE_GLYPHS).then_some(glyph.0),
            Self::Unsupported => None,
            Self::Sids(sids) => sids.get(usize::from(glyph.0) - 1),
            Self::Ranges { data, wide } => {
                let mut r = Reader::new(data, CFF);
                let mut gid = 1u32;
                while r.remaining() > 0 {
                    let first = r.u16().ok()?;
                    let left = if *wide {
                        u32::from(r.u16().ok()?)
                    } else {
                        u32::from(r.u8().ok()?)
                    };
                    let target = u32::from(glyph.0);
                    if target <= gid + left {
                        return u16::try_from(u32::from(first) + target - gid).ok();
                    }
                    gid += left + 1;
                }
                None
            }
        }
    }
}

/// Glyph id to font dict index for CID-keyed fonts.
#[derive(Clone, Copy, Debug)]
enum FdSelect<'a> {
    Single,
    PerGlyph(&'a [u8]),
    Ranges(&'a [u8]),
}

impl FdSelect<'_> {
    fn font_dict(&self, glyph: GlyphId) -> usize {
        match self {
            Self::Single => 0,
            Self::PerGlyph(fds) => fds
                .get(usize::from(glyph.0))
                .map_or(0, |&fd| usize::from(fd)),
            Self::Ranges(data) => {
                let mut r = Reader::new(data, CFF);
                let Ok(count) = r.u16() else { return 0 };
                let mut found = 0;
                for _ in 0..count {
                    let (Ok(first), Ok(fd)) = (r.u16(), r.u8()) else {
                        return found;
                    };
                    if first > glyph.0 {
                        break;
                    }
                    found = usize::from(fd);
                }
                found
            }
        }
    }
}

/// A parsed CFF table.
#[derive(Clone, Debug)]
pub(crate) struct Cff<'a> {
    strings: Index<'a>,
    global_subrs: Index<'a>,
    charstrings: Index<'a>,
    charset: Charset<'a>,
    local_subrs: Vec<Index<'a>>,
    fd_select: FdSelect<'a>,
    is_cid: bool,
}

/// Reads the local subroutines referenced by a Private DICT entry.
fn private_subrs<'a>(data: CffBytes<'a>, private: Option<&[f64]>) -> Result<Index<'a>> {
    let Some(&[size, offset]) = private else {
        return Ok(Index::default());
    };
    let (size, offset) = (size as usize, offset as usize);
    let dict = parse_dict(
        data.get(offset..offset.saturating_add(size))
            .ok_or(FontError::Truncated(CFF))?,
    )?;
    match dict_usize(&dict, OP_SUBRS) {
        Some(subrs) => Index::read(&mut data.at(offset + subrs)?),
        None => Ok(Index::default()),
    }
}

/// The Top DICT INDEX of the CFF table `data` (checking its version) and
/// a reader after it.
fn top_index(data: CffBytes<'_>) -> Result<(Index<'_>, Reader<'_>)> {
    let mut r = data.at(0)?;
    let major = r.u8()?;
    if major != 1 {
        return Err(FontError::UnsupportedFormat {
            table: CFF,
            format: u32::from(major),
        });
    }
    r.skip(1)?;
    let header_size = usize::from(r.u8()?);
    let mut r = data.at(header_size)?;
    Index::read(&mut r)?;
    let top = Index::read(&mut r)?;
    Ok((top, r))
}

/// The Top DICT of a Top DICT INDEX.
fn top_dict(top: &Index<'_>) -> Result<Vec<(Operator, Vec<f64>)>> {
    parse_dict(
        top.get(0)
            .ok_or(FontError::malformed(CFF, "missing Top DICT"))?,
    )
}

/// Where the `CharStrings` INDEX of a CFF table is, in the table, when
/// `head`, its first bytes, hold its header, Name INDEX and Top DICT
/// INDEX: what a font file held for drawing reads glyph by glyph.
pub(crate) fn charstrings_offset(head: &[u8]) -> Result<usize> {
    let top = top_dict(&top_index(CffBytes::Whole(head))?.0)?;
    dict_usize(&top, OP_CHARSTRINGS).ok_or(FontError::malformed(CFF, "no CharStrings"))
}

impl<'a> Cff<'a> {
    /// Parses a CFF table.
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self> {
        Self::parse_bytes(CffBytes::Whole(data))
    }

    /// Parses a CFF table held but for its `CharStrings` data, which are
    /// then read glyph by glyph.
    pub(crate) fn parse_parts(parts: CffParts<'a>) -> Result<Self> {
        Self::parse_bytes(CffBytes::Parts(parts))
    }

    fn parse_bytes(data: CffBytes<'a>) -> Result<Self> {
        let (top, mut r) = top_index(data)?;
        let strings = Index::read(&mut r)?;
        let global_subrs = Index::read(&mut r)?;
        let top = top_dict(&top)?;

        let charstrings_at =
            dict_usize(&top, OP_CHARSTRINGS).ok_or(FontError::malformed(CFF, "no CharStrings"))?;
        let mut r = data.at(charstrings_at)?;
        let charstrings = match data.charstrings() {
            Some(held_elsewhere) => Index::read_offsets(&mut r, held_elsewhere)?,
            None => Index::read(&mut r)?,
        };
        let is_cid = dict_get(&top, OP_ROS).is_some();
        let charset = Self::charset(
            data,
            dict_usize(&top, OP_CHARSET).unwrap_or(0),
            charstrings.len(),
        )?;

        let (local_subrs, fd_select) = if is_cid {
            let fd_array_at = dict_usize(&top, OP_FD_ARRAY)
                .ok_or(FontError::malformed(CFF, "CID font without FDArray"))?;
            let fd_array = Index::read(&mut data.at(fd_array_at)?)?;
            let subrs = (0..fd_array.len())
                .map(|i| {
                    let dict = parse_dict(fd_array.get(i).unwrap_or(&[]))?;
                    private_subrs(data, dict_get(&dict, OP_PRIVATE))
                })
                .collect::<Result<Vec<_>>>()?;
            let select_at = dict_usize(&top, OP_FD_SELECT)
                .ok_or(FontError::malformed(CFF, "CID font without FDSelect"))?;
            let select = data.tail(select_at)?;
            let fd_select = match select.first() {
                Some(0) => FdSelect::PerGlyph(&select[1..]),
                Some(3) => FdSelect::Ranges(&select[1..]),
                _ => return Err(FontError::malformed(CFF, "unsupported FDSelect format")),
            };
            (subrs, fd_select)
        } else {
            (
                vec![private_subrs(data, dict_get(&top, OP_PRIVATE))?],
                FdSelect::Single,
            )
        };
        Ok(Self {
            strings,
            global_subrs,
            charstrings,
            charset,
            local_subrs,
            fd_select,
            is_cid,
        })
    }

    fn charset(data: CffBytes<'a>, offset: usize, glyphs: usize) -> Result<Charset<'a>> {
        match offset {
            0 => Ok(Charset::IsoAdobe),
            1 | 2 => Ok(Charset::Unsupported),
            _ => {
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

    /// Whether glyph names are available (name-keyed fonts only).
    pub(crate) const fn has_glyph_names(&self) -> bool {
        !self.is_cid
    }

    /// PostScript name of `glyph`.
    pub(crate) fn glyph_name(&self, glyph: GlyphId) -> Option<&'a str> {
        if self.is_cid {
            return None;
        }
        let sid = usize::from(self.charset.sid(glyph)?);
        match sid.checked_sub(STANDARD_STRINGS.len()) {
            None => STANDARD_STRINGS.get(sid).copied(),
            Some(custom) => std::str::from_utf8(self.strings.get(custom)?).ok(),
        }
    }

    /// Glyph whose charset SID is `sid` (used for `seac` components).
    fn glyph_for_sid(&self, sid: u16) -> Option<GlyphId> {
        (0..self.charstrings.len())
            .filter_map(|g| u16::try_from(g).ok())
            .map(GlyphId)
            .find(|&g| self.charset.sid(g) == Some(sid))
    }

    fn load_unscaled(&self, glyph: GlyphId, outline: &mut Outline, allow_seac: bool) -> Result<()> {
        let code = self
            .charstrings
            .get(usize::from(glyph.0))
            .ok_or(FontError::malformed(CFF, "glyph without charstring"))?;
        let local = self
            .local_subrs
            .get(self.fd_select.font_dict(glyph))
            .copied()
            .unwrap_or_default();
        let start = outline.points.len();
        let seac = interpret(
            code,
            Subrs {
                global: self.global_subrs,
                local,
            },
            outline,
        )?;
        let Some(seac) = seac.filter(|_| allow_seac) else {
            return Ok(());
        };
        let component = |code: u8| {
            let sid = STANDARD_ENCODING[usize::from(code)];
            self.glyph_for_sid(sid)
                .ok_or(FontError::malformed(CFF, "seac component not in charset"))
        };
        let (base, accent) = (component(seac.base)?, component(seac.accent)?);
        outline.points.truncate(start);
        outline.contour_ends.retain(|&end| end <= start);
        // FreeType draws the accent first, then the base.
        self.load_unscaled(accent, outline, false)?;
        for point in &mut outline.points[start..] {
            point.x += seac.dx;
            point.y += seac.dy;
        }
        self.load_unscaled(base, outline, false)
    }

    /// Appends the outline of `glyph` in the requested coordinate space.
    pub(crate) fn load(
        &self,
        glyph: GlyphId,
        scaling: Scaling,
        outline: &mut Outline,
    ) -> Result<()> {
        let start = outline.points.len();
        self.load_unscaled(glyph, outline, true)?;
        if let Scaling::Fixed { x_scale, y_scale } = scaling {
            for point in &mut outline.points[start..] {
                point.x = (point.x * x_scale as f64 / 65536.0).round();
                point.y = (point.y * y_scale as f64 / 65536.0).round();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encodes an INDEX with 1-byte offsets.
    fn index(items: &[&[u8]]) -> Vec<u8> {
        let mut out = (items.len() as u16).to_be_bytes().to_vec();
        if items.is_empty() {
            return out;
        }
        out.push(1);
        let mut offset = 1u8;
        out.push(offset);
        for item in items {
            offset += item.len() as u8;
            out.push(offset);
        }
        for item in items {
            out.extend_from_slice(item);
        }
        out
    }

    fn n(v: i32) -> u8 {
        (v + 139) as u8
    }

    /// A name-keyed CFF with glyphs `.notdef`, `A` (a square) and `Aacute`
    /// built with `seac` from `A` and `acute`.
    fn font() -> Vec<u8> {
        let notdef: &[u8] = &[14];
        let a: &[u8] = &[n(0), n(0), 21, n(10), 6, n(10), 7, n(-10), 6, 14];
        let acute: &[u8] = &[n(0), n(20), 21, n(2), n(2), 5, 14];
        let aacute: &[u8] = &[n(0), n(1), n(5), n(65), 247, 86, 14];
        let charstrings = index(&[notdef, a, acute, aacute]);
        let strings = index(&[b"Aacute"]);
        // charset format 0: SIDs for glyphs 1..: A=34, acute=125, Aacute=391.
        let charset = [0u8, 0, 34, 0, 125, 1, 135];
        let header = [1u8, 0, 4, 1];
        let name = index(&[b"Test"]);
        let globals = index(&[]);
        // Top DICT with two 2-byte-encoded offsets (filled below).
        let top_len = 3 + 1 + 3 + 1;
        let fixed = header.len() + name.len() + 2 + 1 + 2 + top_len + strings.len() + globals.len();
        let charset_at = fixed;
        let charstrings_at = charset_at + charset.len();
        let top = [28, 0, charset_at as u8, 15, 28, 0, charstrings_at as u8, 17];
        let mut out = header.to_vec();
        out.extend(name);
        out.extend(index(&[&top]));
        out.extend(strings);
        out.extend(globals);
        assert_eq!(out.len(), charset_at);
        out.extend_from_slice(&charset);
        out.extend(charstrings);
        out
    }

    #[test]
    fn names_outlines_and_seac() {
        let data = font();
        let cff = Cff::parse(&data).unwrap();
        assert!(cff.has_glyph_names());
        assert_eq!(cff.glyph_name(GlyphId(0)), Some(".notdef"));
        assert_eq!(cff.glyph_name(GlyphId(1)), Some("A"));
        assert_eq!(cff.glyph_name(GlyphId(2)), Some("acute"));
        assert_eq!(cff.glyph_name(GlyphId(3)), Some("Aacute"));
        let mut square = Outline::default();
        cff.load(GlyphId(1), Scaling::Units, &mut square).unwrap();
        assert_eq!(square.points.len(), 4);
        let mut composite = Outline::default();
        cff.load(GlyphId(3), Scaling::Units, &mut composite)
            .unwrap();
        assert_eq!(composite.contour_ends, vec![2, 6]);
        assert_eq!((composite.points[0].x, composite.points[0].y), (1.0, 25.0));
        let mut scaled = Outline::default();
        cff.load(
            GlyphId(1),
            Scaling::Fixed {
                x_scale: 0x2_0000,
                y_scale: 0x2_0000,
            },
            &mut scaled,
        )
        .unwrap();
        assert_eq!(scaled.points[1].x, 20.0);
    }

    #[test]
    fn truncated_cff_errors() {
        let data = font();
        for len in 0..data.len() - 1 {
            if let Ok(cff) = Cff::parse(&data[..len]) {
                let mut outline = Outline::default();
                let _ = cff.load(GlyphId(3), Scaling::Units, &mut outline);
            }
        }
        assert!(Cff::parse(&[2, 0, 4, 1]).is_err());
    }

    #[test]
    fn charset_ranges_and_fd_select() {
        let charset = Charset::Ranges {
            data: &[0, 10, 2, 0, 50, 0],
            wide: false,
        };
        assert_eq!(charset.sid(GlyphId(1)), Some(10));
        assert_eq!(charset.sid(GlyphId(3)), Some(12));
        assert_eq!(charset.sid(GlyphId(4)), Some(50));
        assert_eq!(charset.sid(GlyphId(5)), None);
        let select = FdSelect::Ranges(&[0, 2, 0, 0, 1, 0, 5, 2, 0, 9]);
        assert_eq!(select.font_dict(GlyphId(3)), 1);
        assert_eq!(select.font_dict(GlyphId(5)), 2);
        assert_eq!(FdSelect::PerGlyph(&[0, 3]).font_dict(GlyphId(1)), 3);
    }
}
