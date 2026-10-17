//! Embedded fonts (`fontName`, `writeFonts`, `embedTTF` with
//! `pdf.fonttype = 3`): every used subset of a TrueType font becomes a
//! Type 3 font whose glyph procedures fill the outlines in 1/1000 em.

use std::fmt::Write as _;

use plt_core::path::convert::{StringCodes, StringOptions};
use plt_core::{Path, PathCode, Point};
use plt_font::GlyphId;
use plt_text::FontId;

use super::object::Obj;
use super::writer::PdfWriter;
use crate::vector::{CharacterTracker, FontCache, FontInfo, Subset, VectorError};

/// The `/Fn` names of the fonts used, in first-use order.
#[derive(Debug, Default)]
pub(super) struct FontNames {
    names: Vec<((FontId, usize), String)>,
}

impl FontNames {
    /// `fontName`: the name of `subset` of `font`, assigned on first use.
    pub(super) fn name(&mut self, font: FontId, subset: usize) -> String {
        if let Some((_, name)) = self.names.iter().find(|(key, _)| *key == (font, subset)) {
            return name.clone();
        }
        let name = format!("F{}", self.names.len() + 1);
        self.names.push(((font, subset), name.clone()));
        name
    }
}

/// `CPython`'s `hash(frozenset(glyphs))` for small non-negative integers,
/// as an unsigned 64-bit value (matplotlib's `hash(charset) % 2**64`).
fn frozenset_hash(glyphs: &[u16]) -> u64 {
    let shuffle = |h: u64| ((h ^ 0x055b_4db3) ^ (h << 16)).wrapping_mul(3_644_798_167);
    let mut unique: Vec<u16> = glyphs.to_vec();
    unique.sort_unstable();
    unique.dedup();
    let mut hash = unique
        .iter()
        .fold(0u64, |acc, &g| acc ^ shuffle(u64::from(g)));
    hash ^= (unique.len() as u64 + 1).wrapping_mul(1_927_868_237);
    hash ^= (hash >> 11) ^ (hash >> 25);
    hash = hash.wrapping_mul(69_069).wrapping_add(907_133_923);
    if hash == u64::MAX { 590_923_713 } else { hash }
}

/// `_get_subset_prefix`: six base-26 letters of the glyph set's hash and
/// a plus sign.
fn subset_prefix(glyphs: &[u16]) -> String {
    let mut n = frozenset_hash(glyphs);
    let mut letters = Vec::new();
    loop {
        letters.push(b'A' + (n % 26) as u8);
        n /= 26;
        if n == 0 {
            break;
        }
    }
    letters.reverse();
    letters.truncate(6);
    format!("{}+", String::from_utf8_lossy(&letters))
}

/// `cvt`: font units to 1/1000 em, to the nearest integer or rounded away
/// from zero.
fn cvt(length: f64, units_per_em: f64, nearest: bool) -> i64 {
    let value = length / units_per_em * 1000.0;
    if nearest {
        value.round_ties_even() as i64
    } else if value < 0.0 {
        value.floor() as i64
    } else {
        value.ceil() as i64
    }
}

/// `_get_pdf_charprocs` for one glyph: the `d1` metrics and the outline,
/// rounded to 1/1000 em, filled.
fn char_proc(font: &FontInfo, glyph: GlyphId) -> Vec<u8> {
    let conv = 1000.0 / font.units_per_em();
    let [x0, y0, x1, y1] = font.control_box(glyph).map(f64::from);
    let d1 = [
        (f64::from(font.advance(glyph)) * conv).round_ties_even() as i64,
        0,
        (x0 * conv).floor() as i64,
        (y0 * conv).floor() as i64,
        (x1 * conv).ceil() as i64,
        (y1 * conv).ceil() as i64,
    ];
    let outline = font.unscaled_outline(glyph);
    let vertices: Vec<Point> = outline
        .vertices()
        .iter()
        .map(|p| {
            Point::new(
                (p.x * conv).round_ties_even(),
                (p.y * conv).round_ties_even(),
            )
        })
        .collect();
    let codes: Vec<PathCode> = outline.resolved_codes();
    let rounded = Path::new(vertices, codes).unwrap_or_default();
    let ops = rounded.convert_to_string(&StringOptions {
        precision: 0,
        codes: StringCodes::PDF,
        postfix: true,
        ..StringOptions::default()
    });
    let metrics: Vec<String> = d1.iter().map(ToString::to_string).collect();
    format!("{} d1\n{ops}f", metrics.join(" ")).into_bytes()
}

