//! TeX text as paths (`RendererBase.draw_tex`, which `RendererSVG` does
//! not override, through its `_draw_text_as_path` with `ismath="TeX"`):
//! the text typeset at `TextToPath`'s size, every glyph of the DVI file
//! defined once from its Type 1 font with light hinting (in `FreeType`'s
//! 26.6 units) and placed with `<use>`, every rule a path; no clip group
//! and no link, which matplotlib only adds in `draw_text`.

use plt_core::{Affine2D, Path, Point};
use plt_text::tex::{
    TEX_FONT_SCALE, TEX_PATH_DPI, TexError, TexGlyph, TexHinting, TexText, glyph_outline,
};

use super::SvgRenderer;
use super::defs::path_data;
use super::style::{TransformOp, transform_attr};
use super::text::{adjusted, url_quote};
use crate::gc::GraphicsContext;

/// `_get_glyph_repr`: the quoted PostScript name of the glyph's font and
/// its glyph index.
fn glyph_repr(glyph: &TexGlyph) -> String {
    let name = glyph.face.postscript_name().unwrap_or_default();
    url_quote(&format!("{name}-{:x}", glyph.index.0))
}

impl SvgRenderer {
    /// Draws one line of TeX text.
    pub(super) fn svg_tex(
        &mut self,
        gc: &GraphicsContext,
        tex: &TexText,
        origin: Point,
        angle: f64,
    ) -> Result<(), TexError> {
        self.text_group(gc, &tex.tex, tex.size(), origin, angle, |svg| {
            svg.tex_glyphs(tex)
        })
    }

    /// `get_glyphs_tex` and the elements placing its glyphs and rules.
    fn tex_glyphs(&mut self, tex: &TexText) -> Result<(), TexError> {
        let page = tex.page(TEX_FONT_SCALE, TEX_PATH_DPI)?;
        let glyphs = tex.glyphs(&page)?;
        let reprs: Vec<String> = glyphs.iter().map(glyph_repr).collect();
        let mut new = Vec::new();
        for (repr, glyph) in reprs.iter().zip(&glyphs) {
            if self.glyphs.contains(repr) || new.iter().any(|(known, _)| known == repr) {
                continue;
            }
            let outline = glyph_outline(
                &glyph.path,
                &glyph.face,
                glyph.index,
                TEX_FONT_SCALE,
                TEX_PATH_DPI,
                TexHinting::Adobe,
            )?;
            new.push((repr.clone(), outline));
        }
        self.define_glyph_paths(new);
        for (repr, glyph) in reprs.iter().zip(&glyphs) {
            self.writer.element(
                "use",
                "",
                &[
                    ("xlink:href", format!("#{}", adjusted(repr))),
                    (
                        "transform",
                        transform_attr(&[
                            TransformOp::Translate(glyph.x, glyph.y),
                            TransformOp::Scale(glyph.size / TEX_FONT_SCALE),
                        ]),
                    ),
                ],
            );
        }
        for b in &page.boxes {
            let outline = Path::compound_from_polygons(&[[
                Point::new(b.x, b.y),
                Point::new(b.x + b.width, b.y),
                Point::new(b.x + b.width, b.y + b.height),
                Point::new(b.x, b.y + b.height),
                Point::new(b.x, b.y),
            ]]);
            self.writer.element(
                "path",
                "",
                &[("d", path_data(&outline, Affine2D::IDENTITY))],
            );
        }
        Ok(())
    }
}
