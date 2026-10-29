//! The `font.*`, `text.*` and `mathtext.*` cases.

use pltrs::figure::Figure;

use super::{LABELLED, SMALL, labelled, rc_cases};
use crate::figures::{CaseResult, FigureCase};

/// Figure texts `(x, y, text)` in `size` points.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the scenes return it, and every case shares the fallible build signature"
)]
fn figure_texts(fig: &Figure, size: f64, texts: &[(f64, f64, &str)]) -> CaseResult {
    for &(x, y, text) in texts {
        fig.text(x, y, text).set_fontsize(size);
    }
    Ok(())
}

/// `_texts`: plain text, text with a math part, and ligature and minus
/// candidates.
fn texts(fig: &Figure) -> CaseResult {
    figure_texts(
        fig,
        14.0,
        &[
            (0.05, 0.72, "Plain AVAT Wo"),
            (0.05, 0.42, r"Math $\alpha^2+\sqrt{x}$"),
            (0.05, 0.12, "fi ffl 1-2"),
        ],
    )
}

/// `_missing_glyph`: characters `DejaVu Sans` has no glyph for, between
/// two it has.
fn missing_glyph(fig: &Figure) -> CaseResult {
    figure_texts(fig, 20.0, &[(0.1, 0.4, "A\u{4e2d}\u{0e01}B")])
}

/// `_cyrillic`: Cyrillic letters with Serbian local forms.
fn cyrillic(fig: &Figure) -> CaseResult {
    figure_texts(fig, 24.0, &[(0.05, 0.35, "бгдпт")])
}

/// `_maths`: math text in every font command.
fn maths(fig: &Figure) -> CaseResult {
    figure_texts(
        fig,
        13.0,
        &[
            (0.04, 0.72, r"$x_i^2+\alpha\int f\,dx$"),
            (
                0.04,
                0.42,
                r"$\mathrm{rm}\ \mathit{it}\ \mathbf{bf}\ \mathsf{sf}$",
            ),
            (0.04, 0.12, r"$\mathtt{tt}\ \mathcal{CAL}\ \mathbfit{bfit}$"),
        ],
    )
}

/// `_math_symbols`: math symbols a monospace text font lacks.
fn math_symbols(fig: &Figure) -> CaseResult {
    figure_texts(
        fig,
        16.0,
        &[(0.05, 0.4, r"$\oint\ \coprod\ \wp\ \bigodot\ x\geq y$")],
    )
}

pub(crate) const CASES: &[FigureCase] = rc_cases! {
    texts, SMALL => {
        "font_family_serif": [("font.family", "serif")],
        "font_family_monospace": [("font.family", "monospace")],
        "font_serif": [("font.family", "serif"), ("font.serif", "STIXGeneral")],
        "font_sans_serif": [("font.family", "sans-serif"), ("font.sans-serif", "DejaVu Sans Mono")],
        "font_cursive": [("font.family", "cursive"), ("font.cursive", "DejaVu Serif")],
        "font_fantasy": [("font.family", "fantasy"), ("font.fantasy", "STIXGeneral")],
        "font_monospace": [("font.family", "monospace"), ("font.monospace", "DejaVu Serif")],
        "font_style": [("font.style", "italic")],
        "font_weight": [("font.weight", "bold")],
        "font_variant_stretch": [("font.variant", "small-caps"), ("font.stretch", "condensed")],
        "text_antialiased": [("text.antialiased", "False")],
        "text_parse_math": [("text.parse_math", "False")],
    }
    labelled, LABELLED => {
        "font_size": [("font.size", "13")],
        "text_color": [("text.color", "tab:blue")],
    }
    missing_glyph, SMALL => {
        "font_enable_last_resort": [("font.enable_last_resort", "False")],
    }
    cyrillic, SMALL => {
        "text_language": [("text.language", "sr")],
    }
    maths, SMALL => {
        "mathtext_fontset_cm": [("mathtext.fontset", "cm")],
        "mathtext_fontset_stix": [("mathtext.fontset", "stix")],
        "mathtext_fontset_stixsans": [("mathtext.fontset", "stixsans")],
        "mathtext_fontset_dejavuserif": [("mathtext.fontset", "dejavuserif")],
        "mathtext_custom": [
            ("mathtext.fontset", "custom"),
            ("mathtext.rm", "serif"),
            ("mathtext.it", "serif:italic"),
            ("mathtext.bf", "serif:bold"),
            ("mathtext.bfit", "serif:italic:bold"),
            ("mathtext.sf", "monospace"),
            ("mathtext.tt", "serif"),
            ("mathtext.cal", "STIXGeneral"),
        ],
        "mathtext_default": [("mathtext.default", "rm")],
    }
    math_symbols, SMALL => {
        "mathtext_fallback": [
            ("mathtext.fontset", "custom"),
            ("mathtext.rm", "DejaVu Sans Mono"),
            ("mathtext.it", "DejaVu Sans Mono:italic"),
            ("mathtext.fallback", "stix"),
        ],
    }
};
