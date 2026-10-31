//! matplotlib's `dviread.Dvi` and `DviFont`: the pages of a DVI file read
//! by pltrs-text (`read_dvi`), and the fonts their glyphs are set in.

use std::collections::HashMap;
use std::sync::Arc;

use pltrs_font::glyph_name_to_unicode;
use pltrs_text::tex::{DviFont, RawPage, read_dvi};

use super::{tex_files, type1_effects};
use crate::binding::new_dict;
use crate::binding::prelude::*;

native_value!(Arc<DviFont>);

/// A number of a page: in DVI units (an `int`) without a resolution, in
/// pixels (a `float`) at one.
#[derive(Clone, Copy)]
enum Number {
    Units(i64),
    Pixels(f64),
}

impl IntoPy for Number {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        match self {
            Self::Units(units) => units.into_py(py),
            Self::Pixels(pixels) => pixels.into_py(py),
        }
    }
}

/// DVI units as a Python `int`.
fn units(value: i128) -> PyResult<Number> {
    i64::try_from(value)
        .map(Number::Units)
        .map_err(|_| PyErr::new(ExceptionKind::OverflowError, "DVI number too large"))
}

/// A glyph: `(x, y, font, glyph, width)`, the font as its index in the
/// fonts of the file.
type Glyph = (Number, Number, usize, i64, Number);

/// A rule: `(x, y, height, width)`.
type Rule = (Number, Number, Number, Number);

/// A page: `(text, boxes, height, width, descent)`, the fields of
/// matplotlib's `Page`.
type Page = (Vec<Glyph>, Vec<Rule>, Number, Number, Number);

/// The fonts of the pages, each once, in the order their glyphs come.
#[derive(Default)]
struct Fonts {
    fonts: Vec<Arc<DviFont>>,
    index: HashMap<*const DviFont, usize>,
}

impl Fonts {
    /// The index of `font`, added when it is new.
    fn index(&mut self, font: &Arc<DviFont>) -> usize {
        *self.index.entry(Arc::as_ptr(font)).or_insert_with(|| {
            self.fonts.push(Arc::clone(font));
            self.fonts.len() - 1
        })
    }
}

/// `page` in DVI units (matplotlib's `Dvi._output` with `dpi=None`).
fn page_in_units(page: &RawPage, fonts: &mut Fonts) -> PyResult<Page> {
    let text = page
        .glyphs()
        .iter()
        .map(|glyph| {
            Ok((
                units(glyph.x)?,
                units(glyph.y)?,
                fonts.index(&glyph.font),
                i64::try_from(glyph.glyph).map_err(|_| PyErr::value_error("glyph too large"))?,
                units(glyph.width)?,
            ))
        })
        .collect::<PyResult<_>>()?;
    let boxes = page
        .rules()
        .iter()
        .map(|rule| {
            Ok((
                units(rule.x)?,
                units(rule.y)?,
                units(rule.height)?,
                units(rule.width)?,
            ))
        })
        .collect::<PyResult<_>>()?;
    let (width, height, descent) = page.extent()?;
    Ok((text, boxes, units(height)?, units(width)?, units(descent)?))
}

/// `page` in pixels at `dpi` (matplotlib's `Dvi._output`).
fn page_in_pixels(page: &RawPage, dpi: f64, fonts: &mut Fonts) -> PyResult<Page> {
    let page = page.at(dpi)?;
    let text = page
        .text
        .iter()
        .map(|text| {
            (
                Number::Pixels(text.x),
                Number::Pixels(text.y),
                fonts.index(&text.font),
                text.glyph,
                Number::Pixels(text.width),
            )
        })
        .collect();
    let boxes = page
        .boxes
        .iter()
        .map(|b| {
            (
                Number::Pixels(b.x),
                Number::Pixels(b.y),
                Number::Pixels(b.height),
                Number::Pixels(b.width),
            )
        })
        .collect();
    Ok((
        text,
        boxes,
        Number::Pixels(page.height),
        Number::Pixels(page.width),
        Number::Pixels(page.descent),
    ))
}

