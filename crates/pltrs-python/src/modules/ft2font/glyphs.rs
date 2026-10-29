//! Glyphs of an `FT2Font`: loading them (`load_char`, `load_glyph`),
//! laying out text (`set_text`, `_layout`), their outlines (`get_path`)
//! and bitmaps (`draw_glyphs_to_bitmap`, `draw_glyph_to_bitmap`,
//! `_render_glyph`), as matplotlib's `ft2font.cpp` does them with
//! `FreeType`.

use pltrs_core::{PathCode, Point};
use pltrs_font::{
    AutohintTarget, Autohinter, Face, FontChain, GlyphId, HintedGlyph, Hinter, Hinting,
    HintingInstance, Rect, ShapeOptions, Size,
};
use pltrs_render::{GlyphBitmap, GlyphShape, render_glyph};
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
fn hinting(flags: u32) -> Hinting {
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

/// A glyph loaded into the `FT2Font`'s list.
#[derive(Clone, Copy, Debug)]
struct Loaded {
    /// The face: 0 for the font itself, `i` for its fallback `i - 1`.
    face: usize,
    glyph: GlyphId,
    hinting: Hinting,
    /// Where `set_text` placed it (26.6); the origin otherwise.
    pen: (i64, i64),
    /// The rotation `set_text` gave it, in degrees.
    angle: f64,
}

/// The extent `set_text` computed: the union of the glyph boxes and the
/// advance, in 26.6 pixels.
#[derive(Clone, Copy, Debug, Default)]
struct TextBox {
    bbox: (i64, i64, i64, i64),
    advance: i64,
}

/// What an `FT2Font` has loaded.
#[derive(Debug, Default)]
pub(super) struct Glyphs {
    /// Every glyph loaded since the last `clear`.
    loaded: Vec<Loaded>,
    /// The glyph slot: the last glyph loaded, for `get_path`.
    slot: Option<Loaded>,
    text: TextBox,
    /// The bitmap `draw_glyphs_to_bitmap` drew: `(rows, columns, pixels)`.
    image: (usize, usize, Vec<u8>),
}

impl Ft2Font {
    /// The face `index` of the font's chain (0 itself, then fallbacks).
    fn chain_face(&self, index: usize) -> PyResult<Face<'_>> {
        match index {
            0 => self.face(),
            _ => self.fallbacks[index - 1].face(),
        }
    }

    /// Every face of the chain.
    fn chain_faces(&self) -> PyResult<Vec<Face<'_>>> {
        (0..=self.fallbacks.len())
            .map(|index| self.chain_face(index))
            .collect()
    }
}

