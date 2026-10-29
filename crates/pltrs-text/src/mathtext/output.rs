//! Typeset math and its conversion to backend-ready runs (matplotlib's
//! `Output.to_vector` and the extents of `Output.to_raster`).

use pltrs_core::Bbox;
use pltrs_font::{FontId, GlyphId};

use super::boxes::Shipped;
use crate::run::{GlyphChars, PlacedGlyph, TextRun};

/// A glyph of typeset math.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MathGlyph {
    /// The face.
    pub font: FontId,
    /// The glyph.
    pub glyph: GlyphId,
    /// The character code it was looked up by.
    pub code: u32,
    /// Size in points.
    pub size: f64,
    /// Horizontal position of the glyph origin, in pixels.
    pub x: f64,
    /// Vertical position of the glyph origin, in pixels downwards from the
    /// top of the box.
    pub y: f64,
    /// Vertical offset matplotlib adds when drawing (non-zero only for the
    /// Computer Modern extension font), pixels upwards.
    pub offset: f64,
    /// Ink box relative to the glyph origin, y up, including `offset`.
    pub ink: Bbox,
}

/// A filled rectangle of typeset math (fraction bars, radicals, ...), in
/// pixels with `y` the top edge measured downwards from the top of the box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MathRect {
    /// Left edge.
    pub x: f64,
    /// Top edge (downwards from the top of the box).
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// Extents of math rendered to a tight bitmap (matplotlib's
/// `RasterParse`), in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterExtents {
    /// Bitmap width.
    pub width: f64,
    /// Bitmap height including the depth.
    pub height: f64,
    /// Depth below the baseline.
    pub depth: f64,
}

/// Math typeset by the TeX box model, before any rounding: the outer box
/// and every glyph and rule at its exact position.
#[derive(Clone, Debug, PartialEq)]
pub struct MathLayout {
    /// Width of the box.
    pub width: f64,
    /// Height of the box above the baseline.
    pub height: f64,
    /// Depth of the box below the baseline.
    pub depth: f64,
    /// Glyphs in drawing order.
    pub glyphs: Vec<MathGlyph>,
    /// Rules in drawing order.
    pub rects: Vec<MathRect>,
    /// Resolution the math was typeset for.
    pub dpi: f64,
}

impl MathLayout {
    /// Collects a shipped box.
    pub(crate) fn new(width: f64, height: f64, depth: f64, shipped: Shipped, dpi: f64) -> Self {
        let glyphs = shipped
            .glyphs
            .into_iter()
            .map(|(x, y, info)| MathGlyph {
                font: info.font,
                glyph: info.glyph,
                code: info.code,
                size: info.fontsize,
                x,
                y,
                offset: info.offset,
                ink: Bbox::from_extents(
                    info.metrics.xmin,
                    info.metrics.ymin,
                    info.metrics.xmax,
                    info.metrics.ymax,
                ),
            })
            .collect();
        let rects = shipped
            .rects
            .into_iter()
            .map(|(x, y, width, height)| MathRect {
                x,
                y,
                width,
                height,
            })
            .collect();
        Self {
            width,
            height,
            depth,
            glyphs,
            rects,
            dpi,
        }
    }

    /// The run backends draw (matplotlib's `VectorParse`): the box rounded
    /// up to whole pixels, glyphs and rules placed from the rounded top,
    /// y up from the baseline. The run has no source; see
    /// [`TextEngine::layout_math`](crate::TextEngine::layout_math).
    pub fn to_text_run(&self) -> TextRun {
        let (width, height, depth) = (self.width.ceil(), self.height.ceil(), self.depth.ceil());
        TextRun {
            glyphs: self
                .glyphs
                .iter()
                .map(|g| PlacedGlyph {
                    font: g.font,
                    glyph: g.glyph,
                    size: g.size,
                    x: g.x,
                    y: height - g.y + g.offset,
                    chars: GlyphChars::Code(g.code),
                })
                .collect(),
            rects: self
                .rects
                .iter()
                .map(|r| Bbox::from_bounds(r.x, height - (r.y + r.height), r.width, r.height))
                .collect(),
            width,
            height: height + depth,
            descent: depth,
            dpi: self.dpi,
            source: None,
            anchor: None,
        }
    }

    /// The extents matplotlib's raster output (`RasterParse`) reports: the
    /// ink of all glyphs and rules plus a one-pixel margin.
    pub fn raster_extents(&self) -> RasterExtents {
        let glyph_x = self
            .glyphs
            .iter()
            .flat_map(|g| [g.x + g.ink.xmin(), g.x + g.ink.xmax()]);
        let glyph_y = self
            .glyphs
            .iter()
            .flat_map(|g| [g.y - g.ink.ymax(), g.y - g.ink.ymin()]);
        let rect_x = self.rects.iter().flat_map(|r| [r.x, r.x + r.width]);
        let rect_y = self.rects.iter().flat_map(|r| [r.y, r.y + r.height]);
        let (xmin, xmax) = bounds(glyph_x.chain(rect_x));
        let (ymin, ymax) = bounds(glyph_y.chain(rect_y));
        let (xmin, ymin, xmax, ymax) = (xmin - 1.0, ymin - 1.0, xmax + 1.0, ymax + 1.0);
        let height = ymax - ymin - self.depth;
        let depth = ymax - ymin - self.height;
        RasterExtents {
            width: xmax - xmin,
            height: height + depth,
            depth,
        }
    }
}

/// Minimum and maximum of `values` and zero.
fn bounds(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((0.0, 0.0), |(lo, hi), v| {
        (if v < lo { v } else { lo }, if v > hi { v } else { hi })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_follows_vector_parse() {
        let layout = MathLayout {
            width: 10.2,
            height: 7.5,
            depth: 0.0,
            glyphs: Vec::new(),
            rects: vec![MathRect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 0.5,
            }],
            dpi: 72.0,
        };
        let run = layout.to_text_run();
        assert_eq!((run.width, run.height, run.descent), (11.0, 8.0, 0.0));
        assert_eq!(run.rects[0], Bbox::from_bounds(1.0, 5.5, 3.0, 0.5));
        let raster = layout.raster_extents();
        assert_eq!(raster.width, 6.0);
        assert_eq!(raster.depth, 4.5 - 7.5);
    }
}
