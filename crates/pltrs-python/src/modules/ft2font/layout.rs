//! Text laid out by an `FT2Font`: `set_text` and the extent it computes
//! (`get_width_height`, `get_descent`, `get_bitmap_offset`), and
//! `_layout`, as matplotlib's `ft2font.cpp` lays text out with libraqm.

use pltrs_font::{FontChain, ShapeOptions};

use super::glyphs::{Loaded, TextBox, control_box, load_hinted};
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

/// `set_text(string, angle, flags, features, language)`: lays the text
/// out with the chain's faces, appends its glyphs and returns their pen
/// positions (26.6, before rotation). The text's box and advance are
/// those of the glyphs rotated by `angle` degrees.
fn set_text(
    font: Native<Ft2Font>,
    text: String,
    angle: f64,
    flags: u32,
    shaping: Shaping,
) -> PyResult<Vec<(i64, i64)>> {
    let mut font = font.borrow_mut();
    let hinting = font.hinting(flags);
    let options = shape_options(shaping)?;
    let faces = font.chain_faces()?;
    let (ptsize, dpi) = font.points_and_dpi(&faces[0]);
    let layout = FontChain::new(faces.clone())
        .layout(&text, ptsize, dpi, &options, hinting)
        .map_err(|err| ft_error("lay out text", &err))?;
    let matrix = Rotation::new(angle);
    let mut bbox: Option<(i64, i64, i64, i64)> = None;
    let mut placed = Vec::with_capacity(layout.glyphs.len());
    for glyph in &layout.glyphs {
        let face = &faces[glyph.face];
        let size = font.size_of(face);
        let hinted = load_hinted(face, &size, glyph.glyph_id, hinting)?;
        let cbox = control_box(face, &size, glyph.glyph_id, hinted.as_ref())?;
        let pen = (i64::from(glyph.x), i64::from(glyph.y));
        let corners = [
            (cbox.x_min, cbox.y_min),
            (cbox.x_max, cbox.y_min),
            (cbox.x_min, cbox.y_max),
            (cbox.x_max, cbox.y_max),
        ]
        .map(|(x, y)| matrix.apply((i64::from(x) + pen.0, i64::from(y) + pen.1)));
        for (x, y) in corners {
            bbox = Some(match bbox {
                None => (x, y, x, y),
                Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
            });
        }
        placed.push(Loaded {
            face: glyph.face,
            glyph: glyph.glyph_id,
            hinting,
            pen,
            angle,
        });
    }
    font.glyphs.text = TextBox {
        bbox: bbox.unwrap_or_default(),
        advance: matrix.apply((i64::from(layout.width), 0)).0,
    };
    let xys = placed.iter().map(|glyph| glyph.pen).collect();
    font.glyphs.loaded.extend(placed);
    Ok(xys)
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
    let font = font.borrow();
    let options = shape_options(shaping)?;
    let faces = font.chain_faces()?;
    let (ptsize, dpi) = font.points_and_dpi(&faces[0]);
    let layout = FontChain::new(faces.clone())
        .layout(&text, ptsize, dpi, &options, font.hinting(flags))
        .map_err(|err| ft_error("lay out text", &err))?;
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
            let linear = font.size_of(face).linear_advance(face, glyph.glyph_id);
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
