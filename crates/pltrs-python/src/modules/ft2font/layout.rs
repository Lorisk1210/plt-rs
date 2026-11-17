//! Text laid out by an `FT2Font`: `set_text` and the extent it computes
//! (`get_width_height`, `get_descent`, `get_bitmap_offset`), and
//! `_layout`, as matplotlib's `ft2font.cpp` lays text out with libraqm.

use pltrs_core::{Path, PathCode};
use pltrs_font::{Face, FontChain, GlyphId, ShapeOptions, TextLayout, UnscaledAdvances};

use super::glyphs::{LoadSize, Loaded, NO_SCALE, TextBox, control_box};
use super::{Ft2Font, ft_error};
use crate::binding::prelude::*;

/// `set_text`'s `features` and `language` keywords: the feature strings,
/// and the language ranges `(tag, start, end)`.
type Shaping = (Option<Vec<String>>, Vec<(String, usize, usize)>);

/// The shaping options of `set_text`'s keywords.
fn shape_options((features, languages): Shaping) -> PyResult<ShapeOptions> {
    let features = features
        .unwrap_or_default()
        .iter()
        .map(|feature| {
            feature
                .parse()
                .map_err(|err: pltrs_font::ParseFeatureError| PyErr::value_error(err.to_string()))
        })
        .collect::<PyResult<_>>()?;
    let languages = languages
        .into_iter()
        .map(|(tag, start, end)| pltrs_font::LanguageRange::new(tag.as_str(), start, end))
        .collect();
    Ok(ShapeOptions {
        features,
        languages,
        ..ShapeOptions::default()
    })
}

