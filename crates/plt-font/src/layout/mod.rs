//! OpenType Layout: `GDEF`, `GSUB` and `GPOS` parsing and application.
//!
//! The engine follows HarfBuzz's semantics closely (lookup flags, glyph
//! skipping, ligature component tracking, nested contextual lookups, mark
//! attachment and offset propagation) because matplotlib shapes text with
//! HarfBuzz and text widths must match.

pub(crate) mod apply;
pub(crate) mod buffer;
pub(crate) mod common;
pub(crate) mod context;
pub(crate) mod gdef;
pub(crate) mod gpos;
pub(crate) mod gsub;
pub(crate) mod plan;