/// Iterating over `Dvi(filename, dpi)`: the fonts of the DVI file `data`
/// and its pages, in DVI units without `dpi`, in pixels at it otherwise;
/// glyphs refer to their font by its index in the fonts.
fn read_pages(data: Bytes, dpi: Option<f64>) -> PyResult<(Vec<Arc<DviFont>>, Vec<Page>)> {
    let raw = read_dvi(&data.0, tex_files()?)?;
    let mut fonts = Fonts::default();
    let pages = raw
        .iter()
        .map(|page| match dpi {
            None => page_in_units(page, &mut fonts),
            Some(dpi) => page_in_pixels(page, dpi, &mut fonts),
        })
        .collect::<PyResult<_>>()?;
    Ok((fonts.fonts, pages))
}

/// A font's `texname`, `_scale` and `size`.
fn font_name_and_size(font: Arc<DviFont>) -> PyResult<(Bytes, i64, f64)> {
    let scale =
        i64::try_from(font.scale()).map_err(|_| PyErr::value_error("font scale too large"))?;
    Ok((Bytes(font.texname().to_vec()), scale, font.size()))
}

/// `DviFont.resolve_path()`: the font file of a native font, or the file
/// the font map gives (or the PK font kpathsea makes) for a TeX font.
fn font_path(font: Arc<DviFont>) -> PyResult<String> {
    Ok(tex_files()?.font_path(&font)?.display().to_string())
}

/// `DviFont.subfont`: the face of a native font's file, 0 for TeX fonts.
fn font_subfont(font: Arc<DviFont>) -> PyResult<u32> {
    Ok(font.native_font().map_or(0, |native| native.subfont))
}

/// `DviFont.effects`: what a native font's document asked for (`rgba`,
/// `extend`, `slant`, `embolden`), or the `slant` and `extend` the font
/// map gives a TeX font (none for a PK font).
fn font_effects(font: Arc<DviFont>) -> PyResult<Owned> {
    let Some(native) = font.native_font() else {
        return type1_effects(tex_files()?.effects(&font)?);
    };
    let effects = native.effects;
    Python::with_gil(|py| {
        let dict = new_dict(py)?;
        if let Some(rgba) = effects.rgba {
            dict.set_item("rgba", rgba.to_vec())?;
        }
        for (key, value) in [
            ("extend", effects.extend),
            ("slant", effects.slant),
            ("embolden", effects.embolden),
        ] {
            if let Some(value) = value {
                dict.set_item(key, value)?;
            }
        }
        Ok(dict.unbind())
    })
}

/// `DviFont._width_of(char)`: the width of a character in DVI units.
fn font_width_of(font: Arc<DviFont>, code: i64) -> PyResult<Number> {
    units(font.width_of(i128::from(code))?)
}

/// `DviFont._height_depth_of(char)`: the height and depth of a character
/// in DVI units.
fn font_height_depth_of(font: Arc<DviFont>, code: i64) -> PyResult<(Number, Number)> {
    let (height, depth) = font.height_depth_of(i128::from(code))?;
    Ok((units(height)?, units(depth)?))
}

/// `DviFont._index_dvi_to_freetype(idx)`: the glyph of the font file a
/// character code shows.
fn font_glyph_index(font: Arc<DviFont>, code: i64) -> PyResult<u16> {
    Ok(tex_files()?.glyph_index(&font, code)?.0)
}

/// `Text._as_unicode_or_name()` of a glyph of an outline font: the
/// characters its glyph name stands for by the Adobe Glyph List, or the
/// name when it stands for none.
fn unicode_or_name(font: Arc<DviFont>, code: i64) -> PyResult<String> {
    let name = tex_files()?.glyph_name(&font, code)?;
    let unicode = glyph_name_to_unicode(&name);
    Ok(if unicode.is_empty() { name } else { unicode })
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("read_pages", read_pages)?;
    module.add("font_name_and_size", font_name_and_size)?;
    module.add("font_path", font_path)?;
    module.add("font_subfont", font_subfont)?;
    module.add("font_effects", font_effects)?;
    module.add("font_width_of", font_width_of)?;
    module.add("font_height_depth_of", font_height_depth_of)?;
    module.add("font_glyph_index", font_glyph_index)?;
    module.add("unicode_or_name", unicode_or_name)
}
