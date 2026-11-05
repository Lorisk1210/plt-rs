//! `_native.ft2font`: matplotlib's `FT2Font`, a face of a font file as
//! `FreeType` opens it, by pltrs-font (which ports `FreeType`'s TrueType
//! loader, bytecode interpreter and autohinter) and pltrs-render (its
//! rasterizers).
//!
//! The face's own properties are here; glyphs are loaded in [`glyphs`],
//! text is laid out in [`layout`], bitmaps are drawn in [`bitmaps`], and
//! the sfnt tables are read in [`tables`].

mod bitmaps;
mod glyphs;
mod layout;
mod tables;

use std::cell::OnceCell;
use std::rc::Rc;

use pltrs_font::{Autohinter, Face, FontError, GlyphId, Hinting, KerningMode, Size, face_count};

use crate::binding::prelude::*;

/// The bytes of a font file and the face of it in use.
#[derive(Debug)]
struct FaceData {
    data: Rc<[u8]>,
    index: u32,
    /// The autohinter's analysis of the face, made the first time one of
    /// its glyphs is autohinted and kept, as `FreeType` keeps it with the
    /// face.
    autohinter: OnceCell<Autohinter>,
}

impl FaceData {
    /// Face `index` of the font file `data`.
    fn new(data: Rc<[u8]>, index: u32) -> Self {
        Self {
            data,
            index,
            autohinter: OnceCell::new(),
        }
    }

    /// The face, with no character map selected beyond `FreeType`'s
    /// default.
    fn face(&self) -> PyResult<Face<'_>> {
        Face::parse(&self.data, self.index).map_err(|err| ft_error("load face", &err))
    }

    /// The autohinter's analysis of `face`, this face (see
    /// [`autohinter`](Self::autohinter)).
    fn autohinter(&self, face: &Face<'_>) -> &Autohinter {
        self.autohinter.get_or_init(|| Autohinter::new(face))
    }
}

/// An `FT2Font`: a face, its fallbacks, the size and transform glyphs are
/// loaded at, and the glyphs loaded so far.
#[derive(Debug)]
pub(super) struct Ft2Font {
    face: FaceData,
    /// The faces missing characters are looked up in, in order.
    fallbacks: Vec<FaceData>,
    /// The active character map (`set_charmap`); `None` is `FreeType`'s
    /// default choice.
    charmap: Option<usize>,
    /// `set_size(ptsize, dpi)`; `None` before any call, while `FreeType`
    /// loads glyphs unscaled ("undefined scale means no scale"): in font
    /// units, unhinted.
    size: Option<(f64, f64)>,
    /// `_set_transform(matrix, delta)`: 16.16 matrix, 26.6 translation.
    transform: ([[i64; 2]; 2], [i64; 2]),
    /// The power of two kerning is divided by (`_kerning_factor`).
    kerning_factor: u32,
    /// What `load_char`, `load_glyph` and `set_text` loaded.
    glyphs: glyphs::Glyphs,
}

impl IntoPy for Ft2Font {
    fn into_py(self, py: Python<'_>) -> PyResult<Bound<'_>> {
        crate::binding::wrap_native(py, self)
    }
}

/// matplotlib's `RuntimeError` for a `FreeType` failure:
/// `In FT2Font: Can not <what> (<reason>)`.
fn ft_error(what: &str, err: &FontError) -> PyErr {
    PyErr::runtime_error(format!("In FT2Font: Can not {what} ({err})"))
}

impl Ft2Font {
    /// The face with the active character map selected.
    fn face(&self) -> PyResult<Face<'_>> {
        let mut face = self.face.face()?;
        if let Some(index) = self.charmap {
            face.select_charmap(index)
                .map_err(|err| ft_error("set charmap", &err))?;
        }
        Ok(face)
    }

    /// The point size and resolution glyphs of `face` load at: those set,
    /// else those mapping font units to 26.6 pixels unchanged (`units_per_EM
    /// / 64` points at 72 dpi), as `FreeType` loads glyphs unscaled while a
    /// face has no size.
    fn points_and_dpi(&self, face: &Face<'_>) -> (f64, f64) {
        self.size
            .unwrap_or_else(|| (f64::from(face.units_per_em()) / 64.0, 72.0))
    }

    /// The size glyphs of `face` load at (see
    /// [`points_and_dpi`](Self::points_and_dpi)).
    fn size_of(&self, face: &Face<'_>) -> Size {
        let (points, dpi) = self.points_and_dpi(face);
        Size::new(face, points, dpi)
    }

    /// Whether glyphs load unscaled, the face having no size.
    fn unscaled(&self) -> bool {
        self.size.is_none()
    }

