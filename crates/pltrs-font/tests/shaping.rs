//! Text layout compared with matplotlib's `FT2Font.set_text` /
//! `_layout` (libraqm + HarfBuzz + FreeType), including the Last Resort
//! fallback (`tests/fixtures/shaping.json`), with right-to-left, bidi and
//! Arabic text (`complex.json`, `arabic.json`).

mod common;

use common::json::{Fixture, Json};
use common::{fixture, font_bytes};
use pltrs_font::{
    Direction, Face, Feature, FontChain, Hinting, ShapeOptions, Size, shape_at_size, shape_with,
};

const LAST_RESORT: &str = "LastResortHE-Regular.ttf";

struct Summary {
    cases: usize,
    failures: Vec<String>,
}

fn check_case(chain: &FontChain<'_>, names: &[&str], case: &Json, summary: &mut Summary) {
    let text = case.field("text").str();
    let features: Vec<Feature> = case
        .field("features")
        .array()
        .iter()
        .map(|f| f.str().parse().expect("feature string"))
        .collect();
    let options = ShapeOptions {
        features,
        ..ShapeOptions::default()
    };
    let (size, dpi) = (case.field("size").f64(), case.field("dpi").f64());
    let layout = chain
        .layout(text, size, dpi, &options, Hinting::None)
        .expect("layout");
    let label = format!(
        "{} {size}/{dpi} {text:?} {:?}",
        names[0],
        case.field("features")
    );
    summary.cases += 1;
    let extents = vec![
        i64::from(layout.width),
        i64::from(layout.height),
        i64::from(layout.descent),
    ];
    if extents != case.field("extents").i64s() {
        summary.failures.push(format!(
            "{label}: extents {extents:?} != {:?}",
            case.field("extents").i64s()
        ));
    }
    if let Some(glyphs) = case.opt("glyphs") {
        let actual: Vec<(String, i64, i64, i64)> = layout
            .glyphs
            .iter()
            .map(|g| {
                (
                    names[g.face].to_owned(),
                    i64::from(g.glyph_id.0),
                    i64::from(g.x),
                    i64::from(g.y),
                )
            })
            .collect();
        let expected: Vec<(String, i64, i64, i64)> = glyphs
            .array()
            .iter()
            .map(|g| {
                let g = g.array();
                (g[0].str().to_owned(), g[1].i64(), g[2].i64(), g[3].i64())
            })
            .collect();
        if actual != expected {
            summary.failures.push(format!(
                "{label}: glyphs\n  actual   {actual:?}\n  expected {expected:?}"
            ));
        }
    }
}

#[test]
fn layout_matches_matplotlib() {
    check_fixture("shaping.json");
}

/// Right-to-left, bidirectional and Arabic text.
#[test]
fn complex_layout_matches_matplotlib() {
    check_fixture("complex.json");
}

/// Lays out every case of fixture file `name` with its font plus the Last
/// Resort fallback and compares with matplotlib.
fn check_fixture(name: &str) {
    let fixtures = fixture(name);
    let resort_data = font_bytes(LAST_RESORT);
    let mut summary = Summary {
        cases: 0,
        failures: Vec::new(),
    };
    let cases = fixtures.array();
    let mut index = 0;
    while index < cases.len() {
        let name = cases[index].field("font").str().to_owned();
        let data = font_bytes(&name);
        let mut resort = Face::parse(&resort_data, 0).expect("Last Resort parses");
        resort.select_charmap(0).expect("Last Resort has a charmap");
        let face = Face::parse(&data, 0).expect("font parses");
        let chain = FontChain::new(vec![face, resort]);
        let names = [name.as_str(), LAST_RESORT];
        while index < cases.len() && cases[index].field("font").str() == name {
            check_case(&chain, &names, &cases[index], &mut summary);
            index += 1;
        }
    }
    assert!(
        summary.failures.is_empty(),
        "{} of {} layouts differ:\n{}",
        summary.failures.len(),
        summary.cases,
        summary.failures.join("\n")
    );
}

