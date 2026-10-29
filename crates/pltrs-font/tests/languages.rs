//! Text layout with languages and features set for parts of the text,
//! compared with matplotlib's `FT2Font._layout(text, NO_HINTING,
//! features=..., language=...)` at 12 pt / 72 dpi (glyphs and pen
//! positions in 26.6 pixels).

mod common;

use common::font_bytes;
use pltrs_font::{Face, Feature, FontChain, Hinting, LanguageRange, ShapeOptions};

fn layout(font: &str, text: &str, options: &ShapeOptions) -> Vec<(u16, i32)> {
    let data = font_bytes(font);
    let chain = FontChain::new(vec![Face::parse(&data, 0).expect("bundled font")]);
    chain
        .layout(text, 12.0, 72.0, options, Hinting::None)
        .expect("layout")
        .glyphs
        .iter()
        .map(|g| (g.glyph_id.0, g.x))
        .collect()
}

fn languages(ranges: &[(&str, usize, usize)]) -> ShapeOptions {
    ShapeOptions {
        languages: ranges
            .iter()
            .map(|&(tag, start, end)| LanguageRange::new(tag, start, end))
            .collect(),
        ..ShapeOptions::default()
    }
}

fn features(settings: &[&str]) -> ShapeOptions {
    ShapeOptions {
        features: settings
            .iter()
            .map(|s| s.parse().expect("feature string"))
            .collect(),
        ..ShapeOptions::default()
    }
}

const SERBIAN: &str = "\u{431}\u{433}\u{434}\u{43f}\u{442}";

#[test]
fn serbian_forms_of_dejavu_serif_italic() {
    let font = "DejaVuSerif-Italic.ttf";
    let default = [(923, 0), (925, 469), (926, 857), (937, 1344), (940, 1839)];
    assert_eq!(layout(font, SERBIAN, &ShapeOptions::default()), default);
    let serbian = [
        (3291, 0),
        (3290, 462),
        (3292, 708),
        (3293, 1200),
        (3294, 1695),
    ];
    assert_eq!(layout(font, SERBIAN, &languages(&[("sr", 0, 5)])), serbian);
    assert_eq!(layout(font, SERBIAN, &languages(&[("sr", 0, 99)])), serbian);
    assert_eq!(
        layout(font, SERBIAN, &languages(&[("sr", 1, 3)])),
        [(923, 0), (3290, 469), (3292, 715), (937, 1207), (940, 1702)]
    );
    assert_eq!(
        layout(font, SERBIAN, &languages(&[("sr", 0, 4), ("ru", 2, 5)])),
        [(3291, 0), (3290, 462), (926, 708), (937, 1195), (940, 1690)],
        "later ranges win"
    );
    assert_eq!(layout(font, SERBIAN, &languages(&[("sr", 3, 1)])), default);
    assert_eq!(layout(font, SERBIAN, &languages(&[("", 0, 5)])), default);
    assert_eq!(layout(font, "", &languages(&[("sr", 0, 9)])), []);
}

#[test]
fn catalan_drops_the_ligatures_and_languages_split_runs() {
    let font = "DejaVuSans.ttf";
    let ligated = [(82, 0), (5041, 470), (70, 1213), (72, 1635)];
    assert_eq!(layout(font, "office", &ShapeOptions::default()), ligated);
    assert_eq!(
        layout(font, "office", &languages(&[("ca", 0, 6)])),
        [
            (82, 0),
            (73, 470),
            (73, 740),
            (76, 1010),
            (70, 1223),
            (72, 1645)
        ]
    );
    // A language for "of" splits the run before the second f, so only
    // "fi" ligates; "@" is the empty language, still a different one.
    let split = [(82, 0), (73, 470), (5039, 740), (70, 1224), (72, 1646)];
    assert_eq!(layout(font, "office", &languages(&[("en", 0, 2)])), split);
    assert_eq!(layout(font, "office", &languages(&[("@", 0, 2)])), split);
}

#[test]
fn feature_values_number_the_alternates() {
    let font = "DejaVuSerif-Italic.ttf";
    let plain = [(74, 0), (68, 492), (74, 950)];
    let alternate = [(3484, 0), (68, 492), (3484, 950)];
    assert_eq!(layout(font, "gag", &ShapeOptions::default()), plain);
    assert_eq!(layout(font, "gag", &features(&["aalt"])), alternate);
    assert_eq!(layout(font, "gag", &features(&["salt"])), alternate);
    assert_eq!(
        layout(font, "gag", &features(&["aalt=2"])),
        plain,
        "one alternate only"
    );
    assert_eq!(layout(font, "gag", &features(&["aalt=0"])), plain);
    assert_eq!(layout(font, "gag", &features(&["aalt[1:2]"])), plain);
}

#[test]
fn features_for_parts_of_the_text() {
    let font = "DejaVuSans.ttf";
    assert_eq!(
        layout(font, "office fi", &features(&["-liga[0:3]"])),
        [
            (82, 0),
            (73, 470),
            (73, 740),
            (76, 1010),
            (70, 1223),
            (72, 1645),
            (3, 2118),
            (5039, 2362)
        ]
    );
    let unkerned = [(36, 0), (57, 525), (36, 1050), (57, 1575), (36, 2100)];
    assert_eq!(layout(font, "AVAVA", &features(&["-kern"])), unkerned);
    // Pair kerning needs both glyphs in the range.
    assert_eq!(
        layout(font, "AVAVA", &features(&["-kern", "kern[1:2]"])),
        unkerned
    );
    let ranged = Feature::new(pltrs_font::Tag::new(b"kern"), 1).with_range(1, 3);
    let options = ShapeOptions {
        features: vec!["-kern".parse().unwrap(), ranged],
        ..ShapeOptions::default()
    };
    assert_eq!(
        layout(font, "AVAVA", &options),
        [(36, 0), (57, 525), (36, 1001), (57, 1526), (36, 2051)]
    );
    let last_pair = [(36, 0), (57, 525), (36, 1050), (57, 1575), (36, 2051)];
    assert_eq!(
        layout(font, "AVAVA", &features(&["kern[1:3]=0"])),
        last_pair
    );
    // A later global setting gives kern the global bit, which the ranged
    // setting then clears for every global feature of those characters.
    assert_eq!(
        layout(font, "AVAVA", &features(&["kern[1:3]=0", "kern"])),
        last_pair
    );
}
