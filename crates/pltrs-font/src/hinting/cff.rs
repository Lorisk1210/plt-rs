//! Glyphs of CFF faces hinted as FreeType's CFF driver hints them: by the
//! Adobe engine, with the metrics grid-fitted afterwards.

use super::loader::HintedGlyph;
use crate::error::{FontError, Result};
use crate::face::Face;
use crate::fixed::pix_round;
use crate::scale::Size;
use crate::scan::ScanRules;
use crate::types::{GlyphId, Tag};

impl Face<'_> {
    /// Loads `glyph` at `size` as FreeType's `FT_Load_Glyph` loads a CFF
    /// glyph it hints natively ([`Hinter::Cff`](crate::Hinter::Cff)): the
    /// Adobe engine aligns horizontal stems to the alignment zones and the
    /// pixel grid, and the advance is rounded to whole pixels
    /// (`ft_glyphslot_grid_fit_metrics`).
    ///
    /// # Errors
    ///
    /// When the face has no CFF outlines, the glyph is beyond the face's
    /// glyphs, or its charstring is malformed.
    pub fn load_cff_hinted(&self, size: &Size, glyph: GlyphId) -> Result<HintedGlyph> {
        let (outline, advance) = self
            .load_cff_outline_hinted(glyph, size.x_scale(), size.y_scale())?
            .ok_or(FontError::MissingTable(Tag::new(b"CFF ")))?;
        let advance = i32::try_from(pix_round(advance))
            .map_err(|_| FontError::malformed(Tag::new(b"hmtx"), "advance out of range"))?;
        Ok(HintedGlyph::new(
            outline,
            advance,
            ScanRules::scaled(size.y_ppem()),
        ))
    }
}
