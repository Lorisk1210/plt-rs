//! `_native.mathtext`: matplotlib's `mathtext` and `_mathtext`, by
//! pltrs-text's mathtext typesetter.
//!
//! Expressions are typeset with the `mathtext.*` rcParams and the font
//! properties given (their `math_fontfamily` picks the fontset), as
//! `MathTextParser.parse` typesets them.

use pltrs::fonts::FontProperties;
use pltrs::text::math_style;
use pltrs_core::Point;
use pltrs_font::FontSource;
use pltrs_render::{GlyphBitmap, GlyphShape, render_glyph};
use pltrs_text::mathtext::{
    MathGlyph, MathLayout, MathRect, accent_map, unicode_index, wide_accents,
};
use pltrs_text::{GlyphChars, TextEngine};

use super::font_manager::FontProps;
use crate::binding::prelude::*;

/// A glyph of a `VectorParse`: `((font file, face index), size,
/// (character code, glyph index), (x, y))`, positions in pixels from the
/// left end of the baseline, y up.
type VectorGlyph = ((String, u32), f64, (u32, u32), (f64, f64));

/// A rule of a `VectorParse`: `(x, y, width, height)`, y up.
type VectorRect = (f64, f64, f64, f64);

/// A `VectorParse`: `((width, height, depth), glyphs, rects)`.
type Vector = ((f64, f64, f64), Vec<VectorGlyph>, Vec<VectorRect>);

/// The file a font of the text engine is read from: its path, or its file
/// name for a font bundled with pltrs (which the Python side places in
/// its data directory).
pub(super) fn font_file(source: &FontSource) -> String {
    match source {
        FontSource::Bundled(name) => (*name).to_owned(),
        FontSource::File(path) => path.display().to_string(),
    }
}

/// `MathTextParser("path").parse(s, dpi, prop)`, the box rounded up to
/// whole pixels.
fn parse_path(s: String, dpi: f64, props: Option<FontProps>) -> PyResult<Vector> {
    let rc = pltrs::rc::params();
    let font = props.map_or_else(|| FontProperties::from_rc(&rc), |props| props.0);
    let engine = TextEngine::global();
    let run = engine
        .layout_math(&s, &math_style(&font, dpi, &rc))
        .map_err(|err| PyErr::value_error(math_message(&err)))?;
    let db = engine.database();
    let glyphs = run
        .glyphs
        .iter()
        .map(|glyph| {
            let file = db.source(glyph.font).map(font_file).unwrap_or_default();
            let index = db.face_index(glyph.font).unwrap_or(0);
            let code = match glyph.chars {
                GlyphChars::Code(code) => code,
                _ => 0,
            };
            (
                (file, index),
                glyph.size,
                (code, u32::from(glyph.glyph.0)),
                (glyph.x, glyph.y),
            )
        })
        .collect();
    let rects = run.rects.iter().map(pltrs_core::Bbox::bounds).collect();
    Ok(((run.width, run.height, run.descent), glyphs, rects))
}

/// `MathTextParser("agg").parse(s, dpi, prop, antialiased=...)`:
/// `((width, height, depth), image)`, the expression drawn as
/// `Output.to_raster` draws it: every glyph rendered by `FreeType`'s
/// rasterizer (hinted as :rc:`text.hinting` asks) and or-ed into the
/// image at its whole-pixel position, every rule filled.
fn parse_raster(
    s: String,
    dpi: f64,
    props: Option<FontProps>,
    antialiased: bool,
) -> PyResult<((f64, f64, f64), NdArray<u8>)> {
    let rc = pltrs::rc::params();
    let font = props.map_or_else(|| FontProperties::from_rc(&rc), |props| props.0);
    let hinting = font.request(&rc).hinting;
    let layout = TextEngine::global()
        .typeset_math(&s, &math_style(&font, dpi, &rc))
        .map_err(|err| PyErr::value_error(math_message(&err)))?;
    let raster = RasterImage::of(&layout);
    let mut image = raster.blank();
    let engine = TextEngine::global();
    let glyph_error = |err: &dyn std::fmt::Display| PyErr::value_error(err.to_string());
    for glyph in &layout.glyphs {
        let (font, id, size) = (glyph.font, glyph.glyph, glyph.size);
        let shape = if antialiased {
            GlyphShape::Smooth(
                engine
                    .raster_glyph_outline(font, id, size, dpi, hinting, true)
                    .map_err(|err| glyph_error(&err))?,
            )
        } else {
            GlyphShape::Mono(
                engine
                    .scan_glyph_outline(font, id, size, dpi, hinting, true)
                    .map_err(|err| glyph_error(&err))?,
            )
        };
        let bitmap = render_glyph(&shape, 0.0, Point::ORIGIN).map_err(|err| glyph_error(&err))?;
        if let Some(bitmap) = bitmap {
            raster.draw_glyph(&mut image, glyph, &bitmap, antialiased);
        }
    }
    for rect in &layout.rects {
        raster.fill_rect(&mut image, rect);
    }
    let shape = vec![raster.rows, raster.columns];
    Ok((raster.metrics, NdArray::with_shape(image, shape)?))
}

