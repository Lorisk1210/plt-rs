//! Type 42 font embedding (`embedTTFType42`): a Type 0 font with
//! `Identity-H` encoding over a `CIDFontType2` font whose program is a
//! subset of the TrueType font; two-byte codes map to the subset's glyphs
//! through the `CIDToGIDMap`.

use pltrs_font::GlyphId;

use super::fonts::{SubsetFont, unicode_cmap};
use super::object::{Obj, text_string};
use super::writer::PdfWriter;
use crate::vector::{CharacterTracker, VectorError};

/// The `W` array: runs of consecutive codes, each as its first code and
/// the widths of the run.
fn width_runs(widths: &[(u32, i64)]) -> Vec<Obj> {
    let mut runs: Vec<(u32, Vec<i64>)> = Vec::new();
    for &(code, width) in widths {
        match runs.last_mut() {
            Some((first, run)) if *first + run.len() as u32 == code => run.push(width),
            _ => runs.push((code, vec![width])),
        }
    }
    runs.into_iter()
        .flat_map(|(first, run)| {
            [
                Obj::Int(i64::from(first)),
                Obj::Array(run.into_iter().map(Obj::Int).collect()),
            ]
        })
        .collect()
}

/// The `CIDToGIDMap` stream: the subset glyph of every code up to the
/// largest, as big-endian 16-bit numbers (0 for unused codes).
fn cid_to_gid_map(glyphs: &[(u32, GlyphId)]) -> Vec<u8> {
    let last = glyphs.iter().map(|&(code, _)| code).max().unwrap_or(0) as usize;
    let mut map = vec![0u16; last + 1];
    for &(code, glyph) in glyphs {
        map[code as usize] = glyph.0;
    }
    map.into_iter().flat_map(u16::to_be_bytes).collect()
}

/// `embedTTFType42` for one subset; returns the Type 0 font dictionary's
/// object.
pub(super) fn embed(
    writer: &mut PdfWriter,
    subset_font: SubsetFont<'_>,
    tracker: &CharacterTracker,
) -> Result<u32, VectorError> {
    let ps_name = subset_font.ps_name();
    let descriptor_id = writer.reserve();
    let cid_font_id = writer.reserve();
    let type0_id = writer.reserve();
    let cid_to_gid_id = writer.reserve();
    let font_file_id = writer.reserve();
    let widths_id = writer.reserve();
    let unicode_id = writer.reserve();
    let entries = subset_font.subset.entries();
    let subset = pltrs_font::subset(
        subset_font.font.face(),
        entries.iter().map(|&(_, glyph)| glyph),
    )
    .map_err(|err| VectorError::Font(err.into()))?;
    let cid_font = Obj::dict([
        ("Type", Obj::name("Font")),
        ("Subtype", Obj::name("CIDFontType2")),
        ("BaseFont", Obj::name(ps_name.clone())),
        (
            "CIDSystemInfo",
            Obj::dict([
                ("Registry", text_string("Adobe")),
                ("Ordering", text_string("Identity")),
                ("Supplement", Obj::Int(0)),
            ]),
        ),
        ("FontDescriptor", Obj::Ref(descriptor_id)),
        ("W", Obj::Ref(widths_id)),
        ("CIDToGIDMap", Obj::Ref(cid_to_gid_id)),
    ]);
    let type0 = Obj::dict([
        ("Type", Obj::name("Font")),
        ("Subtype", Obj::name("Type0")),
        ("BaseFont", Obj::name(ps_name.clone())),
        ("Encoding", Obj::name("Identity-H")),
        ("DescendantFonts", Obj::Array(vec![Obj::Ref(cid_font_id)])),
        ("ToUnicode", Obj::Ref(unicode_id)),
    ]);
    let length = i64::try_from(subset.data.len()).unwrap_or(i64::MAX);
    writer.output_stream(
        font_file_id,
        &subset.data,
        vec![("Length1".to_owned(), Obj::Int(length))],
    )?;
    let mut widths: Vec<(u32, i64)> = entries
        .iter()
        .map(|&(code, glyph)| (code, subset_font.width(glyph)))
        .collect();
    widths.sort_unstable();
    let glyphs: Vec<(u32, GlyphId)> = entries
        .iter()
        .map(|&(code, glyph)| (code, subset.new_id(glyph).unwrap_or_default()))
        .collect();
    writer.output_stream(cid_to_gid_id, &cid_to_gid_map(&glyphs), Vec::new())?;
    let chars = subset_font.chars(tracker);
    writer.output_stream(unicode_id, &unicode_cmap(&chars, 4), Vec::new())?;
    let mut descriptor = subset_font.descriptor(&ps_name);
    descriptor.push(("FontFile2".to_owned(), Obj::Ref(font_file_id)));
    let max_width = widths.iter().map(|&(_, w)| w).max().unwrap_or(0);
    descriptor.push(("MaxWidth".to_owned(), Obj::Int(max_width)));
    writer.write_object(cid_font_id, &cid_font)?;
    writer.write_object(type0_id, &type0)?;
    writer.write_object(descriptor_id, &Obj::Dict(descriptor))?;
    writer.write_object(widths_id, &Obj::Array(width_runs(&widths)))?;
    Ok(type0_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_group_consecutive_codes() {
        let runs = width_runs(&[(32, 318), (33, 401), (99, 550)]);
        let text: Vec<String> = runs
            .iter()
            .map(|o| String::from_utf8(o.repr().unwrap()).unwrap())
            .collect();
        assert_eq!(text, ["32", "[ 318 401 ]", "99", "[ 550 ]"]);
    }

    #[test]
    fn cid_maps_list_every_code() {
        let map = cid_to_gid_map(&[(2, GlyphId(5)), (0, GlyphId(4))]);
        assert_eq!(map, [0, 4, 0, 0, 0, 5]);
    }
}
