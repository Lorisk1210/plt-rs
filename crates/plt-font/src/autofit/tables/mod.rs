//! The autohinter's static data: writing systems, scripts, styles,
//! coverages (`afws-iter.h`, `afscript.h`, `afstyles.h`, `afcover.h`), the
//! blue zone strings (`afblue.dat`), the Unicode ranges of every script
//! (`afranges.c`) and the diacritic adjustment database (`afadjust.c`).
//!
//! The tables themselves are generated from FreeType's sources by
//! `tools/gen_autofit_tables.py`; this module defines their types.

mod adjustments;
mod blues;
mod ranges;
mod scripts;

pub(crate) use adjustments::ADJUSTMENTS;
pub(crate) use scripts::{DEFAULT_SCRIPT, FALLBACK_STYLE, NONE_STYLE, SCRIPTS, STYLES};

use crate::types::Tag;

/// A range of Unicode code points, `first..=last`.
pub(crate) type UniRange = (u32, u32);

/// FreeType's writing systems: families of scripts hinted alike.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WritingSystem {
    /// No hinting, only scaling (`afdummy.c`).
    Dummy,
    /// Latin-like scripts with blue zones (`aflatin.c`).
    Latin,
    /// CJK ideographs (`afcjk.c`).
    Cjk,
    /// Indic scripts, hinted with the CJK routines without blue zones
    /// (`afindic.c`).
    Indic,
}

/// The OpenType feature that selects a style's glyphs (`afcover.h`;
/// FreeType's `ruby` coverage belongs to no style and is left out).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Coverage {
    /// `c2cp`.
    PetiteCapitalsFromCapitals,
    /// `c2sc`.
    SmallCapitalsFromCapitals,
    /// `ordn`.
    Ordinals,
    /// `pcap`.
    PetiteCapitals,
    /// `sinf`.
    ScientificInferiors,
    /// `smcp`.
    SmallCapitals,
    /// `subs`.
    Subscript,
    /// `sups`.
    Superscript,
    /// `titl`.
    Titling,
    /// Everything no other coverage claims, including the glyphs of the
    /// character map.
    Default,
}

impl Coverage {
    /// The feature tag; `None` for the default coverage.
    pub(crate) const fn feature(self) -> Option<Tag> {
        let tag = match self {
            Self::PetiteCapitalsFromCapitals => b"c2cp",
            Self::SmallCapitalsFromCapitals => b"c2sc",
            Self::Ordinals => b"ordn",
            Self::PetiteCapitals => b"pcap",
            Self::ScientificInferiors => b"sinf",
            Self::SmallCapitals => b"smcp",
            Self::Subscript => b"subs",
            Self::Superscript => b"sups",
            Self::Titling => b"titl",
            Self::Default => return None,
        };
        Some(Tag::new(tag))
    }
}

/// One script (`AF_ScriptClassRec`).
#[derive(Debug)]
pub(crate) struct ScriptClass {
    /// HarfBuzz's script (its ISO 15924 tag); `None` for FreeType's
    /// fallback scripts.
    pub(crate) harfbuzz: Option<Tag>,
    /// Whether vertical edges are hinted top to bottom.
    pub(crate) top_to_bottom: bool,
    /// Space-separated characters giving the standard stem widths.
    pub(crate) standard_chars: &'static str,
    /// The script's characters.
    pub(crate) ranges: &'static [UniRange],
    /// Its characters blue zones do not apply to (a subset of `ranges`).
    pub(crate) nonbase_ranges: &'static [UniRange],
}

impl ScriptClass {
    const fn new(
        harfbuzz: Option<Tag>,
        top_to_bottom: bool,
        standard_chars: &'static str,
        ranges: &'static [UniRange],
        nonbase_ranges: &'static [UniRange],
    ) -> Self {
        Self {
            harfbuzz,
            top_to_bottom,
            standard_chars,
            ranges,
            nonbase_ranges,
        }
    }
}

/// One style: a script hinted with a writing system, for the glyphs of a
/// coverage (`AF_StyleClassRec`).
#[derive(Debug)]
pub(crate) struct StyleClass {
    pub(crate) writing_system: WritingSystem,
    /// Index in [`SCRIPTS`].
    pub(crate) script: usize,
    /// The strings blue zones are measured on.
    pub(crate) blues: &'static [BlueString],
    pub(crate) coverage: Coverage,
}

