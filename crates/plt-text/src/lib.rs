//! Text shaping, layout and mathtext typesetting.
//!
//! * [`TextEngine`] — the shared font catalogue; lays out single lines of
//!   text exactly as matplotlib's FreeType/HarfBuzz pipeline does and
//!   provides glyph outlines to backends.
//! * [`TextRun`] — laid-out glyphs (and rules) in pixels, independent of
//!   the output backend, with the [`TextSource`] they were laid out from.
//! * [`Shaping`] — the OpenType features and [`Language`] a line is shaped
//!   with, as matplotlib's backends receive them from a text artist.
//! * [`layout_block`] — matplotlib's multi-line stacking, alignment and
//!   rotation of a text block.
//! * [`mathtext`] — matplotlib's TeX-like math typesetting.
//! * [`fontconfig`] — the fontconfig patterns font properties are written in.
//! * AFM text metrics — [`FontRequest::afm_line_metrics`] and
//!   [`FontRequest::afm_height_metrics`] measure text in the standard
//!   PostScript fonts as matplotlib's PDF and PostScript backends do with
//!   `pdf.use14corefonts` and `ps.useafm`.
#![forbid(unsafe_code)]

mod afm;
mod block;
mod engine;
mod error;
pub mod fontconfig;
pub mod mathtext;
mod run;
mod shaping;

pub use block::{
    BlockLayout, BlockStyle, FontHeightMetrics, HAlign, LineMetrics, LinePlacement, LineSpacing,
    RotationMode, VAlign, layout_block,
};
pub use engine::{FontRequest, TextEngine};
pub use error::TextError;
pub use plt_font::{
    AfmDirectory, AfmFont, AfmMetrics, Feature, FontDatabase, FontEntry, FontId, FontQuery,
    FontSource, FontStyle, FontVariant, GenericFamilies, GlyphId, Hinting, LanguageRange,
    LanguageTag, ParseFeatureError, Stretch, Weight, bundled_font_data,
};
pub use run::{GlyphChars, PlacedGlyph, TextAnchor, TextRun, TextSource};
pub use shaping::{Language, Shaping};
