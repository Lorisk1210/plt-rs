//! TrueType/OpenType parsing, glyph outlines, text shaping and font
//! management for pltrs.
//!
//! The crate has no dependencies and is organised in layers:
//!
//! * [`Face`] — a zero-copy view of one face of a `.ttf`/`.otf`/`.ttc` file:
//!   metrics, character maps, glyph outlines (TrueType `glyf` and `CFF`)
//!   emitted through an [`OutlineSink`], names, kerning and glyph names.
//! * [`Size`] — FreeType-compatible scaling, so scaled advances, outlines
//!   and bounding boxes agree with matplotlib to the 1/64 pixel.
//! * [`HintingInstance`] — FreeType's TrueType bytecode interpreter (version
//!   40) for the grid-fitted outlines and advances matplotlib draws text
//!   with by default ([`Hinting`]), and [`Autohinter`] — FreeType's
//!   autohinter, for fonts without bytecode and `text.hinting: auto`.
//!   Loaded glyphs also give their [`ScanOutline`]: the tagged points and
//!   drop-out rules FreeType's monochrome rasterizer works from.
//! * [`shape`] / [`shape_at_size`] — libraqm/HarfBuzz-style shaping: the
//!   Unicode Bidirectional Algorithm with visual reordering of right-to-left
//!   runs, HarfBuzz's Arabic (joining forms, presentation-form fallback) and
//!   Hebrew shapers, the default OpenType features (ligatures, `GPOS`
//!   kerning with fallback to the legacy `kern` table, cursive and mark
//!   positioning), user [`Feature`]s in HarfBuzz's syntax (also for parts
//!   of the text), BCP 47 languages ([`LanguageTag`], [`LanguageRange`])
//!   mapped to the font's language systems with HarfBuzz's tag tables, and
//!   [`FontChain`] for matplotlib's per-character font fallback and text
//!   extents.
//! * [`FontDatabase`] — the bundled matplotlib fonts plus optional system
//!   fonts, with matplotlib's `findfont` scoring; the system's fonts can
//!   be catalogued once and the catalogue kept in a file between processes
//!   ([`FontDatabase::add_system_fonts_catalogued`]), as matplotlib keeps
//!   its font list, so that starting up opens no font file.
//! * [`AfmFont`] / [`AfmMetrics`] — matplotlib's catalogue of Adobe Font
//!   Metrics and its AFM parser, with the bundled metrics of all of them
//!   (the 14 PDF core fonts and matplotlib's `fonts/afm`), for text
//!   measured and drawn in standard PostScript fonts.
//! * [`glyph_name_to_unicode`] — the characters a glyph name stands for,
//!   by the Adobe Glyph List.
//! * [`Type1Font`] / [`Type1Face`] — PostScript Type 1 fonts
//!   (`.pfa`/`.pfb`), the fonts of TeX and `text.usetex`: matplotlib's
//!   `_type1font` (parsing, slanting, extending, re-encoding and subsetting
//!   for embedding) and FreeType's Type 1 driver (glyph names and indices,
//!   metrics, and outlines from its Adobe charstring engine, hinted or
//!   not).
//!
//! What the crate ports from FreeType 2.14 it takes under the FreeType
//! License (FTL), not FreeType's alternative, the GPL; the modules that
//! port its drivers and engines say so, and `THIRD_PARTY.md` lists them.
#![forbid(unsafe_code)]

mod aat;
mod afm;
mod agl;
mod autofit;
mod cff;
mod db;
mod error;
mod face;
mod fixed;
mod hinting;
mod layout;
mod matrix;
mod outline;
mod psaux;
mod reader;
mod region;
mod scale;
mod scan;
mod sfnt;
mod shape;
mod subset;
mod tables;
mod type1;
mod types;
mod var;

pub use afm::{
    AfmError, AfmMetrics, AfmSection, CharMetrics, CompositePart, StrBbox, bundled_afm_files,
    bundled_metrics, type1_name, uni2type1,
};
pub use agl::glyph_name_to_unicode;
pub use autofit::{AutohintTarget, Autohinter};
pub use db::{
    AfmChoice, AfmDirectory, AfmEntry, AfmFont, AfmStyle, CatalogueReport, CatalogueState,
    FaceStyle, FontDatabase, FontEntry, FontId, FontQuery, FontSource, FontStyle, FontVariant,
    GenericFamilies, GenericFamily, RegistryReader, Stretch, Weight, alternative_family_names,
    bundled_font_data, bundled_font_files, catalogue_file_name, face_style, read_registry_with,
    score_family, score_size, score_stretch, score_style, score_variant, score_weight,
};
pub use error::{FontError, Result};
pub use face::{Face, StyleFlags};
pub use hinting::{
    BytecodeError, BytecodeProgram, GlyphMetrics, HintedGlyph, Hinter, Hinting, HintingInstance,
    VerticalMetrics,
};
pub use layout::feature::{Feature, ParseFeatureError};
pub use outline::{OutlineRecorder, OutlineSink, PathCommand};
pub use scale::{KerningMode, Size};
pub use scan::{Dropouts, ScanOutline, ScanPoint, ScanPointKind, ScanRules};
pub use sfnt::FaceFlags;
pub use shape::{
    FontChain, LaidOutGlyph, LanguageRange, LanguageTag, Script, ShapeOptions, ShapedGlyph,
    TextLayout, shape, shape_at_size, shape_with,
};
pub use subset::{FontSubset, subset};
pub use tables::cmap::CharmapInfo;
pub use tables::directory::{SfntFlavor, face_count};
pub use tables::head::{Head, Hhea, MaxProfile};
pub use tables::name::{NameEntry, NameKey};
pub use tables::os2::{Os2, ScriptMetrics};
pub use tables::pclt::Pclt;
pub use tables::post::Post;
pub use type1::{
    Abbreviations, CharStrings, Program, PsNumber, PsToken, PsTokenKind, PsTokenStream, PsValue,
    Type1Cipher, Type1Effects, Type1Encoding, Type1Error, Type1Face, Type1Font, Type1Glyph,
    Type1Parts, Type1Properties, Type1Section, ps_tokenize,
};
pub use types::{Direction, GlyphId, Rect, Tag};
pub use var::{NamedInstance, VariationAxis};
