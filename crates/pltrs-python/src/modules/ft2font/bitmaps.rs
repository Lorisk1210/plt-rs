//! Glyph bitmaps of an `FT2Font`: the text `set_text` placed drawn into
//! the font's image (`draw_glyphs_to_bitmap`, `get_image`), one loaded
//! glyph drawn into an image (`draw_glyph_to_bitmap`), and a glyph
//! rendered at the font's transform (`_render_glyph`), with pltrs-render's
//! ports of `FreeType`'s rasterizers.

use pltrs_core::Point;
use pltrs_font::GlyphId;
use pltrs_render::{GlyphBitmap, GlyphShape, render_glyph};

use super::glyphs::Loaded;
use super::{Ft2Font, ft_error};
use crate::binding::prelude::*;

/// The bitmap of a loaded glyph rendered at its pen, antialiased or not.
fn glyph_bitmap(
    font: &Ft2Font,
    glyph: &Loaded,
    antialiased: bool,
) -> PyResult<Option<GlyphBitmap>> {
    let face = font.chain_face(glyph.face)?;
    let size = glyph.size.of(&face);
    let outline = match font.load_hinted(glyph.face, &face, &size, glyph.glyph, glyph.hinting)? {
        Some(hinted) => hinted.scan_outline(),
        None => size
            .scan_outline(&face, glyph.glyph)
            .map_err(|err| ft_error("load glyph", &err))?,
    };
    let shape = if antialiased {
        GlyphShape::Smooth(outline)
    } else {
        GlyphShape::Mono(outline)
    };
    let pen = Point::new(glyph.pen.0 as f64 / 64.0, glyph.pen.1 as f64 / 64.0);
    render_glyph(&shape, glyph.angle, pen).map_err(|err| PyErr::runtime_error(err.to_string()))
}

/// Or-s `bitmap` into `image` (`rows × columns`) with its top-left pixel
/// at `(x, y)` (`draw_bitmap`): coverage or-ed in, monochrome pixels set to
/// 255.
fn draw_bitmap(
    image: &mut [u8],
    (rows, columns): (usize, usize),
    bitmap: &GlyphBitmap,
    (x, y): (i64, i64),
    antialiased: bool,
) {
    let width = bitmap.width as usize;
    for (row, line) in bitmap.buffer.chunks(width.max(1)).enumerate() {
        let Some(dy) = usize::try_from(y + row as i64).ok().filter(|&dy| dy < rows) else {
            continue;
        };
        for (column, &value) in line.iter().enumerate() {
            let Some(dx) = usize::try_from(x + column as i64)
                .ok()
                .filter(|&dx| dx < columns)
            else {
                continue;
            };
            let pixel = &mut image[dy * columns + dx];
            if antialiased {
                *pixel |= value;
            } else if value != 0 {
                *pixel = 0xff;
            }
        }
    }
}

/// The bitmap of the loaded glyph `index`, rendered as it was first
/// drawn, else antialiased or not as asked (and so from then on), with
/// whether it is antialiased.
fn drawn_bitmap(
    font: &mut Ft2Font,
    index: usize,
    antialiased: bool,
) -> PyResult<(Option<GlyphBitmap>, bool)> {
    let glyph = font
        .glyphs
        .loaded
        .get_mut(index)
        .ok_or_else(|| PyErr::runtime_error("glyph num is out of range"))?;
    let antialiased = *glyph.rendered.get_or_insert(antialiased);
    let glyph = *glyph;
    Ok((glyph_bitmap(font, &glyph, antialiased)?, antialiased))
}

/// `draw_glyphs_to_bitmap(antialiased)`: renders the text `set_text`
/// placed into the font's image.
fn draw_glyphs_to_bitmap(font: Native<Ft2Font>, antialiased: bool) -> PyResult<()> {
    let mut font = font.borrow_mut();
    let (x0, y0, x1, y1) = font.glyphs.text.bbox;
    let columns = usize::try_from((x1 - x0) / 64 + 2).unwrap_or(0);
    let rows = usize::try_from((y1 - y0) / 64 + 2).unwrap_or(0);
    let mut image = vec![0; rows * columns];
    for index in 0..font.glyphs.loaded.len() {
        if let (Some(bitmap), smooth) = drawn_bitmap(&mut font, index, antialiased)? {
            let x = (f64::from(bitmap.left) - x0 as f64 / 64.0) as i64;
            let y = (y1 as f64 / 64.0 - f64::from(bitmap.top) + 1.0) as i64;
            draw_bitmap(&mut image, (rows, columns), &bitmap, (x, y), smooth);
        }
    }
    font.glyphs.image = (rows, columns, image);
    Ok(())
}

/// `get_image()`: the image `draw_glyphs_to_bitmap` drew.
fn get_image(font: Native<Ft2Font>) -> PyResult<NdArray<u8>> {
    let font = font.borrow();
    let (rows, columns, pixels) = &font.glyphs.image;
    if pixels.is_empty() {
        return NdArray::with_shape(vec![0], vec![1, 1]);
    }
    NdArray::with_shape(pixels.clone(), vec![*rows, *columns])
}

/// `draw_glyph_to_bitmap(image, x, y, glyph, antialiased)`: the image
/// with loaded glyph `index` rendered where it was loaded (the origin for
/// `load_char` and `load_glyph`) and or-ed in with its left edge offset
/// by `x` and its top row at `y`.
fn draw_glyph_to_bitmap(
    font: Native<Ft2Font>,
    image: NdArray<u8>,
    (x, y): (i64, i64),
    index: usize,
    antialiased: bool,
) -> PyResult<NdArray<u8>> {
    let shape = image.shape.clone();
    let [rows, columns] = shape[..] else {
        return Err(PyErr::value_error("image must be 2D"));
    };
    let mut pixels = image.data;
    if let (Some(bitmap), smooth) = drawn_bitmap(&mut font.borrow_mut(), index, antialiased)? {
        let left = x + i64::from(bitmap.left);
        draw_bitmap(&mut pixels, (rows, columns), &bitmap, (left, y), smooth);
    }
    NdArray::with_shape(pixels, shape)
}

/// `_render_glyph(index, flags, render_mode)`: glyph `index` rendered at
/// the font's transform, `((left, top), pixels)`.
fn render_glyph_at(
    font: Native<Ft2Font>,
    index: u16,
    flags: u32,
    mono: bool,
) -> PyResult<((i32, i32), NdArray<u8>)> {
    let font = font.borrow();
    let ([[xx, _], [yx, _]], [dx, dy]) = font.transform;
    let angle = (yx as f64).atan2(xx as f64).to_degrees();
    let glyph = Loaded::new(0, GlyphId(index), font.loading(flags), (dx, dy), angle);
    match glyph_bitmap(&font, &glyph, !mono)? {
        Some(bitmap) => {
            let shape = vec![bitmap.rows as usize, bitmap.width as usize];
            Ok((
                (bitmap.left, bitmap.top),
                NdArray::with_shape(bitmap.buffer, shape)?,
            ))
        }
        None => Ok(((0, 0), NdArray::with_shape(Vec::new(), vec![0, 0])?)),
    }
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("draw_glyphs_to_bitmap", draw_glyphs_to_bitmap)?;
    module.add("get_image", get_image)?;
    module.add("draw_glyph_to_bitmap", draw_glyph_to_bitmap)?;
    module.add("render_glyph", render_glyph_at)
}