/// The `ToUnicode` `CMap` mapping subset codes back to text.
fn unicode_cmap(entries: &[(u32, String)]) -> Vec<u8> {
    let mut groups: Vec<Vec<&(u32, String)>> = Vec::new();
    for entry in entries {
        match groups.last_mut() {
            Some(group) if group.last().is_some_and(|last| last.0 + 1 == entry.0) => {
                group.push(entry);
            }
            _ => groups.push(vec![entry]),
        }
    }
    let ranges: Vec<String> = groups
        .iter()
        .map(|group| {
            let values: Vec<String> = group
                .iter()
                .map(|(_, chars)| {
                    let hex = chars.encode_utf16().fold(String::new(), |mut hex, unit| {
                        let _ = write!(hex, "{unit:04x}");
                        hex
                    });
                    format!("<{hex}>")
                })
                .collect();
            format!(
                "<{:02x}> <{:02x}> [{}]",
                group[0].0,
                group[group.len() - 1].0,
                values.join(" ")
            )
        })
        .collect();
    format!(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo\n\
         << /Registry (Adobe)\n   /Ordering (UCS)\n   /Supplement 0\n>> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n\
         <0000> <ffff>\nendcodespacerange\n{} beginbfrange\n{}\nendbfrange\nendcmap\n\
         CMapName currentdict /CMap defineresource pop\nend\nend",
        groups.len(),
        ranges.join("\n")
    )
    .into_bytes()
}

/// The `Differences` array of a subset: runs of consecutive codes, each
/// starting with its first code, then the glyph names.
fn differences(font: &FontInfo, subset: &Subset) -> Vec<Obj> {
    let mut named: Vec<(u32, String)> = subset
        .entries()
        .iter()
        .map(|&(code, glyph)| (code, font.glyph_name(glyph)))
        .collect();
    named.sort();
    let mut encoding = Vec::new();
    let mut previous: Option<u32> = None;
    for (code, name) in named {
        if previous.is_none_or(|p| p + 1 != code) {
            encoding.push(Obj::Int(i64::from(code)));
        }
        encoding.push(Obj::name(name));
        previous = Some(code);
    }
    encoding
}

/// Writes one glyph procedure stream per glyph name, in name order, and
/// returns the `CharProcs` entries.
fn write_char_procs(
    writer: &mut PdfWriter,
    font: &FontInfo,
    subset: &Subset,
) -> Result<Vec<(String, Obj)>, VectorError> {
    let mut procs: Vec<(String, Vec<u8>)> = Vec::new();
    for &(_, glyph) in subset.entries() {
        let name = font.glyph_name(glyph);
        let proc = char_proc(font, glyph);
        if let Some(entry) = procs.iter_mut().find(|(n, _)| *n == name) {
            entry.1 = proc;
        } else {
            procs.push((name, proc));
        }
    }
    procs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut refs = Vec::new();
    for (name, proc) in procs {
        let id = writer.reserve();
        writer.output_stream(id, &proc, Vec::new())?;
        refs.push((name, Obj::Ref(id)));
    }
    Ok(refs)
}

/// The font descriptor of a subset named `ps_name`.
fn descriptor(font: &FontInfo, ps_name: &str, bbox: Vec<Obj>, max_width: i64) -> Obj {
    let upem = font.units_per_em();
    let face = font.face();
    let (cap_height, x_height) = font.pclt_heights();
    let flags = i64::from(font.is_fixed_pitch()) | 32 | (i64::from(face.style_flags().italic) << 6);
    let rounded_out = |v: i16| Obj::Int(cvt(f64::from(v), upem, false));
    Obj::dict([
        ("Type", Obj::name("FontDescriptor")),
        ("FontName", Obj::name(ps_name)),
        ("Flags", Obj::Int(flags)),
        ("FontBBox", Obj::Array(bbox)),
        ("Ascent", rounded_out(face.ascender())),
        ("Descent", rounded_out(face.descender())),
        (
            "CapHeight",
            Obj::Int(cvt(f64::from(cap_height), upem, false)),
        ),
        ("XHeight", Obj::Int(cvt(f64::from(x_height), upem, true))),
        (
            "ItalicAngle",
            Obj::Int(i64::from(font.italic_angle_fraction())),
        ),
        ("StemV", Obj::Int(0)),
        ("MaxWidth", Obj::Int(max_width)),
    ])
}

