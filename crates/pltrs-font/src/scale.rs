//! FreeType/HarfBuzz-compatible scaling.
//!
//! matplotlib sizes fonts with `FT_Set_Char_Size(ptsize * 64, dpi)` and lays
//! out text with HarfBuzz on top of FreeType (`hb-ft`). Text widths depend on
//! the exact integer arithmetic of both libraries, which [`Size`]
//! reproduces: FreeType's 16.16 scale, `hb-ft`'s rounding of advances, and
//! HarfBuzz's `em_mult` scaling of `GPOS` values.
//!
//! The arithmetic is ported from FreeType, copyright David Turner, Robert
//! Wilhelm and Werner Lemberg, under the FreeType License (FTL), and from
//! HarfBuzz under its MIT licence.

use crate::error::Result;
use crate::face::Face;
use crate::fixed::{div_fix, mul_div, mul_fix, pix_round};
use crate::hinting::HintedFace;
use crate::outline::{Midpoint, OutlineSink};
use crate::scan::{ScanOutline, ScanRules};
use crate::tables::glyf::Scaling;
use crate::types::{GlyphId, Rect};

/// A font size in FreeType terms: 16.16 scales from font units to 26.6
/// pixels, plus the rounded pixels-per-em.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Size {
    x_scale: i64,
    y_scale: i64,
    x_ppem: u16,
    y_ppem: u16,
    units_per_em: u16,
    resolution: u32,
}

/// How `FT_Get_Kerning` scales a kerning value.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KerningMode {
    /// Scaled and rounded to whole pixels (`FT_KERNING_DEFAULT`).
    Default,
    /// Scaled without rounding (`FT_KERNING_UNFITTED`).
    Unfitted,
    /// Font units (`FT_KERNING_UNSCALED`).
    Unscaled,
}

/// Why FreeType refuses a font size (`FT_Set_Char_Size`), which
/// [`Size::new`] clamps instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeError {
    /// Less than one pixel per em in a TrueType face (FreeType's
    /// `Invalid_PPem`).
    InvalidPpem,
    /// More than 65 535 pixels per em (FreeType's `Invalid_Pixel_Size`).
    InvalidPixelSize,
}

impl SizeError {
    /// FreeType's error code.
    pub const fn code(self) -> u8 {
        match self {
            Self::InvalidPpem => 0x97,
            Self::InvalidPixelSize => 0x17,
        }
    }
}

impl std::fmt::Display for SizeError {
    /// FreeType's error string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidPpem => "invalid ppem value",
            Self::InvalidPixelSize => "invalid pixel size",
        })
    }
}

impl std::error::Error for SizeError {}

/// `(FT_UInt)value` as matplotlib's C++ casts a resolution, which differs
/// by processor for what no unsigned integer holds: x86-64 converts to a
/// 64-bit integer (the "integer indefinite" `i64::MIN` for what is out of
/// its range or no number), then keeps its low 32 bits, so a negative
/// resolution wraps around; other processors (arm64's `fcvtzu`) saturate,
/// as Rust's casts do.
fn c_unsigned(value: f64) -> u32 {
    if cfg!(target_arch = "x86_64") {
        const LIMIT: f64 = 9_223_372_036_854_775_808.0;
        let wide = if value.is_nan() || value.abs() >= LIMIT {
            i64::MIN
        } else {
            value as i64
        };
        wide as u32
    } else {
        value as u32
    }
}

impl Size {
    /// The size matplotlib's `FT2Font.set_size(points, dpi)` selects: the
    /// point size is truncated to 26.6 and the resolution to an integer.
    pub fn new(face: &Face<'_>, points: f64, dpi: f64) -> Self {
        Self::from_char_size(face, (points * 64.0) as i64, c_unsigned(dpi))
    }

