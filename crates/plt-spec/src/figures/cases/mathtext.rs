//! Mathtext in every fontset, `figures/cases_mathtext.py`: expressions from
//! matplotlib's own mathtext tests, one centred row of 0.75 in each.

use plt::fonts::Fontset;
use plt::prelude::*;
use plt::text::{HorizontalAlignment, Text, VerticalAlignment};

use crate::figures::FigureCase;

/// `WIDTH`: the figure width of every case, in inches.
const WIDTH: f64 = 5.25;
/// `ROW`: the height of one row, in inches.
const ROW: f64 = 0.75;

/// `TESTS_A`: the first ten `math_tests` drawn.
const TESTS_A: [&str; 10] = [
    r"$a+b+\dot s+\dot{s}+\ldots$",
    r"$x\ x\,x\;x\quad x\qquad x\!x\hspace{ 0.5 }y$",
    r"$\left[\left\lfloor\frac{5}{\frac{\left(3\right)}{4}} y\right)\right]$",
    concat!(
        r"$\sum _{\genfrac{}{}{0}{}{0\leq i\leq m}{0<j<n}}f\left(i,j\right)",
        r"\mathcal{R}\prod_{i=\alpha_{i+1}}^\infty a_i \sin(2 \pi f x_i)",
        r"\sqrt[2]{\prod^\frac{x}{2\pi^2}_\infty}$",
    ),
    r"$x = \frac{x+\frac{5}{2}}{\frac{y+3}{8}}$",
    r"Foo: $\alpha_{i+1}^j = {\rm sin}(2\pi f_j t_i) e^{-5 t_i/\tau}$",
    r"$\ddot{o}\acute{e}\grave{e}\hat{O}\breve{\imath}\tilde{n}\vec{q}$",
    r"$\limsup_{x\to\infty}$",
    r"$\sqrt[3]{\frac{X_2}{Y}}=5$",
    r"$\mathcal{H} = \int d \tau \left(\epsilon E^2 + \mu H^2\right)$",
];

/// `TESTS_B`: ten more `math_tests`.
const TESTS_B: [&str; 10] = [
    r"$\widehat{abc}\widetilde{def}$",
    r"${a}_{0}+\frac{1}{{a}_{1}+\frac{1}{{a}_{2}+\frac{1}{{a}_{3}+\frac{1}{{a}_{4}}}}}$",
    r"$\binom{p}{2}{x}^{2}{y}^{p-2}-\frac{1}{1-x}\frac{1}{1-{x}^{2}}$",
    r"$\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+\sqrt{1+x}}}}}}}$",
    r"$\left(\frac{{\partial }^{2}}{\partial {x}^{2}}+\frac{{\partial }^{2}}{\partial {y}^{2}}\right){|\varphi \left(x+iy\right)|}^{2}=0$",
    r"${x}_{{y}_{b}^{a}}^{{z}_{c}^{d}}$",
    r"$\left\Vert \frac{a}{b} \right\Vert \left\vert \frac{a}{b} \right\vert \left\| \frac{a}{b}\right\| \left| \frac{a}{b} \right| \Vert a \Vert \vert b \vert \| a \| | b |$",
    r"$M \, M \thinspace M \/ M \> M \: M \; M \ M \enspace M \quad M \qquad M \! M$",
    r"$\Cap$ $\Cup$ $\leftharpoonup$ $\barwedge$ $\rightharpoonup$",
    r"$6-2$; $-2$; $ -2$; ${-2}$; ${  -2}$; $20^{+3}_{-2}$",
];

/// `FONTS`: ten `font_tests`, covering every font command.
const FONTS: [&str; 10] = [
    r"mathrm $\mathrm{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathbf $\mathbf{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta \iota \lambda \mu \nu \xi \pi \kappa \rho \sigma \tau \upsilon \phi \chi \psi}$",
    r"mathit $\mathit{\Gamma \Delta \Theta \Lambda \Xi \Pi \Sigma \Upsilon \Phi \Psi \Omega}$",
    r"mathtt $\mathtt{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbb $\mathbb{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathrm mathbb $\mathrm{\mathbb{\Gamma \Pi \Sigma \gamma \pi}}$",
    r"mathcal $\mathcal{ABCDEFGHIJKLMNOPQRSTUVWXYZ}$",
    r"mathfrak $\mathfrak{abcdefghijklmnopqrstuvwxyz}$",
    r"mathscr $\mathscr{abcdefghijklmnopqrstuvwxyz}$",
    r"mathbfit $\mathbfit{0123456789}$ mathsf $\mathsf{0123456789}$",
];

/// One `svgastext_math_tests` row: a delimiter pair after `\left` and
/// `\right` around bodies of five heights, with `\middle` delimiters.
macro_rules! delimiter_row {
    ($l:literal, $r:literal) => {
        concat!(
            r"$\left", $l, r" M \middle/ ? \middle\backslash ? \right", $r, " ",
            r"\left", $l, r" \frac{M}{B} \middle/ ? \middle\backslash ? \right", $r, " ",
            r"\left", $l, r" \frac{\frac{M}{I}}{B} \middle/ ? \middle\backslash ? \right", $r, " ",
            r"\left", $l, r" \frac{\frac{M}{I}}{\frac{B}{U}} \middle/ ? \middle\backslash ? \right", $r, " ",
            r"\left", $l, r"\frac{\frac{\frac{M}{I}}{N}}{\frac{\frac{B}{U}}{G}} \middle/ ? \middle\backslash ? \right", $r, "$",
        )
    };
}

