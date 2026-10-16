//! Text shaping, layout and mathtext typesetting.
//!
//! * [`TextEngine`] — the shared font catalogue; lays out single lines of
//!   text exactly as matplotlib's FreeType/HarfBuzz pipeline does and
//!   provides glyph outlines to backends.
//! * [`TextRun`] — laid-out glyphs (and rules) in pixels, independent of
//!   the output backend.
//! * [`layout_block`] — matplotlib's multi-line stacking, alignment and
//!   rotation of a text block.
#![forbid(unsafe_code)]

mod block;
mod engine;
mod error;
mod run;

pub use block::{
    BlockLayout, BlockStyle, FontHeightMetrics, HAlign, LineMetrics, LinePlacement, LineSpacing,
    RotationMode, VAlign, layout_block,
};
pub use engine::TextEngine;
pub use error::TextError;
pub use plt_font::{FontId, FontQuery, FontStyle, FontVariant, GlyphId, Stretch, Weight};
pub use run::{PlacedGlyph, TextRun};
