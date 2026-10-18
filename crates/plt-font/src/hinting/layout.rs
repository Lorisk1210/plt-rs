//! Hinted glyph metrics for text layout: what HarfBuzz's FreeType
//! integration and matplotlib's `FT2Font.set_text` read from glyphs loaded
//! with hinting.

use std::cell::RefCell;
use std::collections::HashMap;

use super::program::HintingInstance;
use crate::error::Result;
use crate::face::Face;
use crate::fixed::{pix_ceil, pix_floor};
use crate::scale::Size;
use crate::types::{GlyphId, Rect};

/// The metrics of one hinted glyph that layout uses.
#[derive(Clone, Copy, Debug)]
struct GlyphMetrics {
    advance: i32,
    control_box: Option<Rect>,
}

/// One face at one size hinted for laying out a string: glyphs are hinted
/// on first use and their metrics kept, like HarfBuzz's advance cache.
#[derive(Debug)]
pub(crate) struct HintedFace<'s, 'f> {
    face: &'s Face<'f>,
    instance: HintingInstance<'f>,
    metrics: RefCell<HashMap<GlyphId, Result<GlyphMetrics>>>,
}

impl<'s, 'f> HintedFace<'s, 'f> {
    /// `face` hinted at `size`.
    pub(crate) fn new(face: &'s Face<'f>, size: &Size) -> Self {
        Self {
            face,
            instance: HintingInstance::new(face, size),
            metrics: RefCell::new(HashMap::new()),
        }
    }

    fn metrics(&self, glyph: GlyphId) -> Result<GlyphMetrics> {
        self.metrics
            .borrow_mut()
            .entry(glyph)
            .or_insert_with(|| {
                self.instance
                    .load_glyph(self.face, glyph)
                    .map(|hinted| GlyphMetrics {
                        advance: hinted.advance(),
                        control_box: hinted.control_box(),
                    })
            })
            .clone()
    }

    /// The hinted advance in 26.6 pixels; zero for glyphs that fail to
    /// load, as HarfBuzz ignores `FT_Get_Advance` errors.
    pub(crate) fn advance(&self, glyph: GlyphId) -> i32 {
        self.metrics(glyph).map_or(0, |metrics| metrics.advance)
    }

    /// The control box of the hinted outline in 26.6 pixels.
    pub(crate) fn control_box(&self, glyph: GlyphId) -> Result<Option<Rect>> {
        self.metrics(glyph).map(|metrics| metrics.control_box)
    }

    /// The ink extents FreeType reports for a hinted glyph: the control
    /// box with its edges moved out to whole pixels
    /// (`ft_glyphslot_grid_fit_metrics`).
    pub(crate) fn extents(&self, glyph: GlyphId) -> Result<Option<Rect>> {
        let fit = |v: i32, round: fn(i64) -> i64| round(i64::from(v)) as i32;
        Ok(self.control_box(glyph)?.map(|r| Rect {
            x_min: fit(r.x_min, pix_floor),
            y_min: fit(r.y_min, pix_floor),
            x_max: fit(r.x_max, pix_ceil),
            y_max: fit(r.y_max, pix_ceil),
        }))
    }
}
