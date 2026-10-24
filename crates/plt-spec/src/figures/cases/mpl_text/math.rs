//! The mathtext cases of `test_mathtext.py` its parametrised font tests
//! leave out: accents, spaces, operators, delimiters, fractions, braces,
//! symbol styles, phantoms and per-text math fonts.

use plt::fonts::Fontset;
use plt::prelude::*;
use plt::text::HAlign;

use super::classic;
use crate::figures::FigureCase;

/// Adds each `(x, y, s)` text to the figure.
fn texts(fig: &Figure, texts: &[(f64, f64, &str)]) {
    for &(x, y, s) in texts {
        fig.text(x, y, s);
    }
}

/// Adds the expressions one above the other at `x = 0.1`, `y = 0.1, 0.2,
/// ...` (`test_operator_space`).
fn column(fig: &Figure, expressions: &[&str]) {
    for (i, s) in expressions.iter().enumerate() {
        fig.text(0.1, 0.1 * (i + 1) as f64, *s);
    }
}

/// `test_operator_space` (`fig_test`): operators followed by operands,
/// brackets, scripts and other operators.
const OPERATORS: [&str; 9] = [
    r"$\log 6$",
    r"$\log(6)$",
    r"$\arcsin 6$",
    r"$\arcsin|6|$",
    r"$\operatorname{op} 6$",
    r"$\operatorname{op}[6]$",
    r"$\cos^2$",
    r"$\log_2$",
    r"$\sin^2 \max \cos$",
];

/// `test_operator_space` (`fig_ref`): the operators spelled with
/// `\mathrm` and explicit thin spaces.
const OPERATORS_REF: [&str; 9] = [
    r"$\mathrm{log\,}6$",
    r"$\mathrm{log}(6)$",
    r"$\mathrm{arcsin\,}6$",
    r"$\mathrm{arcsin}|6|$",
    r"$\mathrm{op\,}6$",
    r"$\mathrm{op}[6]$",
    r"$\mathrm{cos}^2$",
    r"$\mathrm{log}_2$",
    r"$\mathrm{sin}^2 \mathrm{\,max} \mathrm{\,cos}$",
];

/// `test_phantoms`: text behind zero-width left and right overlaps and an
/// invisible phantom.
fn phantoms(fig: &Figure) {
    fig.text(0.5, 0.9, r"$\rlap{rlap}extra$")
        .set_halign(HAlign::Left);
    fig.text(0.5, 0.8, r"$extra\llap{llap}$")
        .set_halign(HAlign::Right);
    fig.text(0.5, 0.7, r"$\phantom{phantom}$");
}

/// `test_math_fontfamily`: two texts in different math fonts.
fn math_fontfamily(fig: &Figure) {
    for (y, s, fontset) in [
        (
            0.7,
            r"$This\ text\ should\ have\ one\ font$",
            Fontset::DejaVuSans,
        ),
        (0.3, r"$This\ text\ should\ have\ another$", Fontset::Stix),
    ] {
        fig.text(0.2, y, s)
            .set_fontsize(24.0)
            .set_math_fontfamily(fontset);
    }
}

pub(super) const CASES: &[FigureCase] = &[
    // `test_short_long_accents`: every one-character accent command of
    // `Parser._accent_map`, and the long command of the same accent.
    classic("mpl_text/short_long_accents", |fig| {
        texts(fig, &[(0.0, 0.5, r#"$\"a\`a\'a\~a\.a\^a$"#)]);
        Ok(())
    }),
    classic("mpl_text/short_long_accents_ref", |fig| {
        texts(
            fig,
            &[(0.0, 0.5, r"$\ddot a\grave a\acute a\tilde a\dot a\hat a$")],
        );
        Ok(())
    }),
    classic("mpl_text/spaces", |fig| {
        texts(fig, &[(0.5, 0.5, r"$1\,2\>3\ 4$")]);
        Ok(())
    }),
    classic("mpl_text/spaces_ref", |fig| {
        texts(fig, &[(0.5, 0.5, r"$1\/2\:3~4$")]);
        Ok(())
    }),
    classic("mpl_text/operator_space", |fig| {
        column(fig, &OPERATORS);
        Ok(())
    }),
    classic("mpl_text/operator_space_ref", |fig| {
        column(fig, &OPERATORS_REF);
        Ok(())
    }),
    classic("mpl_text/inverted_delimiters", |fig| {
        fig.text(0.5, 0.5, r"$\left)\right($")
            .set_math_fontfamily(Fontset::DejaVuSans);
        Ok(())
    }),
    classic("mpl_text/genfrac_displaystyle", |fig| {
        texts(fig, &[(0.1, 0.1, r"$\dfrac{2x}{3y}$")]);
        Ok(())
    }),
    // The `fig_ref` side: the underline thickness of 12 pt text at 100 dpi
    // as the rule.
    classic("mpl_text/genfrac_displaystyle_ref", |fig| {
        texts(fig, &[(0.1, 0.1, r"$\genfrac{}{}{1.041667}{0}{2x}{3y}$")]);
        Ok(())
    }),
    classic("mpl_text/text_nested_braces", |fig| {
        texts(
            fig,
            &[
                (0.1, 0.2, r"$\text{{example}}$"),
                (0.1, 0.5, r"$\text{a{b}{{c}}d}$"),
            ],
        );
        Ok(())
    }),
    classic("mpl_text/text_escaped_braces", |fig| {
        texts(fig, &[(0.1, 0.2, r"$\text{{\{example\}}}$")]);
        Ok(())
    }),
    classic("mpl_text/boldsymbol", |fig| {
        texts(fig, &[(0.1, 0.2, r"$\boldsymbol{\mathrm{abc0123\alpha}}$")]);
        Ok(())
    }),
    classic("mpl_text/mathnormal", |fig| {
        texts(fig, &[(0.1, 0.2, r"$\mathnormal{0123456789}$")]);
        Ok(())
    }),
    classic("mpl_text/phantoms", |fig| {
        phantoms(fig);
        Ok(())
    }),
    FigureCase::new("mpl_text/math_fontfamily_image", |fig| {
        math_fontfamily(fig);
        Ok(())
    })
    .sized(10.0, 3.0, 40.0),
];
