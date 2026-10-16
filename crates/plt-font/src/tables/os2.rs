//! `OS/2`: weight, width, vertical metrics and style selection flags.

use crate::error::Result;
use crate::reader::Reader;
use crate::types::Tag;

const OS2: Tag = Tag::new(b"OS/2");

/// Size and offset of sub- or superscripts, in font units.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ScriptMetrics {
    /// Horizontal size.
    pub x_size: i16,
    /// Vertical size.
    pub y_size: i16,
    /// Horizontal offset.
    pub x_offset: i16,
    /// Vertical offset.
    pub y_offset: i16,
}

/// Values from the `OS/2` table. Fields introduced by later table versions
/// are `None` when the table is older.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Os2 {
    /// Table version.
    pub version: u16,
    /// Average advance of lowercase glyphs.
    pub avg_char_width: i16,
    /// Visual weight, 1–1000 (400 regular, 700 bold).
    pub weight_class: u16,
    /// Width class, 1 (ultra-condensed) to 9 (ultra-expanded).
    pub width_class: u16,
    /// Embedding licensing flags.
    pub fs_type: u16,
    /// Subscript placement.
    pub subscript: ScriptMetrics,
    /// Superscript placement.
    pub superscript: ScriptMetrics,
    /// Strikeout stroke thickness.
    pub strikeout_size: i16,
    /// Strikeout stroke position above the baseline.
    pub strikeout_position: i16,
    /// IBM font family class.
    pub family_class: i16,
    /// PANOSE classification.
    pub panose: [u8; 10],
    /// Unicode range bits.
    pub unicode_ranges: [u32; 4],
    /// Font vendor identifier.
    pub vendor_id: Tag,
    /// Style selection flags (bit 0 italic, bit 5 bold, bit 6 regular,
    /// bit 7 use typo metrics, bit 8 WWS, bit 9 oblique).
    pub fs_selection: u16,
    /// Smallest character code in the font.
    pub first_char_index: u16,
    /// Largest BMP character code in the font.
    pub last_char_index: u16,
    /// Typographic ascender (zero in short version 0 tables).
    pub typo_ascender: i16,
    /// Typographic descender.
    pub typo_descender: i16,
    /// Typographic line gap.
    pub typo_line_gap: i16,
    /// Windows clipping ascent.
    pub win_ascent: u16,
    /// Windows clipping descent.
    pub win_descent: u16,
    /// Code page bits (version ≥ 1).
    pub code_page_ranges: Option<[u32; 2]>,
    /// Height of lowercase x (version ≥ 2).
    pub x_height: Option<i16>,
    /// Height of uppercase letters (version ≥ 2).
    pub cap_height: Option<i16>,
}

impl Os2 {
    /// Parses the table. Apple's short version 0 tables (68 bytes, without
    /// typographic metrics) are accepted like FreeType does.
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, OS2);
        let version = r.u16()?;
        let avg_char_width = r.i16()?;
        let weight_class = r.u16()?;
        let width_class = r.u16()?;
        let fs_type = r.u16()?;
        let mut script = || -> Result<ScriptMetrics> {
            Ok(ScriptMetrics {
                x_size: r.i16()?,
                y_size: r.i16()?,
                x_offset: r.i16()?,
                y_offset: r.i16()?,
            })
        };
        let subscript = script()?;
        let superscript = script()?;
        let strikeout_size = r.i16()?;
        let strikeout_position = r.i16()?;
        let family_class = r.i16()?;
        let mut panose = [0; 10];
        panose.copy_from_slice(r.bytes(10)?);
        let unicode_ranges = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
        let vendor_id = r.tag()?;
        let fs_selection = r.u16()?;
        let first_char_index = r.u16()?;
        let last_char_index = r.u16()?;
        let has_typo = r.remaining() >= 10;
        let mut field = |present: bool| -> Result<i16> { if present { r.i16() } else { Ok(0) } };
        let typo_ascender = field(has_typo)?;
        let typo_descender = field(has_typo)?;
        let typo_line_gap = field(has_typo)?;
        let win_ascent = field(has_typo)? as u16;
        let win_descent = field(has_typo)? as u16;
        let code_page_ranges = if version >= 1 && r.remaining() >= 8 {
            Some([r.u32()?, r.u32()?])
        } else {
            None
        };
        let (x_height, cap_height) = if version >= 2 && r.remaining() >= 4 {
            (Some(r.i16()?), Some(r.i16()?))
        } else {
            (None, None)
        };
        Ok(Self {
            version,
            avg_char_width,
            weight_class,
            width_class,
            fs_type,
            subscript,
            superscript,
            strikeout_size,
            strikeout_position,
            family_class,
            panose,
            unicode_ranges,
            vendor_id,
            fs_selection,
            first_char_index,
            last_char_index,
            typo_ascender,
            typo_descender,
            typo_line_gap,
            win_ascent,
            win_descent,
            code_page_ranges,
            x_height,
            cap_height,
        })
    }

    /// `fsSelection` bit 0 (italic) or bit 9 (oblique).
    pub const fn is_italic(&self) -> bool {
        self.fs_selection & 0x0201 != 0
    }

    /// `fsSelection` bit 5.
    pub const fn is_bold(&self) -> bool {
        self.fs_selection & 0x0020 != 0
    }

    /// `fsSelection` bit 8: names 1/2 follow the weight/width/slope model.
    pub const fn is_wws(&self) -> bool {
        self.fs_selection & 0x0100 != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os2(version: u16, len: usize) -> Vec<u8> {
        let mut data = version.to_be_bytes().to_vec();
        data.extend_from_slice(&[0x04, 0x00, 0x02, 0xBC, 0, 5, 0, 0]);
        data.resize(62, 7);
        data.extend_from_slice(&[0x02, 0x21, 0, 32, 0xFF, 0xFF]);
        for v in [1901i16, -483, 0, 1901, 483, 0, 1, 0, 0, 1062, 1493] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        data.truncate(len);
        data
    }

    #[test]
    fn parses_full_and_short_tables() {
        let full = Os2::parse(&os2(2, 200)).unwrap();
        assert_eq!(full.weight_class, 700);
        assert_eq!(full.width_class, 5);
        assert!(full.is_italic() && full.is_bold() && !full.is_wws());
        assert_eq!((full.typo_ascender, full.typo_descender), (1901, -483));
        assert_eq!((full.win_ascent, full.win_descent), (1901, 483));
        assert_eq!(full.code_page_ranges, Some([1, 0]));
        assert_eq!((full.x_height, full.cap_height), (Some(1062), Some(1493)));

        let v1 = Os2::parse(&os2(1, 86)).unwrap();
        assert_eq!(v1.x_height, None);
        let short = Os2::parse(&os2(0, 68)).unwrap();
        assert_eq!(short.typo_ascender, 0);
        assert_eq!(short.code_page_ranges, None);
        assert!(Os2::parse(&os2(0, 60)).is_err());
    }
}
