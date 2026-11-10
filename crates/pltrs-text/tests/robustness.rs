//! Robustness: users type arbitrary `$...$` strings. Malformed, deeply
//! nested, huge or random input must produce a layout or a
//! [`TextError::Math`], never a panic, a hang or a stack overflow.

use pltrs_core::rng::SplitMix64;
use pltrs_text::fontconfig::FontconfigPattern;
use pltrs_text::mathtext::{Fontset, MathStyle};
use pltrs_text::{FontQuery, FontRequest, SET_CHAR_SIZE_LINE, TextEngine, TextError};

const FONTSETS: [Fontset; 5] = [
    Fontset::DejaVuSans,
    Fontset::Cm,
    Fontset::Stix,
    Fontset::StixSans,
    Fontset::Custom,
];

/// Lays `source` out in every fontset; only errors are acceptable failures.
fn typeset(source: &str) -> Vec<Result<(), TextError>> {
    FONTSETS
        .iter()
        .map(|&fontset| {
            let style = MathStyle::default().with_fontset(fontset);
            let layout = TextEngine::global().typeset_math(source, &style)?;
            let _ = (layout.to_text_run(), layout.raster_extents());
            Ok(())
        })
        .collect()
}

fn assert_math_error(source: &str) {
    for result in typeset(source) {
        assert!(
            matches!(result, Err(TextError::Math(_))),
            "{source:.60}: {result:?}"
        );
    }
}

#[test]
fn deep_nesting_is_rejected_not_overflowed() {
    let depth = 10_000;
    let nest =
        |open: &str, close: &str| format!("${}x{}$", open.repeat(depth), close.repeat(depth));
    for source in [
        nest("{", "}"),
        nest(r"\frac{", "}{y}"),
        nest(r"\sqrt{", "}"),
        nest("x^{", "}"),
        nest("x_{", "}"),
        nest(r"\left(", r"\right)"),
        nest(r"\overline{", "}"),
        nest(r"\hat{", "}"),
        nest(r"\mathrm{", "}"),
        nest(r"\operatorname{", "}"),
        nest(r"\genfrac{(}{)}{1pt}{0}{", "}{y}"),
        nest(r"\binom{", "}{y}"),
        nest(r"\boldsymbol{", "}"),
        nest(r"\substack{", "}"),
        format!("${}x$", "{".repeat(depth)),
        format!("$x{}$", "}".repeat(depth)),
        format!("${}$", "x^".repeat(depth)),
        format!("${}$", r"\frac".repeat(depth)),
        format!("${}x$", r"\sqrt".repeat(depth)),
    ] {
        let _ = typeset(&source);
    }
}

#[test]
fn malformed_math_reports_errors() {
    for source in [
        r"$\frac$",
        r"$\frac{a}$",
        r"$\frac{a}{$",
        r"$\sqrt[$",
        r"$\left($",
        r"$x^$",
        r"$x_$",
        r"$x^^y$",
        r"$x__y$",
        r"$x^_y$",
        r"$\nosuchcommand$",
        r"${$",
        r"$}$",
        r"$\$",
        r"$\hspace{}$",
        r"$\hspace{x}$",
        r"$\operatorname$",
        r"$\genfrac{}{}{}{}{}$",
    ] {
        assert_math_error(source);
    }
}

#[test]
fn degenerate_math_does_not_panic() {
    for source in [
        "$",
        "$$",
        "$$$",
        "$ $",
        "${}$",
        "${}{}^{}_{}$",
        "$^{}$",
        "$_{}$",
        r"$\hspace{1e300}x$",
        r"$\hspace{-1e300}x$",
        r"$\hspace{1e308}\hspace{1e308}x$",
        r"$\hspace{nan}x$",
        r"$\hspace{inf}x$",
        r"$\hspace{99999999999999999999999999999999999}x$",
        r"$\sqrt[{}]{}$",
        r"$\left.\right.$",
        r"$\left\{\middle|\right\}$",
        r"\\\\\\",
        "$x$\u{0}$y$",
        "$\u{10FFFF}\u{FFFF}\u{200B}$",
        "$\u{301}\u{301}x$",
        "a\tb\n$c$\r\n",
    ] {
        let _ = typeset(source);
    }
}

