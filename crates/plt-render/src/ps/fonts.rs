//! Type 3 font programs (`_font_to_ps_type3`): one font per used
//! 256-glyph subset, glyphs filled from their outlines in font units.

use plt_core::path::convert::{StringCodes, StringOptions};

use crate::vector::{FontInfo, Subset, repr};

/// The glyph procedure tokens: quadratic curves become cubics and the
/// closing `cl` is dropped, the outline decomposer having already moved
/// back to each contour's start.
const GLYPH_CODES: StringCodes<'static> = StringCodes {
    move_to: "m",
    line_to: "l",
    quad_to: "",
    cubic_to: "c",
    close: "",
};

/// The PostScript resource defining subset `index` of `font`.
pub(super) fn type3_font(font: &FontInfo, index: usize, subset: &Subset) -> String {
    let glyphs: Vec<_> = subset.entries().iter().map(|&(_, g)| g).collect();
    let encoding: Vec<String> = glyphs
        .iter()
        .map(|&g| format!("/{}", font.glyph_name(g)))
        .collect();
    let bbox: Vec<String> = font.bbox().iter().map(ToString::to_string).collect();
    let mut out = format!(
        "%!PS-Adobe-3.0 Resource-Font\n\
         %%Creator: Converted from TrueType to Type 3 by Matplotlib.\n\
         10 dict begin\n\
         /FontName /{name}-{index} def\n\
         /PaintType 0 def\n\
         /FontMatrix [{inv} 0 0 {inv} 0 0] def\n\
         /FontBBox [{bbox}] def\n\
         /FontType 3 def\n\
         /Encoding [{encoding}] def\n\
         /CharStrings {count} dict dup begin\n\
         /.notdef 0 def\n",
        name = font.postscript_name(),
        inv = repr(1.0 / font.units_per_em()),
        bbox = bbox.join(" "),
        encoding = encoding.join(" "),
        count = glyphs.len() + 1,
    );
    let entries: Vec<String> = glyphs
        .iter()
        .map(|&glyph| {
            let [x0, y0, x1, y1] = font.control_box(glyph);
            let outline = font.unscaled_outline(glyph);
            let ops = outline.convert_to_string(&StringOptions {
                precision: 0,
                codes: GLYPH_CODES,
                postfix: true,
                ..StringOptions::default()
            });
            format!(
                "/{}{{{} 0 {x0} {y0} {x1} {y1} sc\n{ops}ce}} _d",
                font.glyph_name(glyph),
                font.advance(glyph)
            )
        })
        .collect();
    out.push_str(&entries.join("\n"));
    out.push_str(
        "\nend readonly def\n\n\
         /BuildGlyph {\n exch begin\n CharStrings exch\n 2 copy known not {pop /.notdef} if\n \
         true 3 1 roll get exec\n end\n} _d\n\n\
         /BuildChar {\n 1 index /Encoding get exch get\n 1 index /BuildGlyph get exec\n} _d\n\n\
         FontName currentdict end definefont pop\n",
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::{CharacterTracker, FontCache};
    use plt_font::GlyphId;
    use plt_text::{FontQuery, FontRequest};

    #[test]
    fn programs_follow_matplotlib() {
        let mut fonts = FontCache::default();
        let run = fonts
            .engine()
            .layout_line(
                "1",
                &FontRequest::new(FontQuery::families(["DejaVu Sans"])),
                72.0,
            )
            .expect("bundled font");
        let id = run.glyphs[0].font;
        let mut tracker = CharacterTracker::new(256);
        tracker.track(id, "1", GlyphId(20));
        tracker.track(id, ".", GlyphId(17));
        let subset = &tracker.fonts().next().expect("one font").1[0];
        let font = fonts.get(id).expect("bundled font");
        let program = type3_font(font, 0, subset);
        assert!(program.contains("/FontName /DejaVuSans-0 def\n"));
        assert!(program.contains("/FontMatrix [0.00048828125 0 0 0.00048828125 0 0] def"));
        assert!(program.contains("/FontBBox [-2090 -948 3673 2524] def"));
        assert!(program.contains("/Encoding [/one /period] def\n/CharStrings 3 dict dup begin"));
        assert!(program.contains("/one{1303 0 225 0 1114 1493 sc\n254 170 m\n"));
        assert!(program.contains("254 170 l\n\nce} _d\n/period{651 0 219 0 430 254 sc\n"));
        assert!(program.ends_with("FontName currentdict end definefont pop\n"));
    }
}
