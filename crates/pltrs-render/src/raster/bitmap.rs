//! A glyph rendered alone into a bitmap, as `FreeType` renders it for
//! matplotlib's `FT2Font._render_glyph` (and the bitmaps of its
//! `draw_glyph_to_bitmap` and mathtext's raster output).

use pltrs_core::Point;
use pltrs_font::ScanOutline;
use pltrs_raster::{GrayBitmap, RasterError};

use super::glyph::{GlyphTransform, freetype_contours, is_axis_aligned};
use super::mono::{self, MonoBitmap};

/// A glyph's tagged outline (26.6, origin at the pen) as `FreeType` hands
/// it to a rasterizer.
#[derive(Clone, Debug, PartialEq)]
pub enum GlyphShape {
    /// For anti-aliased text.
    Smooth(ScanOutline),
    /// For text without anti-aliasing.
    Mono(ScanOutline),
}

/// A rendered glyph: `FreeType`'s `FT_Bitmap` and where it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlyphBitmap {
    /// The display column of the leftmost pixels (`bitmap_left`).
    pub left: i32,
    /// The display row above the topmost pixels, y up (`bitmap_top`).
    pub top: i32,
    /// Pixels per row.
    pub width: u32,
    /// Number of rows.
    pub rows: u32,
    /// Row-major from the top row: coverage `0..=255`, or `0`/`1` for a
    /// monochrome glyph (as matplotlib's `PositionedBitmap` unpacks it).
    pub buffer: Vec<u8>,
}

impl From<GrayBitmap> for GlyphBitmap {
    fn from(bitmap: GrayBitmap) -> Self {
        Self {
            left: bitmap.left,
            top: bitmap.top,
            width: bitmap.coverage.width(),
            rows: bitmap.coverage.height(),
            buffer: bitmap.coverage.into_vec(),
        }
    }
}

impl From<MonoBitmap> for GlyphBitmap {
    fn from(bitmap: MonoBitmap) -> Self {
        let (left, top) = bitmap.origin();
        let (width, rows) = bitmap.size();
        Self {
            left,
            top,
            width,
            rows,
            buffer: bitmap.pixels().map(u8::from).collect(),
        }
    }
}

/// The anti-aliased bitmap of a tagged glyph `outline` placed by
/// `transform`, as `FT_Render_Glyph` renders it; `None` for an outline
/// without area or a malformed one.
pub(super) fn smooth_bitmap(
    outline: &ScanOutline,
    transform: &GlyphTransform,
) -> Result<Option<GrayBitmap>, RasterError> {
    match freetype_contours(outline, transform) {
        Some(contours) => pltrs_raster::render_glyph(&contours, false),
        None => Ok(None),
    }
}

/// Whether `FreeType` sees the glyph matrix matplotlib sets for `angle`
/// degrees as a quarter turn: the only transforms it autohints glyphs
/// under.
pub fn glyph_axis_aligned(angle: f64) -> bool {
    is_axis_aligned(angle)
}

/// Renders a glyph outline rotated by `angle` degrees with its origin at
/// `pen` (display pixels, y up, quantized to 1/64 pixel) — matplotlib's
/// `_set_transform` and `_render_glyph`: anti-aliased for a smooth
/// outline, monochrome for a scan outline. `None` for a glyph without
/// ink.
///
/// # Errors
///
/// When the glyph is too large to render.
pub fn render_glyph(
    shape: &GlyphShape,
    angle: f64,
    pen: Point,
) -> Result<Option<GlyphBitmap>, RasterError> {
    let transform = GlyphTransform::new(angle, pen);
    Ok(match shape {
        GlyphShape::Smooth(outline) => smooth_bitmap(outline, &transform)?.map(GlyphBitmap::from),
        GlyphShape::Mono(outline) => mono::render(outline, &transform).map(GlyphBitmap::from),
    })
}
