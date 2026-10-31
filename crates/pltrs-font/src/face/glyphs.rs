//! The face's glyphs: horizontal and vertical metrics, and outlines
//! loaded from `glyf` or `CFF` with FreeType's origin alignment.

use super::{Face, Outlines};
use crate::error::{FontError, Result};
use crate::outline::{Midpoint, Outline, OutlineSink};
use crate::tables::glyf::{Glyf, Scaling};
use crate::tables::head::Hhea;
use crate::tables::hmtx::Hmtx;
use crate::types::{GlyphId, Rect, Tag};

impl<'a> Face<'a> {
    /// Horizontal advance of `glyph` in font units.
    pub fn glyph_advance(&self, glyph: GlyphId) -> u16 {
        self.hmtx.advance(glyph)
    }

    /// Left side bearing of `glyph` in font units (from `hmtx`).
    pub fn left_side_bearing(&self, glyph: GlyphId) -> i16 {
        self.hmtx.left_side_bearing(glyph)
    }

    /// The TrueType outlines (`loca` + `glyf`), if the face has them.
    pub(crate) const fn glyf(&self) -> Option<&Glyf<'a>> {
        match &self.outlines {
            Outlines::TrueType(glyf) => Some(glyf),
            Outlines::Cff(_) | Outlines::None => None,
        }
    }

    /// Top side bearing and advance height of `glyph` in font units, from
    /// `vmtx` or, without one, derived from the typographic ascender and
    /// descender and the glyph's top `y_max` (FreeType's
    /// `TT_Get_VMetrics`).
    pub(crate) fn glyph_vertical_metrics(&self, glyph: GlyphId, y_max: i32) -> (i16, u16) {
        let vmtx = self.table(Tag::new(b"vhea")).and_then(|vhea| {
            let long_count = Hhea::parse(vhea).ok()?.number_of_h_metrics;
            let vmtx = self.table(Tag::new(b"vmtx"))?;
            Hmtx::parse(vmtx, long_count, self.maxp.num_glyphs).ok()
        });
        if let Some(vmtx) = vmtx {
            return (vmtx.left_side_bearing(glyph), vmtx.advance(glyph));
        }
        let (ascender, descender) = self
            .os2
            .as_ref()
            .map_or((self.hhea.ascender, self.hhea.descender), |os2| {
                (os2.typo_ascender, os2.typo_descender)
            });
        let ascender = i32::from(ascender);
        (
            (ascender - y_max) as i16,
            (ascender - i32::from(descender)).unsigned_abs() as u16,
        )
    }

    pub(crate) fn check_glyph(&self, glyph: GlyphId) -> Result<()> {
        if glyph.0 >= self.maxp.num_glyphs {
            return Err(FontError::malformed(
                Tag::new(b"maxp"),
                "glyph id beyond glyph count",
            ));
        }
        Ok(())
    }

    /// Loads the tagged points of `glyph` in the given coordinate space,
    /// with FreeType's origin alignment applied.
    pub(crate) fn load_outline(&self, glyph: GlyphId, scaling: Scaling) -> Result<Outline> {
        self.load_with_advance(glyph, scaling)
            .map(|(outline, _)| outline)
    }

    /// Loads `glyph` like FreeType's `FT_Load_Glyph` with
    /// `FT_LOAD_NO_SCALE`: the outline in whole font units (TrueType
    /// composites transformed in fixed point, CFF coordinates floored as
    /// FreeType's CFF engine stores them) and the advance width in font
    /// units.
    pub(crate) fn load_unscaled(&self, glyph: GlyphId) -> Result<(Outline, i64)> {
        let unit = Scaling::Fixed {
            x_scale: 0x10000,
            y_scale: 0x10000,
        };
        match &self.outlines {
            Outlines::Cff(_) => {
                let (mut outline, advance) = self.load_with_advance(glyph, Scaling::Units)?;
                for point in &mut outline.points {
                    point.x = point.x.floor();
                    point.y = point.y.floor();
                }
                Ok((outline, advance))
            }
            Outlines::TrueType(_) | Outlines::None => self.load_with_advance(glyph, unit),
        }
    }

    /// The outline of `glyph` with the origin aligned, and the advance
    /// width in font units.
    fn load_with_advance(&self, glyph: GlyphId, scaling: Scaling) -> Result<(Outline, i64)> {
        self.check_glyph(glyph)?;
        let mut outline = Outline::default();
        let mut advance = i64::from(self.hmtx.advance(glyph));
        match &self.outlines {
            Outlines::TrueType(glyf) => {
                let metrics = |g: GlyphId| (self.hmtx.left_side_bearing(g), self.hmtx.advance(g));
                let phantoms = glyf.load(glyph, scaling, &metrics, &mut outline)?;
                let shift = match scaling {
                    Scaling::Units => phantoms.origin as f64,
                    Scaling::Fixed { x_scale, .. } => {
                        crate::fixed::mul_fix(phantoms.origin, x_scale) as f64
                    }
                };
                if shift != 0.0 {
                    outline.translate(-shift, 0.0);
                }
                advance = phantoms.advance;
            }
            Outlines::Cff(cff) => cff.load(glyph, scaling, &mut outline)?,
            Outlines::None => {}
        }
        Ok((outline, advance))
    }

    /// Emits the outline of `glyph` in font units and returns its control
    /// box (the bounds of all points, rounded outwards), or `None` for empty
    /// glyphs such as the space.
    pub fn outline(&self, glyph: GlyphId, sink: &mut impl OutlineSink) -> Result<Option<Rect>> {
        let outline = self.load_outline(glyph, Scaling::Units)?;
        outline.decompose(sink, Midpoint::Exact);
        Ok(control_rect(&outline))
    }

    /// Control box of `glyph` in font units, `None` for empty glyphs.
    pub fn glyph_bbox(&self, glyph: GlyphId) -> Result<Option<Rect>> {
        Ok(control_rect(&self.load_outline(glyph, Scaling::Units)?))
    }

    /// The advance width and control box of `glyph` in font units, as
    /// FreeType's `FT_Load_Glyph` with `FT_LOAD_NO_SCALE` gives them (the
    /// `horiAdvance`, and the box of `horiBearingX`, `horiBearingY`,
    /// `width` and `height`); the box is `None` for empty glyphs.
    ///
    /// # Errors
    ///
    /// When the glyph is beyond the face's glyphs or its outline is
    /// malformed.
    pub fn unscaled_metrics(&self, glyph: GlyphId) -> Result<(i64, Option<Rect>)> {
        let (outline, advance) = self.load_unscaled(glyph)?;
        Ok((advance, control_rect(&outline)))
    }

    /// The top side bearing and advance height of `glyph`, in font units,
    /// that FreeType's TrueType loader makes up for a face without
    /// vertical metrics (`compute_glyph_metrics`): an advance of the
    /// typographic ascender minus descender (`hhea`'s without an `OS/2`
    /// table), with the glyph's control box centred in it.
    pub fn synthesized_vertical_metrics(&self, glyph: GlyphId) -> (i64, i64) {
        let bbox = self.glyph_bbox(glyph).ok().flatten().unwrap_or_default();
        let height = i64::from(bbox.y_max) - i64::from(bbox.y_min);
        let advance = match self.os2() {
            Some(os2) => i64::from(os2.typo_ascender) - i64::from(os2.typo_descender),
            None => i64::from(self.hhea().ascender) - i64::from(self.hhea().descender),
        };
        // C's division, truncating towards zero.
        ((advance - height) / 2, advance)
    }

    /// Whether the face has TrueType or CFF outlines.
    pub const fn has_outlines(&self) -> bool {
        !matches!(self.outlines, Outlines::None)
    }
}

