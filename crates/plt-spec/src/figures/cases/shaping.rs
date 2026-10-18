//! Text shaped with OpenType features and languages,
//! `figures/cases_shaping.py`: `Text::set_fontfeatures` (ranges included),
//! `Text::set_language` (a tag or character ranges) and `text.language`,
//! drawn for single-line texts only and never measured, in the bundled
//! sans-serif, serif italic and STIX fonts.

use plt::axes::BarOptions;
use plt::fonts::FontStyle;
use plt::legend::LegendOptions;
use plt::patches::ArrowStyle;
use plt::patheffects::{GcUpdate, PathEffect, Stroke};
use plt::prelude::*;
use plt::text::{
    AnnotateOptions, ArrowProps, FancyArrowProps, Feature, HorizontalAlignment, Language,
    LanguageRange, Text, VerticalAlignment,
};

use crate::figures::{CaseResult, FigureCase};

/// `SAMPLE`: kerning pairs, ligatures and a discretionary ligature.
const SAMPLE: &str = "AVAVA office fi first";
/// `SERBIAN`: the letters with Serbian and Macedonian forms.
const SERBIAN: &str = "\u{431}\u{433}\u{434}\u{43f}\u{442} \u{453}";
/// `LATIN`: letters the Latin language systems of the serif italic
/// treat differently.
const LATIN: &str = "\u{a1}fi ffl \u{14a}!";

/// The font of a row.
#[derive(Clone, Copy)]
enum Font {
    /// The default family.
    Default,
    /// One family, upright.
    Family(&'static str),
    /// `ITALIC`: the serif italic.
    Italic,
}

/// One text of `_rows`: text, font, features and language (each set when
/// given).
type Row<'a> = (&'a str, Font, Option<&'a [&'a str]>, Option<Language>);

/// Parses `HarfBuzz` feature strings.
fn features(settings: &[&str]) -> Result<Vec<Feature>, plt::text::ParseFeatureError> {
    settings.iter().map(|setting| setting.parse()).collect()
}

/// Character ranges of a language.
fn ranges(ranges: &[(&str, usize, usize)]) -> Language {
    ranges
        .iter()
        .map(|&(tag, start, end)| LanguageRange::new(tag, start, end))
        .collect()
}

/// Gives `text` the font of a row.
fn set_font(text: &Text, font: Font) {
    match font {
        Font::Default => {}
        Font::Family(family) => {
            text.set_family([family]);
        }
        Font::Italic => {
            text.set_family(["DejaVu Serif"])
                .set_style(FontStyle::Italic);
        }
    }
}

/// Where `_rows` puts its texts: font size, first baseline, spacing and x.
#[derive(Clone, Copy)]
struct Grid {
    size: f64,
    top: f64,
    step: f64,
    x: f64,
}

/// `_rows`' defaults.
const ROWS: Grid = Grid {
    size: 14.0,
    top: 0.92,
    step: 0.11,
    x: 0.03,
};

/// `_rows(fig, rows, ...)`: one text per row, top to bottom.
fn rows(fig: &Figure, rows: Vec<Row<'_>>, grid: Grid) -> CaseResult {
    for (i, (content, font, settings, language)) in rows.into_iter().enumerate() {
        let text = fig.text(grid.x, grid.top - grid.step * i as f64, content);
        text.set_fontsize(grid.size);
        set_font(&text, font);
        if let Some(settings) = settings {
            text.set_fontfeatures(features(settings)?);
        }
        if let Some(language) = language {
            text.set_language(language);
        }
    }
    Ok(())
}

/// `dict(arrowstyle="->")`.
fn arrow() -> Result<Option<ArrowProps>, Box<dyn std::error::Error>> {
    Ok(Some(ArrowProps::Fancy(FancyArrowProps::new(
        "->".parse::<ArrowStyle>()?,
    ))))
}