    /// [`new`](Self::new), refusing what FreeType refuses: a size of more
    /// than 65 535 pixels per em, or of none in a TrueType face. The
    /// resolution is cast to FreeType's unsigned resolution as matplotlib's
    /// `FT2Font::set_size` casts it (on x86-64, a negative one wraps around
    /// to a huge one, which FreeType refuses).
    ///
    /// ```
    /// use pltrs_font::{Face, Size, SizeError, bundled_font_data};
    /// let face = Face::parse(bundled_font_data("DejaVuSans.ttf").expect("bundled"), 0)?;
    /// assert!(Size::checked(&face, 10.0, 72.0).is_ok());
    /// assert_eq!(Size::checked(&face, 10.0, 2.5), Err(SizeError::InvalidPpem));
    /// assert_eq!(Size::checked(&face, 1e5, 72.0), Err(SizeError::InvalidPixelSize));
    /// if cfg!(target_arch = "x86_64") {
    ///     assert_eq!(Size::checked(&face, 10.0, -1.0), Err(SizeError::InvalidPixelSize));
    ///     assert!(Size::checked(&face, 10.0, f64::INFINITY).is_ok(), "no resolution: 72");
    /// }
    /// # Ok::<(), pltrs_font::FontError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// The [`SizeError`] FreeType gives for the size.
    pub fn checked(face: &Face<'_>, points: f64, dpi: f64) -> Result<Self, SizeError> {
        let char_size = ((points * 64.0) as i64).max(64);
        let resolution = match c_unsigned(dpi) {
            0 => 72,
            resolution => i64::from(resolution),
        };
        let ppem = (char_size.saturating_mul(resolution).saturating_add(36) / 72 + 32) >> 6;
        if ppem > i64::from(u16::MAX) {
            Err(SizeError::InvalidPixelSize)
        } else if ppem == 0 && !face.is_cff() {
            Err(SizeError::InvalidPpem)
        } else {
            Ok(Self::new(face, points, dpi))
        }
    }

    /// `FT_Set_Char_Size(face, char_size, 0, dpi, dpi)` with `char_size` in
    /// 26.6 points.
    ///
    /// FreeType keeps pixels per em in 16 bits (and at least one); larger
    /// requests are clamped to 65 535 pixels per em, which keeps every
    /// later fixed-point product within 64 bits.
    pub fn from_char_size(face: &Face<'_>, char_size: i64, dpi: u32) -> Self {
        Self::for_units_per_em(face.units_per_em(), char_size, dpi)
    }

    /// [`from_char_size`](Self::from_char_size) for a face of
    /// `units_per_em` design units per em.
    pub(crate) fn for_units_per_em(units_per_em: u16, char_size: i64, dpi: u32) -> Self {
        const MAX_SCALED: i64 = (u16::MAX as i64) << 6;
        let char_size = char_size.max(64);
        let resolution = if dpi == 0 { 72 } else { dpi };
        let scaled = (char_size
            .saturating_mul(i64::from(resolution))
            .saturating_add(36)
            / 72)
            .min(MAX_SCALED);
        let scale = div_fix(scaled, i64::from(units_per_em));
        let ppem = u16::try_from((scaled + 32) >> 6).map_or(u16::MAX, |ppem| ppem.max(1));
        Self {
            x_scale: scale,
            y_scale: scale,
            x_ppem: ppem,
            y_ppem: ppem,
            units_per_em,
            resolution,
        }
    }

    /// The device resolution in dots per inch (72 when none was given).
    pub const fn resolution(&self) -> u32 {
        self.resolution
    }

    /// Horizontal scale from font units to 26.6 pixels, as 16.16.
    pub const fn x_scale(&self) -> i64 {
        self.x_scale
    }

    /// Vertical scale from font units to 26.6 pixels, as 16.16.
    pub const fn y_scale(&self) -> i64 {
        self.y_scale
    }

    /// Rounded horizontal pixels per em.
    pub const fn x_ppem(&self) -> u16 {
        self.x_ppem
    }

    /// Rounded vertical pixels per em.
    pub const fn y_ppem(&self) -> u16 {
        self.y_ppem
    }

    /// Scales a horizontal font-unit value to 26.6 (`FT_MulFix`).
    pub fn scale_x(&self, value: i64) -> i64 {
        mul_fix(value, self.x_scale)
    }

    /// Scales a vertical font-unit value to 26.6 (`FT_MulFix`).
    pub fn scale_y(&self, value: i64) -> i64 {
        mul_fix(value, self.y_scale)
    }

    /// Unhinted linear advance in 16.16 pixels (FreeType's
    /// `linearHoriAdvance` / `FT_Get_Advance`).
    pub fn linear_advance(&self, face: &Face<'_>, glyph: GlyphId) -> i64 {
        mul_div(i64::from(face.glyph_advance(glyph)), self.x_scale, 64)
    }

    /// Advance in 26.6 pixels as HarfBuzz's FreeType integration computes
    /// it for unhinted layout.
    pub fn advance(&self, face: &Face<'_>, glyph: GlyphId) -> i32 {
        let linear = self.linear_advance(face, glyph).abs();
        // hb-ft: `(int) (v * x_mult + (1<<9)) >> 10` in single precision.
        ((linear as f32 + 512.0) as i32) >> 10
    }

    /// The HarfBuzz font scale (`hb_font_t::x_scale`) derived from
    /// FreeType's.
    fn hb_scale(scale: i64, units_per_em: u16) -> i64 {
        (scale * i64::from(units_per_em) + 0x8000) >> 16
    }

