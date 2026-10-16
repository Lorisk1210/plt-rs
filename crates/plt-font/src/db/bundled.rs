//! The fonts matplotlib ships in `mpl-data/fonts/ttf`, embedded in the binary.

/// `(file name, contents)` of every bundled font, sorted by file name.
pub(crate) const BUNDLED_FONTS: [(&str, &[u8]); 40] = [
    ("cmb10.ttf", include_bytes!("../../fonts/cmb10.ttf")),
    ("cmex10.ttf", include_bytes!("../../fonts/cmex10.ttf")),
    ("cmmi10.ttf", include_bytes!("../../fonts/cmmi10.ttf")),
    ("cmr10.ttf", include_bytes!("../../fonts/cmr10.ttf")),
    ("cmss10.ttf", include_bytes!("../../fonts/cmss10.ttf")),
    ("cmsy10.ttf", include_bytes!("../../fonts/cmsy10.ttf")),
    ("cmti10.ttf", include_bytes!("../../fonts/cmti10.ttf")),
    ("cmtt10.ttf", include_bytes!("../../fonts/cmtt10.ttf")),
    (
        "DejaVuSans-BoldOblique.ttf",
        include_bytes!("../../fonts/DejaVuSans-BoldOblique.ttf"),
    ),
    (
        "DejaVuSans-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSans-Bold.ttf"),
    ),
    (
        "DejaVuSansDisplay.ttf",
        include_bytes!("../../fonts/DejaVuSansDisplay.ttf"),
    ),
    (
        "DejaVuSansMono-BoldOblique.ttf",
        include_bytes!("../../fonts/DejaVuSansMono-BoldOblique.ttf"),
    ),
    (
        "DejaVuSansMono-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSansMono-Bold.ttf"),
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
        "DejaVuSans-Oblique.ttf",
        include_bytes!("../../fonts/DejaVuSans-Oblique.ttf"),
    ),
    (
        "DejaVuSans.ttf",
        include_bytes!("../../fonts/DejaVuSans.ttf"),
    ),
    (
        "DejaVuSerif-BoldItalic.ttf",
        include_bytes!("../../fonts/DejaVuSerif-BoldItalic.ttf"),
    ),
    (
        "DejaVuSerif-Bold.ttf",
        include_bytes!("../../fonts/DejaVuSerif-Bold.ttf"),
    ),
    (
        "DejaVuSerifDisplay.ttf",
        include_bytes!("../../fonts/DejaVuSerifDisplay.ttf"),
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
        "LastResortHE-Regular.ttf",
        include_bytes!("../../fonts/LastResortHE-Regular.ttf"),
    ),
    (
        "STIXGeneralBolIta.ttf",
        include_bytes!("../../fonts/STIXGeneralBolIta.ttf"),
    ),
    (
        "STIXGeneralBol.ttf",
        include_bytes!("../../fonts/STIXGeneralBol.ttf"),
    ),
    (
        "STIXGeneralItalic.ttf",
        include_bytes!("../../fonts/STIXGeneralItalic.ttf"),
    ),
    (
        "STIXGeneral.ttf",
        include_bytes!("../../fonts/STIXGeneral.ttf"),
    ),
    (
        "STIXNonUniBolIta.ttf",
        include_bytes!("../../fonts/STIXNonUniBolIta.ttf"),
    ),
    (
        "STIXNonUniBol.ttf",
        include_bytes!("../../fonts/STIXNonUniBol.ttf"),
    ),
    (
        "STIXNonUniIta.ttf",
        include_bytes!("../../fonts/STIXNonUniIta.ttf"),
    ),
    (
        "STIXNonUni.ttf",
        include_bytes!("../../fonts/STIXNonUni.ttf"),
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
];

/// File name of the Last Resort font used as final fallback.
pub(crate) const LAST_RESORT: &str = "LastResortHE-Regular.ttf";