fn control_rect(outline: &Outline) -> Option<Rect> {
    let (x0, y0, x1, y1) = outline.control_box()?;
    Some(Rect {
        x_min: x0.floor() as i32,
        y_min: y0.floor() as i32,
        x_max: x1.ceil() as i32,
        y_max: y1.ceil() as i32,
    })
}

#[cfg(test)]
mod tests {
    use crate::{Face, GlyphId, bundled_font_data};

    #[test]
    fn unscaled_metrics_are_those_of_freetype_without_scaling() {
        let data = bundled_font_data("DejaVuSans.ttf").expect("bundled");
        let face = Face::parse(data, 0).expect("parsed");
        // `FT2Font.load_glyph(index, LoadFlags.NO_SCALE)`: `horiAdvance`,
        // `horiBearingY` and `height`.
        for (glyph, expected) in [
            (10, (563, 1493, 555)),
            (36, (1401, 1493, 1493)),
            (70, (1126, 1147, 1176)),
            (200, (1430, 1899, 1928)),
        ] {
            let (advance, bbox) = face.unscaled_metrics(GlyphId(glyph)).expect("glyph");
            let bbox = bbox.expect("ink");
            let metrics = (advance, bbox.y_max, bbox.y_max - bbox.y_min);
            assert_eq!(metrics, expected, "glyph {glyph}");
        }
        let (advance, bbox) = face.unscaled_metrics(GlyphId(3)).expect("space");
        assert_eq!((advance, bbox), (651, None));
        assert!(face.unscaled_metrics(GlyphId(face.num_glyphs())).is_err());
    }
}
