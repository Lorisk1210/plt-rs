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
    /// Table version as 16.16 fixed point (`0x00010000`).
    pub version: u32,
    /// Font revision as a 16.16 fixed-point number.
    pub font_revision: i32,
    /// Checksum adjustment of the whole font file.
    pub checksum_adjustment: u32,
    /// `0x5F0F3CF5` in valid fonts.
    pub magic_number: u32,
    /// Header flags (bit 3: integer ppem scaling hint).
    pub flags: u16,
    /// Design units per em square.
    pub units_per_em: u16,
    /// Creation time in seconds since 1904-01-01.
    pub created: u64,
    /// Modification time in seconds since 1904-01-01.
    pub modified: u64,
    /// Union of all glyph bounding boxes, in font units.
    pub bbox: Rect,
    /// Macintosh style bits (bit 0 bold, bit 1 italic).
    pub mac_style: u16,
    /// Smallest readable size in pixels.
    pub lowest_rec_ppem: u16,
    /// Deprecated direction hint (2 for left-to-right with neutrals).
    pub font_direction_hint: i16,
    /// `loca` offset format: 0 for 16-bit, 1 for 32-bit offsets.
    pub index_to_loc_format: i16,
    /// Glyph data format (0).
    pub glyph_data_format: i16,
}

impl Head {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, HEAD);
        let version = r.u32()?;
        let major = version >> 16;
        if major != 1 {
            return Err(FontError::UnsupportedFormat {
                table: HEAD,
                format: major,
            });
        }
        let font_revision = r.i32()?;
        let checksum_adjustment = r.u32()?;
        let magic_number = r.u32()?;
        let flags = r.u16()?;
        let units_per_em = r.u16()?;
        if !(16..=16384).contains(&units_per_em) {
            return Err(FontError::malformed(HEAD, "unitsPerEm outside 16..=16384"));
        }
        let mut long_date_time =
            || -> Result<u64> { Ok(u64::from(r.u32()?) << 32 | u64::from(r.u32()?)) };
        let created = long_date_time()?;
        let modified = long_date_time()?;
        let bbox = Rect {
            x_min: i32::from(r.i16()?),
            y_min: i32::from(r.i16()?),
            x_max: i32::from(r.i16()?),
            y_max: i32::from(r.i16()?),
        };
        let mac_style = r.u16()?;
        let lowest_rec_ppem = r.u16()?;
        let font_direction_hint = r.i16()?;
        let index_to_loc_format = r.i16()?;
        let glyph_data_format = r.i16()?;
        Ok(Self {
            version,
            font_revision,
            checksum_adjustment,
            magic_number,
            flags,
            units_per_em,
            created,
            modified,
            bbox,
            mac_style,
            lowest_rec_ppem,
            font_direction_hint,
            index_to_loc_format,
            glyph_data_format,
        })
    }
}

/// Values from the `hhea` table (or from `vhea`, which has the same
/// layout with vertical metrics in place of the horizontal ones).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hhea {
    /// Table version as 16.16 fixed point (`0x00010000`).
    pub version: u32,
    /// Typographic ascender in font units.
    pub ascender: i16,
    /// Typographic descender in font units (usually negative).
    pub descender: i16,
    /// Extra line spacing in font units.
    pub line_gap: i16,
    /// Largest advance width.
    pub advance_width_max: u16,
    /// Smallest left side bearing.
    pub min_left_side_bearing: i16,
    /// Smallest right side bearing.
    pub min_right_side_bearing: i16,
    /// Largest `lsb + (x_max - x_min)`.
    pub x_max_extent: i16,
    /// Caret slope numerator.
    pub caret_slope_rise: i16,
    /// Caret slope denominator.
    pub caret_slope_run: i16,
    /// Horizontal shift of the caret for slanted fonts.
    pub caret_offset: i16,
    /// Metric data format (0).
    pub metric_data_format: i16,
    /// Number of long metrics in `hmtx`.
    pub number_of_h_metrics: u16,
}