    /// HarfBuzz's `em_mult` multiplier for one axis.
    fn hb_mult(&self, scale: i64) -> i64 {
        (Self::hb_scale(scale, self.units_per_em) << 16) / i64::from(self.units_per_em)
    }

    /// Scales a horizontal `GPOS` value like HarfBuzz (`em_scale_x`).
    pub fn em_scale_x(&self, value: i32) -> i32 {
        Self::em_scale(value, self.hb_mult(self.x_scale))
    }

    /// Scales a vertical `GPOS` value like HarfBuzz (`em_scale_y`).
    pub fn em_scale_y(&self, value: i32) -> i32 {
        Self::em_scale(value, self.hb_mult(self.y_scale))
    }

    /// HarfBuzz's `em_mult` product, saturating where values far beyond
    /// the 16-bit range of font data would overflow.
    fn em_scale(value: i32, mult: i64) -> i32 {
        let scaled = i64::from(value).saturating_mul(mult).saturating_add(0x8000) >> 16;
        scaled.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }

    /// HarfBuzz's floating-point `em_fscale_x`, used for anchors.
    pub(crate) fn em_fscale_x(&self, value: i32) -> f32 {
        value as f32
            * (Self::hb_scale(self.x_scale, self.units_per_em) as f32
                / f32::from(self.units_per_em))
    }

    /// HarfBuzz's `em_scalef_x`: a fractional font-unit value scaled and
    /// rounded (used for tracking).
    pub(crate) fn em_scalef_x(&self, value: f32) -> i32 {
        let mult =
            Self::hb_scale(self.x_scale, self.units_per_em) as f32 / f32::from(self.units_per_em);
        (value * mult).round() as i32
    }

    /// HarfBuzz's floating-point `em_fscale_y`, used for anchors.
    pub(crate) fn em_fscale_y(&self, value: i32) -> f32 {
        value as f32
            * (Self::hb_scale(self.y_scale, self.units_per_em) as f32
                / f32::from(self.units_per_em))
    }

    /// HarfBuzz font scales `(x, y)` in 26.6 per em, used for device tables.
    pub(crate) fn hb_scales(&self) -> (i64, i64) {
        (
            Self::hb_scale(self.x_scale, self.units_per_em),
            Self::hb_scale(self.y_scale, self.units_per_em),
        )
    }

    /// Control box of the unhinted glyph outline in 26.6 pixels, exactly as
    /// FreeType computes it (`FT_Glyph_Get_CBox` with subpixel units).
    pub fn glyph_cbox(&self, face: &Face<'_>, glyph: GlyphId) -> Result<Option<Rect>> {
        let outline = face.load_outline(glyph, self.scaling())?;
        Ok(outline.control_box().map(|(x0, y0, x1, y1)| Rect {
            x_min: x0 as i32,
            y_min: y0 as i32,
            x_max: x1 as i32,
            y_max: y1 as i32,
        }))
    }

    /// Emits the unhinted outline FreeType loads at this size, in pixels
    /// (26.6 values divided by 64) — what matplotlib's `FT2Font.get_path`
    /// returns.
    pub fn outline(
        &self,
        face: &Face<'_>,
        glyph: GlyphId,
        sink: &mut impl OutlineSink,
    ) -> Result<()> {
        let mut pixels = PixelSink(sink);
        face.load_outline(glyph, self.scaling())?
            .decompose(&mut pixels, Midpoint::Truncated);
        Ok(())
    }

    /// The unhinted outline FreeType loads at this size with the rules
    /// its loader sets, for monochrome rendering.
    ///
    /// # Errors
    ///
    /// When the glyph is missing or its outline data is malformed.
    pub fn scan_outline(&self, face: &Face<'_>, glyph: GlyphId) -> Result<ScanOutline> {
        Ok(face
            .load_outline(glyph, self.scaling())?
            .scan_outline(ScanRules::scaled(self.y_ppem)))
    }

    pub(crate) const fn scaling(&self) -> Scaling {
        Scaling::Fixed {
            x_scale: self.x_scale,
            y_scale: self.y_scale,
        }
    }