    /// The linear advance `FreeType` gives `glyph` of `face` (16.16, the
    /// glyph slot's `linearHoriAdvance`): scaled by the size's scale, which
    /// is zero while the face has no size.
    fn linear_advance(&self, face: &Face<'_>, glyph: GlyphId) -> i64 {
        if self.unscaled() {
            0
        } else {
            self.size_of(face).linear_advance(face, glyph)
        }
    }

    /// The hinting the load flags `flags` select: none while glyphs load
    /// unscaled (`FreeType` adds `FT_LOAD_NO_HINTING` to `FT_LOAD_NO_SCALE`).
    fn hinting(&self, flags: u32) -> Hinting {
        if self.unscaled() {
            Hinting::None
        } else {
            glyphs::hinting(flags)
        }
    }
}

/// `FT2Font(file)`: face `face_index` of the font file `data`, with the
/// faces of `fallbacks` (each `(data, face_index)`) for missing characters.
fn open(data: Bytes, face_index: i64, fallbacks: Vec<(Bytes, u32)>) -> PyResult<Ft2Font> {
    let index = u32::try_from(face_index)
        .ok()
        .filter(|&index| index <= 0xffff)
        .ok_or_else(|| {
            PyErr::new(
                ExceptionKind::ValueError,
                "face_index must be between 0 and 65535, inclusive",
            )
        })?;
    let data: Rc<[u8]> = data.0.into();
    let faces = face_count(&data).map_err(|err| ft_error("load face", &err))?;
    if index >= faces {
        return Err(PyErr::runtime_error(
            "In FT2Font: Can not load face (invalid argument; error code 0x6)",
        ));
    }
    let face = FaceData::new(data, index);
    face.face()?;
    Ok(Ft2Font {
        face,
        fallbacks: fallbacks
            .into_iter()
            .map(|(data, index)| FaceData::new(data.0.into(), index))
            .collect(),
        charmap: None,
        size: None,
        transform: ([[0x10000, 0], [0, 0x10000]], [0, 0]),
        kerning_factor: 0,
        glyphs: glyphs::Glyphs::default(),
    })
}

/// `_backend_pdf_ps.get_glyphs_subset(fontfile, glyphs)`: face
/// `face_index` of the font file `data` reduced to `glyphs` and the glyphs
/// they need, as matplotlib's fontTools subsetting keeps them: the subset
/// font file, and each kept glyph's index with its index in the subset.
fn subset_font(
    data: Bytes,
    face_index: u32,
    glyphs: Vec<u16>,
) -> PyResult<(Bytes, Vec<(u16, u16)>)> {
    let face = FaceData::new(data.0.into(), face_index);
    let subset = pltrs_font::subset(&face.face()?, glyphs.into_iter().map(GlyphId))
        .map_err(|err| ft_error("subset font", &err))?;
    let kept = (0..)
        .zip(&subset.glyphs)
        .map(|(new, old)| (old.0, new))
        .collect();
    Ok((Bytes(subset.data), kept))
}

/// The names of a face: `(postscript_name, family_name, style_name)`.
type Names = (String, String, String);

/// The counts of a face: `(num_faces, num_glyphs, num_charmaps,
/// num_fixed_sizes)`.
type Counts = (u32, u16, usize, u32);

/// The flags and units of a face: `(face_flags, style_flags,
/// units_per_EM, (underline_position, underline_thickness))`.
type Flags = (u32, u32, u16, (i16, i16));

/// The global metrics of a face: `(ascender, descender, height,
/// (max_advance_width, max_advance_height, bbox))`.
type Metrics = (i16, i16, i16, (u16, i16, (i16, i16, i16, i16)));

