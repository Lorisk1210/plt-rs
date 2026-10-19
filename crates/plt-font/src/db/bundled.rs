//! The fonts matplotlib ships in `mpl-data/fonts/ttf`, plus Inter (the
//! typeface of plt-rs' modern look), embedded in the binary.
//!
//! Inter is copyright the Inter Project Authors and distributed under the
//! SIL Open Font License 1.1 (`fonts/LICENSE_INTER`). Its family name
//! appears in none of matplotlib's default font lists, so `findfont` only
//! picks it when a style names it.

/// `(file name, contents)` of every bundled font, in the order matplotlib
/// adds them (Python string order of the file names: upper case sorts
/// before lower case and `-` before `.`), which decides `findfont` ties.
pub(crate) const BUNDLED_FONTS: [(&str, &[u8]); 45] = [
    (
        "DejaVuSans-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSans-Bold.ttf"),
    ),
    (
        "DejaVuSans-BoldOblique.ttf",
        include_bytes!("../../fonts/DejaVuSans-BoldOblique.ttf"),
    ),
    (
        "DejaVuSans-Oblique.ttf",
        include_bytes!("../../fonts/DejaVuSans-Oblique.ttf"),
    ),
    (
        "DejaVuSans.ttf",
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "DejaVuSansDisplay.ttf",
        include_bytes!("../../fonts/DejaVuSansDisplay.ttf"),
    ),
    (
        "DejaVuSansMono-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf"),
    ),
    (
        "DejaVuSansMono-BoldOblique.ttf",
        include_bytes!("../../fonts/DejaVuSansMono-BoldOblique.ttf"),
    ),
    (
        "DejaVuSansMono-Oblique.ttf",
        include_bytes!("../../fonts/DejaVuSansMono-Oblique.ttf"),
    ),
    (
        "DejaVuSansMono.ttf",
        include_bytes!("../../fonts/DejaVuSansMono.ttf"),
    ),
    (
        "DejaVuSerif-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
    ),
    (
        "DejaVuSerif-BoldItalic.ttf",
        include_bytes!("../../fonts/DejaVuSerif-BoldItalic.ttf"),
    ),
    (
        "DejaVuSerif-Italic.ttf",
        include_bytes!("../../fonts/DejaVuSerif-Italic.ttf"),
    ),
    (
        "DejaVuSerif.ttf",
        include_bytes!("../../fonts/DejaVuSerif.ttf"),
    ),
    (
        "DejaVuSerifDisplay.ttf",
        include_bytes!("../../fonts/DejaVuSerifDisplay.ttf"),
    ),
    (
        "Inter-Bold.ttf",
        include_bytes!("../../fonts/Inter-Bold.ttf"),
    ),
    (
        "Inter-BoldItalic.ttf",
        include_bytes!("../../fonts/Inter-BoldItalic.ttf"),
    ),
    (
        "Inter-Italic.ttf",
        include_bytes!("../../fonts/Inter-Italic.ttf"),
    ),
    (
        "Inter-Regular.ttf",
        include_bytes!("../../fonts/Inter-Regular.ttf"),
    ),
    (
        "Inter-SemiBold.ttf",
        include_bytes!("../../fonts/Inter-SemiBold.ttf"),
    ),
    (
        "LastResortHE-Regular.ttf",
        include_bytes!("../../fonts/LastResortHE-Regular.ttf"),
    ),
    (
        "STIXGeneral.ttf",
        include_bytes!("../../fonts/STIXGeneral.ttf"),
    ),
    (
        "STIXGeneralBol.ttf",
        include_bytes!("../../fonts/STIXGeneralBol.ttf"),
    ),
    (
        "STIXGeneralBolIta.ttf",
        include_bytes!("../../fonts/STIXGeneralBolIta.ttf"),
    ),
    (
        "STIXGeneralItalic.ttf",
        include_bytes!("../../fonts/STIXGeneralItalic.ttf"),
    ),
    (
        "STIXNonUni.ttf",
        include_bytes!("../../fonts/STIXNonUni.ttf"),
    ),
    (
        "STIXNonUniBol.ttf",
        include_bytes!("../../fonts/STIXNonUniBol.ttf"),
    ),
    (
        "STIXNonUniBolIta.ttf",
        include_bytes!("../../fonts/STIXNonUniBolIta.ttf"),
    ),
    (
        "STIXNonUniIta.ttf",
        include_bytes!("../../fonts/STIXNonUniIta.ttf"),
    ),
    (
        "STIXSizFiveSymReg.ttf",
        include_bytes!("../../fonts/STIXSizFiveSymReg.ttf"),
    ),
    (
        "STIXSizFourSymBol.ttf",
        include_bytes!("../../fonts/STIXSizFourSymBol.ttf"),
    ),
    (
        "STIXSizFourSymReg.ttf",
        include_bytes!("../../fonts/STIXSizFourSymReg.ttf"),
    ),
    (
        "STIXSizOneSymBol.ttf",
        include_bytes!("../../fonts/STIXSizOneSymBol.ttf"),
    ),
    (
        "STIXSizOneSymReg.ttf",
        include_bytes!("../../fonts/STIXSizOneSymReg.ttf"),
    ),
    (
        "STIXSizThreeSymBol.ttf",
        include_bytes!("../../fonts/STIXSizThreeSymBol.ttf"),
    ),
    (
        "STIXSizThreeSymReg.ttf",
        include_bytes!("../../fonts/STIXSizThreeSymReg.ttf"),
    ),
    (
        "STIXSizTwoSymBol.ttf",
        include_bytes!("../../fonts/STIXSizTwoSymBol.ttf"),
    ),
    (
        "STIXSizTwoSymReg.ttf",
        include_bytes!("../../fonts/STIXSizTwoSymReg.ttf"),
    ),
    ("cmb10.ttf", include_bytes!("../../fonts/cmb10.ttf")),
    ("cmex10.ttf", include_bytes!("../../fonts/cmex10.ttf")),
    ("cmmi10.ttf", include_bytes!("../../fonts/cmmi10.ttf")),
    ("cmr10.ttf", include_bytes!("../../fonts/cmr10.ttf")),
    ("cmss10.ttf", include_bytes!("../../fonts/cmss10.ttf")),
    ("cmsy10.ttf", include_bytes!("../../fonts/cmsy10.ttf")),
    ("cmti10.ttf", include_bytes!("../../fonts/cmti10.ttf")),
    ("cmtt10.ttf", include_bytes!("../../fonts/cmtt10.ttf")),
];

/// File name of the Last Resort font used as final fallback.
pub(crate) const LAST_RESORT: &str = "LastResortHE-Regular.ttf";