/// The image `Output.to_raster` draws an expression into: its size and
/// metrics, and the offset of the box in it.
struct RasterImage {
    /// `(width, height, depth)` as `RasterParse` reports them.
    metrics: (f64, f64, f64),
    rows: usize,
    columns: usize,
    /// The left and top edges of the image in the box's coordinates.
    xmin: f64,
    ymin: f64,
}

impl RasterImage {
    /// The image of `layout`: the ink of its glyphs and rules, and the
    /// origin, with a one-pixel margin.
    fn of(layout: &MathLayout) -> Self {
        let glyph_x = layout
            .glyphs
            .iter()
            .flat_map(|g| [g.x + g.ink.xmin(), g.x + g.ink.xmax()]);
        let glyph_y = layout
            .glyphs
            .iter()
            .flat_map(|g| [g.y - g.ink.ymax(), g.y - g.ink.ymin()]);
        let rect_x = layout.rects.iter().flat_map(|r| [r.x, r.x + r.width]);
        let rect_y = layout.rects.iter().flat_map(|r| [r.y, r.y + r.height]);
        let bounds = |values: &mut dyn Iterator<Item = f64>| {
            values.fold((0.0_f64, 0.0_f64), |(lo, hi), v| (lo.min(v), hi.max(v)))
        };
        let (xmin, xmax) = bounds(&mut glyph_x.chain(rect_x));
        let (ymin, ymax) = bounds(&mut glyph_y.chain(rect_y));
        let (xmin, ymin, xmax, ymax) = (xmin - 1.0, ymin - 1.0, xmax + 1.0, ymax + 1.0);
        let width = xmax - xmin;
        let height = ymax - ymin - layout.depth;
        let depth = ymax - ymin - layout.height;
        let size = |v: f64| v.ceil().max(0.0) as usize;
        Self {
            metrics: (width, height + depth, depth),
            rows: size(height + depth.max(0.0)),
            columns: size(width),
            xmin,
            ymin,
        }
    }

    /// An all-zero image.
    fn blank(&self) -> Vec<u8> {
        vec![0; self.rows * self.columns]
    }

    /// Or-s `bitmap`, the rendering of `glyph`, into `image` with its top
    /// edge at the glyph's ascent (`draw_glyph_to_bitmap`).
    fn draw_glyph(
        &self,
        image: &mut [u8],
        glyph: &MathGlyph,
        bitmap: &GlyphBitmap,
        antialiased: bool,
    ) {
        let left = (glyph.x - self.xmin) as i64 + i64::from(bitmap.left);
        let top = (glyph.y - self.ymin - glyph.ink.ymax()) as i64;
        let scale = if antialiased { 1 } else { 0xff };
        for (row, line) in bitmap
            .buffer
            .chunks(bitmap.width.max(1) as usize)
            .enumerate()
        {
            for (column, &value) in line.iter().enumerate() {
                if let Some(index) = self.index(left + column as i64, top + row as i64) {
                    image[index] |= value.saturating_mul(scale);
                }
            }
        }
    }

    /// Fills `rect` (a rule of the box) as `to_raster` does: at least one
    /// row high, centred on the rule when it is thinner than two pixels.
    fn fill_rect(&self, image: &mut [u8], rect: &MathRect) {
        let (x, y) = (rect.x - self.xmin, rect.y - self.ymin);
        let height = ((rect.height as i64) - 1).max(0);
        let top = if height == 0 {
            (y + rect.height / 2.0 - 0.5) as i64
        } else {
            y as i64
        };
        let (x1, x2) = (x.floor() as i64, (x + rect.width).ceil() as i64);
        for row in top..=top + height {
            for column in x1..=x2 {
                if let Some(index) = self.index(column, row) {
                    image[index] = 0xff;
                }
            }
        }
    }

    /// The index of pixel `(x, y)`, if it lies in the image.
    fn index(&self, x: i64, y: i64) -> Option<usize> {
        let x = usize::try_from(x).ok().filter(|&x| x < self.columns)?;
        let y = usize::try_from(y).ok().filter(|&y| y < self.rows)?;
        Some(y * self.columns + x)
    }
}

/// matplotlib's message for an expression mathtext cannot typeset.
fn math_message(err: &pltrs_text::TextError) -> String {
    match err {
        pltrs_text::TextError::Math(message) => message.clone(),
        other => other.to_string(),
    }
}

/// `get_unicode_index(symbol)`: the code point of a character or TeX
/// symbol, `ValueError` for unknown symbols.
fn get_unicode_index(symbol: String) -> PyResult<u32> {
    unicode_index(&symbol).ok_or_else(|| {
        PyErr::value_error(format!(
            "'{symbol}' is not a valid Unicode character or TeX/Type1 symbol"
        ))
    })
}

/// `Parser._accent_map`: each narrow accent command and the symbol it
/// draws.
fn get_accent_map() -> PyResult<Vec<(&'static str, &'static str)>> {
    Ok(accent_map().to_vec())
}

/// `Parser._wide_accents`.
fn get_wide_accents() -> PyResult<Vec<&'static str>> {
    Ok(wide_accents().to_vec())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("parse_path", parse_path)?;
    module.add("parse_raster", parse_raster)?;
    module.add("get_unicode_index", get_unicode_index)?;
    module.add("accent_map", get_accent_map)?;
    module.add("wide_accents", get_wide_accents)
}
