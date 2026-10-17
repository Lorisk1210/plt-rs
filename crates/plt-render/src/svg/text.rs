//! Text as paths (`_draw_text_as_path`): every glyph is defined once in
//! `FreeType`'s 26.6 units at `TextToPath.FONT_SCALE` (100 pt) and placed
//! with `<use>` inside a group scaled to the font size.

#![allow(
    clippy::float_cmp,
    reason = "matplotlib's SVG writer leaves out exactly-default values"
)]

use std::fmt::Write as _;

use plt_core::{Affine2D, Path, Point};
use plt_text::{PlacedGlyph, TextRun};

use super::SvgRenderer;
use super::defs::path_data;
use super::style::{Style, TransformOp, css, transform_attr};
use crate::gc::GraphicsContext;
use crate::vector::{RunKind, VectorError, base_size, hex_color, kind, shaped_positions, short};

/// The size glyphs are defined at (`TextToPath.FONT_SCALE`).
const FONT_SCALE: f64 = 100.0;

/// One placed glyph: its id, position at [`FONT_SCALE`] and scale.
struct GlyphUse {
    id: String,
    x: f64,
    y: f64,
    scale: f64,
}

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

impl SvgRenderer {
    /// `_get_glyph_repr`: the quoted PostScript name and glyph index.
    fn glyph_id(&mut self, glyph: &PlacedGlyph) -> Result<String, VectorError> {
        let font = self.fonts.get(glyph.font)?;
        Ok(url_quote(&format!(
            "{}-{:x}",
            font.postscript_name(),
            glyph.glyph.0
        )))
    }

    /// `_update_glyph_map_defs`: defines the glyphs not yet defined.
    fn define_glyphs(&mut self, glyphs: &[(String, &PlacedGlyph)]) -> Result<(), VectorError> {
        let engine = self.fonts.engine();
        let mut new = Vec::new();
        for (id, glyph) in glyphs {
            if self.glyphs.contains(id) || new.iter().any(|(known, _)| known == id) {
                continue;
            }
            let outline = engine.glyph_outline(glyph.font, glyph.glyph, FONT_SCALE, 72.0)?;
            new.push((id.clone(), outline));
        }
        if new.is_empty() {
            return Ok(());
        }
        self.writer.start("defs", &[]);
        for (id, outline) in new {
            // Back to FreeType's integral 26.6 units.
            let d = path_data(&outline, Affine2D::scaling(64.0, 64.0));
            self.writer.element(
                "path",
                "",
                &[
                    ("id", id.replace("%20", "_")),
                    ("d", d),
                    (
                        "transform",
                        transform_attr(&[TransformOp::Scale(1.0 / 64.0)]),
                    ),
                ],
            );
            self.glyphs.insert(id);
        }
        self.writer.end(true);
        Ok(())
    }

    /// The glyph placements of a plain-text run at [`FONT_SCALE`]: shaped
    /// again at that size like matplotlib, or scaled when that fails.
    fn plain_uses(&mut self, run: &TextRun, size: f64) -> Result<Vec<GlyphUse>, VectorError> {
        let positions =
            shaped_positions(run, &mut self.fonts, FONT_SCALE, 72.0).unwrap_or_else(|| {
                let factor = FONT_SCALE / size * 72.0 / run.dpi;
                run.glyphs
                    .iter()
                    .map(|g| (g.x * factor, g.y * factor))
                    .collect()
            });
        run.glyphs
            .iter()
            .zip(positions)
            .map(|(glyph, (x, y))| {
                Ok(GlyphUse {
                    id: self.glyph_id(glyph)?,
                    x,
                    y,
                    scale: 1.0,
                })
            })
            .collect()
    }

    /// Writes the group of one run; `size` is the font size in points.
    fn text_group(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) -> Result<(), VectorError> {
        let math = kind(run, &mut self.fonts) == RunKind::Math;
        let Some(size) = base_size(run).filter(|_| !run.glyphs.is_empty()) else {
            return Ok(());
        };
        let color = gc.stroke_color();
        let mut style = Style::new();
        if hex_color(color) != "#000000" {
            style.push(("fill", hex_color(color)));
        }
        let alpha = gc.alpha.unwrap_or(color.a);
        if alpha != 1.0 {
            style.push(("opacity", short(alpha)));
        }
        let font_scale = size / FONT_SCALE;
        let to_points = run.dpi / 72.0;
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
        let uses = if math {
            let factor = FONT_SCALE / size / to_points;
            run.glyphs
                .iter()
                .map(|glyph| {
                    Ok(GlyphUse {
                        id: self.glyph_id(glyph)?.replace("%20", "_"),
                        x: glyph.x * factor,
                        y: glyph.y * factor,
                        scale: glyph.size / size,
                    })
                })
                .collect::<Result<Vec<_>, VectorError>>()
        } else {
            self.plain_uses(run, size)
        };
        let result = uses.and_then(|uses| {
            let pairs: Vec<(String, &PlacedGlyph)> =
                uses.iter().map(|u| u.id.clone()).zip(&run.glyphs).collect();
            self.define_glyphs(&pairs)?;
            for glyph in uses {
                self.writer.element(
                    "use",
                    "",
                    &[
                        ("xlink:href", format!("#{}", glyph.id)),
                        (
                            "transform",
                            transform_attr(&[
                                TransformOp::Translate(glyph.x, glyph.y),
                                TransformOp::Scale(glyph.scale),
                            ]),
                        ),
                    ],
                );
            }
            Ok(())
        });
        if math {
            let factor = FONT_SCALE / size / to_points;
            for rect in &run.rects {
                let (x, y, w, h) = rect.bounds();
                let corners = [(x, y), (x, y + h), (x + w, y + h), (x + w, y), (x, y)];
                let outline = Path::compound_from_polygons(&[
                    corners.map(|(cx, cy)| Point::new(cx * factor, cy * factor))
                ]);
                self.writer.element(
                    "path",
                    "",
                    &[("d", path_data(&outline, Affine2D::IDENTITY))],
                );
            }
        }
        self.writer.end(true);
        result
    }

    /// `draw_text` with `svg.fonttype = "path"`.
    pub(super) fn svg_text(
        &mut self,
        gc: &GraphicsContext,
        run: &TextRun,
        origin: Point,
        angle: f64,
    ) {
        let clip = self.clip_attribute(gc);
        if let Some(clip) = &clip {
            self.writer.start("g", std::slice::from_ref(clip));
        }
        let link = self.open_link(gc);
        if !run.glyphs.is_empty() || !run.rects.is_empty() {
            let text = crate::vector::source_text(run, &mut self.fonts);
            self.writer.comment(&text);
        }
        let result = self.text_group(gc, run, origin, angle);
        self.problems.record(result);
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
    }
}
