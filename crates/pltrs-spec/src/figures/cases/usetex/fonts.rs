//! The font family cases of `figures/cases_usetex.py`: the fonts TeX
//! selects from `font.family` and the family lists, and a custom
//! preamble.

use crate::figures::FigureCase;

/// A 3.2 × 2 in case with `text.usetex` on and `rc`, needing TeX.
const fn family_case(
    name: &'static str,
    rc: &'static [(&'static str, &'static str)],
    lines: fn(&pltrs::figure::Figure) -> crate::figures::CaseResult,
) -> FigureCase {
    FigureCase::new(name, lines)
        .sized(3.2, 2.0, 100.0)
        .with_rc(rc)
        .needing_tex()
}

/// Two lines of text at 12 points.
fn two_lines(fig: &pltrs::figure::Figure, first: &str, second: &str) {
    fig.text(0.05, 0.6, first).set_fontsize(12.0);
    fig.text(0.05, 0.2, second).set_fontsize(12.0);
}

pub(super) const CASES: &[FigureCase] = &[
    family_case(
        "usetex/family_sans_helvetica",
        &[
            ("text.usetex", "True"),
            ("font.family", "sans-serif"),
            ("font.sans-serif", "Helvetica"),
        ],
        |fig| {
            two_lines(fig, r"Helvetica $x^2$ fi ffl", r"\textrm{rm} \texttt{tt}");
            Ok(())
        },
    ),
    family_case(
        "usetex/family_serif_palatino",
        &[
            ("text.usetex", "True"),
            ("font.family", "serif"),
            ("font.serif", "Palatino"),
        ],
        |fig| {
            two_lines(
                fig,
                r"Palatino $\alpha + \beta$",
                r"\textsf{sans} \textit{italic}",
            );
            Ok(())
        },
    ),
    family_case(
        "usetex/family_times",
        &[("text.usetex", "True"), ("font.family", "times")],
        |fig| {
            two_lines(
                fig,
                r"Times $\int x\,dx$",
                r"\textbf{Bold} \textsl{slanted}",
            );
            Ok(())
        },
    ),
    family_case(
        "usetex/family_monospace_courier",
        &[
            ("text.usetex", "True"),
            ("font.family", "monospace"),
            ("font.monospace", "Courier"),
        ],
        |fig| {
            two_lines(fig, "Courier text 0123", r"\textbf{bold} $y$");
            Ok(())
        },
    ),
    family_case(
        "usetex/family_cursive",
        &[("text.usetex", "True"), ("font.family", "cursive")],
        |fig| {
            fig.text(0.05, 0.5, "Zapf Chancery").set_fontsize(14.0);
            Ok(())
        },
    ),
    family_case(
        "usetex/family_computer_modern_sans",
        &[("text.usetex", "True"), ("font.family", "sans-serif")],
        |fig| {
            fig.text(0.05, 0.5, r"CM sans $\Gamma$").set_fontsize(14.0);
            Ok(())
        },
    ),
    family_case(
        "usetex/preamble",
        &[
            ("text.usetex", "True"),
            (
                "text.latex.preamble",
                r"\usepackage{amssymb}\usepackage{bm}",
            ),
        ],
        |fig| {
            two_lines(
                fig,
                r"$\mathbb{R}^n \ni \bm{x}$",
                r"$\mathfrak{g} \circledcirc \lesssim$",
            );
            Ok(())
        },
    ),
];
