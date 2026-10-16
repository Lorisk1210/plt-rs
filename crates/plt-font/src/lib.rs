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
//! * [`shape`] / [`shape_at_size`] — HarfBuzz-style shaping with the
//!   default OpenType features (ligatures, `GPOS` kerning with fallback to
//!   the legacy `kern` table, mark positioning), and [`FontChain`] for
//!   matplotlib's per-character font fallback and text extents.
//! * [`FontDatabase`] — the bundled matplotlib fonts plus optional system
//!   fonts, with matplotlib's `findfont` scoring.
#![forbid(unsafe_code)]

mod cff;
mod db;
mod error;
mod face;
mod fixed;
mod layout;
mod outline;
mod reader;
mod scale;
mod shape;
mod tables;
mod types;

pub use db::{
    FontDatabase, FontEntry, FontId, FontQuery, FontSource, FontStyle, FontVariant,
    GenericFamilies, GenericFamily, Stretch, Weight,
};
pub use error::{FontError, Result};
pub use face::{Face, StyleFlags};
pub use layout::plan::{Feature, ParseFeatureError};
pub use outline::{OutlineRecorder, OutlineSink, PathCommand};
pub use scale::{KerningMode, Size};
pub use shape::{
    FontChain, LaidOutGlyph, Script, ShapeOptions, ShapedGlyph, TextLayout, shape, shape_at_size,
    shape_with,
};
pub use tables::cmap::CharmapInfo;
pub use tables::directory::{SfntFlavor, face_count};
pub use tables::head::{Head, Hhea};
pub use tables::name::NameKey;
pub use tables::os2::{Os2, ScriptMetrics};
pub use tables::post::Post;
pub use types::{GlyphId, Rect, Tag};
