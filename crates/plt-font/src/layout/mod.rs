//! OpenType Layout: `GDEF`, `GSUB` and `GPOS` parsing and application.
//!
//! The engine follows HarfBuzz's semantics closely (feature maps with
//! stages and per-glyph masks, lookup flags, glyph and joiner skipping,
//! ligature component tracking, nested contextual lookups, cursive and
//! mark attachment) because matplotlib shapes text with HarfBuzz and text
//! widths must match.

pub(crate) mod apply;
pub(crate) mod buffer;
pub(crate) mod collect;
pub(crate) mod common;
pub(crate) mod context;
pub(crate) mod feature;
pub(crate) mod gdef;
pub(crate) mod gpos;
pub(crate) mod gsub;
pub(crate) mod map;
pub(crate) mod synthetic;
