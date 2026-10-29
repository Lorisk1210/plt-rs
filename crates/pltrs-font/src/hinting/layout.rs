//! Hinted glyph metrics for text layout: what HarfBuzz's FreeType
//! integration and matplotlib's `FT2Font.set_text` read from glyphs loaded
//! with hinting, by the bytecode interpreter or the autohinter.

use std::cell::RefCell;
use std::collections::HashMap;

use super::loader::HintedGlyph;
use super::program::HintingInstance;
use crate::autofit::{AutohintTarget, Autohinter};
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

/// What hints the glyphs of a [`HintedFace`].
#[derive(Debug)]
enum Loader<'s, 'f> {
    Bytecode(Box<HintingInstance<'f>>),
    Autohinter(&'s Autohinter, Size),
}

/// One face at one size hinted for laying out a string: glyphs are hinted
/// on first use and their metrics kept, like HarfBuzz's advance cache.
#[derive(Debug)]
pub(crate) struct HintedFace<'s, 'f> {
    face: &'s Face<'f>,
    loader: Loader<'s, 'f>,
    metrics: RefCell<HashMap<GlyphId, Result<GlyphMetrics>>>,
}

impl<'s, 'f> HintedFace<'s, 'f> {
    /// `face` hinted at `size` by its bytecode.
    pub(crate) fn bytecode(face: &'s Face<'f>, size: &Size) -> Self {
        let instance = Box::new(HintingInstance::new(face, size));
        Self::with_loader(face, Loader::Bytecode(instance))
    }

    /// `face` autohinted at `size` (with matplotlib's normal target) by
    /// `hinter`, its analysis.
    pub(crate) fn autohinted(face: &'s Face<'f>, size: &Size, hinter: &'s Autohinter) -> Self {
        Self::with_loader(face, Loader::Autohinter(hinter, *size))
    }

    fn with_loader(face: &'s Face<'f>, loader: Loader<'s, 'f>) -> Self {
        Self {
            face,
            loader,
            metrics: RefCell::new(HashMap::new()),
        }
    }

    fn load(&self, glyph: GlyphId) -> Result<HintedGlyph> {
        match &self.loader {
            Loader::Bytecode(instance) => instance.load_glyph(self.face, glyph),
            Loader::Autohinter(hinter, size) => {
                hinter.load_glyph(self.face, size, glyph, AutohintTarget::Normal)
            }
        }
    }

    fn metrics(&self, glyph: GlyphId) -> Result<GlyphMetrics> {
        self.metrics
            .borrow_mut()
            .entry(glyph)
            .or_insert_with(|| {
                self.load(glyph).map(|hinted| GlyphMetrics {
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
    /// (`ft_glyphslot_grid_fit_metrics`, and the autohinter's own glyph
    /// metrics).
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