/// `embedTTF` / `embedTTFType3` for one subset; returns the font
/// dictionary's object.
fn embed_type3(
    writer: &mut PdfWriter,
    font: &FontInfo,
    id: FontId,
    subset_index: usize,
    subset: &Subset,
    tracker: &CharacterTracker,
) -> Result<u32, VectorError> {
    let upem = font.units_per_em();
    let glyphs: Vec<u16> = subset.entries().iter().map(|(_, g)| g.0).collect();
    let ps_name: String = format!("{}{}", subset_prefix(&glyphs), font.postscript_name())
        .chars()
        .map(|c| if c.is_ascii() { c } else { '?' })
        .collect();
    let bbox: Vec<Obj> = font
        .bbox()
        .iter()
        .map(|&v| Obj::Int(cvt(f64::from(v), upem, false)))
        .collect();
    let widths_id = writer.reserve();
    let descriptor_id = writer.reserve();
    let dict_id = writer.reserve();
    let procs_id = writer.reserve();
    let unicode_id = writer.reserve();
    let mut codes: Vec<u32> = subset.entries().iter().map(|(c, _)| *c).collect();
    codes.sort_unstable();
    let (first, last) = (codes[0], codes[codes.len() - 1]);
    let widths: Vec<i64> = (first..=last)
        .map(|code| {
            let glyph = subset.glyph(code).unwrap_or_default();
            cvt(f64::from(font.advance(glyph)), upem, true)
        })
        .collect();
    let proc_refs = write_char_procs(writer, font, subset)?;
    let chars: Vec<(u32, String)> = codes
        .iter()
        .map(|&code| {
            let text = tracker.chars(id, subset_index, code).unwrap_or_default();
            (code, text.to_owned())
        })
        .collect();
    let font_dict = Obj::dict([
        ("Type", Obj::name("Font")),
        ("BaseFont", Obj::name(ps_name.clone())),
        ("FirstChar", Obj::Int(i64::from(first))),
        ("LastChar", Obj::Int(i64::from(last))),
        ("FontDescriptor", Obj::Ref(descriptor_id)),
        ("Subtype", Obj::name("Type3")),
        ("Name", Obj::name(ps_name.clone())),
        ("FontBBox", Obj::Array(bbox.clone())),
        ("FontMatrix", Obj::reals([0.001, 0.0, 0.0, 0.001, 0.0, 0.0])),
        ("CharProcs", Obj::Ref(procs_id)),
        (
            "Encoding",
            Obj::dict([
                ("Type", Obj::name("Encoding")),
                ("Differences", Obj::Array(differences(font, subset))),
            ]),
        ),
        ("Widths", Obj::Ref(widths_id)),
        ("ToUnicode", Obj::Ref(unicode_id)),
    ]);
    let max_width = widths.iter().copied().max().unwrap_or(0);
    writer.write_object(dict_id, &font_dict)?;
    writer.write_object(descriptor_id, &descriptor(font, &ps_name, bbox, max_width))?;
    writer.write_object(
        widths_id,
        &Obj::Array(widths.into_iter().map(Obj::Int).collect()),
    )?;
    writer.write_object(procs_id, &Obj::Dict(proc_refs))?;
    writer.output_stream(unicode_id, &unicode_cmap(&chars), Vec::new())?;
    Ok(dict_id)
}

/// `writeFonts`: embeds every used subset, sorted by font file and subset,
/// and writes the font resource dictionary as object `dictionary`.
pub(super) fn write_fonts(
    writer: &mut PdfWriter,
    fonts: &mut FontCache,
    names: &FontNames,
    tracker: &CharacterTracker,
    dictionary: u32,
) -> Result<(), VectorError> {
    let mut used = Vec::new();
    for ((id, subset), name) in &names.names {
        let key = fonts.get(*id)?.sort_key().to_owned();
        used.push((key, *subset, *id, name.clone()));
    }
    used.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    let mut entries = Vec::new();
    for (_, subset_index, id, name) in used {
        let Some(subset) = tracker.subset(id, subset_index).filter(|s| !s.is_empty()) else {
            continue;
        };
        let font = fonts.get(id)?;
        let dict = embed_type3(writer, font, id, subset_index, subset, tracker)?;
        entries.push((name, Obj::Ref(dict)));
    }
    writer.write_object(dictionary, &Obj::Dict(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset_prefixes_hash_like_cpython() {
        // Python 3.14: hash(frozenset({20, 17, 19})), hash(frozenset({1, 2, 3})).
        assert_eq!(
            frozenset_hash(&[20, 17, 19]) as i64,
            5_386_371_529_308_070_464
        );
        assert_eq!(
            frozenset_hash(&[1, 2, 3, 2]) as i64,
            -272_375_401_224_217_160
        );
        // matplotlib names the digits and period of DejaVu Sans `CIVIRS+`.
        assert_eq!(subset_prefix(&[17, 19, 20, 21, 22, 23, 24]), "CIVIRS+");
    }

    #[test]
    fn conversions_and_cmaps() {
        assert_eq!(cvt(-1021.4, 1000.0, false), -1022);
        assert_eq!(cvt(1303.0, 2048.0, true), 636);
        let cmap = String::from_utf8(unicode_cmap(&[
            (0x2e, ".".to_owned()),
            (0x30, "0".to_owned()),
            (0x31, "1".to_owned()),
        ]))
        .unwrap();
        assert!(
            cmap.contains(
                "2 beginbfrange\n<2e> <2e> [<002e>]\n<30> <31> [<0030> <0031>]\nendbfrange"
            )
        );
    }

    #[test]
    fn font_names_follow_first_use() {
        use plt_text::{FontQuery, FontRequest, TextEngine};
        let run = TextEngine::global()
            .layout_line(
                "a",
                &FontRequest::new(FontQuery::families(["DejaVu Sans"])),
                72.0,
            )
            .expect("bundled font");
        let font = run.glyphs[0].font;
        let mut names = FontNames::default();
        assert_eq!(names.name(font, 0), "F1");
        assert_eq!(names.name(font, 1), "F2");
        assert_eq!(names.name(font, 0), "F1");
    }
}
