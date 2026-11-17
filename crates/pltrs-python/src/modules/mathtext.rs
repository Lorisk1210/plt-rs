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
    MathGlyph, MathNode, MathRect, MathStyle, RasterExtents, accent_map, unicode_index,
    wide_accents,
};
use pltrs_text::{GlyphChars, TextEngine, TextSource};

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
        .map_err(|err| math_error(&err))?;
    if let Some(TextSource::Math { logged, .. }) = &run.source {
        pltrs_text::log_mathtext(logged);
    }
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
    // matplotlib's raster output loads glyphs hinted (`load_glyph_flags`),
    // and their boxes are those of the hinted glyphs.
    let style = MathStyle {
        hinting,
        ..math_style(&font, dpi, &rc)
    };
    let engine = TextEngine::global();
    let raster = engine
        .typeset_math_raster(&s, &style)
        .map_err(|err| math_error(&err))?;
    pltrs_text::log_mathtext(&raster.layout.logged);
    let image = RasterImage::new(raster.rows, raster.columns);
    let mut pixels = image.blank();
    let glyph_error = |err: &dyn std::fmt::Display| PyErr::value_error(err.to_string());
    for glyph in &raster.layout.glyphs {
        let (font, id, size) = (glyph.font, glyph.glyph, glyph.size);
        let outline = engine
            .scan_glyph_outline(font, id, size, dpi, hinting, true)
            .map_err(|err| glyph_error(&err))?;
        let shape = if antialiased {
            GlyphShape::Smooth(outline)
        } else {
            GlyphShape::Mono(outline)
        };
        let bitmap = render_glyph(&shape, 0.0, Point::ORIGIN).map_err(|err| glyph_error(&err))?;
        if let Some(bitmap) = bitmap {
            image.draw_glyph(&mut pixels, glyph, &bitmap, antialiased);
        }
    }
    for rect in &raster.layout.rects {
        image.fill_rect(&mut pixels, rect);
    }
    let RasterExtents {
        width,
        height,
        depth,
    } = raster.extents;
    let shape = vec![image.rows, image.columns];
    Ok(((width, height, depth), NdArray::with_shape(pixels, shape)?))
}

/// The image `Output.to_raster` draws an expression into, which the
/// expression is shipped to with its top-left corner at the origin.
struct RasterImage {
    rows: usize,
    columns: usize,
}

impl RasterImage {
    /// An image of `rows` and `columns`.
    fn new(rows: usize, columns: usize) -> Self {
        Self { rows, columns }
    }

    /// An all-zero image.
    fn blank(&self) -> Vec<u8> {
        vec![0; self.rows * self.columns]
    }

    /// Or-s `bitmap`, the rendering of `glyph`, into `image` with its top
    /// edge at the glyph's iceberg (`draw_glyph_to_bitmap` at `oy -
    /// metrics.iceberg`).
    fn draw_glyph(
        &self,
        image: &mut [u8],
        glyph: &MathGlyph,
        bitmap: &GlyphBitmap,
        antialiased: bool,
    ) {
        let left = glyph.x as i64 + i64::from(bitmap.left);
        let top = (glyph.y - glyph.iceberg) as i64;
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
        let (x, y) = (rect.x, rect.y);
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

/// The exception matplotlib's parser raises for `err`: `FreeType`'s
/// `RuntimeError` for a size it refuses, a `ValueError` otherwise.
fn math_error(err: &pltrs_text::TextError) -> PyErr {
    match err {
        pltrs_text::TextError::FontSize(_) => {
            PyErr::new(ExceptionKind::RuntimeError, err.to_string())
        }
        other => PyErr::value_error(math_message(other)),
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
/// symbol, `ValueError` for unknown symbols (quoted as Python's `repr`
/// quotes them, as matplotlib's message does).
fn get_unicode_index(symbol: String) -> PyResult<u32> {
    unicode_index(&symbol).ok_or_else(|| {
        let quoted = Python::with_gil(|py| symbol.as_str().into_py(py)?.repr())
            .unwrap_or_else(|_| format!("{symbol:?}"));
        PyErr::value_error(format!(
            "{quoted} is not a valid Unicode character or TeX/Type1 symbol"
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

/// A node of a box tree as the Python side rebuilds it: `(class name,
/// payload)`, the payload the symbol of a character, the width of a kern,
/// `None` for glue, boxes and rules, and `(width, height, depth,
/// shift_amount, children)` for a list.
fn box_node<'py>(py: Python<'py>, node: &MathNode) -> PyResult<Bound<'py>> {
    match node {
        MathNode::Char(symbol) => ("Char", symbol).into_py(py),
        MathNode::Kern(width) => ("Kern", *width).into_py(py),
        MathNode::Glue => ("Glue", Bound::none(py)).into_py(py),
        MathNode::Leaf(class) => (*class, Bound::none(py)).into_py(py),
        MathNode::List {
            class,
            width,
            height,
            depth,
            shift_amount,
            children,
        } => {
            let children = children
                .iter()
                .map(|child| box_node(py, child))
                .collect::<PyResult<Vec<_>>>()?;
            (*class, (*width, *height, *depth, *shift_amount, children)).into_py(py)
        }
    }
}

/// `_mathtext.Parser().parse(s, fonts_object, fontsize, dpi)`: the box
/// tree `s` is typeset into with the font properties `props` (their size
/// and math font family) at `dpi`.
fn parse_box_tree(s: String, dpi: f64, props: FontProps) -> PyResult<Owned> {
    let rc = pltrs::rc::params();
    let tree = TextEngine::global()
        .math_box_tree(&s, &math_style(&props.0, dpi, &rc))
        .map_err(|err| math_error(&err))?;
    Python::with_gil(|py| Ok(box_node(py, &tree)?.unbind()))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("parse_box_tree", parse_box_tree)?;
    module.add("parse_path", parse_path)?;
    module.add("parse_raster", parse_raster)?;
    module.add("get_unicode_index", get_unicode_index)?;
    module.add("accent_map", get_accent_map)?;
    module.add("wide_accents", get_wide_accents)?;
    super::mathtext_program::register(module)
}
