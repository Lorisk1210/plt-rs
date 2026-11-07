//! Text in CFF, CFF2 and variable TrueType fonts,
//! `figures/cases_font_formats.py`: matplotlib's default `text.hinting`
//! (`FreeType`'s CFF hinter for CFF fonts, its autohinter for the TrueType
//! font without bytecode), "`force_autohint`" and unhinted text, variable
//! fonts at their default instance. The fonts are fixtures of
//! `pltrs-font`'s tests.

use std::path::PathBuf;

use pltrs::fonts::FontManager;
use pltrs::prelude::*;
use pltrs::text::VAlign;

use crate::figures::{CaseResult, FigureCase};

/// `FONTS`: each fixture file, its family and the characters it maps.
const FONTS: [(&str, &str, &str); 3] = [
    (
        "synthetic-cff.otf",
        "Plt Synthetic CFF",
        "AOBFH C\u{c1}\u{b4}",
    ),
    ("synthetic-var.otf", "Plt Var CFF", "AOH \u{c1}\u{b4}"),
    ("synthetic-var.ttf", "Plt Var", "AOH \u{c1}\u{b4}"),
];

/// `SIZES`: the point sizes of each column.
const SIZES: [u32; 7] = [6, 8, 10, 12, 15, 20, 28];

/// `_fonts(fig)`: each family at each size, a column per family.
fn fonts(fig: &Figure) -> CaseResult {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pltrs-font/tests/fixtures");
    for (file, _, _) in FONTS {
        FontManager::global().add_font(fixtures.join(file))?;
    }
    for (i, (_, family, chars)) in FONTS.into_iter().enumerate() {
        for (j, size) in SIZES.into_iter().enumerate() {
            fig.text(
                0.02 + 0.33 * i as f64,
                0.95 - 0.13 * j as f64,
                format!("{chars}{chars}"),
            )
            .set_family([family])
            .set_fontsize(f64::from(size))
            .set_valign(VAlign::Top);
        }
    }
    Ok(())
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("font_formats/default_hinting", fonts)
        .sized(6.0, 3.5, 100.0)
        .with_rc(&[("text.hinting", "default")]),
    FigureCase::new("font_formats/autohint", fonts)
        .sized(6.0, 3.5, 100.0)
        .with_rc(&[("text.hinting", "force_autohint")]),
    FigureCase::new("font_formats/unhinted", fonts).sized(6.0, 3.5, 150.0),
];
