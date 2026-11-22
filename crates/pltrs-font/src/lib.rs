//! TrueType and OpenType parsing, glyph outlines, text shaping and font
//! management for pltrs.
//!
//! The crate has no dependencies. It has these layers:
//!
//! * [`Face`]: a zero-copy view of one face of a `.ttf`, `.otf` or `.ttc`
//!   file. It gives metrics, character maps, names, kerning, glyph names
//!   and glyph outlines (TrueType `glyf` and `CFF`) through an
//!   [`OutlineSink`].
//! * [`Size`]: scaling as in FreeType. Scaled advances, outlines and
//!   bounding boxes agree with matplotlib to 1/64 pixel.
//! * [`HintingInstance`]: the FreeType TrueType bytecode interpreter
//!   (version 40). It gives the grid-fitted outlines and advances that
//!   matplotlib uses for text by default ([`Hinting`]).
//! * [`Autohinter`]: the FreeType autohinter, for fonts without bytecode
//!   and for `text.hinting: auto`. A loaded glyph also gives its
//!   [`ScanOutline`], the tagged points and drop-out rules for the
//!   monochrome rasterizer of FreeType.
//! * [`shape`], [`shape_at_size`]: shaping as in libraqm and HarfBuzz.
//!   This includes the Unicode Bidirectional Algorithm with visual
//!   reordering of right-to-left runs and the HarfBuzz Arabic and Hebrew
//!   shapers. It applies the default OpenType features: ligatures, `GPOS`
//!   kerning (or the legacy `kern` table), and cursive and mark
//!   positioning. It accepts user [`Feature`]s in HarfBuzz syntax, also
//!   for parts of the text. BCP 47 languages ([`LanguageTag`],
//!   [`LanguageRange`]) map to the language systems of the font through
//!   the HarfBuzz tag tables. [`FontChain`] gives the matplotlib font
//!   fallback for each character and the text extents.
//! * [`FontDatabase`]: the included matplotlib fonts and optional system
//!   fonts, with the matplotlib `findfont` scoring.
//!   [`FontDatabase::add_system_fonts_catalogued`] keeps a catalogue of the
//!   system fonts in a file between processes, as matplotlib keeps its
//!   font list. Thus start-up opens no font file.
//! * [`AfmFont`], [`AfmMetrics`]: the matplotlib catalogue of Adobe Font
//!   Metrics and its AFM parser. The crate includes the metrics of the 14
//!   PDF core fonts and of the matplotlib `fonts/afm`. Text in standard
//!   PostScript fonts uses them.
//! * [`glyph_name_to_unicode`]: the characters of a glyph name, from the
//!   Adobe Glyph List.
//! * [`Type1Font`], [`Type1Face`]: PostScript Type 1 fonts (`.pfa`,
//!   `.pfb`), the fonts of TeX and `text.usetex`. `Type1Font` ports the
//!   matplotlib `_type1font`: parsing, slanting, extending, re-encoding and
//!   subsetting for embedding. `Type1Face` ports the FreeType Type 1
//!   driver: glyph names and indices, metrics, and outlines from its Adobe
//!   charstring engine, with or without hinting.
//!
//! The code ported from FreeType 2.14 uses the FreeType License (FTL), not
//! the GPL alternative of FreeType. The modules that port its drivers and
//! engines say so, and `THIRD_PARTY.md` lists them.
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
    bundled_font_data, bundled_font_files, catalogue_file_name, face_style, is_modern_look_font,
    read_registry_with, score_family, score_size, score_stretch, score_style, score_variant,
    score_weight,
};
pub use error::{FontError, Result};
pub use face::{Face, StyleFlags};
pub use hinting::{
    BytecodeError, BytecodeProgram, GlyphMetrics, HintedGlyph, Hinter, Hinting, HintingInstance,
    VerticalMetrics,
};
pub use layout::feature::{Feature, ParseFeatureError};
pub use outline::{OutlineRecorder, OutlineSink, PathCommand};
pub use scale::{KerningMode, Size, SizeError, UnscaledAdvances};
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
