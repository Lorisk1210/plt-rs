//! The fonts matplotlib ships in `mpl-data/fonts/ttf`, plus Inter (the
//! typeface of pltrs' modern look), embedded in the binary.
//!
//! Inter is copyright the Inter Project Authors and distributed under the
//! SIL Open Font License 1.1 (`fonts/LICENSE_INTER`). Its family name
//! appears in none of matplotlib's default font lists, so `findfont` only
//! picks it when a style names it.

use super::query::FontStyle;

/// The bundled fonts of pltrs' modern look (Inter), which matplotlib does
/// not ship.
pub(crate) const MODERN_LOOK_FONTS: [&str; 5] = [
    "Inter-Bold.ttf",
    "Inter-BoldItalic.ttf",
    "Inter-Italic.ttf",
    "Inter-Regular.ttf",
    "Inter-SemiBold.ttf",
];

/// A font embedded in the crate, with what the catalogue knows of it.
#[derive(Debug)]
pub(crate) struct BundledFont {
    /// The file name, as in matplotlib's `mpl-data/fonts/ttf`.
    pub(crate) file: &'static str,
    /// The font file's contents (one face).
    pub(crate) data: &'static [u8],
    /// The catalogue entries `entry::descriptions` reads from the face, kept
    /// here so that building the catalogue reads none of the embedded
    /// bytes (`db::tests` checks them against the fonts).
    pub(crate) entries: &'static [BundledEntry],
}

/// A catalogue entry of a bundled font: its family name, slant and
/// weight (all bundled fonts have the normal variant and stretch).
#[derive(Debug)]
pub(crate) struct BundledEntry {
    /// The family name.
    pub(crate) name: &'static str,
    /// The slant.
    pub(crate) style: FontStyle,
    /// The weight (100–1000).
    pub(crate) weight: u16,
}

/// A bundled font of `file` under `fonts/` and its entries.
macro_rules! bundled {
    ($file:literal, [$(($name:literal, $style:ident, $weight:literal)),+ $(,)?]) => {
        BundledFont {
            file: $file,
            data: include_bytes!(concat!("../../fonts/", $file)),
            entries: &[$(BundledEntry {
                name: $name,
                style: FontStyle::$style,
                weight: $weight,
            }),+],
        }
    };
}

/// Every bundled font, in the order matplotlib adds them (Python string
/// order of the file names: upper case sorts before lower case and `-`
/// before `.`), which decides `findfont` ties.
pub(crate) static BUNDLED_FONTS: [BundledFont; 45] = [
    bundled!("DejaVuSans-Bold.ttf", [("DejaVu Sans", Normal, 700)]),
    bundled!(
        "DejaVuSans-BoldOblique.ttf",
        [("DejaVu Sans", Oblique, 700)]
    ),
    bundled!("DejaVuSans-Oblique.ttf", [("DejaVu Sans", Oblique, 400)]),
    bundled!("DejaVuSans.ttf", [("DejaVu Sans", Normal, 400)]),
    bundled!(
        "DejaVuSansDisplay.ttf",
        [("DejaVu Sans Display", Normal, 400)]
    ),
    bundled!(
        "DejaVuSansMono-Bold.ttf",
        [("DejaVu Sans Mono", Normal, 700)]
    ),
    bundled!(
        "DejaVuSansMono-BoldOblique.ttf",
        [("DejaVu Sans Mono", Oblique, 700)]
    ),
    bundled!(
        "DejaVuSansMono-Oblique.ttf",
        [("DejaVu Sans Mono", Oblique, 400)]
    ),
    bundled!("DejaVuSansMono.ttf", [("DejaVu Sans Mono", Normal, 400)]),
    bundled!("DejaVuSerif-Bold.ttf", [("DejaVu Serif", Normal, 700)]),
    bundled!(
        "DejaVuSerif-BoldItalic.ttf",
        [("DejaVu Serif", Italic, 700)]
    ),
    bundled!("DejaVuSerif-Italic.ttf", [("DejaVu Serif", Italic, 400)]),
    bundled!("DejaVuSerif.ttf", [("DejaVu Serif", Normal, 400)]),
    bundled!(
        "DejaVuSerifDisplay.ttf",
        [("DejaVu Serif Display", Normal, 400)]
    ),
    bundled!("Inter-Bold.ttf", [("Inter", Normal, 700)]),
    bundled!("Inter-BoldItalic.ttf", [("Inter", Italic, 700)]),
    bundled!("Inter-Italic.ttf", [("Inter", Italic, 400)]),
    bundled!("Inter-Regular.ttf", [("Inter", Normal, 400)]),
    bundled!(
        "Inter-SemiBold.ttf",
        [("Inter", Normal, 600), ("Inter SemiBold", Normal, 400)]
    ),
    bundled!(
        "LastResortHE-Regular.ttf",
        [("Last Resort High-Efficiency", Normal, 400)]
    ),
    bundled!("STIXGeneral.ttf", [("STIXGeneral", Normal, 400)]),
    bundled!("STIXGeneralBol.ttf", [("STIXGeneral", Normal, 700)]),
    bundled!("STIXGeneralBolIta.ttf", [("STIXGeneral", Italic, 700)]),
    bundled!("STIXGeneralItalic.ttf", [("STIXGeneral", Italic, 400)]),
    bundled!("STIXNonUni.ttf", [("STIXNonUnicode", Normal, 400)]),
    bundled!("STIXNonUniBol.ttf", [("STIXNonUnicode", Normal, 700)]),
    bundled!("STIXNonUniBolIta.ttf", [("STIXNonUnicode", Italic, 700)]),
    bundled!("STIXNonUniIta.ttf", [("STIXNonUnicode", Italic, 400)]),
    bundled!("STIXSizFiveSymReg.ttf", [("STIXSizeFiveSym", Normal, 400)]),
    bundled!("STIXSizFourSymBol.ttf", [("STIXSizeFourSym", Normal, 700)]),
    bundled!("STIXSizFourSymReg.ttf", [("STIXSizeFourSym", Normal, 400)]),
    bundled!("STIXSizOneSymBol.ttf", [("STIXSizeOneSym", Normal, 700)]),
    bundled!("STIXSizOneSymReg.ttf", [("STIXSizeOneSym", Normal, 400)]),
    bundled!(
        "STIXSizThreeSymBol.ttf",
        [("STIXSizeThreeSym", Normal, 700)]
    ),
    bundled!(
        "STIXSizThreeSymReg.ttf",
        [("STIXSizeThreeSym", Normal, 400)]
    ),
    bundled!("STIXSizTwoSymBol.ttf", [("STIXSizeTwoSym", Normal, 700)]),
    bundled!("STIXSizTwoSymReg.ttf", [("STIXSizeTwoSym", Normal, 400)]),
    bundled!("cmb10.ttf", [("cmb10", Normal, 400)]),
    bundled!("cmex10.ttf", [("cmex10", Normal, 400)]),
    bundled!("cmmi10.ttf", [("cmmi10", Normal, 400)]),
    bundled!("cmr10.ttf", [("cmr10", Normal, 400)]),
    bundled!("cmss10.ttf", [("cmss10", Normal, 400)]),
    bundled!("cmsy10.ttf", [("cmsy10", Normal, 400)]),
    bundled!("cmti10.ttf", [("cmti10", Normal, 400)]),
    bundled!("cmtt10.ttf", [("cmtt10", Normal, 400)]),
];

/// File name of the Last Resort font used as final fallback.
pub(crate) const LAST_RESORT: &str = "LastResortHE-Regular.ttf";