/// The read-only properties of an `FT2Font`, read once when it is made.
fn attributes(font: Native<Ft2Font>) -> PyResult<(Names, Counts, Flags, Metrics)> {
    let font = font.borrow();
    let face = font.face.face()?;
    let names = (
        face.postscript_name().unwrap_or_default(),
        face.family_name().unwrap_or_default(),
        face.style_name().unwrap_or_default(),
    );
    let faces = face_count(&font.face.data).map_err(|err| ft_error("load face", &err))?;
    let counts = (faces, face.num_glyphs(), face.charmaps().len(), 0);
    let flags = face.face_flags();
    let style = face.style_flags();
    let bit = |on: bool, value: u32| if on { value } else { 0 };
    // FreeType's FT_FACE_FLAG_* values; every sfnt face has a hinter, and
    // matplotlib opens every face from a Python stream (EXTERNAL_STREAM).
    let face_flags = bit(flags.scalable, 1)
        | bit(flags.fixed_width, 1 << 2)
        | bit(flags.sfnt, 1 << 3)
        | bit(flags.horizontal, 1 << 4)
        | bit(flags.vertical, 1 << 5)
        | bit(flags.kerning, 1 << 6)
        | bit(flags.glyph_names, 1 << 9)
        | (1 << 10)
        | (1 << 11);
    let style_flags = bit(style.italic, 1) | bit(style.bold, 2);
    let (position, thickness) = face.underline_metrics().unwrap_or_default();
    // FreeType reports the centre of the underline.
    let underline = (position - thickness / 2, thickness);
    let bbox = face.bbox();
    let metrics = (
        face.ascender(),
        face.descender(),
        face.height(),
        (
            face.hhea().advance_width_max,
            face.max_advance_height(),
            (
                bbox.x_min as i16,
                bbox.y_min as i16,
                bbox.x_max as i16,
                bbox.y_max as i16,
            ),
        ),
    );
    Ok((
        names,
        counts,
        (face_flags, style_flags, face.units_per_em(), underline),
        metrics,
    ))
}

/// `set_size(ptsize, dpi)`.
fn set_size(font: Native<Ft2Font>, ptsize: f64, dpi: f64) -> PyResult<()> {
    let mut font = font.borrow_mut();
    font.size = Some((ptsize, dpi));
    font.transform = ([[0x10000, 0], [0, 0x10000]], [0, 0]);
    Ok(())
}

/// `_set_transform(matrix, delta)`.
fn set_transform(
    font: Native<Ft2Font>,
    matrix: ((i64, i64), (i64, i64)),
    delta: (i64, i64),
) -> PyResult<()> {
    let ((xx, xy), (yx, yy)) = matrix;
    font.borrow_mut().transform = ([[xx, xy], [yx, yy]], [delta.0, delta.1]);
    Ok(())
}

/// `_set_kerning_factor(factor)`.
fn set_kerning_factor(font: Native<Ft2Font>, factor: u32) -> PyResult<()> {
    font.borrow_mut().kerning_factor = factor;
    Ok(())
}

/// `set_charmap(i)`.
fn set_charmap(font: Native<Ft2Font>, index: i64) -> PyResult<()> {
    let mut font = font.borrow_mut();
    let count = font.face.face()?.charmaps().len();
    let index = usize::try_from(index)
        .ok()
        .filter(|&index| index < count)
        .ok_or_else(|| PyErr::runtime_error("i exceeds the available number of char maps"))?;
    font.charmap = Some(index);
    Ok(())
}

/// `select_charmap(encoding)`: the first character map of a `FreeType`
/// encoding tag (`unic` for Unicode, `armn` for Apple Roman).
fn select_charmap(font: Native<Ft2Font>, encoding: u32) -> PyResult<()> {
    let mut font = font.borrow_mut();
    let charmaps = font.face.face()?.charmaps();
    let wanted = |platform: u16, encoding_id: u16| match &encoding.to_be_bytes() {
        b"unic" => platform == 0 || (platform == 3 && matches!(encoding_id, 1 | 10)),
        b"armn" => platform == 1 && encoding_id == 0,
        b"symb" => platform == 3 && encoding_id == 0,
        _ => false,
    };
    let index = if &encoding.to_be_bytes() == b"unic" {
        // FreeType prefers the full-repertoire Unicode map.
        charmaps
            .iter()
            .rposition(|info| wanted(info.platform_id, info.encoding_id) && info.format == 12)
            .or_else(|| {
                charmaps
                    .iter()
                    .rposition(|info| wanted(info.platform_id, info.encoding_id))
            })
    } else {
        charmaps
            .iter()
            .position(|info| wanted(info.platform_id, info.encoding_id))
    };
    let index = index.ok_or_else(|| {
        PyErr::runtime_error(
            "In FT2Font: Can not select charmap (invalid argument; error code 0x6)",
        )
    })?;
    font.charmap = Some(index);
    Ok(())
}

/// `get_charmap()`: every character code of the active map with its
/// glyph.
fn get_charmap(font: Native<Ft2Font>) -> PyResult<Vec<(u32, u16)>> {
    let font = font.borrow();
    Ok(font
        .face()?
        .mappings()
        .into_iter()
        .filter(|&(_, glyph)| glyph != GlyphId::NOTDEF)
        .map(|(code, glyph)| (code, glyph.0))
        .collect())
}