fn features_case(fig: &Figure) -> CaseResult {
    let (sans, stix) = (Font::Family("DejaVu Sans"), Font::Family("STIXGeneral"));
    let stix_sample = "fi fl ffi ffl AV To";
    rows(
        fig,
        vec![
            (SAMPLE, sans, None, None),
            (SAMPLE, sans, Some(&["-kern"]), None),
            (SAMPLE, sans, Some(&["-liga", "dlig"]), None),
            (
                SAMPLE,
                sans,
                Some(&["-kern[0:3]", "liga=0", "liga[13:]"]),
                None,
            ),
            (
                "Ialy \u{a1}\u{14a}! \u{bf}Ola?",
                sans,
                Some(&["salt", "case"]),
                None,
            ),
            (stix_sample, stix, None, None),
            (stix_sample, stix, Some(&["-liga", "-kern"]), None),
            (
                "gag \u{a1}fi! office",
                Font::Italic,
                Some(&["aalt", "case", "aalt[2]=0"]),
                None,
            ),
        ],
        ROWS,
    )?;
    for (i, ha) in [HorizontalAlignment::Right, HorizontalAlignment::Center]
        .into_iter()
        .enumerate()
    {
        fig.text(0.97 - 0.35 * i as f64, 0.04, "WAVE To")
            .set_halign(ha)
            .set_fontsize(14.0)
            .set_fontfeatures(features(&["-kern"])?);
    }
    Ok(())
}

fn languages_case(fig: &Figure) -> CaseResult {
    let grid = Grid {
        size: 16.0,
        step: 0.12,
        ..ROWS
    };
    let serbian = [
        None,
        Some("sr"),
        Some("mk"),
        Some("SR_rs"),
        Some("x-hbotsrb"),
        Some("ru"),
    ];
    rows(
        fig,
        serbian
            .into_iter()
            .map(|tag| (SERBIAN, Font::Italic, None, tag.map(Language::from)))
            .collect(),
        grid,
    )?;
    let latin = [
        None,
        Some("tr"),
        Some("se"),
        Some("ca"),
        Some("azb"),
        Some("trk"),
    ];
    rows(
        fig,
        latin
            .into_iter()
            .map(|tag| {
                let case: Option<&[&str]> = Some(&["case"]);
                (LATIN, Font::Italic, case, tag.map(Language::from))
            })
            .collect(),
        Grid { x: 0.55, ..grid },
    )?;
    for (x, tag) in [(0.03, "sr"), (0.55, "ca")] {
        fig.text(x, 0.12, "office \u{431}\u{433}\u{434}")
            .set_fontsize(16.0)
            .set_language(Language::from(tag));
    }
    Ok(())
}

fn language_ranges_case(fig: &Figure) -> CaseResult {
    let words = "\u{431}\u{433}\u{434}\u{43f}\u{442} \u{431}\u{433}\u{434}\u{43f}\u{442}";
    let grid = Grid {
        size: 15.0,
        top: 0.88,
        step: 0.15,
        x: 0.03,
    };
    rows(
        fig,
        vec![
            (words, Font::Italic, None, Some(ranges(&[("sr", 0, 5)]))),
            (
                words,
                Font::Italic,
                None,
                Some(ranges(&[("sr", 0, 11), ("ru", 3, 8)])),
            ),
            (
                words,
                Font::Italic,
                None,
                Some(ranges(&[("mk", 6, 11), ("sr", 4, 2)])),
            ),
            (words, Font::Italic, None, Some(ranges(&[]))),
            (
                "fi \u{431}\u{433}\u{434} fi",
                Font::Italic,
                None,
                Some(ranges(&[("tr", 0, 3), ("sr", 3, 7)])),
            ),
        ],
        grid,
    )?;
    rows(
        fig,
        vec![
            (
                "office office",
                Font::Default,
                None,
                Some(ranges(&[("ca", 0, 6)])),
            ),
            (
                "office AV",
                Font::Default,
                None,
                Some(ranges(&[("en", 0, 2)])),
            ),
            (
                "office AV",
                Font::Default,
                None,
                Some(ranges(&[("ca", 2, 4)])),
            ),
        ],
        Grid { x: 0.6, ..grid },
    )
}

