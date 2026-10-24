//! `FT2Font.get_sfnt_table`: the sfnt tables of a face as the dictionaries
//! matplotlib builds from `FreeType`'s structures, with its key names, 16.16
//! numbers as `[major, minor]` pairs and byte strings as byte lists.

use plt_font::{Face, Head, Hhea, MaxProfile, Os2, Pclt, Post};

use crate::json::Json;

/// `[major, minor]` of a 16.16 number (matplotlib's `FIXED_MAJOR` and
/// `FIXED_MINOR`: the high half signed, the low half unsigned).
fn fixed(value: u32) -> Json {
    Json::Array(vec![
        Json::from(f64::from((value >> 16) as u16 as i16)),
        Json::from(f64::from(value as u16)),
    ])
}

/// A byte string as a list of byte values.
fn bytes(values: &[u8]) -> Json {
    Json::Array(values.iter().map(|&b| Json::from(f64::from(b))).collect())
}

/// A list of numbers.
fn numbers<T: Into<f64> + Copy>(values: &[T]) -> Json {
    Json::Array(values.iter().map(|&v| Json::from(v.into())).collect())
}

/// An object of named values.
fn dict(entries: Vec<(&str, Json)>) -> Json {
    Json::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// A 64-bit `LONGDATETIME` as matplotlib's pair of 32-bit halves.
fn long_date_time(value: u64) -> Json {
    numbers(&[f64::from((value >> 32) as u32), f64::from(value as u32)])
}

fn head(t: &Head) -> Json {
    dict(vec![
        ("version", fixed(t.version)),
        ("fontRevision", fixed(t.font_revision as u32)),
        // FreeType keeps the checksum adjustment in a signed `FT_Long`.
        (
            "checkSumAdjustment",
            Json::from(f64::from(t.checksum_adjustment as i32)),
        ),
        ("magicNumber", Json::from(f64::from(t.magic_number))),
        ("flags", Json::from(f64::from(t.flags))),
        ("unitsPerEm", Json::from(f64::from(t.units_per_em))),
        ("created", long_date_time(t.created)),
        ("modified", long_date_time(t.modified)),
        ("xMin", Json::from(f64::from(t.bbox.x_min))),
        ("yMin", Json::from(f64::from(t.bbox.y_min))),
        ("xMax", Json::from(f64::from(t.bbox.x_max))),
        ("yMax", Json::from(f64::from(t.bbox.y_max))),
        ("macStyle", Json::from(f64::from(t.mac_style))),
        ("lowestRecPPEM", Json::from(f64::from(t.lowest_rec_ppem))),
        (
            "fontDirectionHint",
            Json::from(f64::from(t.font_direction_hint)),
        ),
        (
            "indexToLocFormat",
            Json::from(f64::from(t.index_to_loc_format)),
        ),
        (
            "glyphDataFormat",
            Json::from(f64::from(t.glyph_data_format)),
        ),
    ])
}

fn maxp(t: &MaxProfile) -> Json {
    let number = |v: u16| Json::from(f64::from(v));
    dict(vec![
        ("version", fixed(t.version)),
        ("numGlyphs", number(t.num_glyphs)),
        ("maxPoints", number(t.max_points)),
        ("maxContours", number(t.max_contours)),
        ("maxComponentPoints", number(t.max_composite_points)),
        ("maxComponentContours", number(t.max_composite_contours)),
        ("maxZones", number(t.max_zones)),
        ("maxTwilightPoints", number(t.max_twilight_points)),
        ("maxStorage", number(t.max_storage)),
        ("maxFunctionDefs", number(t.max_function_defs)),
        ("maxInstructionDefs", number(t.max_instruction_defs)),
        ("maxStackElements", number(t.max_stack_elements)),
        ("maxSizeOfInstructions", number(t.max_size_of_instructions)),
        ("maxComponentElements", number(t.max_component_elements)),
        ("maxComponentDepth", number(t.max_component_depth)),
    ])
}

fn os2(t: &Os2) -> Json {
    let short = |v: i16| Json::from(f64::from(v));
    let ushort = |v: u16| Json::from(f64::from(v));
    let mut entries = vec![
        ("version", ushort(t.version)),
        ("xAvgCharWidth", short(t.avg_char_width)),
        ("usWeightClass", ushort(t.weight_class)),
        ("usWidthClass", ushort(t.width_class)),
        // FreeType reads `fsType` as a signed short.
        ("fsType", short(t.fs_type as i16)),
        ("ySubscriptXSize", short(t.subscript.x_size)),
        ("ySubscriptYSize", short(t.subscript.y_size)),
        ("ySubscriptXOffset", short(t.subscript.x_offset)),
        ("ySubscriptYOffset", short(t.subscript.y_offset)),
        ("ySuperscriptXSize", short(t.superscript.x_size)),
        ("ySuperscriptYSize", short(t.superscript.y_size)),
        ("ySuperscriptXOffset", short(t.superscript.x_offset)),
        ("ySuperscriptYOffset", short(t.superscript.y_offset)),
        ("yStrikeoutSize", short(t.strikeout_size)),
        ("yStrikeoutPosition", short(t.strikeout_position)),
        ("sFamilyClass", short(t.family_class)),
        ("panose", bytes(&t.panose)),
        ("ulUnicodeRange", numbers(&t.unicode_ranges)),
        ("achVendID", bytes(&t.vendor_id.0)),
        ("fsSelection", ushort(t.fs_selection)),
        ("usFirstCharIndex", ushort(t.first_char_index)),
        ("usLastCharIndex", ushort(t.last_char_index)),
        ("sTypoAscender", short(t.typo_ascender)),
        ("sTypoDescender", short(t.typo_descender)),
        ("sTypoLineGap", short(t.typo_line_gap)),
        ("usWinAscent", ushort(t.win_ascent)),
        ("usWinDescent", ushort(t.win_descent)),
    ];
    // FreeType leaves the fields a table version lacks at zero, and
    // matplotlib lists them by version.
    if t.version >= 1 {
        entries.push((
            "ulCodePageRange",
            numbers(&t.code_page_ranges.unwrap_or_default()),
        ));
    }
    if t.version >= 2 {
        entries.extend([
            ("sxHeight", short(t.x_height.unwrap_or_default())),
            ("sCapHeight", short(t.cap_height.unwrap_or_default())),
            ("usDefaultChar", ushort(t.default_char.unwrap_or_default())),
            ("usBreakChar", ushort(t.break_char.unwrap_or_default())),
            ("usMaxContext", ushort(t.max_context.unwrap_or_default())),
        ]);
    }
    if t.version >= 5 {
        let [lower, upper] = t.optical_point_sizes.unwrap_or_default();
        entries.extend([
            ("usLowerOpticalPointSize", ushort(lower)),
            ("usUpperOpticalPointSize", ushort(upper)),
        ]);
    }
    dict(entries)
}

/// `hhea` or, with the vertical names, `vhea`.
fn metrics_header(t: &Hhea, vertical: bool) -> Json {
    let short = |v: i16| Json::from(f64::from(v));
    let names: [&str; 8] = if vertical {
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
    dict(vec![
        ("version", fixed(t.version)),
        (names[0], short(t.ascender)),
        (names[1], short(t.descender)),
        (names[2], short(t.line_gap)),
        (names[3], Json::from(f64::from(t.advance_width_max))),
        (names[4], short(t.min_left_side_bearing)),
        (names[5], short(t.min_right_side_bearing)),
        (names[6], short(t.x_max_extent)),
        ("caretSlopeRise", short(t.caret_slope_rise)),
        ("caretSlopeRun", short(t.caret_slope_run)),
        ("caretOffset", short(t.caret_offset)),
        ("metricDataFormat", short(t.metric_data_format)),
        (names[7], Json::from(f64::from(t.number_of_h_metrics))),
    ])
}

fn post(t: &Post) -> Json {
    let ulong = |v: u32| Json::from(f64::from(v));
    // The italic angle is a 16.16 number, which an `f64` holds exactly.
    let italic_angle = (t.italic_angle * 65536.0) as i32 as u32;
    dict(vec![
        ("format", fixed(t.format)),
        ("italicAngle", fixed(italic_angle)),
        (
            "underlinePosition",
            Json::from(f64::from(t.underline_position)),
        ),
        (
            "underlineThickness",
            Json::from(f64::from(t.underline_thickness)),
        ),
        (
            "isFixedPitch",
            Json::from(f64::from(u8::from(t.is_fixed_pitch))),
        ),
        ("minMemType42", ulong(t.min_mem_type42)),
        ("maxMemType42", ulong(t.max_mem_type42)),
        ("minMemType1", ulong(t.min_mem_type1)),
        ("maxMemType1", ulong(t.max_mem_type1)),
    ])
}

fn pclt(t: &Pclt) -> Json {
    let ushort = |v: u16| Json::from(f64::from(v));
    dict(vec![
        ("version", fixed(t.version)),
        ("fontNumber", Json::from(f64::from(t.font_number))),
        ("pitch", ushort(t.pitch)),
        ("xHeight", ushort(t.x_height)),
        ("style", ushort(t.style)),
        ("typeFamily", ushort(t.type_family)),
        ("capHeight", ushort(t.cap_height)),
        ("symbolSet", ushort(t.symbol_set)),
        ("typeFace", bytes(&t.type_face)),
        ("characterComplement", bytes(&t.character_complement)),
        ("strokeWeight", Json::from(f64::from(t.stroke_weight))),
        ("widthType", Json::from(f64::from(t.width_type))),
        ("serifStyle", Json::from(f64::from(t.serif_style))),
    ])
}

/// The table matplotlib's `get_sfnt_table(name)` gives, or `null` for an
/// unknown name or a table the face lacks.
pub(super) fn sfnt_table(face: &Face<'_>, name: &str) -> Json {
    let table = match name {
        "head" => Some(head(face.head())),
        "maxp" => Some(maxp(face.max_profile())),
        "OS/2" => face.os2().map(os2),
        "hhea" => Some(metrics_header(face.hhea(), false)),
        "vhea" => face.vhea().map(|vhea| metrics_header(&vhea, true)),
        "post" => face.post().map(post),
        "pclt" => face.pclt().as_ref().map(pclt),
        _ => None,
    };
    table.unwrap_or(Json::Null)
}