/// `DELIMITERS`: the `svgastext_math_tests`, every `AutoHeightChar`
/// substitution.
const DELIMITERS: [&str; 7] = [
    "$-$-",
    delimiter_row!("(", ")"),
    delimiter_row!("[", "]"),
    delimiter_row!("<", ">"),
    delimiter_row!(r"\{", r"\}"),
    delimiter_row!(r"\lfloor", r"\rfloor"),
    delimiter_row!(r"\lceil", r"\rceil"),
];

/// `LIGHTWEIGHT`: the `lightweight_math_tests`.
const LIGHTWEIGHT: [&str; 10] = [
    r"$\sqrt[ab]{123}$",
    r"$x \overset{f}{\rightarrow} \overset{f}{x} \underset{xx}{ff} \overset{xx}{ff} \underset{f}{x} \underset{f}{\leftarrow} x$",
    r"$\sum x\quad\sum^nx\quad\sum_nx\quad\sum_n^nx\quad\prod x\quad\prod^nx\quad\prod_nx\quad\prod_n^nx$",
    r"$1.$ $2.$ $19680801.$ $a.$ $b.$ $mpl.$",
    r"$\text{text}_{\text{sub}}^{\text{sup}} + \text{\$foo\$} + \frac{\text{num}}{\mathbf{\text{den}}}\text{with space, curly brackets \{\}, and dash -}$",
    r"$\boldsymbol{abcde} \boldsymbol{+} \boldsymbol{\Gamma + \Omega} \boldsymbol{01234} \boldsymbol{\alpha * \beta}$",
    r"$\left\lbrace\frac{\left\lbrack A^b_c\right\rbrace}{\left\leftbrace D^e_f \right\rbrack}\right\rightbrace\ \left\leftparen\max_{x} \left\lgroup \frac{A}{B}\right\rgroup \right\rightparen$",
    r"$\left( a\middle. b \right)$ $\left( \frac{a}{b} \middle\vert x_i \in P^S \right)$ $\left[ 1 - \middle| a\middle| + \left( x  - \left\lfloor \dfrac{a}{b}\right\rfloor \right)  \right]$",
    r"$\sum_{\substack{k = 1\\ k \neq \lfloor n/2\rfloor}}^{n}P(i,j) \sum_{\substack{i \neq 0\\ -1 \leq i \leq 3\\ 1 \leq j \leq 5}} F^i(x,y) \sum_{\substack{\left \lfloor \frac{n}{2} \right\rfloor}} F(n)$",
    concat!(
        r"$a\underline{a}$ $b\underline{b}$ $f\underline{f}$ $g\underline{g}$ ",
        r"$h\underline{h}$ $y\underline{y}$ ",
        r"$\underline{\left(\dfrac{num}{\underline{den}}\right)}^{\underline{p_1}}$",
    ),
];

/// `_rows(expressions)`: one centred expression per row, top to bottom.
fn rows(fig: &Figure, expressions: &[&str]) -> Vec<Text> {
    let n = expressions.len() as f64;
    expressions
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let text = fig.text(0.5, 1.0 - (i as f64 + 0.5) / n, *s);
            text.set_fontsize(12.0)
                .set_halign(HorizontalAlignment::Center)
                .set_valign(VerticalAlignment::Center);
            text
        })
        .collect()
}

/// The cases drawing `$expressions` once per fontset (by default every
/// fontset), named `mathtext/<$group>_<fontset>`.
macro_rules! per_fontset {
    ($group:literal, $expressions:ident) => {
        per_fontset!(
            $group,
            $expressions,
            ["dejavusans", "dejavuserif", "cm", "stix", "stixsans", "custom"]
        )
    };
    ($group:literal, $expressions:ident, [$($fontset:literal),+]) => {
        &[$(
            FigureCase::new(concat!("mathtext/", $group, "_", $fontset), |fig| {
                rows(fig, &$expressions);
                Ok(())
            })
            .sized(WIDTH, ROW * $expressions.len() as f64, 100.0)
            .with_rc(&[("mathtext.fontset", $fontset)])
        ),+]
    };
}

/// `tests_a_<fontset>`.
pub(crate) const TESTS_A_CASES: &[FigureCase] = per_fontset!("tests_a", TESTS_A);

/// `tests_b_<fontset>`.
pub(crate) const TESTS_B_CASES: &[FigureCase] = per_fontset!("tests_b", TESTS_B);

/// `fonts_<fontset>`.
pub(crate) const FONT_CASES: &[FigureCase] = per_fontset!("fonts", FONTS);

/// `delimiters_<fontset>`.
pub(crate) const DELIMITER_CASES: &[FigureCase] =
    per_fontset!("delimiters", DELIMITERS, ["cm", "dejavusans", "stix"]);

/// `lightweight`: the texts ask for `DejaVu Sans` math while the rcParams
/// say Computer Modern.
pub(crate) const LIGHTWEIGHT_CASES: &[FigureCase] =
    &[FigureCase::new("mathtext/lightweight", |fig| {
        for text in rows(fig, &LIGHTWEIGHT) {
            text.set_math_fontfamily(Fontset::DejaVuSans);
        }
        Ok(())
    })
    .sized(WIDTH, ROW * LIGHTWEIGHT.len() as f64, 100.0)
    .with_rc(&[("mathtext.fontset", "cm")])];