    /// `FT_Get_Kerning` for the legacy `kern` table, in 26.6 pixels (or
    /// font units for [`KerningMode::Unscaled`]).
    pub fn kerning(
        &self,
        face: &Face<'_>,
        left: GlyphId,
        right: GlyphId,
        mode: KerningMode,
    ) -> i64 {
        let raw = i64::from(face.legacy_kerning(left, right));
        match mode {
            KerningMode::Unscaled => raw,
            KerningMode::Unfitted => mul_fix(raw, self.x_scale),
            KerningMode::Default => {
                let scaled = mul_fix(raw, self.x_scale);
                let damped = if self.x_ppem < 25 {
                    mul_div(scaled, i64::from(self.x_ppem), 25)
                } else {
                    scaled
                };
                pix_round(damped)
            }
        }
    }
}

/// Divides 26.6 coordinates by 64 on their way to a sink.
pub(crate) struct PixelSink<'s, S: OutlineSink>(pub(crate) &'s mut S);

impl<S: OutlineSink> OutlineSink for PixelSink<'_, S> {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x / 64.0, y / 64.0);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x / 64.0, y / 64.0);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.quad_to(x1 / 64.0, y1 / 64.0, x / 64.0, y / 64.0);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.curve_to(
            x1 / 64.0,
            y1 / 64.0,
            x2 / 64.0,
            y2 / 64.0,
            x / 64.0,
            y / 64.0,
        );
    }
    fn close(&mut self) {
        self.0.close();
    }
}

/// Value scaling used while shaping: raw font units, HarfBuzz's 26.6 pixel
/// arithmetic at a [`Size`], or that arithmetic with the hinted advances
/// and extents HarfBuzz reads from FreeType under hinting load flags
/// (`GPOS` values still scale unhinted). `Unscaled` is HarfBuzz over a
/// FreeType face whose size was never set: the advances and extents
/// FreeType loads unscaled (in font units, which the [`Size`] of one
/// pixel per font unit gives), and positioning values scaled by the
/// font's scale of zero.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Scaler<'h> {
    Units,
    Pixels(Size),
    Hinted(Size, &'h HintedFace<'h, 'h>),
    Unscaled(Size),
}

impl Scaler<'_> {
    /// The size positioning values scale at (`None`: by one, in font
    /// units).
    const fn size(&self) -> Option<&Size> {
        match self {
            Self::Units | Self::Unscaled(_) => None,
            Self::Pixels(size) | Self::Hinted(size, _) => Some(size),
        }
    }

    /// Whether positioning values scale to zero (see `Unscaled`).
    const fn zero_scale(&self) -> bool {
        matches!(self, Self::Unscaled(_))
    }

    pub(crate) fn advance(&self, face: &Face<'_>, glyph: GlyphId) -> i32 {
        match self {
            Self::Units => i32::from(face.glyph_advance(glyph)),
            Self::Pixels(size) | Self::Unscaled(size) => size.advance(face, glyph),
            Self::Hinted(_, hinted) => hinted.advance(glyph),
        }
    }

    /// The glyph's ink box as HarfBuzz's glyph extents report it.
    pub(crate) fn extents(&self, face: &Face<'_>, glyph: GlyphId) -> Result<Option<Rect>> {
        match self {
            Self::Units => face.glyph_bbox(glyph),
            Self::Pixels(size) | Self::Unscaled(size) => size.glyph_cbox(face, glyph),
            Self::Hinted(_, hinted) => hinted.extents(glyph),
        }
    }

    /// The em size per axis: font units, or HarfBuzz's font scales.
    pub(crate) fn em(&self, face: &Face<'_>) -> (i64, i64) {
        if self.zero_scale() {
            return (0, 0);
        }
        let units = i64::from(face.units_per_em());
        self.size().map_or((units, units), Size::hb_scales)
    }

    pub(crate) fn x(&self, value: i32) -> i32 {
        if self.zero_scale() {
            return 0;
        }
        self.size().map_or(value, |size| size.em_scale_x(value))
    }

    pub(crate) fn y(&self, value: i32) -> i32 {
        if self.zero_scale() {
            return 0;
        }
        self.size().map_or(value, |size| size.em_scale_y(value))
    }

    pub(crate) fn fx(&self, value: i32) -> f32 {
        if self.zero_scale() {
            return 0.0;
        }
        self.size()
            .map_or(value as f32, |size| size.em_fscale_x(value))
    }

    /// A fractional font-unit value scaled and rounded like HarfBuzz's
    /// `em_scalef_x`.
    pub(crate) fn scale_float_x(&self, value: f32) -> i32 {
        if self.zero_scale() {
            return 0;
        }
        self.size()
            .map_or(value.round() as i32, |size| size.em_scalef_x(value))
    }

    pub(crate) fn fy(&self, value: i32) -> f32 {
        if self.zero_scale() {
            return 0.0;
        }
        self.size()
            .map_or(value as f32, |size| size.em_fscale_y(value))
    }
}
