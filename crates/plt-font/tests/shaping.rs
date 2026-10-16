//! Text layout compared with matplotlib's `FT2Font.set_text` /
//! `_layout` (libraqm + HarfBuzz + FreeType), including the Last Resort
//! fallback (`tests/fixtures/shaping.json`).

mod common;

use common::{Json, fixture, font_bytes};
use plt_font::{Face, Feature, FontChain, ShapeOptions, Size, shape_at_size};

const LAST_RESORT: &str = "LastResortHE-Regular.ttf";

struct Summary {
    cases: usize,
    failures: Vec<String>,
}

fn check_case(chain: &FontChain<'_>, names: &[&str], case: &Json, summary: &mut Summary) {
    let text = case.get("text").as_str();
    let features: Vec<Feature> = case
        .get("features")
        .as_array()
        .iter()
        .map(|f| f.as_str().parse().expect("feature string"))
        .collect();
    let options = ShapeOptions {
        features,
        ..ShapeOptions::default()
    };
    let (size, dpi) = (case.get("size").as_f64(), case.get("dpi").as_f64());
    let layout = chain.layout(text, size, dpi, &options).expect("layout");
    let label = format!(
        "{} {size}/{dpi} {text:?} {:?}",
        names[0],
        case.get("features")
    );
    summary.cases += 1;
    let extents = vec![
        i64::from(layout.width),
        i64::from(layout.height),
        i64::from(layout.descent),
    ];
    if extents != case.get("extents").i64s() {
        summary.failures.push(format!(
            "{label}: extents {extents:?} != {:?}",
            case.get("extents").i64s()
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
            .as_array()
            .iter()
            .map(|g| {
                let g = g.as_array();
                (
                    g[0].as_str().to_owned(),
                    g[1].as_i64(),
                    g[2].as_i64(),
                    g[3].as_i64(),
                )
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
    let fixtures = fixture("shaping.json");
    let resort_data = font_bytes(LAST_RESORT);
    let mut summary = Summary {
        cases: 0,
        failures: Vec::new(),
    };
    let cases = fixtures.as_array();
    let mut index = 0;
    while index < cases.len() {
        let name = cases[index].get("font").as_str().to_owned();
        let data = font_bytes(&name);
        let mut resort = Face::parse(&resort_data, 0).unwrap();
        resort.select_charmap(0).unwrap();
        let chain = FontChain::new(vec![Face::parse(&data, 0).unwrap(), resort]);
        let names = [name.as_str(), LAST_RESORT];
        while index < cases.len() && cases[index].get("font").as_str() == name {
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

#[test]
fn ascii_bigram_widths_match_matplotlib() {
    let fixtures = fixture("bigrams.json");
    let alphabet: Vec<char> = fixtures.get("alphabet").as_str().chars().collect();
    for (name, widths) in fixtures.as_object() {
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