/// The layout `set_text` and `_layout` give `text` with the chain's
/// `faces` for load `flags`: at the font's size, or unscaled before any
/// `set_size` or with `FT_LOAD_NO_SCALE` (see
/// [`FontChain::layout_unscaled`]: no kerning then, as `FreeType`'s
/// `HarfBuzz` font has a scale of zero).
fn chain_layout(
    font: &Ft2Font,
    faces: &[Face<'_>],
    text: &str,
    options: &ShapeOptions,
    flags: u32,
) -> PyResult<TextLayout> {
    let (load_size, hinting) = font.loading(flags);
    // The analyses of the faces the font autohinted glyphs of so far.
    let analyses = (0..faces.len())
        .map(|index| font.chain_data(index).autohinter.get())
        .collect();
    let chain = FontChain::new(faces.to_vec()).with_autohinters(analyses);
    let layout = match load_size {
        LoadSize::Unscaled => chain.layout_unscaled(text, options, unscaled_advances(flags)),
        LoadSize::Scaled { points, dpi } => chain.layout(text, points, dpi, options, hinting),
    };
    layout.map_err(|err| ft_error("lay out text", &err))
}

/// The advances `FreeType` gives `HarfBuzz` for glyphs loaded unscaled
/// with load `flags` (`FT_Get_Advance`): its fast path, taken without
/// hinting or for the light target, scales font units by the scale of
/// zero; otherwise it loads the glyph, unscaled.
fn unscaled_advances(flags: u32) -> UnscaledAdvances {
    /// `FT_LOAD_NO_HINTING`.
    const NO_HINTING: u32 = 1 << 1;
    /// `FT_RENDER_MODE_LIGHT`, the load target `FT_LOAD_TARGET_LIGHT`
    /// selects (bits 16 to 19).
    const LIGHT: u32 = 1;
    if flags & NO_SCALE != 0 {
        UnscaledAdvances::NoScale
    } else if flags & NO_HINTING != 0 || (flags >> 16) & 15 == LIGHT {
        UnscaledAdvances::Zero
    } else {
        UnscaledAdvances::FontUnits
    }
}

/// The part of `text` matplotlib's layout (libraqm) shapes: up to the
/// first paragraph separator (a line feed, a carriage return, the
/// information separators 1C to 1E, NEL, U+2029), which, with all after
/// it, it shapes as one cluster of the separator's glyph (of a carriage
/// return and line feed, the line feed's).
fn first_paragraph(text: &str) -> &str {
    let is_separator =
        |c: char| matches!(c, '\n' | '\r' | '\u{1c}'..='\u{1e}' | '\u{85}' | '\u{2029}');
    let Some((start, separator)) = text.char_indices().find(|&(_, c)| is_separator(c)) else {
        return text;
    };
    let start = if separator == '\r' && text[start + 1..].starts_with('\n') {
        start + 1
    } else {
        start
    };
    let end = start + text[start..].chars().next().map_or(0, char::len_utf8);
    &text[..end]
}

/// A glyph `set_text` laid out, for the warnings about characters no font
/// has: the character of its cluster and the face it comes from (`None`
/// for a character no face has).
type LaidOutChar = (u32, Option<usize>);

/// What `set_text` returns: the pen positions of the glyphs placed, and
/// every glyph laid out (see [`LaidOutChar`]).
type SetText = (Vec<(i64, i64)>, Vec<LaidOutChar>);

/// `set_text(string, angle, flags, features, language)`: lays the text
/// out with the chain's faces, makes its glyphs the loaded ones and
/// returns their pen positions (26.6, before rotation), with the character
/// and face of every glyph laid out. A character no face has is passed
/// over, the pen staying where it is, as matplotlib's `set_text` passes
/// over a glyph of index 0. The text's box and advance are those of the
/// glyphs rotated by `angle` degrees.
fn set_text(
    font: Native<Ft2Font>,
    text: String,
    angle: f64,
    flags: u32,
    shaping: Shaping,
) -> PyResult<SetText> {
    let mut font = font.borrow_mut();
    // matplotlib's layout starts by clearing what the font loaded.
    font.glyphs.clear();
    let (load_size, hinting) = font.loading(flags);
    let options = shape_options(shaping)?;
    let faces = font.chain_faces()?;
    let layout = chain_layout(&font, &faces, first_paragraph(&text), &options, flags)?;
    let matrix = Rotation::new(angle);
    let mut bbox: Option<(i64, i64, i64, i64)> = None;
    let mut placed = Vec::with_capacity(layout.glyphs.len());
    let mut chars = Vec::with_capacity(layout.glyphs.len());
    let mut passed_over = 0i64;
    for glyph in &layout.glyphs {
        let code = text[glyph.cluster..].chars().next().map_or(0, u32::from);
        if glyph.glyph_id == GlyphId::NOTDEF {
            chars.push((code, None));
            passed_over += i64::from(glyph.x_advance);
            continue;
        }
        chars.push((code, Some(glyph.face)));
        let face = &faces[glyph.face];
        let size = load_size.of(face);
        let pen = (i64::from(glyph.x) - passed_over, i64::from(glyph.y));
        let loaded = Loaded::new(glyph.face, glyph.glyph_id, (load_size, hinting), pen, angle);
        let extent = if matrix.is_identity() {
            let hinted = font.load_hinted(glyph.face, face, &size, glyph.glyph_id, hinting)?;
            control_box(face, &size, glyph.glyph_id, hinted.as_ref())?.map(|cbox| {
                let (x0, y0) = (i64::from(cbox.x_min) + pen.0, i64::from(cbox.y_min) + pen.1);
                let (x1, y1) = (i64::from(cbox.x_max) + pen.0, i64::from(cbox.y_max) + pen.1);
                (x0, y0, x1, y1)
            })
        } else {
            rotated_control_box(&font.outline(&loaded)?, pen, &matrix)
        };
        // A glyph without an outline stays at the origin, which moving
        // its outline of no points leaves its box at.
        let (x0, y0, x1, y1) = extent.unwrap_or_default();
        bbox = Some(match bbox {
            None => (x0, y0, x1, y1),
            Some((a, b, c, d)) => (a.min(x0), b.min(y0), c.max(x1), d.max(y1)),
        });
        placed.push(loaded);
    }
    let text = TextBox {
        bbox: bbox.unwrap_or_default(),
        advance: matrix.apply((i64::from(layout.width), 0)).0,
    };
    let xys = placed.iter().map(|glyph| glyph.pen).collect();
    font.glyphs.text = text;
    // Each glyph is loaded into the slot of the face it comes from.
    for &glyph in &placed {
        font.glyphs.load_into_slot(glyph);
    }
    font.glyphs.loaded = placed;
    Ok((xys, chars))
}

/// The control box (26.6) of `outline` moved to `pen` and turned by
/// `matrix`, as `FT_Glyph_Transform` then `FT_Glyph_Get_CBox` give it:
/// the extremes of its points, `None` without any.
fn rotated_control_box(
    outline: &Path,
    pen: (i64, i64),
    matrix: &Rotation,
) -> Option<(i64, i64, i64, i64)> {
    let codes = outline.codes();
    outline
        .vertices()
        .iter()
        .enumerate()
        // A closing code's vertex is no point of the outline.
        .filter(|&(index, _)| codes.is_none_or(|codes| codes[index] != PathCode::ClosePoly))
        .map(|(_, point)| {
            // The outline's points are whole 26.6 units.
            let to_units = |v: f64| (v * 64.0).round() as i64;
            matrix.apply((to_units(point.x) + pen.0, to_units(point.y) + pen.1))
        })
        .fold(None, |extent, (x, y)| {
            Some(match extent {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            })
        })
}

/// The rotation matrix `set_text` applies, in 16.16 as `FreeType` keeps it.
struct Rotation {
    cos: i64,
    sin: i64,
}

impl Rotation {
    /// The rotation by `angle` degrees (truncated to 16.16, as matplotlib
    /// casts it).
    fn new(angle: f64) -> Self {
        let (sin, cos) = angle.to_radians().sin_cos();
        Self {
            cos: (cos * 65536.0) as i64,
            sin: (sin * 65536.0) as i64,
        }
    }

    /// Whether the rotation leaves vectors as they are (no angle).
    fn is_identity(&self) -> bool {
        self.cos == 65536 && self.sin == 0
    }

    /// `FT_Vector_Transform` of a 26.6 vector.
    fn apply(&self, (x, y): (i64, i64)) -> (i64, i64) {
        let mul_fix = |a: i64, b: i64| {
            let product = a * b;
            (product + 0x8000 - i64::from(product < 0)) >> 16
        };
        (
            mul_fix(x, self.cos) - mul_fix(y, self.sin),
            mul_fix(x, self.sin) + mul_fix(y, self.cos),
        )
    }
}

/// `get_width_height()`: the advance and height of the text, 26.6.
fn get_width_height(font: Native<Ft2Font>) -> PyResult<(i64, i64)> {
    let text = font.borrow().glyphs.text;
    Ok((text.advance, text.bbox.3 - text.bbox.1))
}

/// `get_descent()`: how far the text reaches below the baseline, 26.6.
fn get_descent(font: Native<Ft2Font>) -> PyResult<i64> {
    Ok(-font.borrow().glyphs.text.bbox.1)
}

/// `get_bitmap_offset()`: the left edge of the text, 26.6.
fn get_bitmap_offset(font: Native<Ft2Font>) -> PyResult<(i64, i64)> {
    Ok((font.borrow().glyphs.text.bbox.0, 0))
}

/// A glyph of `_layout`: `((face, first character, end character),
/// glyph, (x, y), previous kerning)`, positions in pixels, the face 0 for
/// the font and `i` for its fallback `i - 1`.
type LayoutGlyph = ((usize, usize, usize), u16, (f64, f64), f64);

/// `_layout(text, flags, features, language)`: each glyph of the text
/// with the characters of its cluster, its position and the kerning
/// before it (how far it is from the previous glyph beyond that glyph's
/// linear advance).
fn layout(
    font: Native<Ft2Font>,
    text: String,
    flags: u32,
    shaping: Shaping,
) -> PyResult<Vec<LayoutGlyph>> {
    // matplotlib's layout starts by clearing what the font loaded.
    font.borrow_mut().glyphs.clear();
    let font = font.borrow();
    let options = shape_options(shaping)?;
    let faces = font.chain_faces()?;
    // The last cluster reaches the end of the text (see `first_paragraph`).
    let layout = chain_layout(&font, &faces, first_paragraph(&text), &options, flags)?;
    let mut clusters: Vec<usize> = layout.glyphs.iter().map(|glyph| glyph.cluster).collect();
    clusters.sort_unstable();
    clusters.dedup();
    let chars = |byte: usize| text[..byte].chars().count();
    let mut previous: Option<(i32, f64)> = None;
    Ok(layout
        .glyphs
        .iter()
        .map(|glyph| {
            let end = clusters
                .iter()
                .find(|&&cluster| cluster > glyph.cluster)
                .copied()
                .unwrap_or(text.len());
            let kern = previous.map_or(0.0, |(x, advance)| f64::from(glyph.x - x) - advance);
            let face = &faces[glyph.face];
            let linear = font.linear_advance(face, glyph.glyph_id);
            previous = Some((glyph.x, linear as f64 / 1024.0));
            (
                (glyph.face, chars(glyph.cluster), chars(end)),
                glyph.glyph_id.0,
                (f64::from(glyph.x) / 64.0, f64::from(glyph.y) / 64.0),
                kern / 64.0,
            )
        })
        .collect())
}

/// Registers the module's functions.
pub(super) fn register(module: &Module<'_>) -> PyResult<()> {
    module.add("set_text", set_text)?;
    module.add("get_width_height", get_width_height)?;
    module.add("get_descent", get_descent)?;
    module.add("get_bitmap_offset", get_bitmap_offset)?;
    module.add("layout", layout)
}