/// Small fonts from `tests/fixtures/gen_arabic_fonts.py`: Arabic fallback
/// shaping (presentation forms, ligatures, mark stacking) and cursive and
/// mark attachment in both directions.
#[test]
fn synthetic_arabic_fonts_match_matplotlib() {
    let fixtures = fixture("arabic.json");
    let mut failures = Vec::new();
    for (kind, cases) in fixtures.object() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(format!("arabic-{kind}.ttf"));
        let data = std::fs::read(&path).unwrap();
        let chain = FontChain::new(vec![Face::parse(&data, 0).unwrap()]);
        for case in cases.array() {
            let text = case.field("text").str();
            let (size, dpi) = (case.field("size").f64(), case.field("dpi").f64());
            let layout = chain
                .layout(text, size, dpi, &ShapeOptions::default(), Hinting::None)
                .expect("layout");
            let glyphs: Vec<Vec<i64>> = layout
                .glyphs
                .iter()
                .map(|g| vec![i64::from(g.glyph_id.0), i64::from(g.x), i64::from(g.y)])
                .collect();
            let expected: Vec<Vec<i64>> = case
                .field("glyphs")
                .array()
                .iter()
                .map(Json::i64s)
                .collect();
            let extents = vec![
                i64::from(layout.width),
                i64::from(layout.height),
                i64::from(layout.descent),
            ];
            if glyphs != expected || extents != case.field("extents").i64s() {
                failures.push(format!(
                    "{kind} {size}/{dpi} {text:?}:\n  actual   {extents:?} {glyphs:?}\n  expected {:?} {expected:?}",
                    case.field("extents").i64s()
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Glyphs and pen positions (26.6 pixels at 12 pt / 72 dpi) with
/// `DejaVuSans.ttf` as matplotlib's `FT2Font._layout` reports them.
#[test]
fn arabic_and_mixed_text_in_visual_order() {
    let data = font_bytes("DejaVuSans.ttf");
    let chain = FontChain::new(vec![Face::parse(&data, 0).unwrap()]);
    let placed = |text: &str| {
        let layout = chain
            .layout(text, 12.0, 72.0, &ShapeOptions::default(), Hinting::None)
            .unwrap();
        let glyphs: Vec<(u16, i32)> = layout.glyphs.iter().map(|g| (g.glyph_id.0, g.x)).collect();
        (glyphs, layout.width)
    };
    // MEEM REH HAH BEH ALEF: joined forms, rightmost letter last.
    assert_eq!(
        placed("\u{645}\u{631}\u{62D}\u{628}\u{627}"),
        (
            vec![
                (5253, 0),
                (5257, 234),
                (5274, 466),
                (5285, 941),
                (5338, 1365)
            ],
            1776
        )
    );
    // The digits after the Arabic word are Arabic numbers (W2) and move
    // with it to the left of the Arabic text.
    assert_eq!(
        placed("abc \u{645}\u{631}\u{62D}\u{628}\u{627} 123"),
        (
            vec![
                (68, 0),
                (69, 471),
                (70, 959),
                (3, 1381),
                (20, 1625),
                (21, 2114),
                (22, 2603),
                (3, 3092),
                (5253, 3336),
                (5257, 3570),
                (5274, 3802),
                (5285, 4277),
                (5338, 4701)
            ],
            5112
        )
    );
}

#[test]
fn paragraph_direction_can_be_forced() {
    let data = font_bytes("DejaVuSans.ttf");
    let face = Face::parse(&data, 0).unwrap();
    let text = "ab \u{5D0}";
    let order = |direction| {
        let options = ShapeOptions {
            direction,
            ..ShapeOptions::default()
        };
        shape_with(&face, text, &options)
            .iter()
            .map(|g| g.cluster)
            .collect::<Vec<_>>()
    };
    assert_eq!(order(None), vec![0, 1, 2, 3]);
    assert_eq!(order(Some(Direction::RightToLeft)), vec![3, 2, 0, 1]);
}

#[test]
fn ascii_bigram_widths_match_matplotlib() {
    let fixtures = fixture("bigrams.json");
    let alphabet: Vec<char> = fixtures.field("alphabet").str().chars().collect();
    for (name, widths) in fixtures.object() {
        if name == "alphabet" {
            continue;
        }
        let data = font_bytes(name);
        let face = Face::parse(&data, 0).unwrap();
        let size = Size::new(&face, 10.0, 100.0);
        let widths = widths.i64s();
        let mut mismatches = Vec::new();
        for (index, expected) in widths.iter().enumerate() {
            let text: String = [
                alphabet[index / alphabet.len()],
                alphabet[index % alphabet.len()],
            ]
            .iter()
            .collect();
            let glyphs = shape_at_size(&face, &text, &size, &ShapeOptions::default());
            let width: i64 = glyphs.iter().map(|g| i64::from(g.x_advance)).sum();
            if width != *expected {
                mismatches.push(format!("{text:?}: {width} != {expected}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "{name}: {} bigrams differ: {mismatches:?}",
            mismatches.len()
        );
    }
}
