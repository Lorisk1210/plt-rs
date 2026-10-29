//! Text in the 14 PDF core fonts (`pdf.use14corefonts`): matplotlib's
//! `draw_text` shows the whole string with one `Tj` in the font's
//! `WinAnsiEncoding`, without kerning, and `writeFonts` references the font
//! by its PostScript name (`_write_afm_font`) instead of embedding it.

use pltrs_core::Point;
use pltrs_text::{AfmDirectory, AfmFont, FontRequest};

use super::PdfRenderer;
use super::object::Obj;
use super::writer::PdfWriter;
use crate::vector::VectorError;

/// The characters of Windows code page 1252 at bytes `0x80` to `0x9f`
/// (`None` where the code page has none).
const CP1252_HIGH: [Option<char>; 32] = [
    Some('\u{20ac}'),
    None,
    Some('\u{201a}'),
    Some('\u{192}'),
    Some('\u{201e}'),
    Some('\u{2026}'),
    Some('\u{2020}'),
    Some('\u{2021}'),
    Some('\u{2c6}'),
    Some('\u{2030}'),
    Some('\u{160}'),
    Some('\u{2039}'),
    Some('\u{152}'),
    None,
    Some('\u{17d}'),
    None,
    None,
    Some('\u{2018}'),
    Some('\u{2019}'),
    Some('\u{201c}'),
    Some('\u{201d}'),
    Some('\u{2022}'),
    Some('\u{2013}'),
    Some('\u{2014}'),
    Some('\u{2dc}'),
    Some('\u{2122}'),
    Some('\u{161}'),
    Some('\u{203a}'),
    Some('\u{153}'),
    None,
    Some('\u{17e}'),
    Some('\u{178}'),
];

/// `encode_string(s, fonttype=1)`: `s.encode("cp1252", "replace")`, a
/// question mark for every character the code page lacks.
fn encode_cp1252(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match u32::from(c) {
            code @ (0..=0x7f | 0xa0..=0xff) => code as u8,
            _ => CP1252_HIGH
                .iter()
                .position(|&high| high == Some(c))
                .map_or(b'?', |index| 0x80 + index as u8),
        })
        .collect()
}

/// `_write_afm_font`: the dictionary of a core font, referenced by name in
/// `WinAnsiEncoding`; returns its object.
pub(super) fn write_core_font(writer: &mut PdfWriter, font: &AfmFont) -> Result<u32, VectorError> {
    let name = font
        .metrics()
        .and_then(|metrics| metrics.font_name())
        .map_err(|err| VectorError::Font(err.into()))?;
    let id = writer.reserve();
    writer.write_object(
        id,
        &Obj::dict([
            ("Type", Obj::name("Font")),
            ("Subtype", Obj::name("Type1")),
            ("BaseFont", Obj::name(name)),
            ("Encoding", Obj::name("WinAnsiEncoding")),
        ]),
    )?;
    Ok(id)
}

impl PdfRenderer {
    /// `draw_text` with `pdf.use14corefonts` for text in `font`: the text
    /// matrix places (and rotates) the string, shown in the core font of
    /// the font's first family that has one. The fonts of its other
    /// families are named (and written) too, as `fontName` does.
    pub(super) fn core_font_text(
        &mut self,
        text: &str,
        font: &FontRequest,
        origin: Point,
        angle: f64,
    ) {
        let fonts = font
            .afm_fonts(AfmDirectory::PdfCore)
            .map_err(VectorError::from);
        let Some(fonts) = self.problems.record(fonts) else {
            return;
        };
        let names: Vec<String> = fonts
            .into_iter()
            .map(|core| self.font_names.core_name(core))
            .collect();
        let Some(name) = names.into_iter().next() else {
            return;
        };
        self.output(&[
            Obj::op("BT"),
            Obj::name(name),
            Obj::Real(font.query.size),
            Obj::op("Tf"),
        ]);
        self.setup_textpos(origin, angle, Point::ORIGIN);
        self.output(&[Obj::Str(encode_cp1252(text)), Obj::op("Tj"), Obj::op("ET")]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_encoded_in_code_page_1252() {
        // Python: s.encode('cp1252', 'replace').
        assert_eq!(encode_cp1252("AVA"), b"AVA");
        assert_eq!(encode_cp1252("caf\u{e9} \u{b1}\u{ff}"), b"caf\xe9 \xb1\xff");
        assert_eq!(
            encode_cp1252("\u{20ac}\u{2013}\u{201c}\u{178}"),
            b"\x80\x96\x93\x9f"
        );
        assert_eq!(encode_cp1252("\u{2212}\u{3b1}\u{81}\u{1f600}"), b"????");
        assert_eq!(encode_cp1252(""), b"");
    }
}