fn rc_language_case(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.bar(
        ["\u{431}\u{433}", "\u{434}\u{43f}", "\u{442}\u{431}"],
        [3.0, 1.0, 2.0],
        BarOptions {
            label: Some("\u{433}\u{434}\u{43f}".to_owned()),
            ..BarOptions::default()
        },
    )?;
    ax.set_title("\u{41d}\u{430}\u{441}\u{43b}\u{43e}\u{432} \u{431}\u{433}\u{434}\u{43f}\u{442}");
    ax.set_xlabel("\u{43f}\u{443}\u{442}")
        .set_ylabel("\u{434}\u{443}\u{433}");
    ax.legend(LegendOptions::default().loc("upper right"))?;
    ax.text(1.0, 2.6, "\u{431}\u{433}\u{434}\u{43f}\u{442} ru")
        .set_halign(HorizontalAlignment::Center)
        .set_language(Language::from("ru"));
    ax.text(
        1.0,
        2.0,
        "\u{431}\u{433}\u{434}\u{43f}\u{442}\n\u{431}\u{433}\u{434}",
    )
    .set_halign(HorizontalAlignment::Center);
    ax.annotate(
        "\u{433}\u{434}\u{43f}",
        (2.0, 2.0),
        AnnotateOptions {
            xytext: Some((1.6, 1.4).into()),
            arrowprops: arrow()?,
            ..AnnotateOptions::default()
        },
    )?;
    Ok(())
}

fn single_lines_only_case(fig: &Figure) -> CaseResult {
    let ax = fig.subplot();
    ax.set_xlim(0.0, 10.0).set_ylim(0.0, 10.0);
    ax.set_axis_off();
    ax.text(0.3, 9.0, "office AV\n\u{431}\u{433}\u{434} fi")
        .set_fontsize(13.0)
        .set_valign(VerticalAlignment::Top)
        .set_fontfeatures(features(&["-liga", "-kern"])?)
        .set_language(Language::from("sr"));
    ax.text(5.5, 8.6, r"office $\alpha$ fi")
        .set_fontsize(13.0)
        .set_fontfeatures(features(&["-liga"])?);
    let grey = Stroke::new(GcUpdate::default().linewidth(3.0).foreground("0.7")?);
    let stroked = ax.text(0.3, 5.0, "office \u{431}\u{433}\u{434}");
    stroked
        .set_fontsize(18.0)
        .set_path_effects(vec![PathEffect::with_stroke(grey)]);
    set_font(&stroked, Font::Italic);
    stroked
        .set_fontfeatures(features(&["-liga"])?)
        .set_language(ranges(&[("sr", 7, 10)]));
    let rotated = ax.text(8.0, 1.0, "office \u{431}\u{433}\u{434}");
    rotated.set_fontsize(14.0).set_rotation(30.0);
    set_font(&rotated, Font::Italic);
    rotated
        .set_fontfeatures(features(&["-liga"])?)
        .set_language(Language::from("mk"));
    ax.annotate(
        "office fi",
        (5.0, 2.0),
        AnnotateOptions {
            xytext: Some((1.0, 1.2).into()),
            arrowprops: arrow()?,
            ..AnnotateOptions::default()
        },
    )?
    .set_fontsize(13.0)
    .set_fontfeatures(features(&["-liga"])?);
    Ok(())
}

fn hinted_case(fig: &Figure) -> CaseResult {
    let case: Option<&[&str]> = Some(&["case"]);
    rows(
        fig,
        vec![
            (SAMPLE, Font::Default, Some(&["-kern", "-liga"]), None),
            (SERBIAN, Font::Italic, None, Some(Language::from("sr"))),
            (LATIN, Font::Italic, case, Some(Language::from("tr"))),
        ],
        Grid {
            top: 0.75,
            step: 0.3,
            ..ROWS
        },
    )
}

pub(crate) const CASES: &[FigureCase] = &[
    FigureCase::new("shaping/features", features_case).sized(6.0, 4.0, 100.0),
    FigureCase::new("shaping/languages", languages_case).sized(6.0, 4.0, 100.0),
    FigureCase::new("shaping/language_ranges", language_ranges_case).sized(6.0, 3.5, 100.0),
    FigureCase::new("shaping/rc_language", rc_language_case)
        .sized(5.0, 4.0, 100.0)
        .with_rc(&[
            ("text.language", "sr"),
            ("font.family", "DejaVu Serif"),
            ("font.style", "italic"),
        ]),
    FigureCase::new("shaping/single_lines_only", single_lines_only_case).sized(5.0, 3.5, 100.0),
    FigureCase::new("shaping/hinted", hinted_case)
        .sized(5.0, 2.5, 100.0)
        .with_rc(&[("text.hinting", "default")]),
];
