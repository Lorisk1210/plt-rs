//! `FT2Font.get_sfnt_table`: the sfnt tables of a face as the
//! dictionaries matplotlib builds from `FreeType`'s structures, with its key
//! names, 16.16 numbers as `(major, minor)` pairs and byte strings as
//! `bytes`.

use pltrs_font::{Face, Head, Hhea, MaxProfile, Os2, Pclt, Post};

use super::Ft2Font;
use crate::binding::prelude::*;

/// A value of a table entry.
enum Value {
    Int(i64),
    Pair(i64, i64),
    Quad([u32; 4]),
    Bytes(Vec<u8>),
}

impl IntoPy for Value {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::Int(value) => value.into_py(py),
            Self::Pair(first, second) => (first, second).into_py(py),
            Self::Quad([a, b, c, d]) => (a, b, c, d).into_py(py),
            Self::Bytes(bytes) => Bytes(bytes).into_py(py),
        }
    }
}

/// A table: its entries in matplotlib's order.
type Table = Vec<(&'static str, Value)>;

/// `(major, minor)` of a 16.16 number (matplotlib's `FIXED_MAJOR` and
/// `FIXED_MINOR`: the high half signed, the low half unsigned).
fn fixed(value: u32) -> Value {
    Value::Pair(
        i64::from((value >> 16) as u16 as i16),
        i64::from(value as u16),
    )
}

fn int(value: impl Into<i64>) -> Value {
    Value::Int(value.into())
}

/// A 64-bit `LONGDATETIME` as matplotlib's pair of 32-bit halves.
fn long_date_time(value: u64) -> Value {
    Value::Pair(i64::from((value >> 32) as u32), i64::from(value as u32))
}

fn head(t: &Head) -> Table {
    vec![
        ("version", fixed(t.version)),
        ("fontRevision", fixed(t.font_revision as u32)),
        // FreeType keeps the checksum adjustment in a signed `FT_Long`.
        ("checkSumAdjustment", int(t.checksum_adjustment as i32)),
        ("magicNumber", int(t.magic_number)),
        ("flags", int(t.flags)),
        ("unitsPerEm", int(t.units_per_em)),
        ("created", long_date_time(t.created)),
        ("modified", long_date_time(t.modified)),
        ("xMin", int(t.bbox.x_min)),
        ("yMin", int(t.bbox.y_min)),
        ("xMax", int(t.bbox.x_max)),
        ("yMax", int(t.bbox.y_max)),
        ("macStyle", int(t.mac_style)),
        ("lowestRecPPEM", int(t.lowest_rec_ppem)),
        ("fontDirectionHint", int(t.font_direction_hint)),
        ("indexToLocFormat", int(t.index_to_loc_format)),
        ("glyphDataFormat", int(t.glyph_data_format)),
    ]
}

fn maxp(t: &MaxProfile) -> Table {
    vec![
        ("version", fixed(t.version)),
        ("numGlyphs", int(t.num_glyphs)),
        ("maxPoints", int(t.max_points)),
        ("maxContours", int(t.max_contours)),
        ("maxComponentPoints", int(t.max_composite_points)),
        ("maxComponentContours", int(t.max_composite_contours)),
        ("maxZones", int(t.max_zones)),
        ("maxTwilightPoints", int(t.max_twilight_points)),
        ("maxStorage", int(t.max_storage)),
        ("maxFunctionDefs", int(t.max_function_defs)),
        ("maxInstructionDefs", int(t.max_instruction_defs)),
        ("maxStackElements", int(t.max_stack_elements)),
        ("maxSizeOfInstructions", int(t.max_size_of_instructions)),
        ("maxComponentElements", int(t.max_component_elements)),
        ("maxComponentDepth", int(t.max_component_depth)),
    ]
}

fn os2(t: &Os2) -> Table {
    let mut entries = vec![
        ("version", int(t.version)),
        ("xAvgCharWidth", int(t.avg_char_width)),
        ("usWeightClass", int(t.weight_class)),
        ("usWidthClass", int(t.width_class)),
        // FreeType reads `fsType` as a signed short.
        ("fsType", int(t.fs_type as i16)),
        ("ySubscriptXSize", int(t.subscript.x_size)),
        ("ySubscriptYSize", int(t.subscript.y_size)),
        ("ySubscriptXOffset", int(t.subscript.x_offset)),
        ("ySubscriptYOffset", int(t.subscript.y_offset)),
        ("ySuperscriptXSize", int(t.superscript.x_size)),
        ("ySuperscriptYSize", int(t.superscript.y_size)),
        ("ySuperscriptXOffset", int(t.superscript.x_offset)),
        ("ySuperscriptYOffset", int(t.superscript.y_offset)),
        ("yStrikeoutSize", int(t.strikeout_size)),
        ("yStrikeoutPosition", int(t.strikeout_position)),
        ("sFamilyClass", int(t.family_class)),
        ("panose", Value::Bytes(t.panose.to_vec())),
        ("ulUnicodeRange", Value::Quad(t.unicode_ranges)),
        ("achVendID", Value::Bytes(t.vendor_id.0.to_vec())),
        ("fsSelection", int(t.fs_selection)),
        ("usFirstCharIndex", int(t.first_char_index)),
        ("usLastCharIndex", int(t.last_char_index)),
        ("sTypoAscender", int(t.typo_ascender)),
        ("sTypoDescender", int(t.typo_descender)),
        ("sTypoLineGap", int(t.typo_line_gap)),
        ("usWinAscent", int(t.win_ascent)),
        ("usWinDescent", int(t.win_descent)),
    ];
    // FreeType leaves the fields a table version lacks at zero, and
    // matplotlib lists them by version.
    if t.version >= 1 {
        let [low, high] = t.code_page_ranges.unwrap_or_default();
        entries.push((
            "ulCodePageRange",
            Value::Pair(i64::from(low), i64::from(high)),
        ));
    }
    if t.version >= 2 {
        entries.extend([
            ("sxHeight", int(t.x_height.unwrap_or_default())),
            ("sCapHeight", int(t.cap_height.unwrap_or_default())),
            ("usDefaultChar", int(t.default_char.unwrap_or_default())),
            ("usBreakChar", int(t.break_char.unwrap_or_default())),
            ("usMaxContext", int(t.max_context.unwrap_or_default())),
        ]);
    }
    if t.version >= 5 {
        let [lower, upper] = t.optical_point_sizes.unwrap_or_default();
        entries.extend([
            ("usLowerOpticalPointSize", int(lower)),
            ("usUpperOpticalPointSize", int(upper)),
        ]);
    }
    entries
}

/// `hhea` or, with the vertical names, `vhea`.
fn metrics_header(t: &Hhea, vertical: bool) -> Table {
    let names: [&'static str; 8] = if vertical {
        [
            "vertTypoAscender",
            "vertTypoDescender",
            "vertTypoLineGap",
            "advanceHeightMax",
            "minTopSideBearing",
            "minBottomSideBearing",
            "yMaxExtent",
            "numOfLongVerMetrics",
        ]
    } else {
        [
            "ascent",
            "descent",
            "lineGap",
            "advanceWidthMax",
            "minLeftBearing",
            "minRightBearing",
            "xMaxExtent",
            "numOfLongHorMetrics",
        ]
    };
    vec![
        ("version", fixed(t.version)),
        (names[0], int(t.ascender)),
        (names[1], int(t.descender)),
        (names[2], int(t.line_gap)),
        (names[3], int(t.advance_width_max)),
        (names[4], int(t.min_left_side_bearing)),
        (names[5], int(t.min_right_side_bearing)),
        (names[6], int(t.x_max_extent)),
        ("caretSlopeRise", int(t.caret_slope_rise)),
        ("caretSlopeRun", int(t.caret_slope_run)),
        ("caretOffset", int(t.caret_offset)),
        ("metricDataFormat", int(t.metric_data_format)),
        (names[7], int(t.number_of_h_metrics)),
    ]
}

fn post(t: &Post) -> Table {
    // The italic angle is a 16.16 number, which an `f64` holds exactly.
    let italic_angle = (t.italic_angle * 65536.0) as i32 as u32;
    vec![
        ("format", fixed(t.format)),
        ("italicAngle", fixed(italic_angle)),
        ("underlinePosition", int(t.underline_position)),
        ("underlineThickness", int(t.underline_thickness)),
        ("isFixedPitch", int(u8::from(t.is_fixed_pitch))),
        ("minMemType42", int(t.min_mem_type42)),
        ("maxMemType42", int(t.max_mem_type42)),
        ("minMemType1", int(t.min_mem_type1)),
        ("maxMemType1", int(t.max_mem_type1)),
    ]
}

fn pclt(t: &Pclt) -> Table {
    vec![
        ("version", fixed(t.version)),
        ("fontNumber", int(t.font_number)),
        ("pitch", int(t.pitch)),
        ("xHeight", int(t.x_height)),
        ("style", int(t.style)),
        ("typeFamily", int(t.type_family)),
        ("capHeight", int(t.cap_height)),
        ("symbolSet", int(t.symbol_set)),
        ("typeFace", Value::Bytes(t.type_face.to_vec())),
        (
            "characterComplement",
            Value::Bytes(t.character_complement.to_vec()),
        ),
        ("strokeWeight", int(t.stroke_weight)),
        ("widthType", int(t.width_type)),
        ("serifStyle", int(t.serif_style)),
    ]
}

/// The table `name` of `face`, `None` for an unknown name or a table the
/// face lacks.
fn sfnt_table(face: &Face<'_>, name: &str) -> Option<Table> {
    match name {
        "head" => Some(head(face.head())),
        "maxp" => Some(maxp(face.max_profile())),
        "OS/2" => face.os2().map(os2),
        "hhea" => Some(metrics_header(face.hhea(), false)),
        "vhea" => face.vhea().map(|vhea| metrics_header(&vhea, true)),
        "post" => face.post().map(post),
        "pclt" => face.pclt().as_ref().map(pclt),
        _ => None,
    }
}

/// `get_sfnt_table(name)`: the entries of the table, `None` when there is
/// none.
fn get_sfnt_table(font: Native<Ft2Font>, name: String) -> PyResult<Option<Table>> {
    Ok(sfnt_table(&font.borrow().face()?, &name))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("get_sfnt_table", get_sfnt_table)
}
