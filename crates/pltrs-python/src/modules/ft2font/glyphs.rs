//! Glyphs of an `FT2Font`: loading them (`load_char`, `load_glyph`) with
//! their metrics, and the outline of the glyph last loaded (`get_path`),
//! as matplotlib's `ft2font.cpp` does them with `FreeType`. What the font
//! has loaded is kept here for the text layout ([`super::layout`]) and the
//! bitmaps ([`super::bitmaps`]).

use pltrs_core::PathCode;
use pltrs_font::{
    AutohintTarget, Autohinter, Face, GlyphId, HintedGlyph, Hinter, Hinting, HintingInstance, Rect,
    Size,
};
use pltrs_text::PathSink;

use super::{Ft2Font, ft_error};
use crate::binding::prelude::*;

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

/// A glyph loaded into the `FT2Font`'s list.
#[derive(Clone, Copy, Debug)]
pub(super) struct Loaded {
    /// The face: 0 for the font itself, `i` for its fallback `i - 1`.
    pub(super) face: usize,
    pub(super) glyph: GlyphId,
    pub(super) hinting: Hinting,
    /// Where `set_text` placed it (26.6); the origin otherwise.
    pub(super) pen: (i64, i64),
    /// The rotation `set_text` gave it, in degrees.
    pub(super) angle: f64,
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
    /// The glyph slot: the last glyph loaded, for `get_path`.
    slot: Option<Loaded>,
    pub(super) text: TextBox,
    /// The bitmap `draw_glyphs_to_bitmap` drew: `(rows, columns, pixels)`.
    pub(super) image: (usize, usize, Vec<u8>),
}

impl Ft2Font {
    /// The face `index` of the font's chain (0 itself, then fallbacks).
    pub(super) fn chain_face(&self, index: usize) -> PyResult<Face<'_>> {
        match index {
            0 => self.face(),
            _ => self.fallbacks[index - 1].face(),
        }
    }

    /// Every face of the chain.
    pub(super) fn chain_faces(&self) -> PyResult<Vec<Face<'_>>> {
        (0..=self.fallbacks.len())
            .map(|index| self.chain_face(index))
            .collect()
    }
}

/// `glyph` of `face` loaded as `FT_Load_Glyph` loads it for `hinting`:
/// grid-fitted, or `None` when `FreeType` leaves it unhinted.
pub(super) fn load_hinted(
    face: &Face<'_>,
    size: &Size,
    glyph: GlyphId,
    hinting: Hinting,
) -> PyResult<Option<HintedGlyph>> {
    let loaded = match hinting.hinter(face, true) {
        Hinter::Bytecode => HintingInstance::new(face, size).load_glyph(face, glyph),
        Hinter::Autohinter => {
            Autohinter::new(face).load_glyph(face, size, glyph, AutohintTarget::Normal)
        }
        Hinter::Unhinted => return Ok(None),
    };
    loaded.map(Some).map_err(|err| ft_error("load glyph", &err))
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

/// The metrics `FreeType` gives `glyph` loaded for `hinting`.
fn metrics(face: &Face<'_>, size: &Size, glyph: GlyphId, hinting: Hinting) -> PyResult<Metrics> {
    let hinted = load_hinted(face, size, glyph, hinting)?;
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
    let hinting = font.hinting(flags);
    let metrics = {
        let face = font.chain_face(face_index)?;
        let mut metrics = metrics(&face, &font.size_of(&face), glyph, hinting)?;
        metrics.1.1 = font.linear_advance(&face, glyph);
        metrics
    };
    let loaded = Loaded {
        face: face_index,
        glyph,
        hinting,
        pen: (0, 0),
        angle: 0.0,
    };
    font.glyphs.loaded.push(loaded);
    font.glyphs.slot = Some(loaded);
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
    found
        .map(|(index, glyph)| load(&mut font, index, glyph, flags))
        .transpose()
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
    font.borrow_mut().glyphs = Glyphs::default();
    Ok(())
}

/// `get_path()`: the outline of the glyph last loaded, `(vertices,
/// codes)` with matplotlib's path codes, empty before any.
fn get_path(font: Native<Ft2Font>) -> PyResult<(NdArray, NdArray<u8>)> {
    let font = font.borrow();
    let Some(glyph) = font.glyphs.slot else {
        return Ok((
            NdArray::with_shape(Vec::new(), vec![0, 2])?,
            NdArray::from_vec(Vec::new()),
        ));
    };
    let face = font.chain_face(glyph.face)?;
    let size = font.size_of(&face);
    let mut sink = PathSink::default();
    match load_hinted(&face, &size, glyph.glyph, glyph.hinting)? {
        Some(hinted) => hinted.outline(&mut sink),
        None => size
            .outline(&face, glyph.glyph, &mut sink)
            .map_err(|err| ft_error("load glyph", &err))?,
    }
    let path = sink.finish();
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
