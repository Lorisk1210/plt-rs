//! Glyphs of an `FT2Font`: loading them (`load_char`, `load_glyph`) with
//! their metrics, and the outline of the glyph last loaded (`get_path`),
//! as matplotlib's `ft2font.cpp` does them with `FreeType`. What the font
//! has loaded is kept here for the text layout ([`super::layout`]) and the
//! bitmaps ([`super::bitmaps`]).

use std::collections::HashMap;

use pltrs_core::PathCode;
use pltrs_font::{
    AutohintTarget, Face, GlyphId, HintedGlyph, Hinter, Hinting, HintingInstance, Rect, Size,
};
use pltrs_text::PathSink;

use super::{FaceData, Ft2Font, ft_error};
use crate::binding::prelude::*;

/// `FT_LOAD_NO_SCALE`: the glyph in font units, unhinted.
pub(super) const NO_SCALE: u32 = 1;
/// `FT_LOAD_NO_HINTING`.
const NO_HINTING: u32 = 1 << 1;
/// `FT_LOAD_FORCE_AUTOHINT`.
const FORCE_AUTOHINT: u32 = 1 << 5;
/// `FT_LOAD_NO_AUTOHINT`.
const NO_AUTOHINT: u32 = 1 << 15;

/// The hinting `FreeType` applies for load flags.
pub(super) fn hinting(flags: u32) -> Hinting {
    if flags & NO_HINTING != 0 {
        Hinting::None
    } else if flags & FORCE_AUTOHINT != 0 {
        Hinting::Auto
    } else if flags & NO_AUTOHINT != 0 {
        Hinting::Native
    } else {
        Hinting::Default
    }
}

/// The load flag selecting `hinting` (the inverse of [`hinting`]).
const fn load_flag(hinting: Hinting) -> u32 {
    match hinting {
        Hinting::None => NO_HINTING,
        Hinting::Auto => FORCE_AUTOHINT,
        Hinting::Native => NO_AUTOHINT,
        Hinting::Default => 0,
    }
}

/// The load flag of the hinting `text.hinting` selects (matplotlib's
/// `backend_agg.get_hinting_flag`).
fn hinting_flag() -> PyResult<u32> {
    Ok(load_flag(pltrs::rc::params().text_hinting.hinting()))
}

/// The size a glyph is loaded at, which it keeps: `FreeType`'s glyph
/// holds the outline scaled when it was loaded, whatever size the font
/// has since.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum LoadSize {
    /// Unscaled, in font units (`FT_LOAD_NO_SCALE`, or a face with no
    /// size yet: "undefined scale means no scale").
    Unscaled,
    /// At `points` and `dpi` (`set_size`).
    Scaled { points: f64, dpi: f64 },
}

impl LoadSize {
    /// The size glyphs of `face` load at: font units as 26.6 pixels
    /// (`units_per_EM / 64` points at 72 dpi) when unscaled.
    pub(super) fn of(self, face: &Face<'_>) -> Size {
        match self {
            Self::Unscaled => Size::new(face, f64::from(face.units_per_em()) / 64.0, 72.0),
            Self::Scaled { points, dpi } => Size::new(face, points, dpi),
        }
    }
}

/// A glyph loaded into the `FT2Font`'s list.
#[derive(Clone, Copy, Debug)]
pub(super) struct Loaded {
    /// The face: 0 for the font itself, `i` for its fallback `i - 1`.
    pub(super) face: usize,
    pub(super) glyph: GlyphId,
    /// The size it was loaded at.
    pub(super) size: LoadSize,
    pub(super) hinting: Hinting,
    /// Where `set_text` placed it (26.6); the origin otherwise.
    pub(super) pen: (i64, i64),
    /// The rotation `set_text` gave it, in degrees.
    pub(super) angle: f64,
    /// Whether its bitmap was rendered antialiased, once drawn:
    /// matplotlib's drawing turns `FreeType`'s glyph into its bitmap
    /// (`FT_Glyph_To_Bitmap` destroying the outline), which every later
    /// drawing reuses whatever antialiasing it asks for.
    pub(super) rendered: Option<bool>,
}

