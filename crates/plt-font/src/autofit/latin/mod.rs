//! The latin writing system (FreeType's `aflatin.c`): blue zones measured
//! on each script's characters, segments linked into stems and serifs,
//! edges snapped to blue zones and the pixel grid, and FreeType 2.14's
//! diacritic adjustments. The CJK writing system reuses its segments and
//! stem widths.

mod apply;
mod blues;
mod diacritics;
mod edges;
mod hint;
mod metrics;
pub(crate) mod segments;
mod separation;
pub(crate) mod stems;

pub(crate) use apply::{LatinGlyph, hints_apply, hints_init};
pub(crate) use metrics::LatinMetrics;
