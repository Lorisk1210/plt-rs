//! TrueType/OpenType parsing, glyph outlines, text shaping and font
//! management for plt-rs.
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
//! * [`shape`] / [`shape_at_size`] — libraqm/HarfBuzz-style shaping: the
//!   Unicode Bidirectional Algorithm with visual reordering of right-to-left
//!   runs, HarfBuzz's Arabic (joining forms, presentation-form fallback) and
//!   Hebrew shapers, the default OpenType features (ligatures, `GPOS`
//!   kerning with fallback to the legacy `kern` table, cursive and mark
//!   positioning), and [`FontChain`] for matplotlib's per-character font
//!   fallback and text extents.
//! * [`FontDatabase`] — the bundled matplotlib fonts plus optional system
//!   fonts, with matplotlib's `findfont` scoring.
#![forbid(unsafe_code)]

mod autofit;
mod cff;
mod db;
mod error;
mod face;
mod fixed;
mod hinting;
mod layout;
mod outline;
mod reader;
mod scale;
mod shape;
mod subset;
mod tables;
mod types;

pub use autofit::{AutohintTarget, Autohinter};
pub use db::{
    AfmFont, FontDatabase, FontEntry, FontId, FontQuery, FontSource, FontStyle, FontVariant,
    GenericFamilies, GenericFamily, Stretch, Weight,
};
pub use error::{FontError, Result};
pub use face::{Face, StyleFlags};
pub use hinting::{BytecodeError, BytecodeProgram, HintedGlyph, Hinter, Hinting, HintingInstance};
pub use layout::feature::{Feature, ParseFeatureError};
pub use outline::{OutlineRecorder, OutlineSink, PathCommand};
pub use scale::{KerningMode, Size};
pub use shape::{
    FontChain, LaidOutGlyph, Script, ShapeOptions, ShapedGlyph, TextLayout, shape, shape_at_size,
    shape_with,
};
pub use subset::{FontSubset, subset};
pub use tables::cmap::CharmapInfo;
pub use tables::directory::{SfntFlavor, face_count};
pub use tables::head::{Head, Hhea};
pub use tables::name::NameKey;
pub use tables::os2::{Os2, ScriptMetrics};
pub use tables::post::Post;
pub use types::{Direction, GlyphId, Rect, Tag};
