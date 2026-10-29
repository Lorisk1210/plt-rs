//! The shaping cases of `test_text.py`: complex scripts with font
//! fallback, OpenType features and languages, drawn as text and, through
//! the renderer's fallback, as paths.

use crate::figures::cases::common::ranges;
use pltrs::figure::SubfigureOptions;
use pltrs::patheffects::PathEffect;
use pltrs::prelude::*;
use pltrs::text::{Feature, HAlign, Language, Text, VAlign};

use crate::figures::{CaseResult, FigureCase};

/// What the fallible shaping helpers return: the texts they added.
type Texts = Result<Vec<Text>, Box<dyn std::error::Error>>;

/// `_test_complex_shaping`: Arabic between n-ary operators, then the same
/// with `cmr10` and `DejaVu Sans Display` ahead of the fallback.
fn complex_shaping(fig: &Figure) -> Vec<Text> {
    const MATH_SIGNS: &str = "\u{220f}\u{2210}\u{2211}\u{222b}";
    const ARABIC: &str = "Arabic: \u{631}\u{64e}\u{642}\u{652}\u{645}";
    let text = format!("{MATH_SIGNS}{ARABIC}{MATH_SIGNS}");
    let texts: Vec<Text> = [0.75, 0.25]
        .into_iter()
        .map(|y| {
            let t = fig.text(0.5, y, text.as_str());
            t.set_fontsize(32.0)
                .set_halign(HAlign::Center)
                .set_valign(VAlign::Center);
            t
        })
        .collect();
    texts[1].set_family(["cmr10", "DejaVu Sans Display", "DejaVu Sans"]);
    texts
}

/// `_test_text_features`: ligatures by default, disabled, and
/// discretionary.
fn text_features(fig: &Figure) -> Texts {
    let rows: [(f64, &str, &[&str]); 3] = [
        (0.7, "Default: fi ffi fl st", &[]),
        (0.4, "Disabled: fi ffi fl st", &["-liga"]),
        (0.1, "Discretionary: fi ffi fl st", &["dlig"]),
    ];
    let mut texts = Vec::new();
    for (y, content, settings) in rows {
        let text = fig.text(1.0, y, content);
        text.set_fontsize(32.0).set_halign(HAlign::Right);
        if !settings.is_empty() {
            let features: Vec<Feature> = settings
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, _>>()?;
            text.set_fontfeatures(features);
        }
        texts.push(text);
    }
    Ok(texts)
}

/// `_test_text_language`: Serbian forms of a Cyrillic letter and Sámi
/// forms of a Latin one, by text, by range, and from `text.language`.
fn text_language(fig: &Figure) -> Texts {
    const CYRILLIC: &str = "\u{431}";
    const LATIN: &str = "\u{14a}";
    let big = |x: f64, y: f64, content: &str| {
        let text = fig.text(x, y, content);
        text.set_fontsize(32.0);
        text
    };
    let mut texts: Vec<Text> = [
        (0.8, "Default"),
        (0.55, "Lang A"),
        (0.3, "Lang B"),
        (0.05, "Mixed"),
    ]
    .into_iter()
    .map(|(y, label)| big(0.0, y, label))
    .collect();
    texts.push(big(0.4, 0.8, CYRILLIC));
    let serbian = big(0.4, 0.55, CYRILLIC);
    serbian.set_language(Language::from("sr"));
    let russian = big(0.4, 0.3, CYRILLIC);
    russian.set_language(Language::from("ru"));
    let mixed = big(0.4, 0.05, &CYRILLIC.repeat(4));
    mixed.set_language(ranges(&[
        ("ru", 0, 1),
        ("sr", 1, 2),
        ("ru", 2, 3),
        ("sr", 3, 4),
    ]));
    texts.extend([serbian, russian, mixed]);
    texts.push(big(0.7, 0.8, LATIN));
    let english = pltrs::rc::context(|| {
        pltrs::rc::set("text.language", "en")?;
        Ok::<_, pltrs::rc::RcError>(big(0.7, 0.55, LATIN))
    })?;
    let sami = big(0.7, 0.3, LATIN);
    sami.set_language(Language::from("smn"));
    let mixed = big(0.7, 0.05, &LATIN.repeat(4));
    mixed.set_language(ranges(&[
        ("en", 0, 1),
        ("smn", 1, 2),
        ("en", 2, 3),
        ("smn", 3, 4),
    ]));
    texts.extend([english, sami, mixed]);
    Ok(texts)
}

/// `test_draw_text_as_path_fallback`: the three shaping figures in
/// subfigures, every text drawn as paths (matplotlib's
/// `RendererBase.draw_text`, which a `Normal` path effect takes too).
fn draw_text_fallback(fig: &Figure) -> CaseResult {
    let subfigs = fig.subfigures(
        3,
        1,
        SubfigureOptions {
            height_ratios: Some(vec![2.0, 1.5, 3.0]),
            ..SubfigureOptions::default()
        },
    )?;
    let texts = [
        complex_shaping(&subfigs[(0, 0)]),
        text_features(&subfigs[(1, 0)])?,
        text_language(&subfigs[(2, 0)])?,
    ];
    for text in texts.iter().flatten() {
        text.set_path_effects(vec![PathEffect::normal()]);
    }
    Ok(())
}

pub(super) const CASES: &[FigureCase] = &[
    FigureCase::new("mpl_text/complex", |fig| {
        complex_shaping(fig);
        Ok(())
    })
    .sized(6.0, 2.0, 100.0),
    FigureCase::new("mpl_text/features", |fig| {
        text_features(fig)?;
        Ok(())
    })
    .sized(5.0, 1.5, 100.0),
    FigureCase::new("mpl_text/language", |fig| {
        text_language(fig)?;
        Ok(())
    })
    .sized(5.0, 3.0, 100.0),
    FigureCase::new("mpl_text/draw_text_fallback", draw_text_fallback).sized(6.0, 6.5, 100.0),
];
