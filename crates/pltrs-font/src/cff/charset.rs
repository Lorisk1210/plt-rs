//! The charset (glyph to string id) and the `FDSelect` (glyph to Font
//! DICT) of a CFF table.

use super::index::CFF;
use crate::error::{FontError, Result};
use crate::reader::{Array, Reader};
use crate::types::GlyphId;

/// Number of glyphs covered by the predefined `ISOAdobe` charset.
const ISO_ADOBE_GLYPHS: u16 = 229;

/// Glyph id to string id mapping.
#[derive(Clone, Copy, Debug)]
pub(super) enum Charset<'a> {
    IsoAdobe,
    /// A predefined charset FreeType does not map (or a CFF2 table, which
    /// has none).
    Unsupported,
    Sids(Array<'a, u16>),
    Ranges {
        data: &'a [u8],
        wide: bool,
    },
}

impl Charset<'_> {
    /// The string id of `glyph`.
    pub(super) fn sid(&self, glyph: GlyphId) -> Option<u16> {
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

/// Glyph id to Font DICT index (`CFF_FDSelect`): FreeType reads formats 0
/// and 3.
#[derive(Clone, Copy, Debug)]
pub(super) enum FdSelect<'a> {
    /// No `FDSelect`: every glyph uses the first Font DICT.
    Single,
    PerGlyph(&'a [u8]),
    Ranges(&'a [u8]),
}

impl<'a> FdSelect<'a> {
    /// `CFF_Load_FD_Select`: the `FDSelect` at the start of `data`, for
    /// `glyphs` glyphs.
    pub(super) fn parse(data: &'a [u8], glyphs: usize) -> Result<Self> {
        let mut r = Reader::new(data, CFF);
        match r.u8()? {
            0 => Ok(Self::PerGlyph(r.bytes(glyphs)?)),
            3 => {
                let ranges = usize::from(r.u16()?);
                if ranges == 0 {
                    return Err(FontError::malformed(CFF, "FDSelect without ranges"));
                }
                r.skip(ranges * 3 + 2)?;
                Ok(Self::Ranges(&data[1..r.pos()]))
            }
            format => Err(FontError::UnsupportedFormat {
                table: CFF,
                format: u32::from(format),
            }),
        }
    }

    /// `cff_fd_select_get`: the Font DICT of `glyph`; 0 for glyphs no range
    /// covers.
    pub(super) fn font_dict(&self, glyph: GlyphId) -> usize {
        match self {
            Self::Single => 0,
            Self::PerGlyph(fds) => fds
                .get(usize::from(glyph.0))
                .map_or(0, |&fd| usize::from(fd)),
            Self::Ranges(data) => {
                let mut r = Reader::new(data, CFF);
                let (Ok(count), Ok(mut first)) = (r.u16(), r.u16()) else {
                    return 0;
                };
                for _ in 0..count {
                    if glyph.0 < first {
                        break;
                    }
                    let (Ok(fd), Ok(limit)) = (r.u8(), r.u16()) else {
                        break;
                    };
                    if glyph.0 < limit {
                        return usize::from(fd);
                    }
                    first = limit;
                }
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charset_ranges() {
        let charset = Charset::Ranges {
            data: &[0, 10, 2, 0, 50, 0],
            wide: false,
        };
        assert_eq!(charset.sid(GlyphId(0)), Some(0));
        assert_eq!(charset.sid(GlyphId(1)), Some(10));
        assert_eq!(charset.sid(GlyphId(3)), Some(12));
        assert_eq!(charset.sid(GlyphId(4)), Some(50));
        assert_eq!(charset.sid(GlyphId(5)), None);
        assert_eq!(Charset::IsoAdobe.sid(GlyphId(228)), Some(228));
        assert_eq!(Charset::IsoAdobe.sid(GlyphId(229)), None);
        assert_eq!(Charset::Unsupported.sid(GlyphId(3)), None);
    }

    #[test]
    fn fd_select_formats() {
        // Format 3: glyphs 0 to 4 in FD 0, 5 to 8 in FD 2, sentinel 9.
        let ranges = [3, 0, 2, 0, 0, 0, 0, 5, 2, 0, 9];
        let select = FdSelect::parse(&ranges, 9).unwrap();
        assert_eq!(select.font_dict(GlyphId(3)), 0);
        assert_eq!(select.font_dict(GlyphId(5)), 2);
        assert_eq!(select.font_dict(GlyphId(9)), 0);
        let per_glyph = FdSelect::parse(&[0, 1, 3], 2).unwrap();
        assert_eq!(per_glyph.font_dict(GlyphId(1)), 3);
        assert_eq!(per_glyph.font_dict(GlyphId(7)), 0);
        assert!(FdSelect::parse(&[0, 1], 2).is_err());
        assert!(FdSelect::parse(&[3, 0, 0, 0, 0], 2).is_err());
        assert!(FdSelect::parse(&[3, 0, 2, 0, 0], 2).is_err());
        assert!(FdSelect::parse(&[4, 0, 0, 0, 1], 2).is_err());
        assert_eq!(FdSelect::Single.font_dict(GlyphId(7)), 0);
    }
}