impl Hhea {
    pub(crate) fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data, HHEA);
        let version = r.u32()?;
        let ascender = r.i16()?;
        let descender = r.i16()?;
        let line_gap = r.i16()?;
        let advance_width_max = r.u16()?;
        let min_left_side_bearing = r.i16()?;
        let min_right_side_bearing = r.i16()?;
        let x_max_extent = r.i16()?;
        let caret_slope_rise = r.i16()?;
        let caret_slope_run = r.i16()?;
        let caret_offset = r.i16()?;
        r.skip(8)?;
        let metric_data_format = r.i16()?;
        let number_of_h_metrics = r.u16()?;
        Ok(Self {
            version,
            ascender,
            descender,
            line_gap,
            advance_width_max,
            min_left_side_bearing,
            min_right_side_bearing,
            x_max_extent,
            caret_slope_rise,
            caret_slope_run,
            caret_offset,
            metric_data_format,
            number_of_h_metrics,
        })
    }
}

/// The `maxp` table as FreeType loads it (`TT_MaxProfile`): the glyph
/// count and, in version 1.0 tables, the TrueType interpreter's resource
/// limits, at least 64 function definitions and at most `0xFFFF - 4`
/// twilight points (`tt_face_load_maxp`). Version 0.5 (CFF) tables, and
/// version 1.0 tables cut short, leave the limits at zero.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct MaxProfile {
    /// Table version as 16.16 fixed point (`0x00005000` or `0x00010000`).
    pub version: u32,
    /// Number of glyphs.
    pub num_glyphs: u16,
    /// Points in a simple glyph.
    pub max_points: u16,
    /// Contours in a simple glyph.
    pub max_contours: u16,
    /// Points in a composite glyph.
    pub max_composite_points: u16,
    /// Contours in a composite glyph.
    pub max_composite_contours: u16,
    /// 2 when instructions use the twilight zone, 1 otherwise.
    pub max_zones: u16,
    /// Points in the twilight zone.
    pub max_twilight_points: u16,
    /// Storage area locations.
    pub max_storage: u16,
    /// Function definitions.
    pub max_function_defs: u16,
    /// Instruction definitions.
    pub max_instruction_defs: u16,
    /// Interpreter stack depth.
    pub max_stack_elements: u16,
    /// Bytes of glyph instructions.
    pub max_size_of_instructions: u16,
    /// Components referenced at the top level of a composite glyph.
    pub max_component_elements: u16,
    /// Levels of composite recursion.
    pub max_component_depth: u16,
}

impl MaxProfile {
    /// Parses a version 0.5 (CFF) or 1.0 (TrueType) `maxp` table.
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
            version,
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
            let [
                max_points,
                max_contours,
                max_composite_points,
                max_composite_contours,
                max_zones,
                max_twilight_points,
                max_storage,
                max_function_defs,
                max_instruction_defs,
                max_stack_elements,
                max_size_of_instructions,
                max_component_elements,
                max_component_depth,
            ] = limits;
            profile = Self {
                max_points,
                max_contours,
                max_composite_points,
                max_composite_contours,
                max_zones,
                max_twilight_points: max_twilight_points.min(0xFFFF - 4),
                max_storage,
                max_function_defs: max_function_defs.max(64),
                max_instruction_defs,
                max_stack_elements,
                max_size_of_instructions,
                max_component_elements,
                max_component_depth,
                ..profile
            };
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
        assert_eq!(
            (head.version, head.magic_number),
            (0x0001_0000, 0x5F0F_3CF5)
        );
        assert_eq!((head.created, head.modified), (0, 0));
        assert_eq!((head.font_direction_hint, head.glyph_data_format), (2, 0));
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
        assert_eq!(
            (hhea.version, hhea.advance_width_max),
            (0x0001_0000, 0x1000)
        );
        assert_eq!((hhea.min_left_side_bearing, hhea.x_max_extent), (0, 0));
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
        assert_eq!((profile.version, profile.max_zones), (0x0001_0000, 2));
        let truncated = MaxProfile::parse(&data[..10]).unwrap();
        assert_eq!((truncated.num_glyphs, truncated.max_stack_elements), (9, 0));
    }
}
