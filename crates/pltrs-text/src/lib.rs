//! Text shaping, layout, mathtext typesetting and TeX typesetting.
//!
//! * [`TextEngine`]: the shared font catalogue. It lays out single lines
//!   of text as the FreeType/HarfBuzz pipeline of matplotlib does, and
//!   gives glyph outlines to the backends.
//! * [`TextRun`]: laid-out glyphs (and rules) in pixels, for any backend,
//!   with the [`TextSource`] of the layout.
//! * [`Shaping`]: the OpenType features and the [`Language`] for shaping a
//!   line, as the matplotlib backends get them from a text artist.
//! * [`layout_block`]: the matplotlib stacking, alignment and rotation of a
//!   text block with several lines.
//! * [`mathtext`]: the TeX-like math typesetting of matplotlib.
//! * [`fontconfig`]: the fontconfig patterns of font properties.
//! * [`tex`]: `text.usetex`, which typesets with LaTeX and reads its DVI
//!   output (the matplotlib `texmanager` and `dviread`).
//! * AFM text metrics: [`FontRequest::afm_line_metrics`] and
//!   [`FontRequest::afm_height_metrics`] measure text in the standard
//!   PostScript fonts. The matplotlib PDF and PostScript backends do this
//!   with `pdf.use14corefonts` and `ps.useafm`.
#![forbid(unsafe_code)]

mod afm;
mod block;
mod deep;
mod engine;
mod error;
pub mod fontconfig;
mod lookups;
pub mod mathtext;
mod mathtext_log;
mod memo;
mod missing;
mod run;
mod shaping;
pub mod tex;

/// The harness of pltrs's fuzz tests (`pltrs_core::fuzz`), driving the
/// unit tests that fuzz TeX's file formats.
#[cfg(test)]
use pltrs_core::fuzz;

pub use block::{
    BlockLayout, BlockStyle, FontHeightMetrics, HAlign, LineMetrics, LinePlacement, LineSpacing,
    RotationMode, VAlign, layout_block,
};
pub use engine::{FontRequest, PathSink, TEXT_PATH_DPI, TEXT_PATH_FONT_SCALE, TextEngine};
pub use error::{SET_CHAR_SIZE_LINE, TextError};
pub use lookups::{
    FontLookup, FontNote, LookupKind, LookupRun, keep_every_lookup, take_font_notes,
};
pub use mathtext_log::{MathNote, log_mathtext, take_mathtext_log};
pub use missing::{MissingGlyph, take_missing_glyphs};
pub use pltrs_font::{
    AfmChoice, AfmDirectory, AfmEntry, AfmFont, AfmMetrics, CatalogueReport, CatalogueState,
    Feature, FontDatabase, FontEntry, FontId, FontQuery, FontSource, FontStyle, FontVariant,
    GenericFamilies, GlyphId, Hinting, LanguageRange, LanguageTag, ParseFeatureError,
    RegistryReader, Stretch, Type1Effects, Type1Encoding, Type1Face, Type1Font, Type1Glyph, Weight,
    bundled_afm_files, bundled_font_data, bundled_font_files, catalogue_file_name,
    is_modern_look_font, read_registry_with, score_family, score_size, score_stretch, score_style,
    score_variant, score_weight,
};
pub use run::{GlyphChars, PlacedGlyph, TextAnchor, TextRun, TextSource};
pub use shaping::{Language, Shaping};