impl Loaded {
    /// A glyph of `face` loaded at `size`, not drawn yet.
    pub(super) fn new(
        face: usize,
        glyph: GlyphId,
        (size, hinting): (LoadSize, Hinting),
        pen: (i64, i64),
        angle: f64,
    ) -> Self {
        Self {
            face,
            glyph,
            size,
            hinting,
            pen,
            angle,
            rendered: None,
        }
    }
}

/// The extent `set_text` computed: the union of the glyph boxes and the
/// advance, in 26.6 pixels.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TextBox {
    pub(super) bbox: (i64, i64, i64, i64),
    pub(super) advance: i64,
}

/// What an `FT2Font` has loaded.
#[derive(Debug, Default)]
pub(super) struct Glyphs {
    /// Every glyph loaded since the last `clear`.
    pub(super) loaded: Vec<Loaded>,
    /// The glyph slot of the font's own face: the last glyph loaded from
    /// it, for `get_path` (a glyph loaded from a fallback goes to that
    /// fallback's face, as `FreeType` loads into the face it comes from).
    slot: Option<Loaded>,
    pub(super) text: TextBox,
    /// The bitmap `draw_glyphs_to_bitmap` drew: `(rows, columns, pixels)`.
    pub(super) image: (usize, usize, Vec<u8>),
    /// The face of the chain each character `load_char` loaded came from
    /// (matplotlib's `char_to_font`), for `get_char_index`.
    pub(super) char_faces: HashMap<u32, usize>,
}

impl Glyphs {
    /// Makes `glyph` the one in the glyph slot if it comes from the font's
    /// own face.
    pub(super) fn load_into_slot(&mut self, glyph: Loaded) {
        if glyph.face == 0 {
            self.slot = Some(glyph);
        }
    }

    /// Forgets the loaded glyphs, the text's extent and the faces the
    /// characters came from (matplotlib's `FT2Font::clear`, which leaves
    /// the glyph slot and the image).
    pub(super) fn clear(&mut self) {
        self.loaded.clear();
        self.text = TextBox::default();
        self.char_faces.clear();
    }
}

impl Ft2Font {
    /// The face `index` of the font's chain (0 itself, then fallbacks).
    pub(super) fn chain_face(&self, index: usize) -> PyResult<Face<'_>> {
        match index {
            0 => self.face(),
            _ => self.fallbacks[index - 1].face(),
        }
    }

    /// The data of the face `index` of the font's chain.
    pub(super) fn chain_data(&self, index: usize) -> &FaceData {
        match index {
            0 => &self.face,
            _ => &self.fallbacks[index - 1],
        }
    }

    /// `glyph` of `face`, the face `index` of the font's chain, loaded at
    /// `size` as `FT_Load_Glyph` loads it for `hinting`: grid-fitted, or
    /// `None` when `FreeType` leaves it unhinted. The autohinter analyses
    /// each face once, as `FreeType` does.
    pub(super) fn load_hinted(
        &self,
        index: usize,
        face: &Face<'_>,
        size: &Size,
        glyph: GlyphId,
        hinting: Hinting,
    ) -> PyResult<Option<HintedGlyph>> {
        let loaded = match hinting.hinter(face, true) {
            Hinter::Bytecode => HintingInstance::new(face, size).load_glyph(face, glyph),
            Hinter::Autohinter => self.chain_data(index).autohinter(face).load_glyph(
                face,
                size,
                glyph,
                AutohintTarget::Normal,
            ),
            Hinter::Cff => face.load_cff_hinted(size, glyph),
            Hinter::Unhinted => return Ok(None),
        };
        loaded.map(Some).map_err(|err| ft_error("load glyph", &err))
    }

    /// Every face of the chain.
    pub(super) fn chain_faces(&self) -> PyResult<Vec<Face<'_>>> {
        (0..=self.fallbacks.len())
            .map(|index| self.chain_face(index))
            .collect()
    }
}

/// A glyph's metrics as `Glyph` reports them: `((width, height,
/// horiBearingX, horiBearingY), (horiAdvance, linearHoriAdvance),
/// (vertBearingX, vertBearingY, vertAdvance), bbox)`, in 26.6 pixels
/// (16.16 for the linear advance).
type Metrics = (
    (i64, i64, i64, i64),
    (i64, i64),
    (i64, i64, i64),
    (i64, i64, i64, i64),
);

