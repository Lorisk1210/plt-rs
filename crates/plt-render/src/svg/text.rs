//! Text as paths (`_draw_text_as_path`): the text is laid out again by
//! `TextToPath` at `FONT_SCALE` (100 pt, 72 dpi), every glyph is defined
//! once in `FreeType`'s 26.6 units at that size and placed with `<use>`
//! inside a group scaled to the font size.

#![expect(
    clippy::float_cmp,
    reason = "matplotlib's SVG writer leaves out exactly-default values"
)]

use std::fmt::Write as _;

use plt_core::{Affine2D, Path, Point};
use plt_text::{PlacedGlyph, TextRun, TextSource};

use super::defs::path_data;
use super::style::{Style, TransformOp, css, transform_attr};
use super::{SvgFontType, SvgRenderer};
use crate::gc::GraphicsContext;
use crate::vector::{POINTS_DPI, VectorError, in_points, short};

/// The size text is laid out at (`TextToPath.FONT_SCALE`).
const FONT_SCALE: f64 = 100.0;

/// `urllib.parse.quote` with its default safe characters.
fn url_quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"_.-~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// `_adjust_glyph_repr`: the id a glyph is defined and referenced by.
fn adjusted(glyph_repr: &str) -> String {
    glyph_repr.replace("%20", "_")
}

impl SvgRenderer {
    /// `_get_glyph_repr`: the quoted PostScript name and glyph index.
    fn glyph_repr(&mut self, glyph: &PlacedGlyph) -> Result<String, VectorError> {
        let font = self.fonts.get(glyph.font)?;
        Ok(url_quote(&format!(
            "{}-{:x}",
            font.postscript_name(),
            glyph.glyph.0
        )))
    }

    /// `_update_glyph_map_defs`: defines the glyphs (by `glyph_repr`) not
    /// yet defined.
    fn define_glyphs(&mut self, glyphs: &[(String, &PlacedGlyph)]) -> Result<(), VectorError> {
        let engine = self.fonts.engine();
        let mut new = Vec::new();
        for (repr, glyph) in glyphs {
            if self.glyphs.contains(repr) || new.iter().any(|(known, _)| known == repr) {
                continue;
            }
            let outline = engine.glyph_outline(glyph.font, glyph.glyph, FONT_SCALE, POINTS_DPI)?;
            new.push((repr.clone(), outline));
        }
        if new.is_empty() {
            return Ok(());
        }
        self.writer.start("defs", &[]);
        for (repr, outline) in new {
            // Back to FreeType's integral 26.6 units.
            let d = path_data(&outline, Affine2D::scaling(64.0, 64.0));
            self.writer.element(
                "path",
                "",
                &[
                    ("id", adjusted(&repr)),
                    ("d", d),
                    (
                        "transform",
                        transform_attr(&[TransformOp::Scale(1.0 / 64.0)]),
                    ),
                ],
            );
            self.glyphs.insert(repr);
        }
        self.writer.end(true);
        Ok(())
    }

    /// The glyphs and rules of one text laid out at [`FONT_SCALE`]
    /// (`get_glyphs_with_font`, `get_glyphs_mathtext`): the glyphs placed
    /// with `<use>`, the rules of mathtext as paths.
    fn text_glyphs(&mut self, source: &TextSource) -> Result<(), VectorError> {
        let scaled = source.layout(self.fonts.engine(), FONT_SCALE, POINTS_DPI)?;
        let reprs = scaled
            .glyphs
            .iter()
            .map(|glyph| Ok((self.glyph_repr(glyph)?, glyph)))
            .collect::<Result<Vec<_>, VectorError>>()?;
        self.define_glyphs(&reprs)?;
        for (repr, glyph) in reprs {
            let (id, scale) = if source.is_math() {
                (adjusted(&repr), glyph.size / FONT_SCALE)
            } else {
                (repr, 1.0)
            };
            self.writer.element(
                "use",
                "",
                &[
                    ("xlink:href", format!("#{id}")),
                    (
                        "transform",
                        transform_attr(&[
                            TransformOp::Translate(glyph.x, glyph.y),
                            TransformOp::Scale(scale),
                        ]),
                    ),
                ],
            );
        }
        for rect in &scaled.rects {
            let (x, y, w, h) = rect.bounds();
            let outline = Path::compound_from_polygons(&[[
                Point::new(x, y),
                Point::new(x, y + h),
                Point::new(x + w, y + h),
                Point::new(x + w, y),
                Point::new(x, y),
            ]]);
            self.writer.element(
                "path",
                "",
                &[("d", path_data(&outline, Affine2D::IDENTITY))],
            );
        }
        Ok(())
    }

    /// Writes the comment and group of one text.
    fn text_group(
        &mut self,
        gc: &GraphicsContext,
        source: &TextSource,
        origin: Point,
        angle: f64,
    ) -> Result<(), VectorError> {
        self.writer.comment(source.text());
        let color = gc.stroke_color();
        let mut style = Style::new();
        if color.to_hex(false) != "#000000" {
            style.push(("fill", color.to_hex(false)));
        }
        let alpha = gc.alpha.unwrap_or(color.a);
        if alpha != 1.0 {
            style.push(("opacity", short(alpha)));
        }
        let font_scale = source.size() / FONT_SCALE;
        self.writer.start(
            "g",
            &[
                ("style", css(&style)),
                (
                    "transform",
                    transform_attr(&[
                        TransformOp::Translate(origin.x, self.height - origin.y),
                        TransformOp::Rotate(-angle),
                        TransformOp::ScaleXY(font_scale, -font_scale),
                    ]),
                ),
            ],
        );
        let result = self.text_glyphs(source);
        self.writer.end(true);
        result
    }

    /// `draw_text`: the text as glyph paths or text elements; runs without
    /// a source are filled as paths.
    pub(super) fn svg_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        let Some(source) = &run.source else {
            crate::renderer::fill_text(self, gc, run, origin, angle);
            return;
        };
        let clip = self.clip_attribute(gc);
        if let Some(clip) = &clip {
            self.writer.start("g", std::slice::from_ref(clip));
        }
        let link = self.open_link(gc);
        match self.options.fonttype {
            SvgFontType::Path => {
                let result = self.text_group(gc, source, origin, angle);
                self.problems.record(result);
            }
            SvgFontType::None => {
                let points = in_points(run, source, &self.fonts).map_err(VectorError::from);
                if let Some(points) = self.problems.record(points) {
                    self.text_elements(gc, &points, source, origin, angle);
                }
            }
        }
        self.close_link(link);
        if clip.is_some() {
            self.writer.end(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_urllib() {
        assert_eq!(url_quote("DejaVuSans-14"), "DejaVuSans-14");
        assert_eq!(url_quote("My Font-1f"), "My%20Font-1f");
        assert_eq!(adjusted("My%20Font-1f"), "My_Font-1f");
    }
}
