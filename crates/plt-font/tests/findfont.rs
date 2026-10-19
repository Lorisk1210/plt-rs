//! Font catalogue entries and `findfont` results compared with matplotlib's
//! font manager restricted to its bundled fonts
//! (`tests/fixtures/findfont.json`); Inter, which plt-rs bundles besides
//! them, is left out of the catalogue comparison.

mod common;

use common::{Json, fixture};
use plt_font::{FontDatabase, FontQuery, FontSource, FontStyle, FontVariant, Stretch, Weight};

fn file_of(db: &FontDatabase, id: plt_font::FontId) -> String {
    db.source(id)
        .and_then(FontSource::file_name)
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn catalogue_matches_matplotlib_ttflist() {
    let expected = fixture("findfont.json");
    let db = FontDatabase::with_bundled_fonts();
    let mut actual: Vec<String> = db
        .entries()
        .iter()
        .filter(|e| !file_of(&db, e.font).starts_with("Inter-"))
        .map(|e| {
            format!(
                "{}|{}|{:?}|{:?}|{}|{}",
                file_of(&db, e.font),
                e.name,
                e.style,
                e.variant,
                e.weight,
                e.stretch.0
            )
        })
        .collect();
    let mut wanted: Vec<String> = expected
        .get("entries")
        .as_array()
        .iter()
        .map(|e| {
            let style = FontStyle::parse(e.get("style").as_str()).unwrap();
            let variant = if e.get("variant").as_str() == "small-caps" {
                FontVariant::SmallCaps
            } else {
                FontVariant::Normal
            };
            let stretch = Stretch::parse(e.get("stretch").as_str()).unwrap();
            format!(
                "{}|{}|{style:?}|{variant:?}|{}|{}",
                e.get("file").as_str(),
                e.get("name").as_str(),
                e.get("weight").as_i64(),
                stretch.0
            )
        })
        .collect();
    actual.sort();
    wanted.sort();
    assert_eq!(actual, wanted);
}

#[test]
fn findfont_matches_matplotlib() {
    let expected = fixture("findfont.json");
    let db = FontDatabase::with_bundled_fonts();
    let mut failures = Vec::new();
    let queries = expected.get("queries").as_array();
    for query in queries {
        let families: Vec<String> = query
            .get("families")
            .as_array()
            .iter()
            .map(|f| f.as_str().to_owned())
            .collect();
        let request = FontQuery {
            families,
            style: FontStyle::parse(query.get("style").as_str()).unwrap(),
            variant: if query.get("variant").as_str() == "small-caps" {
                FontVariant::SmallCaps
            } else {
                FontVariant::Normal
            },
            weight: Weight::parse(query.get("weight").as_str()).unwrap(),
            stretch: Stretch::parse(query.get("stretch").as_str()).unwrap(),
            size: 10.0,
        };
        let found = file_of(&db, db.find_font(&request).unwrap());
        let ties: Vec<&str> = query
            .get("ties")
            .as_array()
            .iter()
            .map(Json::as_str)
            .collect();
        let ok = if ties.len() > 1 {
            ties.contains(&found.as_str())
        } else {
            found == query.get("file").as_str()
        };
        if !ok {
            failures.push(format!(
                "{request}: {found} != {}",
                query.get("file").as_str()
            ));
        }
    }
    assert!(queries.len() > 500);
    assert!(
        failures.is_empty(),
        "{} of {} queries differ:\n{}",
        failures.len(),
        queries.len(),
        failures.join("\n")
    );
}