impl StyleClass {
    const fn new(
        writing_system: WritingSystem,
        script: usize,
        blues: &'static [BlueString],
        coverage: Coverage,
    ) -> Self {
        Self {
            writing_system,
            script,
            blues,
            coverage,
        }
    }

    /// The style's script.
    pub(crate) fn script(&self) -> &'static ScriptClass {
        &SCRIPTS[self.script]
    }
}

/// The characters one blue zone is measured on (`AF_Blue_StringRec`):
/// clusters separated by spaces, plus the zone's [`blue`] properties.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BlueString {
    pub(crate) chars: &'static str,
    pub(crate) properties: u8,
}

impl BlueString {
    const fn new(chars: &'static str, properties: u8) -> Self {
        Self { chars, properties }
    }

    /// Whether the zone has property `property`.
    pub(crate) const fn has(self, property: u8) -> bool {
        self.properties & property != 0
    }
}

/// Blue zone properties (`AF_BLUE_PROPERTY_*`).
pub(crate) mod blue {
    /// A top zone (latin).
    pub(crate) const LATIN_TOP: u8 = 1 << 0;
    /// A subscript top zone (latin).
    pub(crate) const LATIN_SUB_TOP: u8 = 1 << 1;
    /// A zone aligning edges of either direction (latin).
    pub(crate) const LATIN_NEUTRAL: u8 = 1 << 2;
    /// The x-height zone, used to adjust the vertical scale (latin).
    pub(crate) const LATIN_X_HEIGHT: u8 = 1 << 3;
    /// Measured on long horizontal segments only (latin).
    pub(crate) const LATIN_LONG: u8 = 1 << 4;
    /// The bottom zone of capital letters (latin).
    pub(crate) const LATIN_CAPITAL_BOTTOM: u8 = 1 << 5;
    /// The bottom zone of small letters (latin).
    pub(crate) const LATIN_SMALL_BOTTOM: u8 = 1 << 6;
    /// A top zone (CJK).
    pub(crate) const CJK_TOP: u8 = 1 << 0;
}

/// Diacritic adjustment flags of the [`ADJUSTMENTS`] database
/// (`AF_ADJUST_*`, `AF_IGNORE_*`).
pub(crate) mod adjust {
    /// Push the topmost contour up, one pixel clear of the rest.
    pub(crate) const UP: u16 = 0x01;
    /// Push the bottommost contour down, one pixel clear of the rest.
    pub(crate) const DOWN: u16 = 0x02;
    /// Push the contour below the topmost one (and the topmost) up.
    pub(crate) const UP2: u16 = 0x04;
    /// Push the contour above the bottommost one (and the bottommost)
    /// down (no database entry uses it).
    pub(crate) const DOWN2: u16 = 0x08;
    /// The topmost contour is a tilde, to be enlarged vertically.
    pub(crate) const TILDE_TOP: u16 = 0x10;
    /// The bottommost contour is a tilde, to be enlarged vertically.
    pub(crate) const TILDE_BOTTOM: u16 = 0x20;
    /// The contour below the topmost one is a tilde.
    pub(crate) const TILDE_TOP2: u16 = 0x40;
    /// The contour above the bottommost one is a tilde (no database entry
    /// uses it).
    pub(crate) const TILDE_BOTTOM2: u16 = 0x80;
    /// Ignore blue zones above an uppercase base character.
    pub(crate) const IGNORE_CAPITAL_TOP: u16 = 0x100;
    /// Ignore blue zones below an uppercase base character.
    pub(crate) const IGNORE_CAPITAL_BOTTOM: u16 = 0x200;
    /// Ignore blue zones above a lowercase base character.
    pub(crate) const IGNORE_SMALL_TOP: u16 = 0x400;
    /// Ignore blue zones below a lowercase base character.
    pub(crate) const IGNORE_SMALL_BOTTOM: u16 = 0x800;
    /// Adjust whatever the height of the diacritic.
    pub(crate) const NO_HEIGHT_CHECK: u16 = 0x1000;
}