/// The control box of a glyph's outline, `None` for a glyph without one
/// (a space).
pub(super) fn control_box(
    face: &Face<'_>,
    size: &Size,
    glyph: GlyphId,
    hinted: Option<&HintedGlyph>,
) -> PyResult<Option<Rect>> {
    match hinted {
        Some(hinted) => Ok(hinted.control_box()),
        None => size
            .glyph_cbox(face, glyph)
            .map_err(|err| ft_error("load glyph", &err)),
    }
}

/// The metrics `FreeType` gives `glyph` of `face`, the face `index` of
/// `font`'s chain, loaded for `hinting`.
fn metrics(
    font: &Ft2Font,
    (index, face): (usize, &Face<'_>),
    size: &Size,
    glyph: GlyphId,
    hinting: Hinting,
) -> PyResult<Metrics> {
    let hinted = font.load_hinted(index, face, size, glyph, hinting)?;
    let cbox = control_box(face, size, glyph, hinted.as_ref())?.unwrap_or_default();
    let (x0, y0, x1, y1) = (
        i64::from(cbox.x_min),
        i64::from(cbox.y_min),
        i64::from(cbox.x_max),
        i64::from(cbox.y_max),
    );
    let horizontal = match &hinted {
        Some(hinted) => {
            let m = hinted.metrics();
            (
                i64::from(m.width),
                i64::from(m.height),
                i64::from(m.hori_bearing_x),
                i64::from(m.hori_bearing_y),
                i64::from(m.hori_advance),
            )
        }
        None => (
            x1 - x0,
            y1 - y0,
            x0,
            y1,
            size.scale_x(i64::from(face.glyph_advance(glyph))),
        ),
    };
    let (width, height, bearing_x, bearing_y, advance) = horizontal;
    let vertical = vertical_metrics(face, size, glyph, (bearing_x, advance), hinted.as_ref());
    Ok((
        (width, height, bearing_x, bearing_y),
        (advance, size.linear_advance(face, glyph)),
        vertical,
        (x0, y0, x1, y1),
    ))
}

/// The vertical metrics `FreeType` gives a glyph: the autohinter's own
/// for an autohinted glyph, else those its TrueType loader makes up for a
/// face without them (`compute_glyph_metrics`), grid-fitted when the
/// glyph is hinted (`ft_glyphslot_grid_fit_metrics`).
fn vertical_metrics(
    face: &Face<'_>,
    size: &Size,
    glyph: GlyphId,
    (bearing_x, advance): (i64, i64),
    hinted: Option<&HintedGlyph>,
) -> (i64, i64, i64) {
    if let Some(vertical) = hinted.and_then(HintedGlyph::vertical_metrics) {
        return (
            i64::from(vertical.bearing_x),
            i64::from(vertical.bearing_y),
            i64::from(vertical.advance),
        );
    }
    let (top, advance_units) = face.synthesized_vertical_metrics(glyph);
    let top = size.scale_y(top);
    let bearing = bearing_x - advance / 2;
    let vert_advance = size.scale_y(advance_units);
    if hinted.is_some() {
        let floor = |v: i64| v & !63;
        (floor(bearing), floor(top), (vert_advance + 32) & !63)
    } else {
        (bearing, top, vert_advance)
    }
}

/// `load_char(charcode, flags)` and `load_glyph(index, flags)`: loads a
/// glyph of the font (a fallback's when the font lacks the character),
/// appends it to the loaded glyphs and returns `(metrics, its index in
/// them, the face it came from)`.
fn load(
    font: &mut Ft2Font,
    face_index: usize,
    glyph: GlyphId,
    flags: u32,
) -> PyResult<(Metrics, usize, usize)> {
    let (size, hinting) = font.loading(flags);
    let metrics = {
        let face = font.chain_face(face_index)?;
        let mut metrics = metrics(font, (face_index, &face), &size.of(&face), glyph, hinting)?;
        metrics.1.1 = font.linear_advance(&face, glyph);
        metrics
    };
    let loaded = Loaded::new(face_index, glyph, (size, hinting), (0, 0), 0.0);
    font.glyphs.loaded.push(loaded);
    font.glyphs.load_into_slot(loaded);
    Ok((metrics, font.glyphs.loaded.len() - 1, face_index))
}

/// `load_char(charcode, flags)`: `None` when no face of the chain has
/// the character.
fn load_char(
    font: Native<Ft2Font>,
    code: u32,
    flags: u32,
) -> PyResult<Option<(Metrics, usize, usize)>> {
    let mut font = font.borrow_mut();
    let found = (0..=font.fallbacks.len()).find_map(|index| {
        let face = font.chain_face(index).ok()?;
        face.glyph_index_for_code(code)
            .filter(|&glyph| glyph != GlyphId::NOTDEF)
            .map(|glyph| (index, glyph))
    });
    let Some((index, glyph)) = found else {
        return Ok(None);
    };
    font.glyphs.char_faces.insert(code, index);
    load(&mut font, index, glyph, flags).map(Some)
}

/// `load_glyph(index, flags)`.
fn load_glyph(font: Native<Ft2Font>, glyph: u16, flags: u32) -> PyResult<(Metrics, usize, usize)> {
    let mut font = font.borrow_mut();
    {
        let face = font.face()?;
        if glyph >= face.num_glyphs() {
            return Err(PyErr::runtime_error(
                "In FT2Font: Can not load glyph (invalid glyph index; error code 0x10)",
            ));
        }
    }
    load(&mut font, 0, GlyphId(glyph), flags)
}

/// `get_num_glyphs()`.
fn get_num_glyphs(font: Native<Ft2Font>) -> PyResult<usize> {
    Ok(font.borrow().glyphs.loaded.len())
}

/// `clear()`: forgets the loaded glyphs and the text.
fn clear(font: Native<Ft2Font>) -> PyResult<()> {
    font.borrow_mut().glyphs.clear();
    Ok(())
}

impl Ft2Font {
    /// The outline of `glyph` as `FreeType` loads it, untransformed, in
    /// pixels.
    pub(super) fn outline(&self, glyph: &Loaded) -> PyResult<pltrs_core::Path> {
        let face = self.chain_face(glyph.face)?;
        let size = glyph.size.of(&face);
        let mut sink = PathSink::default();
        match self.load_hinted(glyph.face, &face, &size, glyph.glyph, glyph.hinting)? {
            Some(hinted) => hinted.outline(&mut sink),
            None => size
                .outline(&face, glyph.glyph, &mut sink)
                .map_err(|err| ft_error("load glyph", &err))?,
        }
        Ok(sink.finish())
    }
}

/// `get_path()`: the outline of the glyph in the font's glyph slot (the
/// last glyph `load_char`, `load_glyph` or `set_text` loaded from its own
/// face), `(vertices, codes)` with matplotlib's path codes, empty before
/// any.
fn get_path(font: Native<Ft2Font>) -> PyResult<(NdArray, NdArray<u8>)> {
    let font = font.borrow();
    let Some(glyph) = font.glyphs.slot else {
        return Ok((
            NdArray::with_shape(Vec::new(), vec![0, 2])?,
            NdArray::from_vec(Vec::new()),
        ));
    };
    let path = font.outline(&glyph)?;
    let vertices: Vec<f64> = path.vertices().iter().flat_map(|p| [p.x, p.y]).collect();
    let codes = path.codes().map_or_else(Vec::new, |codes| {
        codes
            .iter()
            .map(|code| match code {
                PathCode::MoveTo => 1,
                PathCode::LineTo => 2,
                PathCode::Curve3 => 3,
                PathCode::Curve4 => 4,
                PathCode::ClosePoly => 79,
            })
            .collect()
    });
    let count = path.vertices().len();
    Ok((
        NdArray::with_shape(vertices, vec![count, 2])?,
        NdArray::from_vec(codes),
    ))
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("load_char", load_char)?;
    module.add("load_glyph", load_glyph)?;
    module.add("get_num_glyphs", get_num_glyphs)?;
    module.add("clear", clear)?;
    module.add("get_path", get_path)?;
    module.add("hinting_flag", hinting_flag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_flags_select_the_hinting_they_name() {
        for hinting in [
            Hinting::None,
            Hinting::Default,
            Hinting::Native,
            Hinting::Auto,
        ] {
            assert_eq!(super::hinting(load_flag(hinting)), hinting);
        }
        assert_eq!(load_flag(Hinting::Default), 0);
    }
}
