//! `head`, `hhea` and `maxp`: global font header values.

use crate::error::{FontError, Result};
use crate::reader::Reader;
use crate::types::{Rect, Tag};

const HEAD: Tag = Tag::new(b"head");
const HHEA: Tag = Tag::new(b"hhea");
const MAXP: Tag = Tag::new(b"maxp");

/// Values from the `head` table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Head {
    /// Font revision as a 16.16 fixed-point number.
    pub font_revision: i32,
    /// Header flags (bit 3: integer ppem scaling hint).
    pub flags: u16,
    /// Design units per em square.
    pub units_per_em: u16,
    /// Union of all glyph bounding boxes, in font units.
    pub bbox: Rect,
    /// Macintosh style bits (bit 0 bold, bit 1 italic).
    pub mac_style: u16,
    /// Smallest readable size in pixels.
    pub lowest_rec_ppem: u16,
    /// `loca` offset format: 0 for 16-bit, 1 for 32-bit offsets.
    pub index_to_loc_format: i16,
}

impl Head {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, HEAD);
        let major = r.u16()?;
        if major != 1 {
            return Err(FontError::UnsupportedFormat {
                table: HEAD,
                format: u32::from(major),
            });
        }
        r.skip(2)?;
        let font_revision = r.i32()?;
        r.skip(8)?;
        let flags = r.u16()?;
        let units_per_em = r.u16()?;
        if !(16..=16384).contains(&units_per_em) {
            return Err(FontError::malformed(HEAD, "unitsPerEm outside 16..=16384"));
        }
        r.skip(16)?;
        let bbox = Rect {
            x_min: i32::from(r.i16()?),
            y_min: i32::from(r.i16()?),
            x_max: i32::from(r.i16()?),
            y_max: i32::from(r.i16()?),
        };
        let mac_style = r.u16()?;
        let lowest_rec_ppem = r.u16()?;
        r.skip(2)?;
        let index_to_loc_format = r.i16()?;
        Ok(Self {
            font_revision,
            flags,
            units_per_em,
            bbox,
            mac_style,
            lowest_rec_ppem,
            index_to_loc_format,
        })
    }
}

/// Values from the `hhea` table.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hhea {
    /// Typographic ascender in font units.
    pub ascender: i16,
    /// Typographic descender in font units (usually negative).
    pub descender: i16,
    /// Extra line spacing in font units.
    pub line_gap: i16,
    /// Largest advance width.
    pub advance_width_max: u16,
    /// Caret slope numerator.
    pub caret_slope_rise: i16,
    /// Caret slope denominator.
    pub caret_slope_run: i16,
    /// Number of long metrics in `hmtx`.
    pub number_of_h_metrics: u16,
}

impl Hhea {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, HHEA);
        r.skip(4)?;
        let ascender = r.i16()?;
        let descender = r.i16()?;
        let line_gap = r.i16()?;
        let advance_width_max = r.u16()?;
        r.skip(6)?;
        let caret_slope_rise = r.i16()?;
        let caret_slope_run = r.i16()?;
        r.skip(12)?;
        let number_of_h_metrics = r.u16()?;
        Ok(Self {
            ascender,
            descender,
            line_gap,
            advance_width_max,
            caret_slope_rise,
            caret_slope_run,
            number_of_h_metrics,
        })
    }
}

/// The `maxp` values FreeType uses: the glyph count plus the TrueType
/// interpreter's resource limits, adjusted like `tt_face_load_maxp`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) struct MaxProfile {
    /// Number of glyphs.
    pub(crate) num_glyphs: u16,
    /// Points in the twilight zone (at most `0xFFFF - 4`).
    pub(crate) max_twilight_points: u16,
    /// Storage area locations.
    pub(crate) max_storage: u16,
    /// Function definitions (at least 64).
    pub(crate) max_function_defs: u16,
    /// Instruction definitions.
    pub(crate) max_instruction_defs: u16,
    /// Interpreter stack depth.
    pub(crate) max_stack_elements: u16,
}

