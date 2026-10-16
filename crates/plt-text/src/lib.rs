//! Text shaping, layout and mathtext typesetting.
//!
//! * [`TextEngine`] — the shared font catalogue; lays out single lines of
//!   text exactly as matplotlib's FreeType/HarfBuzz pipeline does and
//!   provides glyph outlines to backends.
//! * [`TextRun`] — laid-out glyphs (and rules) in pixels, independent of
//!   the output backend.
//! * [`layout_block`] — matplotlib's multi-line stacking, alignment and
//!   rotation of a text block.
//! * [`mathtext`] — matplotlib's TeX-like math typesetting.
//! * [`fontconfig`] — the fontconfig patterns font properties are written in.
#![forbid(unsafe_code)]

mod block;
mod engine;
mod error;
pub mod fontconfig;
pub mod mathtext;
mod run;

pub use block::{
    BlockLayout, BlockStyle, FontHeightMetrics, HAlign, LineMetrics, LinePlacement, LineSpacing,
    RotationMode, VAlign, layout_block,
};
pub use engine::{FontRequest, TextEngine};
pub use error::TextError;
pub use plt_font::{
    AfmFont, FontDatabase, FontEntry, FontId, FontQuery, FontSource, FontStyle, FontVariant,
    GenericFamilies, GlyphId, Stretch, Weight,
};
pub use run::{PlacedGlyph, TextRun};