/// `get_char_index(codepoint)`: the glyph of a character in the active
/// map, 0 for none.
fn get_char_index(font: Native<Ft2Font>, code: u32) -> PyResult<u16> {
    Ok(font
        .borrow()
        .face()?
        .glyph_index_for_code(code)
        .map_or(0, |glyph| glyph.0))
}

/// `get_kerning(left, right, mode)` (mode 0 default, 1 unfitted, 2
/// unscaled).
fn get_kerning(font: Native<Ft2Font>, left: u16, right: u16, mode: u32) -> PyResult<i64> {
    let font = font.borrow();
    let face = font.face()?;
    if !face.face_flags().kerning {
        return Ok(0);
    }
    let mode = match mode {
        1 => KerningMode::Unfitted,
        2 => KerningMode::Unscaled,
        _ => KerningMode::Default,
    };
    if font.unscaled() && mode != KerningMode::Unscaled {
        // `FT_Get_Kerning` scales by the size's scale, zero without one.
        return Ok(0);
    }
    let kern = font
        .size_of(&face)
        .kerning(&face, GlyphId(left), GlyphId(right), mode);
    Ok(kern / (1_i64 << font.kerning_factor))
}

/// `get_glyph_name(index)`: the glyph's name, or matplotlib's `uniXXXXXXXX`
/// for faces without glyph names.
fn get_glyph_name(font: Native<Ft2Font>, index: u16) -> PyResult<String> {
    let font = font.borrow();
    let face = font.face()?;
    if !face.has_glyph_names() {
        return Ok(format!("uni{index:08x}"));
    }
    Ok(face
        .glyph_name(GlyphId(index))
        .map(str::to_owned)
        .unwrap_or_default())
}

/// `get_name_index(name)`: the glyph a name gives, 0 for none.
fn get_name_index(font: Native<Ft2Font>, name: String) -> PyResult<u16> {
    Ok(font
        .borrow()
        .face()?
        .glyph_index_by_name(&name)
        .map_or(0, |glyph| glyph.0))
}

/// A name record: `((platform, encoding, language, name id), bytes)`.
type NameRecord = ((u16, u16, u16, u16), Bytes);

/// `get_sfnt()`: the name records, `((platform, encoding, language,
/// name id), bytes)`.
fn get_sfnt(font: Native<Ft2Font>) -> PyResult<Vec<NameRecord>> {
    let font = font.borrow();
    let face = font.face()?;
    if !face.face_flags().sfnt {
        return Err(PyErr::value_error("No SFNT name table"));
    }
    Ok(face
        .name_entries()
        .into_iter()
        .map(|entry| {
            let key = entry.key;
            (
                (
                    key.platform_id,
                    key.encoding_id,
                    key.language_id,
                    entry.name_id,
                ),
                Bytes(entry.bytes.to_vec()),
            )
        })
        .collect())
}

/// `get_ps_font_info()`: `FreeType` has PostScript font information only for
/// Type 1 and CFF fonts; TrueType faces raise its error.
fn get_ps_font_info(font: Native<Ft2Font>) -> PyResult<()> {
    font.borrow().face()?;
    Err(PyErr::value_error("Could not get PS font info"))
}

/// `ttfFontProperty(font)`: `(family name, style, variant, weight,
/// stretch)` as matplotlib reads them from the face.
fn face_properties(
    font: Native<Ft2Font>,
) -> PyResult<(String, &'static str, &'static str, (u16, &'static str))> {
    let font = font.borrow();
    let style = pltrs_font::face_style(&font.face.face()?);
    let stretch = match style.stretch.0 {
        300 => "condensed",
        400 => "semi-condensed",
        700 => "expanded",
        _ => "normal",
    };
    Ok((
        style.name,
        style.style.name(),
        style.variant.name(),
        (style.weight, stretch),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("open", open)?;
    module.add("attributes", attributes)?;
    module.add("set_size", set_size)?;
    module.add("set_transform", set_transform)?;
    module.add("set_kerning_factor", set_kerning_factor)?;
    module.add("set_charmap", set_charmap)?;
    module.add("select_charmap", select_charmap)?;
    module.add("get_charmap", get_charmap)?;
    module.add("get_char_index", get_char_index)?;
    module.add("get_kerning", get_kerning)?;
    module.add("get_glyph_name", get_glyph_name)?;
    module.add("get_name_index", get_name_index)?;
    module.add("get_sfnt", get_sfnt)?;
    module.add("get_ps_font_info", get_ps_font_info)?;
    module.add("face_properties", face_properties)?;
    module.add("subset_font", subset_font)?;
    tables::register(module)?;
    glyphs::register(module)?;
    layout::register(module)?;
    bitmaps::register(module)
}
