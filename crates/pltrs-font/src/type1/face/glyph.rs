//! Glyphs of a [`Type1Face`], as FreeType's `T1_Load_Glyph` loads them:
//! unscaled (`FT_LOAD_NO_SCALE`), scaled without hinting, or hinted at a
//! size by the Adobe engine.

use super::{Type1Face, fixed_to_int};
use crate::fixed::{mul_div, mul_fix, pix_round};
use crate::outline::{Midpoint, Outline, OutlineSink};
use crate::psaux::{Rendering, decode, engine_rendering};
use crate::scale::Size;
use crate::type1::error::Type1Error;
use crate::types::{GlyphId, Rect};

/// A loaded glyph: its outline and advance width.
#[derive(Clone, Debug, PartialEq)]
pub struct Type1Glyph {
    outline: Outline,
    advance: i64,
    linear_advance: i64,
}

impl Type1Glyph {
    /// The advance width: in font units after the font matrix
    /// (`horiAdvance` with `FT_LOAD_NO_SCALE`), or in 26.6 pixels for a
    /// [`scaled`](Self::scaled) glyph.
    pub fn advance(&self) -> i64 {
        self.advance
    }

    /// The advance width before the font matrix, in font units
    /// (`linearHoriAdvance`), or in 16.16 pixels for a scaled glyph.
    pub fn linear_advance(&self) -> i64 {
        self.linear_advance
    }

    /// Emits the outline (cubic segments) in the glyph's units, y up.
    pub fn outline(&self, sink: &mut impl OutlineSink) {
        self.outline.decompose(sink, Midpoint::Exact);
    }

    /// The control box of the outline, `None` for an empty glyph.
    pub fn control_box(&self) -> Option<Rect> {
        let (x0, y0, x1, y1) = self.outline.control_box()?;
        Some(Rect {
            x_min: x0 as i32,
            y_min: y0 as i32,
            x_max: x1 as i32,
            y_max: y1 as i32,
        })
    }

    /// The glyph at a size, as FreeType scales unhinted Type 1 glyphs:
    /// every coordinate and the advance multiplied by the 16.16 scales of
    /// the size (`FT_MulFix`, into 26.6 pixels), the linear advance into
    /// 16.16 pixels.
    #[must_use]
    pub fn scaled(&self, x_scale: i64, y_scale: i64) -> Self {
        let mut outline = self.outline.clone();
        for point in &mut outline.points {
            point.x = mul_fix(point.x as i64, x_scale) as f64;
            point.y = mul_fix(point.y as i64, y_scale) as f64;
        }
        Self {
            outline,
            advance: mul_fix(self.advance, x_scale),
            linear_advance: mul_div(self.linear_advance, x_scale, 64),
        }
    }
}

impl Type1Face {
    /// Loads `glyph` unhinted and unscaled, as FreeType does with
    /// `FT_LOAD_NO_SCALE`: the charstring run by FreeType's Adobe engine,
    /// in whole font units, then transformed by the font matrix (which
    /// slants or extends the glyphs of a transformed font). Scale it with
    /// [`Type1Glyph::scaled`] for `FT_LOAD_NO_HINTING` at a size.
    pub fn load_glyph(&self, glyph: GlyphId) -> Result<Type1Glyph, Type1Error> {
        self.load(glyph, Rendering::Unhinted)
    }

    /// Loads `glyph` hinted at `size`, as FreeType does with hinting
    /// (`FT_LOAD_DEFAULT` or `FT_LOAD_TARGET_LIGHT`, which `textpath`
    /// draws usetex glyphs with): the Adobe engine aligns horizontal
    /// stems to the alignment zones and the pixel grid. The outline is in
    /// 26.6 pixels, the advance rounded to whole pixels.
    pub fn load_glyph_hinted(&self, glyph: GlyphId, size: &Size) -> Result<Type1Glyph, Type1Error> {
        let (x_scale, y_scale) = (size.x_scale(), size.y_scale());
        let rendering = engine_rendering(x_scale, y_scale, true, self.units_per_em)?;
        let mut loaded = if rendering == Rendering::Unhinted {
            // Too large for the engine: FreeType scales the unhinted glyph.
            self.load_glyph(glyph)?.scaled(x_scale, y_scale)
        } else {
            let glyph = self.load(glyph, rendering)?;
            Type1Glyph {
                advance: mul_fix(glyph.advance, x_scale),
                linear_advance: mul_div(glyph.linear_advance, x_scale, 64),
                ..glyph
            }
        };
        loaded.advance = pix_round(loaded.advance);
        Ok(loaded)
    }

