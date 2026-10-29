//! Mathtext: TeX-ish expressions built from every command pltrs knows
//! with random arguments, then broken (braces dropped, commands cut,
//! spans repeated), typeset in every fontset at random sizes and
//! resolutions, and drawn as figure text.

use pltrs::figure::Figure;
use pltrs::text::{MathImageOptions, math_to_image};
use pltrs_text::TextEngine;
use pltrs_text::mathtext::{Fontset, MathStyle};

use crate::harness::{Gen, fuzz};
use crate::support::{format, render};

/// Commands without arguments: symbols, big operators, functions,
/// spaces and delimiters.
const SYMBOLS: [&str; 72] = [
    r"\alpha",
    r"\beta",
    r"\Gamma",
    r"\Omega",
    r"\infty",
    r"\partial",
    r"\nabla",
    r"\pm",
    r"\times",
    r"\div",
    r"\cdot",
    r"\leq",
    r"\geq",
    r"\neq",
    r"\approx",
    r"\equiv",
    r"\rightarrow",
    r"\Leftarrow",
    r"\mapsto",
    r"\in",
    r"\notin",
    r"\subset",
    r"\cup",
    r"\cap",
    r"\forall",
    r"\exists",
    r"\hbar",
    r"\ell",
    r"\aleph",
    r"\dots",
    r"\ldots",
    r"\cdots",
    r"\vdots",
    r"\ddots",
    r"\prime",
    r"\circ",
    r"\bullet",
    r"\star",
    r"\dag",
    r"\S",
    r"\$",
    r"\%",
    r"\#",
    r"\&",
    r"\_",
    r"\{",
    r"\}",
    r"\sum",
    r"\prod",
    r"\int",
    r"\oint",
    r"\coprod",
    r"\bigcup",
    r"\bigoplus",
    r"\iint",
    r"\sin",
    r"\lim",
    r"\log",
    r"\max",
    r"\liminf",
    r"\,",
    r"\;",
    r"\:",
    r"\!",
    r"\ ",
    r"\quad",
    r"\qquad",
    r"\enspace",
    r"\langle",
    r"\rangle",
    r"\lfloor",
    r"\|",
];

/// Commands taking one braced argument (fonts, accents, lines, laps).
const UNARY: [&str; 34] = [
    r"\mathrm",
    r"\mathit",
    r"\mathbf",
    r"\mathbfit",
    r"\mathsf",
    r"\mathtt",
    r"\mathcal",
    r"\mathbb",
    r"\mathfrak",
    r"\mathscr",
    r"\mathcircled",
    r"\mathdefault",
    r"\mathregular",
    r"\text",
    r"\textbf",
    r"\boldsymbol",
    r"\operatorname",
    r"\hat",
    r"\breve",
    r"\bar",
    r"\tilde",
    r"\dot",
    r"\ddot",
    r"\vec",
    r"\overrightarrow",
    r"\widehat",
    r"\widetilde",
    r"\mathring",
    r"\sqrt",
    r"\overline",
    r"\underline",
    r"\phantom",
    r"\llap",
    r"\rlap",
];

/// Commands taking two braced arguments.
const BINARY: [&str; 6] = [
    r"\frac",
    r"\dfrac",
    r"\tfrac",
    r"\binom",
    r"\overset",
    r"\underset",
];

/// Delimiters for `\left`, `\middle` and `\right`.
const DELIMITERS: [&str; 16] = [
    "(",
    ")",
    "[",
    "]",
    r"\{",
    r"\}",
    "|",
    r"\|",
    ".",
    "/",
    r"\backslash",
    r"\uparrow",
    r"\Updownarrow",
    r"\langle",
    r"\rceil",
    r"\vert",
];

/// Atoms: letters, digits, operators and characters outside ASCII.
const ATOMS: [&str; 16] = [
    "x",
    "y",
    "1",
    "42",
    "+",
    "-",
    "=",
    "<",
    "'",
    ",",
    ".",
    "é",
    "\u{2211}",
    "\u{211D}",
    "\u{1D538}",
    "?",
];

/// Pieces that break expressions when inserted.
const BREAKERS: [&str; 12] = [
    "{", "}", "^", "_", "$", "\\", r"\left", r"\right", r"\frac", "^{", "_{", "\\nosuch",
];