/// `glyph` of `face` loaded as `FT_Load_Glyph` loads it for `hinting`:
/// grid-fitted, or `None` when `FreeType` leaves it unhinted.
fn load_hinted(
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

/// The control box of a glyph's outline.
fn control_box(
    face: &Face<'_>,
    size: &Size,
    glyph: GlyphId,
    hinted: Option<&HintedGlyph>,
) -> PyResult<Rect> {
    let cbox = match hinted {
        Some(hinted) => hinted.control_box(),
        None => size
            .glyph_cbox(face, glyph)
            .map_err(|err| ft_error("load glyph", &err))?,
    };
    Ok(cbox.unwrap_or_default())
}

/// The metrics `FreeType` gives `glyph` loaded for `hinting`.
fn metrics(face: &Face<'_>, size: &Size, glyph: GlyphId, hinting: Hinting) -> PyResult<Metrics> {
    let hinted = load_hinted(face, size, glyph, hinting)?;
    let cbox = control_box(face, size, glyph, hinted.as_ref())?;
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
    let vertical = vertical_metrics(face, size, glyph, (bearing_x, advance), hinted.is_some());
    Ok((
        (width, height, bearing_x, bearing_y),
        (advance, size.linear_advance(face, glyph)),
        vertical,
        (x0, y0, x1, y1),
    ))
}

/// The vertical metrics `FreeType`'s TrueType loader makes up for a face
/// without them (`compute_glyph_metrics`): an advance of the typographic
/// ascent and descent with the glyph centred in it, grid-fitted when the
/// glyph is hinted (`ft_glyphslot_grid_fit_metrics`). The autohinter
/// rescales them with its own fitted scale, which is not reproduced.
fn vertical_metrics(
    face: &Face<'_>,
    size: &Size,
    glyph: GlyphId,
    (bearing_x, advance): (i64, i64),
    hinted: bool,
) -> (i64, i64, i64) {
    let units = face.glyph_bbox(glyph).ok().flatten().unwrap_or_default();
    let height = i64::from(units.y_max) - i64::from(units.y_min);
    let advance_units = match face.os2() {
        Some(os2) => i64::from(os2.typo_ascender) - i64::from(os2.typo_descender),
        None => i64::from(face.ascender()) - i64::from(face.descender()),
    };
    // C's division, truncating towards zero.
    let top = size.scale_y((advance_units - height) / 2);
    let bearing = bearing_x - advance / 2;
    let vert_advance = size.scale_y(advance_units);
    if hinted {
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
    let hinting = hinting(flags);
    let metrics = {
        let face = font.chain_face(face_index)?;
        metrics(&face, &font.size_of(&face), glyph, hinting)?
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
    let hinting = hinting(flags);
    let options = shape_options(shaping)?;
    let (ptsize, dpi) = font.size;
    let faces = font.chain_faces()?;
    let layout = FontChain::new(faces.clone())
        .layout(&text, ptsize, dpi, &options, hinting)
        .map_err(|err| ft_error("lay out text", &err))?;
    let matrix = Rotation::new(angle);
    let mut bbox: Option<(i64, i64, i64, i64)> = None;
    let mut placed = Vec::with_capacity(layout.glyphs.len());
    for glyph in &layout.glyphs {
        let face = &faces[glyph.face];
        let size = Size::new(face, ptsize, dpi);
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

/// The bitmap of a loaded glyph rendered at its pen.
fn glyph_bitmap(
    font: &Ft2Font,
    glyph: &Loaded,
    antialiased: bool,
) -> PyResult<Option<GlyphBitmap>> {
    let face = font.chain_face(glyph.face)?;
    let size = font.size_of(&face);
    let hinted = load_hinted(&face, &size, glyph.glyph, glyph.hinting)?;
    let shape = if antialiased {
        let mut sink = PathSink::default();
        match &hinted {
            Some(hinted) => hinted.raster_outline(&mut sink),
            None => size
                .raster_outline(&face, glyph.glyph, &mut sink)
                .map_err(|err| ft_error("load glyph", &err))?,
        }
        GlyphShape::Smooth(sink.finish())
    } else {
        GlyphShape::Mono(match &hinted {
            Some(hinted) => hinted.scan_outline(),
            None => size
                .scan_outline(&face, glyph.glyph)
                .map_err(|err| ft_error("load glyph", &err))?,
        })
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

/// `draw_glyphs_to_bitmap(antialiased)`: renders the text `set_text`
/// placed into the font's image.
fn draw_glyphs_to_bitmap(font: Native<Ft2Font>, antialiased: bool) -> PyResult<()> {
    let mut font = font.borrow_mut();
    let (x0, y0, x1, y1) = font.glyphs.text.bbox;
    let columns = usize::try_from((x1 - x0) / 64 + 2).unwrap_or(0);
    let rows = usize::try_from((y1 - y0) / 64 + 2).unwrap_or(0);
    let mut image = vec![0; rows * columns];
    for glyph in &font.glyphs.loaded {
        if let Some(bitmap) = glyph_bitmap(&font, glyph, antialiased)? {
            let x = (f64::from(bitmap.left) - x0 as f64 / 64.0) as i64;
            let y = (y1 as f64 / 64.0 - f64::from(bitmap.top) + 1.0) as i64;
            draw_bitmap(&mut image, (rows, columns), &bitmap, (x, y), antialiased);
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
/// with loaded glyph `index` rendered at the origin and or-ed in with its
/// left edge offset by `x` and its top row at `y`.
fn draw_glyph_to_bitmap(
    font: Native<Ft2Font>,
    image: NdArray<u8>,
    (x, y): (i64, i64),
    index: usize,
    antialiased: bool,
) -> PyResult<NdArray<u8>> {
    let font = font.borrow();
    let mut glyph = *font
        .glyphs
        .loaded
        .get(index)
        .ok_or_else(|| PyErr::runtime_error("glyph num is out of range"))?;
    glyph.pen = (0, 0);
    glyph.angle = 0.0;
    let shape = image.shape.clone();
    let [rows, columns] = shape[..] else {
        return Err(PyErr::value_error("image must be 2D"));
    };
    let mut pixels = image.data;
    if let Some(bitmap) = glyph_bitmap(&font, &glyph, antialiased)? {
        let left = x + i64::from(bitmap.left);
        draw_bitmap(
            &mut pixels,
            (rows, columns),
            &bitmap,
            (left, y),
            antialiased,
        );
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
    let glyph = Loaded {
        face: 0,
        glyph: GlyphId(index),
        hinting: hinting(flags),
        pen: (dx, dy),
        angle,
    };
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
    let (ptsize, dpi) = font.size;
    let faces = font.chain_faces()?;
    let layout = FontChain::new(faces.clone())
        .layout(&text, ptsize, dpi, &options, hinting(flags))
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
            let linear = Size::new(face, ptsize, dpi).linear_advance(face, glyph.glyph_id);
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
    module.add("load_char", load_char)?;
    module.add("load_glyph", load_glyph)?;
    module.add("get_num_glyphs", get_num_glyphs)?;
    module.add("clear", clear)?;
    module.add("set_text", set_text)?;
    module.add("get_width_height", get_width_height)?;
    module.add("get_descent", get_descent)?;
    module.add("get_bitmap_offset", get_bitmap_offset)?;
    module.add("draw_glyphs_to_bitmap", draw_glyphs_to_bitmap)?;
    module.add("get_image", get_image)?;
    module.add("draw_glyph_to_bitmap", draw_glyph_to_bitmap)?;
    module.add("render_glyph", render_glyph_at)?;
    module.add("get_path", get_path)?;
    module.add("layout", layout)
}
