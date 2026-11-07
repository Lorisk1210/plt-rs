//! Glyphs of a CFF or CFF2 table, loaded as FreeType's `cff_slot_load`
//! loads them through the Adobe engine: unscaled, scaled without hinting,
//! or hinted at a size; then the font matrix and offset, and the scales.

use super::{Cff, STANDARD_ENCODING, SubFont, Version};
use crate::error::{FontError, Result};
use crate::fixed::{mul_div, mul_fix};
use crate::outline::Outline;
use crate::psaux::{
    CharstringError, Charstrings, Flavor, PrivateDict, Rendering, decode, engine_rendering,
};
use crate::types::GlyphId;

/// How a glyph is loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CffScale {
    /// `FT_LOAD_NO_SCALE`: in whole font units.
    Units,
    /// `FT_LOAD_NO_HINTING` at a size of these 16.16 scales, into 26.6
    /// pixels.
    Scaled {
        /// The horizontal scale.
        x_scale: i64,
        /// The vertical scale.
        y_scale: i64,
    },
    /// Hinted by the engine at a size of these scales, into 26.6 pixels.
    Hinted {
        /// The horizontal scale.
        x_scale: i64,
        /// The vertical scale.
        y_scale: i64,
    },
}

/// `cff_compute_bias`: what is added to a subroutine number.
fn bias(count: usize) -> i64 {
    if count < 1240 {
        107
    } else if count < 33900 {
        1131
    } else {
        32768
    }
}

/// What a glyph's charstring reaches: the table's subroutines and glyphs,
/// and its Font DICT.
pub(super) struct Glyph<'c, 'a> {
    pub(super) cff: &'c Cff<'a>,
    subfont: &'c SubFont<'a>,
}

impl<'c, 'a: 'c> Charstrings<'c> for Glyph<'c, 'a> {
    fn flavor(&self) -> Flavor {
        self.cff.flavor()
    }

    fn local_subr(&self, number: i32) -> Option<&'c [u8]> {
        let subrs = &self.subfont.subrs;
        let index = usize::try_from(i64::from(number) + bias(subrs.len())).ok()?;
        subrs.get(index)
    }

    fn global_subr(&self, number: i32) -> Option<&'c [u8]> {
        let subrs = &self.cff.global_subrs;
        let index = usize::try_from(i64::from(number) + bias(subrs.len())).ok()?;
        subrs.get(index)
    }

    /// `cff_lookup_glyph_by_stdcharcode`: the first glyph whose charset
    /// string is the one the Standard Encoding gives `code`.
    fn standard_glyph(&self, code: i32) -> Result<&'c [u8], CharstringError> {
        let missing = CharstringError::NotStandardCode(i64::from(code));
        let sid = usize::try_from(code)
            .ok()
            .and_then(|code| STANDARD_ENCODING.get(code))
            .ok_or(missing.clone())?;
        let glyph = self.cff.glyph_for_sid(*sid).ok_or(missing.clone())?;
        self.cff
            .charstrings
            .get(usize::from(glyph.0))
            .ok_or(missing)
    }

    fn private(&self) -> &PrivateDict {
        &self.subfont.private.dict
    }

    fn stack_size(&self) -> usize {
        self.cff.stack_size()
    }

    fn blend_vector(&self, vsindex: u32) -> Result<Vec<i32>, CharstringError> {
        if !self.cff.vstore.has_data() {
            return Err(CharstringError::Malformed(
                "blend in a font without variations",
            ));
        }
        self.cff.vstore.blend_vector(vsindex, &self.cff.coords)
    }
}

impl<'a> Cff<'a> {
    /// The charstring of `glyph` and what it reaches.
    ///
    /// # Errors
    ///
    /// When the glyph is beyond the table's or belongs to no Font DICT.
    pub(super) fn glyph_program(&self, glyph: GlyphId) -> Result<(&'a [u8], Glyph<'_, 'a>)> {
        let tag = self.version.tag();
        let code = self
            .charstrings
            .get(usize::from(glyph.0))
            .ok_or(FontError::malformed(tag, "glyph beyond the CharStrings"))?;
        let subfont = self
            .subfonts
            .get(self.fd_select.font_dict(glyph))
            .ok_or(FontError::malformed(tag, "glyph of no Font DICT"))?;
        Ok((code, Glyph { cff: self, subfont }))
    }

    /// The 16.16 scales of `subfont`'s glyphs at a size of `x_scale` and
    /// `y_scale`: a Font DICT of other units per em than the Top DICT's
    /// scales by the ratio, even glyphs loaded unscaled.
    fn subfont_scales(
        &self,
        subfont: &SubFont<'_>,
        x_scale: i64,
        y_scale: i64,
    ) -> (i64, i64, bool) {
        let (top, sub) = (self.top_matrix.units_per_em, subfont.matrix.units_per_em);
        let has_font_dicts = self.is_cid || self.version == Version::Cff2;
        if has_font_dicts && top != sub {
            (mul_div(x_scale, top, sub), mul_div(y_scale, top, sub), true)
        } else {
            (x_scale, y_scale, false)
        }
    }

    /// Loads `glyph`, whose `hmtx` advance is `advance`, in a face of
    /// `units_per_em`, as `cff_slot_load` does: the outline, in font units
    /// or 26.6 pixels, and the advance in the same units.
    ///
    /// # Errors
    ///
    /// When the glyph is beyond the table's, belongs to no Font DICT, or
    /// its charstring fails, or the size is invalid.
    pub(crate) fn load(
        &self,
        glyph: GlyphId,
        advance: i64,
        scale: CffScale,
        units_per_em: u16,
    ) -> Result<(Outline, i64)> {
        let tag = self.version.tag();
        let (code, font) = self.glyph_program(glyph)?;
        let subfont = font.subfont;
        let (x_scale, y_scale, hinting) = match scale {
            CffScale::Units => (0x10000, 0x10000, false),
            CffScale::Scaled { x_scale, y_scale } => (x_scale, y_scale, false),
            CffScale::Hinted { x_scale, y_scale } => (x_scale, y_scale, true),
        };
        let (x_scale, y_scale, forced) = self.subfont_scales(subfont, x_scale, y_scale);
        let scaled = scale != CffScale::Units;
        let failed = |error: CharstringError| FontError::malformed(tag, error.reason());
        let rendering = if scaled {
            engine_rendering(x_scale, y_scale, hinting, units_per_em).map_err(failed)?
        } else {
            Rendering::Unhinted
        };
        let mut outline = decode(&font, code, rendering).map_err(failed)?.outline;
        let transform = subfont.matrix.transform;
        let mut advance = transform.apply(&mut outline, advance);
        let (dx, dy) = transform.offset;
        if dx != 0 || dy != 0 {
            outline.translate(dx as f64, dy as f64);
            advance += dx;
        }
        if scaled || forced {
            if rendering == Rendering::Unhinted {
                for point in &mut outline.points {
                    point.x = mul_fix(point.x as i64, x_scale) as f64;
                    point.y = mul_fix(point.y as i64, y_scale) as f64;
                }
            }
            advance = mul_fix(advance, x_scale);
        }
        Ok((outline, advance))
    }
}