/// A random expression at most `depth` levels deep.
fn expression(g: &mut Gen, depth: usize) -> String {
    let parts = 1 + g.below(4);
    (0..parts).map(|_| term(g, depth)).collect()
}

/// One term of an expression.
fn term(g: &mut Gen, depth: usize) -> String {
    let leaf = depth == 0 || g.chance(0.35);
    let base = if leaf {
        match g.below(3) {
            0 => g.pick(&SYMBOLS).to_owned(),
            _ => g.pick(&ATOMS).to_owned(),
        }
    } else {
        compound(g, depth - 1)
    };
    match g.below(8) {
        0 => format!("{base}^{{{}}}", expression(g, depth.saturating_sub(1))),
        1 => format!("{base}_{}", g.pick(&ATOMS)),
        2 => format!(
            "{base}_{{{}}}^{}",
            expression(g, depth.saturating_sub(1)),
            g.pick(&ATOMS)
        ),
        _ => base,
    }
}

/// A command with arguments.
fn compound(g: &mut Gen, depth: usize) -> String {
    match g.below(9) {
        0 | 1 => format!("{}{{{}}}", g.pick(&UNARY), expression(g, depth)),
        2 | 3 => format!(
            "{}{{{}}}{{{}}}",
            g.pick(&BINARY),
            expression(g, depth),
            expression(g, depth)
        ),
        4 => format!(
            r"\left{}{}\middle{}{}\right{}",
            g.pick(&DELIMITERS),
            expression(g, depth),
            g.pick(&DELIMITERS),
            expression(g, depth),
            g.pick(&DELIMITERS)
        ),
        5 => format!(
            r"\sqrt[{}]{{{}}}",
            expression(g, depth),
            expression(g, depth)
        ),
        6 => format!(
            r"\genfrac{{{}}}{{{}}}{{{}pt}}{{{}}}{{{}}}{{{}}}",
            g.pick(&DELIMITERS),
            g.pick(&DELIMITERS),
            g.number_text(),
            g.below(5),
            expression(g, depth),
            expression(g, depth)
        ),
        7 => format!(r"\hspace{{{}}}{}", g.number_text(), expression(g, depth)),
        _ => format!(
            r"\substack{{{}\\{}}}",
            expression(g, depth),
            expression(g, depth)
        ),
    }
}

/// A mathtext string: an expression between dollars, maybe broken, maybe
/// with text around it, maybe a long `\frac` chain.
fn source(g: &mut Gen) -> String {
    let depth = g.below(5);
    let math = match g.below(10) {
        0 => {
            let n = 1 + g.below(40);
            format!("{}x{}", r"\frac{".repeat(n), "}{y}".repeat(n))
        }
        1 => expression(g, depth).repeat(1 + g.below(30)),
        _ => expression(g, depth),
    };
    let text = match g.below(4) {
        0 => format!("a ${math}$ b ${}$", expression(g, 1)),
        _ => format!("${math}$"),
    };
    if g.chance(0.4) {
        g.mutate(&text, &BREAKERS)
    } else {
        text
    }
}

/// The fontsets.
const FONTSETS: [Fontset; 6] = [
    Fontset::DejaVuSans,
    Fontset::DejaVuSerif,
    Fontset::Cm,
    Fontset::Stix,
    Fontset::StixSans,
    Fontset::Custom,
];

#[test]
fn typesetting() {
    fuzz(
        "mathtext",
        2_000,
        |g| {
            let style = MathStyle::default()
                .with_fontset(g.pick(&FONTSETS))
                .with_size(g.number_in(1.0, 40.0))
                .with_dpi(g.number_in(10.0, 300.0));
            (source(g), style)
        },
        |(source, style)| {
            if let Ok(layout) = TextEngine::global().typeset_math(source, style) {
                let _ = (layout.to_text_run(), layout.raster_extents());
            }
        },
    );
}

#[test]
fn drawing() {
    fuzz(
        "mathtext drawing",
        300,
        |g| {
            let size = g.number_in(2.0, 30.0);
            let rotation = g.number_in(-360.0, 360.0);
            (source(g), size, rotation, format(g))
        },
        |(source, size, rotation, format)| {
            let fig = Figure::with_size(2.0, 1.0, 50.0);
            let text = fig.text(0.1, 0.5, source.as_str());
            text.set_fontsize(*size).set_rotation(*rotation);
            render(&fig, *format);
            let _ = math_to_image(source, MathImageOptions::default());
        },
    );
}
