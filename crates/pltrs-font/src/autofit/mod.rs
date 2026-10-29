//! FreeType's autohinter ("autofitter", `src/autofit`), as matplotlib's
//! bundled FreeType 2.14.3 runs it: built with HarfBuzz, with the CJK and
//! Indic writing systems, without stem darkening (off by default, and
//! matplotlib cannot turn it on).
//!
//! The port follows FreeType's modules:
//!
//! * [`Autohinter`] (`afglobal.c`, `afmodule.c`) — a face's analysis: each
//!   glyph's style, from the Unicode ranges of FreeType's scripts
//!   (`afranges.c`, `afscript.h`, `afstyles.h`) and the glyphs `GSUB`
//!   features reach (`afshaper.c` with HarfBuzz), and each style's global
//!   metrics, computed on first use.
//! * `loader` (`afloader.c`) — [`Autohinter::load_glyph`]: hinting, side
//!   bearings and the advance width.
//! * `hints` (`afhints.c`) — points, segments and edges, and the point
//!   alignment after edges are fitted.
//! * `latin` (`aflatin.c`, `afadjust.c`) — blue zones, stems and serifs,
//!   and FreeType 2.14's diacritic adjustments; `cjk` (`afcjk.c`,
//!   `afindic.c`); the dummy writing system (`afdummy.c`) only scales.
//! * `shaper`, `gsub`, `adjust` — HarfBuzz shaping of blue strings and
//!   feature coverage, the `GSUB` substitutions diacritics follow
//!   (`afgsub.c`) and the adjustment database (`afadjust.c`).
//!
//! FreeType's quirks are kept wherever they change results (they are
//! pointed out where they occur). Like FreeType, the analysis of a face is
//! computed once and reused for every size: an [`Autohinter`] owns its
//! data and is meant to be cached next to its font.
//!
//! Font data is untrusted: glyph outlines are loaded with the crate's
//! bounded loaders, and every loop over points, segments, edges or
//! substitutions is bounded by the face's sizes.
//!
//! The autofitter is copyright David Turner, Robert Wilhelm and Werner
//! Lemberg, and ported here under the FreeType License (FTL).

mod adjust;
mod cjk;
mod cmap;
mod globals;
mod gsub;
mod hash;
mod hints;
mod latin;
mod loader;
mod metrics;
mod shaper;
mod tables;
#[cfg(test)]
mod tests;
mod widths;

pub use globals::Autohinter;

/// FreeType's rendering target (`FT_LOAD_TARGET_*`), which tunes the
/// autohinter: light hinting only fits heights, LCD targets snap stems on
/// their subpixel axis, monochrome snaps everything to whole pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum AutohintTarget {
    /// Anti-aliased rendering (`FT_LOAD_TARGET_NORMAL`), what matplotlib
    /// uses.
    #[default]
    Normal,
    /// Vertical hinting only, with fractional advances kept on the grid
    /// (`FT_LOAD_TARGET_LIGHT`).
    Light,
    /// Monochrome rendering (`FT_LOAD_TARGET_MONO`).
    Mono,
    /// Horizontal LCD subpixels (`FT_LOAD_TARGET_LCD`).
    Lcd,
    /// Vertical LCD subpixels (`FT_LOAD_TARGET_LCD_V`).
    LcdVertical,
}

impl AutohintTarget {
    /// The latin and CJK hinters' flags for this target: stem snapping
    /// for monochrome and the LCD axis, stem adjustment unless light or
    /// horizontal LCD.
    pub(crate) const fn hinter_flags(self) -> u32 {
        use hints::other_flag::{HORZ_SNAP, MONO, STEM_ADJUST, VERT_SNAP};
        match self {
            Self::Normal => STEM_ADJUST,
            Self::Light => 0,
            Self::Mono => HORZ_SNAP | VERT_SNAP | STEM_ADJUST | MONO,
            Self::Lcd => HORZ_SNAP,
            Self::LcdVertical => VERT_SNAP | STEM_ADJUST,
        }
    }
}