    /// Decodes `glyph` and applies the font matrix, as `T1_Load_Glyph`
    /// does before scaling.
    fn load(&self, glyph: GlyphId, rendering: Rendering) -> Result<Type1Glyph, Type1Error> {
        let index = usize::from(glyph.0);
        if index >= self.charstrings.len() {
            return Err(Type1Error::GlyphOutOfRange(glyph.0));
        }
        let set = self.glyph_set();
        let decoded = decode(&set, &self.charstrings[index], rendering)?;
        let mut outline = decoded.outline;
        let linear_advance = fixed_to_int(i64::from(decoded.advance.0));
        let transform = self.transform();
        let mut advance = transform.apply(&mut outline, linear_advance);
        // FreeType adds the offset in font units even to hinted outlines.
        let (dx, dy) = transform.offset;
        if dx != 0 || dy != 0 {
            outline.translate(dx as f64, dy as f64);
            advance += dx;
        }
        Ok(Type1Glyph {
            outline,
            advance,
            linear_advance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outline::{OutlineRecorder, PathCommand};
    use crate::type1::test_support::fixture;
    use crate::type1::{Type1Effects, Type1Font};

    fn commands(glyph: &Type1Glyph) -> Vec<PathCommand> {
        let mut recorder = OutlineRecorder::default();
        glyph.outline(&mut recorder);
        recorder.commands
    }

    #[test]
    fn gamma_of_cmr10() {
        let face = Type1Face::parse(&fixture("cmr10.pfb")).unwrap();
        let gamma = face.load_glyph(GlyphId(1)).unwrap();
        assert_eq!((gamma.advance(), gamma.linear_advance()), (625, 625));
        let commands = commands(&gamma);
        assert_eq!(commands[0], PathCommand::MoveTo(554.0, 680.0));
        assert_eq!(commands[1], PathCommand::LineTo(33.0, 680.0));
        assert_eq!(commands[3], PathCommand::LineTo(57.0, 649.0));
        assert_eq!(
            gamma.control_box(),
            Some(Rect {
                x_min: 33,
                y_min: 0,
                x_max: 582,
                y_max: 680
            })
        );
        let notdef = face.load_glyph(GlyphId(0)).unwrap();
        assert_eq!((notdef.advance(), notdef.control_box()), (500, None));
        assert!(face.load_glyph(GlyphId(500)).is_err());
    }

    #[test]
    fn slanted_and_scaled_glyphs() {
        let font = Type1Font::from_bytes(&fixture("cmr10.pfb")).unwrap();
        let slanted = font
            .transform(Type1Effects {
                slant: Some(0.5),
                extend: Some(0.5),
            })
            .unwrap();
        let face = Type1Face::new(&slanted).unwrap();
        let gamma = face.load_glyph(GlyphId(1)).unwrap();
        // x' = 0.5 x + 0.5 y: (554, 680) becomes (617, 680).
        assert_eq!(commands(&gamma)[0], PathCommand::MoveTo(617.0, 680.0));
        assert_eq!((gamma.advance(), gamma.linear_advance()), (313, 625));
        let scaled = gamma.scaled(0x2_0000, 0x1_0000);
        assert_eq!(commands(&scaled)[0], PathCommand::MoveTo(1234.0, 680.0));
        assert_eq!(scaled.advance(), 626);
    }

    #[test]
    fn hinted_glyphs() {
        let face = Type1Face::parse(&fixture("cmr10.pfb")).unwrap();
        let size = face.size(100.0, 72.0);
        assert_eq!(size.x_scale(), 419_430);
        let gamma = face.load_glyph_hinted(GlyphId(1), &size).unwrap();
        // 62.5 pixels, rounded.
        assert_eq!(gamma.advance(), 64 * 63);
        // Beyond 2000 pixels per em the engine leaves glyphs unhinted.
        let huge = face.size(3000.0, 72.0);
        let large = face.load_glyph_hinted(GlyphId(1), &huge).unwrap();
        let unhinted = face.load_glyph(GlyphId(1)).unwrap();
        assert_eq!(
            large.control_box(),
            unhinted
                .scaled(huge.x_scale(), huge.y_scale())
                .control_box()
        );
    }
}