impl MaxProfile {
    /// Parses a version 0.5 (CFF) or 1.0 (TrueType) `maxp` table; the
    /// limits of a truncated version 1.0 table read as zero.
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, MAXP);
        let version = r.u32()?;
        if version != 0x0000_5000 && version != 0x0001_0000 {
            return Err(FontError::UnsupportedFormat {
                table: MAXP,
                format: version,
            });
        }
        let num_glyphs = r.u16()?;
        let mut profile = Self {
            num_glyphs,
            ..Self::default()
        };
        if version == 0x0001_0000 {
            let mut limits = [0u16; 13];
            for limit in &mut limits {
                match r.u16() {
                    Ok(value) => *limit = value,
                    Err(_) => return Ok(profile),
                }
            }
            profile.max_twilight_points = limits[5].min(0xFFFF - 4);
            profile.max_storage = limits[6];
            profile.max_function_defs = limits[7].max(64);
            profile.max_instruction_defs = limits[8];
            profile.max_stack_elements = limits[9];
        }
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn head_bytes(upem: u16) -> Vec<u8> {
        let mut data = vec![
            0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0x5F, 0x0F, 0x3C, 0xF5, 0, 0x1F,
        ];
        data.extend_from_slice(&upem.to_be_bytes());
        data.extend_from_slice(&[0; 16]);
        for v in [-100i16, -200, 1000, 900] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        data.extend_from_slice(&[0, 1, 0, 8, 0, 2, 0, 1, 0, 0]);
        data
    }

    #[test]
    fn parses_head() {
        let head = Head::parse(&head_bytes(2048)).unwrap();
        assert_eq!(head.units_per_em, 2048);
        assert_eq!(head.flags, 0x1F);
        assert_eq!(
            head.bbox,
            Rect {
                x_min: -100,
                y_min: -200,
                x_max: 1000,
                y_max: 900
            }
        );
        assert_eq!(head.mac_style, 1);
        assert_eq!(head.index_to_loc_format, 1);
        assert_eq!(head.font_revision, 0x0001_0000);
    }

    #[test]
    fn rejects_bad_head() {
        assert!(Head::parse(&head_bytes(0)).is_err());
        let bytes = head_bytes(1000);
        for len in 0..bytes.len() - 2 {
            assert!(Head::parse(&bytes[..len]).is_err(), "len {len}");
        }
    }

    #[test]
    fn parses_hhea_and_maxp() {
        let mut data = vec![0, 1, 0, 0];
        for v in [1901i16, -483, 0] {
            data.extend_from_slice(&v.to_be_bytes());
        }
        data.extend_from_slice(&[0x10, 0, 0, 0, 0, 0, 0, 0]);
        data.extend_from_slice(&[0, 1, 0, 0]);
        data.extend_from_slice(&[0; 12]);
        data.extend_from_slice(&42u16.to_be_bytes());
        let hhea = Hhea::parse(&data).unwrap();
        assert_eq!(
            (hhea.ascender, hhea.descender, hhea.line_gap),
            (1901, -483, 0)
        );
        assert_eq!(hhea.number_of_h_metrics, 42);
        assert_eq!(hhea.caret_slope_rise, 1);
        assert!(Hhea::parse(&data[..30]).is_err());

        let maxp = |data: &[u8]| MaxProfile::parse(data).map(|p| p.num_glyphs);
        assert_eq!(maxp(&[0, 0, 0x50, 0, 1, 2]), Ok(258));
        assert!(maxp(&[0, 2, 0, 0, 1, 2]).is_err());
        assert!(maxp(&[0, 0, 0x50, 0, 1]).is_err());
    }

    #[test]
    fn maxp_limits_are_adjusted_like_freetype() {
        let mut data = vec![0, 1, 0, 0, 0, 9];
        for value in [0u16, 0, 0, 0, 2, 0xFFFF, 7, 3, 1, 200, 0, 0, 0] {
            data.extend_from_slice(&value.to_be_bytes());
        }
        let profile = MaxProfile::parse(&data).unwrap();
        assert_eq!(profile.num_glyphs, 9);
        assert_eq!(profile.max_twilight_points, 0xFFFB);
        assert_eq!(profile.max_storage, 7);
        assert_eq!(profile.max_function_defs, 64, "at least 64 functions");
        assert_eq!(profile.max_instruction_defs, 1);
        assert_eq!(profile.max_stack_elements, 200);
        let truncated = MaxProfile::parse(&data[..10]).unwrap();
        assert_eq!((truncated.num_glyphs, truncated.max_stack_elements), (9, 0));
    }
}