#[test]
fn long_inputs_stay_linear() {
    let long = format!("${}$", "x+".repeat(20_000));
    let _ = typeset(&long);
    let scripts = format!("${}$", "x^{2}_{3}".repeat(2_000));
    let _ = typeset(&scripts);
    let unbalanced = format!("${}$", "{x".repeat(2_000));
    let _ = typeset(&unbalanced);
    let plain = "word ".repeat(20_000);
    let _ = typeset(&plain);
}

#[test]
fn extreme_sizes_and_resolutions_do_not_panic() {
    let engine = TextEngine::global();
    let nasty = [0.0, -10.0, 1e-300, 1e9, 1e300, f64::NAN, f64::INFINITY];
    for size in nasty {
        for dpi in [72.0, 0.0, -1.0, 1e12, f64::NAN] {
            let style = MathStyle::default().with_size(size).with_dpi(dpi);
            if let Ok(layout) = engine.typeset_math(r"$\frac{a}{b}\sqrt{x^2}$ text", &style) {
                let _ = (layout.to_text_run(), layout.raster_extents());
            }
            let mut request = FontRequest::new(FontQuery::default());
            request.query.size = size;
            if let Ok(run) = engine.layout_line("Hello, world", &request, dpi) {
                for glyph in &run.glyphs {
                    let _ = engine.glyph_outline(glyph.font, glyph.glyph, size, dpi);
                }
            }
            let _ = engine.font_height_metrics(&request, dpi);
        }
    }
}

#[test]
fn fontconfig_soups_do_not_panic() {
    const PIECES: [&str; 14] = [
        ":", "-", ",", "=", "\\", "_", " ", "a", "1", ".", "bold", "size=", "e", "\u{301}",
    ];
    let mut rng = SplitMix64::new(0xFC);
    for _ in 0..2_000 {
        let soup: String = (0..rng.below(30))
            .map(|_| PIECES[rng.below(PIECES.len())])
            .collect();
        let _ = FontconfigPattern::parse(&soup);
    }
}

/// Strings mixing TeX-significant characters, commands and letters.
#[test]
fn character_soups_do_not_panic() {
    const PIECES: [&str; 40] = [
        "\\", "{", "}", "^", "_", "$", "%", "&", "#", "~", "'", "\"", ",", ".", ";", ":", "!", "[",
        "]", "(", ")", "|", "/", "<", ">", "=", "+", "-", " ", "x", "1", r"\frac", r"\sqrt",
        r"\left", r"\right", r"\mathrm", r"\hat", r"\alpha", r"\hspace", r"\sum",
    ];
    let mut rng = SplitMix64::new(0x5EED);
    for _ in 0..300 {
        let len = 1 + rng.below(24);
        let soup: String = (0..len).map(|_| PIECES[rng.below(PIECES.len())]).collect();
        let _ = typeset(&format!("${soup}$"));
        let _ = typeset(&soup);
    }
}

#[test]
fn sizes_of_no_pixel_fail_as_matplotlibs_ft2font_reports_freetype() {
    let request = FontRequest::new(FontQuery::default());
    let refused = TextEngine::global().layout_line("ab", &request, 2.5);
    let message = format!(
        "FT_Set_Char_Size (ft2font.cpp line {SET_CHAR_SIZE_LINE}) failed with error 0x97: \
         invalid ppem value"
    );
    assert!(matches!(&refused, Err(TextError::FontSize(_))));
    assert_eq!(
        refused.err().map(|err| err.to_string()).as_deref(),
        Some(message.as_str())
    );
    let math = TextEngine::global().typeset_math(r"$x^2$", &MathStyle::default().with_dpi(2.5));
    assert_eq!(
        math.err().map(|err| err.to_string()).as_deref(),
        Some(message.as_str())
    );
    assert!(
        TextEngine::global()
            .layout_line("ab", &request, 72.0)
            .is_ok()
    );
}
